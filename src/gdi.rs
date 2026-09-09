//! Owned GDI objects are always deselected before deletion. Stock objects are borrowed.
use crate::{
    error::AppError,
    font::FontSpec,
    layout::{buffer_bytes, Rect},
    utf16,
};
use std::ptr;
use windows_sys::Win32::Foundation::{POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::*;

pub(crate) fn require(ok: bool, operation: &'static str) -> Result<(), AppError> {
    if ok {
        Ok(())
    } else {
        Err(AppError::OperationFailed(operation))
    }
}
fn valid(object: HGDIOBJ) -> bool {
    !object.is_null() && object as isize != -1
}

pub(crate) struct Object(pub HGDIOBJ);
impl Object {
    pub fn new(object: HGDIOBJ, operation: &'static str) -> Result<Self, AppError> {
        require(valid(object), operation)?;
        Ok(Self(object))
    }
}
impl Drop for Object {
    fn drop(&mut self) {
        // SAFETY: Only owned, deselected objects enter this wrapper.
        unsafe {
            DeleteObject(self.0);
        }
    }
}

pub(crate) struct Selection {
    dc: HDC,
    old: HGDIOBJ,
}
impl Selection {
    pub fn new(dc: HDC, object: HGDIOBJ) -> Result<Self, AppError> {
        // SAFETY: Internal callers provide a live DC and a live compatible object.
        let old = unsafe { SelectObject(dc, object) };
        require(valid(old), "SelectObject")?;
        Ok(Self { dc, old })
    }
}
impl Drop for Selection {
    fn drop(&mut self) {
        // SAFETY: Restore while DC and the borrowed previous object are still live.
        unsafe {
            SelectObject(self.dc, self.old);
        }
    }
}

pub(crate) struct SavedDc {
    dc: HDC,
    level: i32,
}
impl SavedDc {
    pub fn new(dc: HDC) -> Result<Self, AppError> {
        // SAFETY: dc is owned/borrowed by the active render call.
        let level = unsafe { SaveDC(dc) };
        require(level != 0, "SaveDC")?;
        Ok(Self { dc, level })
    }
}
impl Drop for SavedDc {
    fn drop(&mut self) {
        // SAFETY: The matching saved level belongs to this still-live DC.
        unsafe {
            RestoreDC(self.dc, self.level);
        }
    }
}

pub(crate) struct Buffer {
    pub dc: HDC,
    pub bitmap: HBITMAP,
    old: HGDIOBJ,
    pub width: i32,
    pub height: i32,
}
impl Buffer {
    #[cfg(test)]
    pub fn pixels(&self, screen: HDC) -> Result<Vec<u8>, AppError> {
        // GetDIBits requires the bitmap not to be selected into any DC.
        let _unselected = Selection::new(self.dc, self.old)?;
        let mut info = BITMAPINFO::default();
        info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = self.width;
        info.bmiHeader.biHeight = -self.height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut pixels = vec![
            0u8;
            buffer_bytes(self.width, self.height)
                .ok_or(AppError::OperationFailed("fixture bitmap size"))?
        ];
        // SAFETY: Header and output allocation describe the same 32-bpp top-down bitmap.
        require(
            unsafe {
                GetDIBits(
                    screen,
                    self.bitmap,
                    0,
                    self.height as u32,
                    pixels.as_mut_ptr().cast(),
                    &mut info,
                    DIB_RGB_COLORS,
                )
            } == self.height,
            "GetDIBits",
        )?;
        Ok(pixels)
    }
    pub fn new(screen: HDC, width: i32, height: i32) -> Result<Self, AppError> {
        require(
            buffer_bytes(width, height).is_some_and(|n| n > 0),
            "bitmap size limit",
        )?;
        // SAFETY: screen is a real paint/display DC. Do not derive bitmap depth
        // from the new memory DC's initial monochrome stock bitmap.
        unsafe {
            let dc = CreateCompatibleDC(screen);
            require(!dc.is_null(), "CreateCompatibleDC")?;
            let bitmap = CreateCompatibleBitmap(screen, width, height);
            if bitmap.is_null() {
                DeleteDC(dc);
                return Err(AppError::OperationFailed("CreateCompatibleBitmap"));
            }
            let old = SelectObject(dc, bitmap);
            if !valid(old) {
                DeleteObject(bitmap);
                DeleteDC(dc);
                return Err(AppError::OperationFailed("SelectObject(bitmap)"));
            }
            Ok(Self {
                dc,
                bitmap,
                old,
                width,
                height,
            })
        }
    }
}
impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: All temporary selections/saved states have been restored first.
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

pub(crate) use crate::model::FontMode;

pub(crate) struct Font {
    object: Option<Object>,
    stock: HFONT,
}
impl Font {
    pub fn handle(&self) -> HFONT {
        self.object.as_ref().map_or(self.stock, |font| font.0)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn fit(
        dc: HDC,
        mode: FontMode,
        custom: Option<FontSpec>,
        sample: &str,
        max_width: f64,
        max_height: f64,
        desired: f64,
        cjk: bool,
    ) -> Result<Self, AppError> {
        let mut height = desired.min(max_height).floor().max(1.0) as i32;
        let preferred = match mode {
            FontMode::MingLiu => "PMingLiU",
            _ => "Consolas",
        };
        let candidates = if cjk {
            [preferred, "Microsoft JhengHei", "PMingLiU"]
        } else {
            [preferred, "Consolas", "Microsoft JhengHei"]
        };
        for _ in 0..32 {
            let font = Self::create(dc, mode, custom, &candidates, height, sample)?;
            let _selected = Selection::new(dc, font.handle())?;
            let size = measure(dc, sample)?;
            if (f64::from(size.cx) <= max_width && f64::from(size.cy) <= max_height) || height == 1
            {
                return Ok(font);
            }
            let ratio =
                (max_width / f64::from(size.cx.max(1))).min(max_height / f64::from(size.cy.max(1)));
            height = ((f64::from(height) * ratio * 0.96).floor() as i32).clamp(1, height - 1);
        }
        Err(AppError::OperationFailed("font fit convergence"))
    }
    fn create(
        dc: HDC,
        mode: FontMode,
        custom: Option<FontSpec>,
        candidates: &[&str],
        height: i32,
        sample: &str,
    ) -> Result<Self, AppError> {
        let probe: Vec<u16> = sample.encode_utf16().collect();
        if mode == FontMode::Custom {
            if let Some(custom) = custom {
                let logfont = custom.logfont(height);
                // SAFETY: The validated LOGFONTW is initialized and live for this call.
                let handle = unsafe { CreateFontIndirectW(&logfont) };
                if !handle.is_null() {
                    let font = Object(handle);
                    if supports(dc, handle, &probe)? {
                        return Ok(Self {
                            object: Some(font),
                            stock: ptr::null_mut(),
                        });
                    }
                }
            }
        }
        for face in candidates {
            let wide =
                utf16::nul_terminated(face).map_err(|_| AppError::OperationFailed("font face"))?;
            // SAFETY: face is terminated and live; numeric properties are fixed,
            // horizontal screen-font defaults. Only lfHeight scales with pixels.
            let handle = unsafe {
                CreateFontW(
                    -height,
                    0,
                    0,
                    0,
                    400,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    ANTIALIASED_QUALITY as u32,
                    DEFAULT_PITCH as u32,
                    wide.as_ptr(),
                )
            };
            if handle.is_null() {
                continue;
            }
            let font = Object(handle);
            if supports(dc, handle, &probe)? {
                return Ok(Self {
                    object: Some(font),
                    stock: ptr::null_mut(),
                });
            }
        }
        // SAFETY: This is a borrowed system font. Never pass it to DeleteObject.
        let stock = unsafe { GetStockObject(DEFAULT_GUI_FONT) };
        require(valid(stock), "DEFAULT_GUI_FONT")?;
        Ok(Self {
            object: None,
            stock,
        })
    }
}

fn supports(dc: HDC, handle: HFONT, probe: &[u16]) -> Result<bool, AppError> {
    let _selected = Selection::new(dc, handle)?;
    let mut glyphs = vec![0u16; probe.len()];
    // SAFETY: Both buffers are sized in UTF-16 units. Missing glyphs are marked
    // explicitly; a non-null HFONT alone is insufficient.
    let count = unsafe {
        GetGlyphIndicesW(
            dc,
            probe.as_ptr(),
            probe.len() as i32,
            glyphs.as_mut_ptr(),
            GGI_MARK_NONEXISTING_GLYPHS,
        )
    };
    Ok(count != GDI_ERROR as u32 && !glyphs.contains(&0xffff))
}

pub(crate) fn measure(dc: HDC, text: &str) -> Result<SIZE, AppError> {
    let text: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    // SAFETY: Explicit character count matches the live UTF-16 slice.
    require(
        unsafe { GetTextExtentPoint32W(dc, text.as_ptr(), text.len() as i32, &mut size) } != 0,
        "GetTextExtentPoint32W",
    )?;
    Ok(size)
}

#[derive(Default)]
pub(crate) struct Pens {
    entries: Vec<(u32, i32, Object)>,
}
impl Pens {
    fn get(&mut self, color: u32, width: i32) -> Result<HPEN, AppError> {
        let width = width.max(1);
        if let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.0 == color && entry.1 == width)
        {
            return Ok(entry.2 .0);
        }
        // Previous drawing selections have ended before eviction. Capacity is
        // independent of dates, seconds, frames, and the duration of the process.
        if self.entries.len() == 32 {
            self.entries.remove(0);
        }
        // SAFETY: Width is positive and color is a COLORREF.
        let pen = Object::new(unsafe { CreatePen(PS_SOLID, width, color) }, "CreatePen")?;
        let handle = pen.0;
        self.entries.push((color, width, pen));
        Ok(handle)
    }
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

pub(crate) struct Canvas<'a> {
    pub dc: HDC,
    pub pens: &'a mut Pens,
}
pub(crate) fn px(value: f64) -> i32 {
    value.round() as i32
}
pub(crate) fn win_rect(rect: Rect) -> RECT {
    RECT {
        left: px(rect.x),
        top: px(rect.y),
        right: px(rect.right()),
        bottom: px(rect.bottom()),
    }
}
impl Canvas<'_> {
    pub fn fill(&mut self, rect: Rect, color: u32) -> Result<(), AppError> {
        let rect = win_rect(rect);
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return Ok(());
        }
        // SAFETY: DC_BRUSH is borrowed and its color belongs to this DC only.
        unsafe {
            SetDCBrushColor(self.dc, color);
            require(
                FillRect(self.dc, &rect, GetStockObject(DC_BRUSH)) != 0,
                "FillRect",
            )
        }
    }
    pub fn gradient(
        &mut self,
        rect: Rect,
        start: u32,
        end: u32,
        vertical: bool,
    ) -> Result<(), AppError> {
        let rect = win_rect(rect);
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return Ok(());
        }
        let vertex = |x, y, color: u32| TRIVERTEX {
            x,
            y,
            Red: ((color & 0xff) as u16) << 8,
            Green: (((color >> 8) & 0xff) as u16) << 8,
            Blue: (((color >> 16) & 0xff) as u16) << 8,
            Alpha: 0,
        };
        let vertices = [
            vertex(rect.left, rect.top, start),
            vertex(rect.right, rect.bottom, end),
        ];
        let mesh = GRADIENT_RECT {
            UpperLeft: 0,
            LowerRight: 1,
        };
        // SAFETY: Both vertices and the single rectangular mesh remain live for
        // the synchronous call. Coordinates are finite device pixels.
        require(
            unsafe {
                GradientFill(
                    self.dc,
                    vertices.as_ptr(),
                    vertices.len() as u32,
                    std::ptr::from_ref(&mesh).cast(),
                    1,
                    if vertical {
                        GRADIENT_FILL_RECT_V
                    } else {
                        GRADIENT_FILL_RECT_H
                    },
                )
            } != 0,
            "GradientFill",
        )
    }
    pub fn line(&mut self, points: &[(f64, f64)], width: f64, color: u32) -> Result<(), AppError> {
        let points: Vec<POINT> = points
            .iter()
            .map(|&(x, y)| POINT { x: px(x), y: px(y) })
            .collect();
        let pen = self.pens.get(color, px(width).max(1))?;
        let _selected = Selection::new(self.dc, pen)?;
        // SAFETY: The point count matches the live array; pen is selected until return.
        require(
            unsafe { Polyline(self.dc, points.as_ptr(), points.len() as i32) } != 0,
            "Polyline",
        )
    }
    pub fn polygon(
        &mut self,
        points: &[(f64, f64)],
        color: u32,
        outline: Option<(u32, f64)>,
    ) -> Result<(), AppError> {
        let points: Vec<POINT> = points
            .iter()
            .map(|&(x, y)| POINT { x: px(x), y: px(y) })
            .collect();
        // SAFETY: Stock brushes/pens are borrowed. Each selection restores before eviction.
        let (brush, null_pen) = unsafe { (GetStockObject(DC_BRUSH), GetStockObject(NULL_PEN)) };
        let pen = match outline {
            Some((color, width)) => self.pens.get(color, px(width).max(1))?,
            None => null_pen,
        };
        let _brush = Selection::new(self.dc, brush)?;
        let _pen = Selection::new(self.dc, pen)?;
        // SAFETY: Polygon copies its input synchronously; DC_BRUSH color is per DC.
        unsafe {
            SetDCBrushColor(self.dc, color);
            require(
                Polygon(self.dc, points.as_ptr(), points.len() as i32) != 0,
                "Polygon",
            )
        }
    }
    pub fn ellipse(&mut self, rect: Rect, color: u32) -> Result<(), AppError> {
        let r = win_rect(rect);
        if r.right <= r.left || r.bottom <= r.top {
            return Ok(());
        }
        // SAFETY: Borrow system objects and scope their selections.
        let _brush = Selection::new(self.dc, unsafe { GetStockObject(DC_BRUSH) })?;
        // SAFETY: NULL_PEN is a borrowed stock object.
        let _pen = Selection::new(self.dc, unsafe { GetStockObject(NULL_PEN) })?;
        // SAFETY: Rectangle is finite and nonempty; color belongs to this DC.
        unsafe {
            SetDCBrushColor(self.dc, color);
            require(
                Ellipse(self.dc, r.left, r.top, r.right, r.bottom) != 0,
                "Ellipse",
            )
        }
    }
    pub fn rounded(
        &mut self,
        rect: Rect,
        color: Option<u32>,
        border: u32,
        width: f64,
        radius: f64,
    ) -> Result<(), AppError> {
        let r = win_rect(rect);
        if r.right <= r.left || r.bottom <= r.top {
            return Ok(());
        }
        let pen = self.pens.get(border, px(width).max(1))?;
        let _pen = Selection::new(self.dc, pen)?;
        // SAFETY: Stock brush is borrowed; no deletion or ownership transfer.
        let brush = unsafe {
            GetStockObject(if color.is_some() {
                DC_BRUSH
            } else {
                NULL_BRUSH
            })
        };
        let _brush = Selection::new(self.dc, brush)?;
        // SAFETY: Coordinates fit the validated canvas; selected objects stay live.
        unsafe {
            if let Some(color) = color {
                SetDCBrushColor(self.dc, color);
            }
            require(
                RoundRect(
                    self.dc,
                    r.left,
                    r.top,
                    r.right,
                    r.bottom,
                    px(radius).max(1),
                    px(radius).max(1),
                ) != 0,
                "RoundRect",
            )
        }
    }
    pub fn text(
        &mut self,
        font: &Font,
        value: &str,
        rect: Rect,
        color: u32,
        shadow: bool,
    ) -> Result<(), AppError> {
        let _font = Selection::new(self.dc, font.handle())?;
        let size = measure(self.dc, value)?;
        let wide: Vec<u16> = value.encode_utf16().collect();
        let (x, y) = (
            px(rect.cx() - f64::from(size.cx) / 2.0),
            px(rect.cy() - f64::from(size.cy) / 2.0),
        );
        // SAFETY: Text slices are live for each synchronous call. All text draws
        // occur inside the saved scene clipping rectangle.
        unsafe {
            SetBkMode(self.dc, TRANSPARENT as i32);
            if shadow {
                SetTextColor(self.dc, rgb(32, 36, 32));
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    require(
                        TextOutW(self.dc, x + dx, y + dy, wide.as_ptr(), wide.len() as i32) != 0,
                        "TextOutW(shadow)",
                    )?;
                }
            }
            SetTextColor(self.dc, color);
            require(
                TextOutW(self.dc, x, y, wide.as_ptr(), wide.len() as i32) != 0,
                "TextOutW",
            )
        }
    }
    pub fn clip_polygon(&mut self, points: &[(f64, f64)]) -> Result<SavedDc, AppError> {
        let saved = SavedDc::new(self.dc)?;
        // SAFETY: Path creation uses this live DC; SelectClipPath consumes the
        // path and clips in the current logical-to-device coordinate mapping.
        unsafe {
            require(BeginPath(self.dc) != 0, "BeginPath")?;
            let mut success = true;
            for (i, &(x, y)) in points.iter().enumerate() {
                success &= if i == 0 {
                    MoveToEx(self.dc, px(x), px(y), ptr::null_mut()) != 0
                } else {
                    LineTo(self.dc, px(x), px(y)) != 0
                };
            }
            success &= CloseFigure(self.dc) != 0;
            success &= EndPath(self.dc) != 0;
            if !success {
                AbortPath(self.dc);
                return Err(AppError::OperationFailed("glass clip path"));
            }
            require(SelectClipPath(self.dc, RGN_AND) != 0, "SelectClipPath")?;
        }
        Ok(saved)
    }
    pub fn clip_rounded(&mut self, rect: Rect, radius: f64) -> Result<SavedDc, AppError> {
        let saved = SavedDc::new(self.dc)?;
        let rect = win_rect(rect);
        if rect.right - rect.left <= 1 || rect.bottom - rect.top <= 1 {
            return Ok(saved);
        }
        let diameter = px(radius * 2.0).max(1);
        // SAFETY: The path is constructed synchronously on the live DC and is
        // consumed by SelectClipPath before this function returns.
        unsafe {
            require(BeginPath(self.dc) != 0, "BeginPath")?;
            if RoundRect(
                self.dc,
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                diameter,
                diameter,
            ) == 0
                || EndPath(self.dc) == 0
            {
                AbortPath(self.dc);
                return Err(AppError::OperationFailed("rounded clip path"));
            }
            require(SelectClipPath(self.dc, RGN_AND) != 0, "SelectClipPath")?;
        }
        Ok(saved)
    }
}

pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}
pub(crate) fn dim(color: u32, scale: f64) -> u32 {
    rgb(
        ((color & 255) as f64 * scale) as u8,
        (((color >> 8) & 255) as f64 * scale) as u8,
        (((color >> 16) & 255) as f64 * scale) as u8,
    )
}
