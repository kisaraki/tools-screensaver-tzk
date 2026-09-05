//! Validated custom-font data independent of GDI object lifetime management.
use std::{mem::size_of, ptr};

use windows_sys::Win32::Graphics::Gdi::{
    ANSI_CHARSET, ARABIC_CHARSET, BALTIC_CHARSET, CHINESEBIG5_CHARSET, DEFAULT_CHARSET,
    EASTEUROPE_CHARSET, GB2312_CHARSET, GREEK_CHARSET, HANGUL_CHARSET, HEBREW_CHARSET,
    JOHAB_CHARSET, LOGFONTW, MAC_CHARSET, OEM_CHARSET, RUSSIAN_CHARSET, SHIFTJIS_CHARSET,
    SYMBOL_CHARSET, THAI_CHARSET, TURKISH_CHARSET, VIETNAMESE_CHARSET,
};

pub const DEFAULT_POINT_SIZE_TENTH: u32 = 480;

const _: [(); 92] = [(); size_of::<LOGFONTW>()];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontSpec {
    face: [u16; 32],
    weight: i32,
    italic: u8,
    charset: u8,
    out_precision: u8,
    clip_precision: u8,
    quality: u8,
    pitch_and_family: u8,
    point_size_tenth: u32,
}

impl FontSpec {
    pub fn from_logfont(mut value: LOGFONTW, point_size_tenth: u32) -> Option<Self> {
        if !(180..=2400).contains(&point_size_tenth)
            || value.lfHeight == i32::MIN
            || !(0..=1000).contains(&value.lfWeight)
            || value.lfItalic > 1
            || value.lfUnderline > 1
            || value.lfStrikeOut > 1
        {
            return None;
        }
        if !valid_charset(value.lfCharSet) {
            value.lfCharSet = DEFAULT_CHARSET;
        }
        if !matches!(value.lfOutPrecision, 0..=10) {
            value.lfOutPrecision = 0;
        }
        if !valid_clip_precision(value.lfClipPrecision) {
            value.lfClipPrecision = 0;
        }
        if !matches!(value.lfQuality, 0..=6) {
            value.lfQuality = 0;
        }
        if !valid_pitch_family(value.lfPitchAndFamily) {
            value.lfPitchAndFamily = 0;
        }
        let nul = value.lfFaceName.iter().position(|&unit| unit == 0)?;
        if nul == 0 || String::from_utf16(&value.lfFaceName[..nul]).is_err() {
            return None;
        }
        value.lfFaceName[nul..].fill(0);
        Some(Self {
            face: value.lfFaceName,
            weight: value.lfWeight,
            italic: value.lfItalic,
            charset: value.lfCharSet,
            out_precision: value.lfOutPrecision,
            clip_precision: value.lfClipPrecision,
            quality: value.lfQuality,
            pitch_and_family: value.lfPitchAndFamily,
            point_size_tenth,
        })
    }

    pub fn from_registry(bytes: &[u8], point_size_tenth: u32) -> Option<Self> {
        if bytes.len() != size_of::<LOGFONTW>() {
            return None;
        }
        let mut value = LOGFONTW::default();
        // SAFETY: The destination is aligned and initialized; the exact-size byte
        // slice is copied into it before any field is read.
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (&mut value as *mut LOGFONTW).cast::<u8>(),
                bytes.len(),
            );
        }
        Self::from_logfont(value, point_size_tenth)
    }

    pub fn default_choice() -> Self {
        let mut face = [0; 32];
        for (target, source) in face.iter_mut().zip("Consolas".encode_utf16().chain([0])) {
            *target = source;
        }
        Self {
            face,
            weight: 400,
            italic: 0,
            charset: DEFAULT_CHARSET,
            out_precision: 0,
            clip_precision: 0,
            quality: 0,
            pitch_and_family: 0,
            point_size_tenth: DEFAULT_POINT_SIZE_TENTH,
        }
    }

    pub fn face(&self) -> &[u16; 32] {
        &self.face
    }

    pub fn point_size_tenth(self) -> u32 {
        self.point_size_tenth
    }

    pub fn logfont(self, pixel_height: i32) -> LOGFONTW {
        LOGFONTW {
            lfHeight: -pixel_height.max(1),
            lfWidth: 0,
            lfEscapement: 0,
            lfOrientation: 0,
            lfWeight: self.weight,
            lfItalic: self.italic,
            lfUnderline: 0,
            lfStrikeOut: 0,
            lfCharSet: self.charset,
            lfOutPrecision: self.out_precision,
            lfClipPrecision: self.clip_precision,
            lfQuality: self.quality,
            lfPitchAndFamily: self.pitch_and_family,
            lfFaceName: self.face,
        }
    }

    pub(crate) fn registry_bytes(self) -> Vec<u8> {
        let value = self.logfont(1);
        // Preserve a complete ABI LOGFONTW while ensuring fields unused by the
        // renderer are normalized and cannot contain untrusted dimensions.
        unsafe {
            std::slice::from_raw_parts(
                (&value as *const LOGFONTW).cast::<u8>(),
                size_of::<LOGFONTW>(),
            )
            .to_vec()
        }
    }
}

fn valid_charset(value: u8) -> bool {
    [
        ANSI_CHARSET,
        DEFAULT_CHARSET,
        SYMBOL_CHARSET,
        MAC_CHARSET,
        SHIFTJIS_CHARSET,
        HANGUL_CHARSET,
        JOHAB_CHARSET,
        GB2312_CHARSET,
        CHINESEBIG5_CHARSET,
        GREEK_CHARSET,
        TURKISH_CHARSET,
        VIETNAMESE_CHARSET,
        HEBREW_CHARSET,
        ARABIC_CHARSET,
        BALTIC_CHARSET,
        RUSSIAN_CHARSET,
        THAI_CHARSET,
        EASTEUROPE_CHARSET,
        OEM_CHARSET,
    ]
    .contains(&value)
}

fn valid_clip_precision(value: u8) -> bool {
    matches!(value & 0x0f, 0..=2)
}

fn valid_pitch_family(value: u8) -> bool {
    matches!(value & 0x0f, 0..=2) && matches!(value & 0xf0, 0x00 | 0x10 | 0x20 | 0x30 | 0x40 | 0x50)
}
