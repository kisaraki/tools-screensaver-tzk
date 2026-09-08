//! Embedded travel surroundings shared by the local player and native previews.
//! Only decoded pixels are cached; no COM interface, file or network handle survives
//! decoding, and no WebView2 Runtime is needed to display the artwork.

use crate::{
    error::AppError,
    gdi::{px, require, SavedDc},
    layout::Rect,
    model::TravelStyle,
};
use std::{ptr, sync::OnceLock};
use windows::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    Graphics::Imaging::{
        CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGR, IWICImagingFactory,
        WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnLoad,
    },
    System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    },
};
use windows_sys::Win32::Graphics::Gdi::{
    SetBrushOrgEx, SetStretchBltMode, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, GDI_ERROR, HALFTONE, HDC, SRCCOPY,
};

const FREE_FLIGHT: &[u8] = include_bytes!("../assets/travel/free-flight.png");
const TRAIN_JOURNEY: &[u8] = include_bytes!("../assets/travel/train-journey.png");
const JAPANESE_INN: &[u8] = include_bytes!("../assets/travel/japanese-inn.png");
const TRAIN_CAB: &[u8] = include_bytes!("../assets/travel/train-cab.png");
const WALKING: &[u8] = include_bytes!("../assets/travel/walking.png");
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const MAX_ENCODED_BYTES: usize = 16 * 1024 * 1024;
const MAX_DIMENSION: u32 = 4096;
const MAX_PIXEL_BYTES: usize = 64 * 1024 * 1024;

pub fn png(style: TravelStyle) -> &'static [u8] {
    match style {
        TravelStyle::FreeFlight => FREE_FLIGHT,
        TravelStyle::TrainJourney => TRAIN_JOURNEY,
        TravelStyle::JapaneseInn => JAPANESE_INN,
        TravelStyle::TrainCab => TRAIN_CAB,
        TravelStyle::Walking => WALKING,
    }
}

pub fn file_name(style: TravelStyle) -> &'static str {
    match style {
        TravelStyle::FreeFlight => "free-flight.png",
        TravelStyle::TrainJourney => "train-journey.png",
        TravelStyle::JapaneseInn => "japanese-inn.png",
        TravelStyle::TrainCab => "train-cab.png",
        TravelStyle::Walking => "walking.png",
    }
}

struct ComApartment {
    uninitialize: bool,
}

impl ComApartment {
    fn initialize() -> Result<Self, AppError> {
        // SAFETY: Called on the current rendering thread. S_OK and S_FALSE both
        // acquire an initialization count; a pre-existing MTA remains untouched.
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if result.is_ok() {
            Ok(Self { uninitialize: true })
        } else if result == RPC_E_CHANGED_MODE {
            Ok(Self {
                uninitialize: false,
            })
        } else {
            Err(AppError::OperationFailed("CoInitializeEx(travel artwork)"))
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.uninitialize {
            // SAFETY: This guard owns exactly one successful initialization on
            // this thread and outlives every WIC interface created below.
            unsafe { CoUninitialize() };
        }
    }
}

struct Pixels {
    width: i32,
    height: i32,
    bytes: Vec<u8>,
}

static FLIGHT_PIXELS: OnceLock<Result<Pixels, AppError>> = OnceLock::new();
static TRAIN_PIXELS: OnceLock<Result<Pixels, AppError>> = OnceLock::new();
static INN_PIXELS: OnceLock<Result<Pixels, AppError>> = OnceLock::new();
static CAB_PIXELS: OnceLock<Result<Pixels, AppError>> = OnceLock::new();
static WALKING_PIXELS: OnceLock<Result<Pixels, AppError>> = OnceLock::new();

fn cached_pixels(style: TravelStyle) -> Result<&'static Pixels, AppError> {
    let cache = match style {
        TravelStyle::FreeFlight => &FLIGHT_PIXELS,
        TravelStyle::TrainJourney => &TRAIN_PIXELS,
        TravelStyle::JapaneseInn => &INN_PIXELS,
        TravelStyle::TrainCab => &CAB_PIXELS,
        TravelStyle::Walking => &WALKING_PIXELS,
    };
    cache
        .get_or_init(|| decode(png(style)))
        .as_ref()
        .map_err(|error| *error)
}

fn decode(encoded: &[u8]) -> Result<Pixels, AppError> {
    if !encoded.starts_with(PNG_SIGNATURE) || encoded.len() > MAX_ENCODED_BYTES {
        return Err(AppError::InvalidResource("travel artwork PNG"));
    }
    // The bindings convert both slice lengths to u32 before calling WIC.
    u32::try_from(encoded.len()).map_err(|_| AppError::InvalidResource("travel PNG size"))?;
    let _apartment = ComApartment::initialize()?;
    // Declare the backing allocation before the stream/decoder so those COM
    // objects always drop first, including every early-error return.
    let encoded = encoded.to_vec();
    // SAFETY: COM is initialized on this thread and all interfaces remain local.
    let factory: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
            .map_err(|_| AppError::OperationFailed("WIC imaging factory"))?;
    // SAFETY: The factory is live. The stream borrows a bounded, stable allocation
    // that outlives the decoder and is used only for reading an embedded PNG.
    let stream = unsafe { factory.CreateStream() }
        .map_err(|_| AppError::OperationFailed("WIC memory stream"))?;
    unsafe { stream.InitializeFromMemory(&encoded) }
        .map_err(|_| AppError::OperationFailed("WIC PNG memory input"))?;
    // SAFETY: The stream and its backing allocation stay alive until all decoder
    // and frame interfaces have been released; a null vendor selects Windows WIC.
    let decoder = unsafe {
        factory.CreateDecoderFromStream(&stream, ptr::null(), WICDecodeMetadataCacheOnLoad)
    }
    .map_err(|_| AppError::OperationFailed("WIC PNG decoder"))?;
    // SAFETY: Read the first image from this live decoder. Output pointers refer
    // to initialized local integers for the duration of GetSize.
    let frame =
        unsafe { decoder.GetFrame(0) }.map_err(|_| AppError::OperationFailed("WIC PNG frame"))?;
    let (mut width, mut height) = (0u32, 0u32);
    unsafe { frame.GetSize(&mut width, &mut height) }
        .map_err(|_| AppError::OperationFailed("WIC PNG dimensions"))?;
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(AppError::InvalidResource("travel artwork dimensions"));
    }
    let stride = width
        .checked_mul(4)
        .ok_or(AppError::InvalidResource("travel artwork stride"))?;
    let byte_count = stride
        .checked_mul(height)
        .ok_or(AppError::InvalidResource("travel artwork pixel size"))?;
    let byte_count = usize::try_from(byte_count)
        .map_err(|_| AppError::InvalidResource("travel artwork pixel size"))?;
    if byte_count > MAX_PIXEL_BYTES {
        return Err(AppError::InvalidResource("travel artwork pixel limit"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(byte_count)
        .map_err(|_| AppError::OperationFailed("travel artwork pixel allocation"))?;
    bytes.resize(byte_count, 0);
    // SAFETY: The converter and source are in the same apartment; 32-bpp BGR
    // matches a BI_RGB DIB. Artwork is opaque and does not need alpha blending.
    let converter = unsafe { factory.CreateFormatConverter() }
        .map_err(|_| AppError::OperationFailed("WIC pixel converter"))?;
    unsafe {
        converter.Initialize(
            &frame,
            &GUID_WICPixelFormat32bppBGR,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )
    }
    .map_err(|_| AppError::OperationFailed("WIC BGR conversion"))?;
    // SAFETY: A null rectangle copies the whole bounded image. The stride and
    // allocation cover every 32-bpp row; the byte length was checked as u32 above.
    unsafe { converter.CopyPixels(ptr::null(), stride, &mut bytes) }
        .map_err(|_| AppError::OperationFailed("WIC decoded pixels"))?;
    Ok(Pixels {
        width: i32::try_from(width)
            .map_err(|_| AppError::InvalidResource("travel artwork width"))?,
        height: i32::try_from(height)
            .map_err(|_| AppError::InvalidResource("travel artwork height"))?,
        bytes,
    })
}

pub fn draw(dc: HDC, rect: Rect, style: TravelStyle) -> Result<(), AppError> {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return Ok(());
    }
    for value in [rect.x, rect.y, rect.w, rect.h, rect.right(), rect.bottom()] {
        if !value.is_finite() || value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
            return Err(AppError::OperationFailed("travel artwork destination"));
        }
    }
    let (x, y) = (px(rect.x), px(rect.y));
    let width = px(rect.right())
        .checked_sub(x)
        .ok_or(AppError::OperationFailed(
            "travel artwork destination width",
        ))?;
    let height = px(rect.bottom())
        .checked_sub(y)
        .ok_or(AppError::OperationFailed(
            "travel artwork destination height",
        ))?;
    if width == 0 || height == 0 {
        return Ok(());
    }
    let pixels = cached_pixels(style)?;
    let mut info = BITMAPINFO::default();
    info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = pixels.width;
    info.bmiHeader.biHeight = -pixels.height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    info.bmiHeader.biSizeImage = u32::try_from(pixels.bytes.len())
        .map_err(|_| AppError::InvalidResource("travel artwork DIB size"))?;
    let _saved = SavedDc::new(dc)?;
    // SAFETY: The caller supplies a live rendering DC. SaveDC restores the
    // stretch mode and brush origin; the bounded top-down pixels and header stay
    // alive throughout the synchronous copy. HALFTONE improves reduced previews.
    unsafe {
        require(
            SetStretchBltMode(dc, HALFTONE) != 0,
            "travel artwork stretch mode",
        )?;
        require(
            SetBrushOrgEx(dc, 0, 0, ptr::null_mut()) != 0,
            "travel artwork brush origin",
        )?;
        let lines = StretchDIBits(
            dc,
            x,
            y,
            width,
            height,
            0,
            0,
            pixels.width,
            pixels.height,
            pixels.bytes.as_ptr().cast(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        require(
            lines != 0 && lines != GDI_ERROR,
            "travel artwork StretchDIBits",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Com::{
        CoGetApartmentType, APTTYPEQUALIFIER_NONE, APTTYPE_MTA, COINIT_MULTITHREADED,
    };

    #[test]
    fn embedded_artwork_decodes_and_preserves_existing_com_apartments() {
        for style in [
            TravelStyle::FreeFlight,
            TravelStyle::TrainJourney,
            TravelStyle::JapaneseInn,
            TravelStyle::TrainCab,
            TravelStyle::Walking,
        ] {
            let pixels = cached_pixels(style).unwrap();
            assert_eq!((pixels.width, pixels.height), (1586, 992));
            assert_eq!(pixels.bytes.len(), 1586 * 992 * 4);
            assert!(pixels
                .bytes
                .chunks_exact(4)
                .any(|pixel| pixel[..3] != [0, 0, 0]));
            assert!(ptr::eq(pixels, cached_pixels(style).unwrap()));
        }
        assert!(decode(b"not a PNG").is_err());
        std::thread::spawn(|| {
            // SAFETY: This fresh test thread owns the successful MTA count and
            // its guard outlives both independent decoder calls below.
            unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
                .ok()
                .unwrap();
            let _mta = ComApartment { uninitialize: true };
            for style in [
                TravelStyle::FreeFlight,
                TravelStyle::TrainJourney,
                TravelStyle::JapaneseInn,
                TravelStyle::TrainCab,
                TravelStyle::Walking,
            ] {
                assert_eq!(decode(png(style)).unwrap().bytes.len(), 1586 * 992 * 4);
            }
            let (mut apartment, mut qualifier) = (APTTYPE_MTA, APTTYPEQUALIFIER_NONE);
            // SAFETY: These output pointers reference initialized local values.
            unsafe { CoGetApartmentType(&mut apartment, &mut qualifier) }.unwrap();
            assert_eq!(apartment, APTTYPE_MTA);
        })
        .join()
        .unwrap();
    }
}
