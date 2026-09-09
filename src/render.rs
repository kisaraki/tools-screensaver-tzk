use crate::config::{AppConfig, ColorPreset, ConfigDraft};
use crate::error::AppError;
use crate::font::{FontSpec, DEFAULT_POINT_SIZE_TENTH};
use crate::gdi::{
    self, dim, require, rgb, Buffer, Canvas, Font, FontMode, Pens, SavedDc, Selection,
};
use crate::layout::{buffer_bytes, Detail, Layout, Rect};
use crate::model::{hms, Calendar, DisplayMode, FrameSnapshot, TravelStyle, SEGMENTS};
use crate::native::client_size;
use crate::travel::TravelCaption;
use std::ptr;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::*;

const AUTO_COLOR_INTERVAL_MS: u64 = 120_000;
const AUTO_COLORS: [ColorPreset; 7] = [
    ColorPreset::DarkRed,
    ColorPreset::DarkOrange,
    ColorPreset::BrightGreen,
    ColorPreset::OffWhite,
    ColorPreset::MutedLightBlue,
    ColorPreset::Amber,
    ColorPreset::IronGray,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Style {
    pub palette: ColorPreset,
    pub font: FontMode,
    pub custom: Option<FontSpec>,
    pub point_size_tenth: u32,
    pub travel_style: TravelStyle,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            palette: ColorPreset::BrightGreen,
            font: FontMode::SevenSegment,
            custom: None,
            point_size_tenth: DEFAULT_POINT_SIZE_TENTH,
            travel_style: TravelStyle::FreeFlight,
        }
    }
}
impl Style {
    pub fn from_config(config: AppConfig) -> Self {
        Self {
            palette: config.color_preset,
            font: config.effective_font_mode(),
            custom: config.custom_font,
            point_size_tenth: config
                .custom_font
                .map_or(DEFAULT_POINT_SIZE_TENTH, FontSpec::point_size_tenth),
            travel_style: config.travel_style,
        }
    }

    pub fn from_draft(draft: ConfigDraft) -> Self {
        let font = if draft.font_mode == FontMode::Custom && draft.custom_font.is_none() {
            FontMode::SevenSegment
        } else {
            draft.font_mode
        };
        Self {
            palette: draft.color_preset,
            font,
            custom: draft.custom_font,
            point_size_tenth: draft
                .custom_font
                .map_or(DEFAULT_POINT_SIZE_TENTH, FontSpec::point_size_tenth),
            travel_style: draft.travel_style,
        }
    }

    pub fn color(self) -> u32 {
        Self::preset_color(match self.palette {
            ColorPreset::Auto => ColorPreset::BrightGreen,
            preset => preset,
        })
    }

    fn color_at(self, tick: u64) -> u32 {
        let preset = if self.palette == ColorPreset::Auto {
            AUTO_COLORS[(tick / AUTO_COLOR_INTERVAL_MS) as usize % AUTO_COLORS.len()]
        } else {
            self.palette
        };
        Self::preset_color(preset)
    }

    fn preset_color(preset: ColorPreset) -> u32 {
        match preset {
            ColorPreset::DarkRed => rgb(139, 0, 0),
            ColorPreset::DarkOrange => rgb(255, 140, 0),
            ColorPreset::OffWhite => rgb(245, 245, 245),
            ColorPreset::BrightGreen => rgb(0, 255, 0),
            ColorPreset::MutedLightBlue => rgb(101, 151, 178),
            ColorPreset::Amber => rgb(255, 191, 0),
            ColorPreset::IronGray => rgb(154, 160, 163),
            ColorPreset::Auto => unreachable!("auto palette must be resolved"),
        }
    }
    fn outline(self) -> bool {
        matches!(
            self.palette,
            ColorPreset::BrightGreen | ColorPreset::OffWhite | ColorPreset::Auto
        )
    }
}

struct Fonts {
    clock: Font,
    month: Font,
    weekday: Font,
    day: Font,
    countdown: Font,
    day_height: f64,
}
impl Fonts {
    fn new(dc: HDC, layout: Layout, style: Style) -> Result<Self, AppError> {
        let cal = layout.calendar;
        let cell = (cal.w / 7.0).min(cal.h * 0.75 / 6.0);
        let point_scale = f64::from(style.point_size_tenth) / f64::from(DEFAULT_POINT_SIZE_TENTH);
        let number_box = clock_number_rect(layout.clock, 0.0);
        let clock = Font::fit(
            dc,
            style.font,
            style.custom,
            "12",
            number_box.w,
            number_box.h,
            number_box.h * 0.90 * point_scale,
            false,
        )?;
        let month = Font::fit(
            dc,
            style.font,
            style.custom,
            "8888年 88月",
            cal.w * 0.97,
            cal.h * 0.13,
            cal.h * 0.09 * point_scale,
            true,
        )?;
        let weekday = Font::fit(
            dc,
            style.font,
            style.custom,
            "日",
            cal.w / 7.0 * 0.85,
            cal.h * 0.10,
            cal.h * 0.065 * point_scale,
            true,
        )?;
        let day = Font::fit(
            dc,
            style.font,
            style.custom,
            "88",
            cell * 0.72,
            cell * 0.9 / 1.65,
            cal.h * 0.065 * point_scale,
            false,
        )?;
        let day_height = {
            let _font = Selection::new(dc, day.handle())?;
            f64::from(gdi::measure(dc, "88")?.cy)
        };
        let countdown = Font::fit(
            dc,
            style.font,
            style.custom,
            "88:88:88",
            layout.inner.w * 0.96,
            layout.inner.h * 0.96,
            layout.inner.h * 0.95 * point_scale,
            false,
        )?;
        Ok(Self {
            clock,
            month,
            weekday,
            day,
            countdown,
            day_height,
        })
    }
}

#[derive(Default)]
pub(crate) struct Renderer {
    buffer: Option<Buffer>,
    fonts: Option<Fonts>,
    pens: Pens,
    key: Option<(i32, i32, u32, DisplayMode, Style)>,
    direct_fallback: bool,
    travel_caption: TravelCaption,
}

struct Paint {
    hwnd: HWND,
    info: PAINTSTRUCT,
    dc: HDC,
}
impl Paint {
    fn begin(hwnd: HWND) -> Self {
        let mut info = PAINTSTRUCT::default();
        // SAFETY: Called while handling WM_PAINT; RAII pairs EndPaint on every path.
        let dc = unsafe { BeginPaint(hwnd, &mut info) };
        Self { hwnd, info, dc }
    }
}
impl Drop for Paint {
    fn drop(&mut self) {
        // SAFETY: Pair the original BeginPaint even if it returned NULL.
        unsafe {
            EndPaint(self.hwnd, &self.info);
        }
    }
}

impl Renderer {
    pub(crate) fn set_travel_caption(&mut self, caption: TravelCaption) {
        self.travel_caption = caption;
    }
    pub fn warm(
        &mut self,
        hwnd: HWND,
        dpi: u32,
        mode: DisplayMode,
        style: Style,
    ) -> Result<(), AppError> {
        let (width, height) = client_size(hwnd)?;
        self.prepare(width, height, dpi, mode, style)?;
        if width == 0 || height == 0 {
            return Ok(());
        }
        struct WindowDc(HWND, HDC);
        impl Drop for WindowDc {
            fn drop(&mut self) {
                // SAFETY: Restore the GetDC borrow while the creation callback owns the HWND.
                unsafe {
                    ReleaseDC(self.0, self.1);
                }
            }
        }
        // SAFETY: WM_CREATE supplies a live, already sized but not yet shown HWND.
        let dc = WindowDc(hwnd, unsafe { GetDC(hwnd) });
        require(!dc.1.is_null(), "GetDC(initial frame resources)")?;
        match Buffer::new(dc.1, width, height) {
            Ok(buffer) => self.buffer = Some(buffer),
            Err(_) => self.direct_fallback = true,
        }
        if let Some(layout) = Layout::new(width, height, mode) {
            if layout.detail != Detail::Tiny {
                let font_dc = self.buffer.as_ref().map_or(dc.1, |buffer| buffer.dc);
                self.fonts = Some(Fonts::new(font_dc, layout, style)?);
            }
        }
        Ok(())
    }
    fn prepare(
        &mut self,
        width: i32,
        height: i32,
        dpi: u32,
        mode: DisplayMode,
        style: Style,
    ) -> Result<(), AppError> {
        require(
            buffer_bytes(width, height).is_some(),
            "canvas dimensions / allocation limit",
        )?;
        let key = (width, height, dpi, mode, style);
        if self.key != Some(key) {
            self.buffer = None;
            self.fonts = None;
            self.pens = Pens::default();
            self.direct_fallback = false;
            self.key = Some(key);
        }
        Ok(())
    }
    pub fn paint(
        &mut self,
        hwnd: HWND,
        dpi: u32,
        mode: DisplayMode,
        frame: FrameSnapshot,
        style: Style,
        offset: (i32, i32),
    ) -> Result<(), AppError> {
        let paint = Paint::begin(hwnd);
        require(!paint.dc.is_null(), "BeginPaint")?;
        let (width, height) = client_size(hwnd)?;
        self.prepare(width, height, dpi, mode, style)?;
        if width == 0 || height == 0 {
            return Ok(());
        }
        if self.buffer.is_none() && !self.direct_fallback {
            match Buffer::new(paint.dc, width, height) {
                Ok(buffer) => self.buffer = Some(buffer),
                Err(_) => self.direct_fallback = true,
            }
        }
        // After a buffer failure, attempt direct painting once per frame without
        // retrying allocation until size/DPI/style changes. Essential GDI failures
        // propagate to the session, which closes the whole window group.
        let dc = self.buffer.as_ref().map_or(paint.dc, |buffer| buffer.dc);
        self.draw_scene(dc, width, height, dpi, mode, frame, style, (0, 0), offset)?;
        if let Some(buffer) = &self.buffer {
            // SAFETY: Both DCs are live and the bitmap covers the full client.
            require(
                unsafe {
                    BitBlt(
                        paint.dc,
                        0,
                        0,
                        buffer.width,
                        buffer.height,
                        buffer.dc,
                        0,
                        0,
                        SRCCOPY,
                    )
                } != 0,
                "BitBlt",
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_borrowed(
        &mut self,
        dc: HDC,
        rect: windows_sys::Win32::Foundation::RECT,
        dpi: u32,
        mode: DisplayMode,
        frame: FrameSnapshot,
        style: Style,
    ) -> Result<(), AppError> {
        require(!dc.is_null(), "owner-draw HDC")?;
        let width = rect.right.saturating_sub(rect.left);
        let height = rect.bottom.saturating_sub(rect.top);
        self.prepare(width, height, dpi, mode, style)?;
        if width == 0 || height == 0 {
            return Ok(());
        }
        if self.buffer.is_none() && !self.direct_fallback {
            match Buffer::new(dc, width, height) {
                Ok(buffer) => self.buffer = Some(buffer),
                Err(_) => self.direct_fallback = true,
            }
        }
        if let Some(buffer) = &self.buffer {
            let buffer_dc = buffer.dc;
            self.draw_scene(
                buffer_dc,
                width,
                height,
                dpi,
                mode,
                frame,
                style,
                (0, 0),
                (0, 0),
            )?;
            // SAFETY: The destination is a borrowed owner-draw DC and the source
            // bitmap exactly matches the control rectangle.
            require(
                unsafe {
                    BitBlt(
                        dc, rect.left, rect.top, width, height, buffer_dc, 0, 0, SRCCOPY,
                    )
                } != 0,
                "BitBlt(owner draw)",
            )?;
        } else {
            self.draw_scene(
                dc,
                width,
                height,
                dpi,
                mode,
                frame,
                style,
                (rect.left, rect.top),
                (0, 0),
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_scene(
        &mut self,
        dc: HDC,
        width: i32,
        height: i32,
        dpi: u32,
        mode: DisplayMode,
        frame: FrameSnapshot,
        style: Style,
        origin: (i32, i32),
        offset: (i32, i32),
    ) -> Result<(), AppError> {
        require(
            frame.local.valid() && frame.countdown.ratio.is_finite(),
            "frame snapshot",
        )?;
        let Some(layout) = Layout::new(width, height, mode) else {
            return Ok(());
        };
        let _saved = SavedDc::new(dc)?;
        // SAFETY: The borrowed owner-draw origin or normal surface origin uses
        // checked Win32 client coordinates and is restored by SavedDc.
        unsafe {
            require(
                SetViewportOrgEx(dc, origin.0, origin.1, ptr::null_mut()) != 0,
                "SetViewportOrgEx(origin)",
            )?;
        }
        if self.fonts.is_none() && layout.detail != Detail::Tiny && mode != DisplayMode::JapanTravel
        {
            self.fonts = Some(Fonts::new(dc, layout, style)?);
        }
        let mut canvas = Canvas {
            dc,
            pens: &mut self.pens,
        };
        canvas.fill(
            Rect {
                x: 0.0,
                y: 0.0,
                w: f64::from(width),
                h: f64::from(height),
            },
            0,
        )?;
        // SAFETY: Layout and offsets are finite client-pixel coordinates. Clip
        // includes all outlines and prevents tiny preview fallback glyphs escaping.
        unsafe {
            require(
                SetViewportOrgEx(
                    dc,
                    origin.0.saturating_add(offset.0),
                    origin.1.saturating_add(offset.1),
                    ptr::null_mut(),
                ) != 0,
                "SetViewportOrgEx",
            )?;
            let r = gdi::win_rect(layout.group);
            require(
                IntersectClipRect(dc, r.left, r.top, r.right, r.bottom) != ERROR,
                "IntersectClipRect",
            )?;
        }
        match mode {
            DisplayMode::TimeDate => {
                time_date(&mut canvas, layout, self.fonts.as_ref(), frame, style)
            }
            DisplayMode::Countdown => {
                countdown(&mut canvas, layout, self.fonts.as_ref(), frame, style, dpi)
            }
            DisplayMode::JapanTravel => {
                japan_travel(&mut canvas, layout, style, &self.travel_caption)
            }
        }
    }
}

fn japan_travel(
    canvas: &mut Canvas<'_>,
    layout: Layout,
    style: Style,
    caption: &TravelCaption,
) -> Result<(), AppError> {
    let accent = style.color();
    crate::travel_art::draw(canvas.dc, layout.panel, style.travel_style)?;
    if layout.detail != Detail::Tiny {
        let plaque = layout.calendar;
        let title_rect = Rect {
            x: plaque.x,
            y: plaque.y,
            w: plaque.w,
            h: plaque.h * 0.52,
        };
        let status_rect = Rect {
            x: plaque.x,
            y: plaque.y + plaque.h * 0.48,
            w: plaque.w,
            h: plaque.h * 0.52,
        };
        let scene_name = match style.travel_style {
            TravelStyle::FreeFlight => "自在飛行",
            TravelStyle::TrainJourney => "列車旅行",
            TravelStyle::JapaneseInn => "和風庭園",
            TravelStyle::TrainCab => "御運轉士",
            TravelStyle::Walking => "地方散策",
        };
        let title_text = format!("{scene_name}・{}", caption.place);
        let title = Font::fit(
            canvas.dc,
            FontMode::MingLiu,
            None,
            &title_text,
            title_rect.w * 0.92,
            title_rect.h * 0.82,
            title_rect.h * 0.65,
            true,
        )?;
        let status = Font::fit(
            canvas.dc,
            FontMode::MingLiu,
            None,
            &caption.status,
            status_rect.w * 0.92,
            status_rect.h * 0.72,
            status_rect.h * 0.5,
            false,
        )?;
        canvas.text(&title, &title_text, title_rect, accent, true)?;
        canvas.text(
            &status,
            &caption.status,
            status_rect,
            rgb(190, 202, 196),
            false,
        )?;
    }
    Ok(())
}

fn centered(cx: f64, cy: f64, width: f64, height: f64) -> Rect {
    Rect {
        x: cx - width / 2.0,
        y: cy - height / 2.0,
        w: width,
        h: height,
    }
}

fn clock_number_rect(clock: Rect, degrees: f64) -> Rect {
    let r = clock.w * 0.45;
    let angle = degrees.to_radians();
    // Keep the entire text box inside the major ticks' inner radius (0.82r),
    // including the widest "12" label and a gap for rasterized stroke edges.
    centered(
        clock.cx() + 0.58 * r * angle.sin(),
        clock.cy() - 0.58 * r * angle.cos(),
        r * 0.38,
        r * 0.30,
    )
}

fn time_date(
    canvas: &mut Canvas<'_>,
    layout: Layout,
    fonts: Option<&Fonts>,
    frame: FrameSnapshot,
    style: Style,
) -> Result<(), AppError> {
    let color = style.color_at(frame.tick);
    let clock = layout.clock;
    let (cx, cy, r) = (clock.cx(), clock.cy(), clock.w * 0.45);
    for tick in 0..60 {
        let major = tick % 5 == 0;
        if !major && layout.detail != Detail::Full {
            continue;
        }
        let angle = (f64::from(tick) * 6.0).to_radians();
        let (u, v) = (angle.sin(), -angle.cos());
        let radius = r / (u.abs().powi(6) + v.abs().powi(6)).powf(1.0 / 6.0);
        let length = r * if major { 0.18 } else { 0.075 };
        canvas.line(
            &[
                (cx + (radius - length) * u, cy + (radius - length) * v),
                (cx + radius * u, cy + radius * v),
            ],
            r * if major { 0.012 } else { 0.006 },
            if major { color } else { dim(color, 0.45) },
        )?;
    }
    if let Some(fonts) = fonts {
        for (value, degrees) in [("12", 0.0_f64), ("3", 90.0), ("6", 180.0), ("9", 270.0)] {
            let rect = clock_number_rect(clock, degrees);
            if style.font == FontMode::SevenSegment {
                digits(canvas, value, rect, color, false)?;
            } else {
                canvas.text(&fonts.clock, value, rect, color, false)?;
            }
        }
    }
    for ((angle, length), width) in frame
        .local
        .hand_angles()
        .into_iter()
        .zip([0.55, 0.82, 0.94])
        .zip([0.055, 0.035, 0.010])
    {
        let a = angle.to_radians();
        canvas.line(
            &[
                (cx - r * 0.08 * a.sin(), cy + r * 0.08 * a.cos()),
                (cx + r * length * a.sin(), cy - r * length * a.cos()),
            ],
            r * width,
            color,
        )?;
    }
    canvas.ellipse(centered(cx, cy, r * 0.10, r * 0.10), color)?;
    canvas.ellipse(centered(cx, cy, r * 0.05, r * 0.05), 0)?;

    let cal = layout.calendar;
    let header = Rect {
        x: cal.x,
        y: cal.y,
        w: cal.w,
        h: cal.h * 0.14,
    };
    let week_y = cal.y + cal.h * 0.14;
    let first_y = cal.y + cal.h * 0.25;
    let (cw, ch) = (cal.w / 7.0, cal.h * 0.75 / 6.0);
    let calendar = Calendar::new(frame.local.year, frame.local.month)
        .ok_or(AppError::OperationFailed("calendar snapshot"))?;
    if let Some(fonts) = fonts {
        let mut title = format!("{}年 {}月", frame.local.year, frame.local.month);
        let _selected = Selection::new(canvas.dc, fonts.month.handle())?;
        if layout.detail != Detail::Full
            || f64::from(gdi::measure(canvas.dc, &title)?.cx) > header.w
        {
            title = format!("{}月", frame.local.month);
        }
        canvas.text(&fonts.month, &title, header, color, false)?;
        for (column, label) in ["一", "二", "三", "四", "五", "六", "日"]
            .into_iter()
            .enumerate()
        {
            canvas.text(
                &fonts.weekday,
                label,
                Rect {
                    x: cal.x + column as f64 * cw,
                    y: week_y,
                    w: cw,
                    h: cal.h * 0.11,
                },
                color,
                false,
            )?;
        }
    } else {
        for row in 0..=6 {
            canvas.line(
                &[
                    (cal.x, first_y + f64::from(row) * ch),
                    (cal.right(), first_y + f64::from(row) * ch),
                ],
                1.0,
                dim(color, 0.45),
            )?;
        }
        for col in 0..=7 {
            canvas.line(
                &[
                    (cal.x + f64::from(col) * cw, first_y),
                    (cal.x + f64::from(col) * cw, cal.bottom()),
                ],
                1.0,
                dim(color, 0.45),
            )?;
        }
    }
    for day in 1..=calendar.days {
        let Some((row, col)) = calendar.cell(day) else {
            continue;
        };
        let cell = Rect {
            x: cal.x + f64::from(col) * cw,
            y: first_y + f64::from(row) * ch,
            w: cw,
            h: ch,
        };
        if day == frame.local.day {
            let diameter = fonts.map_or(cw.min(ch) * 0.8, |fonts| {
                (fonts.day_height * 1.65).min(cw.min(ch) * 0.9)
            });
            canvas.ellipse(centered(cell.cx(), cell.cy(), diameter, diameter), color)?;
        }
        if let Some(fonts) = fonts {
            canvas.text(
                &fonts.day,
                &day.to_string(),
                cell,
                if day == frame.local.day { 0 } else { color },
                false,
            )?;
        }
    }
    Ok(())
}

fn countdown(
    canvas: &mut Canvas<'_>,
    layout: Layout,
    fonts: Option<&Fonts>,
    frame: FrameSnapshot,
    style: Style,
    dpi: u32,
) -> Result<(), AppError> {
    let state = frame.countdown;
    let ratio = state.ratio.clamp(0.0, 1.0);
    let color = style.color();
    let hg = layout.hourglass;
    let (x, y, w, h) = (hg.x, hg.y, hg.w, hg.h);
    let top = [
        (x + w * 0.12, y + h * 0.10),
        (x + w * 0.88, y + h * 0.10),
        (x + w * 0.54, y + h * 0.50),
        (x + w * 0.46, y + h * 0.50),
    ];
    let bottom = [
        (x + w * 0.46, y + h * 0.50),
        (x + w * 0.54, y + h * 0.50),
        (x + w * 0.88, y + h * 0.90),
        (x + w * 0.12, y + h * 0.90),
    ];
    {
        let _clip = canvas.clip_polygon(&top)?;
        canvas.fill(
            Rect {
                x,
                y: y + h * (0.10 + (1.0 - ratio) * 0.40),
                w,
                h: h * ratio * 0.40,
            },
            color,
        )?;
    }
    {
        let _clip = canvas.clip_polygon(&bottom)?;
        canvas.fill(
            Rect {
                x,
                y: y + h * (0.90 - (1.0 - ratio) * 0.40),
                w,
                h: h * (1.0 - ratio) * 0.40,
            },
            color,
        )?;
    }
    for polygon in [top, bottom] {
        let mut closed = polygon.to_vec();
        closed.push(polygon[0]);
        canvas.line(&closed, w * 0.035, rgb(145, 151, 157))?;
    }
    if ratio > 0.0 {
        canvas.line(
            &[
                (hg.cx(), y + h * 0.47),
                (hg.cx(), y + h * (0.50 + ratio * 0.40)),
            ],
            w * 0.025,
            color,
        )?;
    }
    for cap_y in [y, y + h * 0.92] {
        canvas.rounded(
            Rect {
                x,
                y: cap_y,
                w,
                h: h * 0.08,
            },
            Some(rgb(80, 87, 96)),
            rgb(175, 181, 186),
            1.0,
            h * 0.04,
        )?;
    }

    let border = (f64::from(dpi) / 96.0 * 2.0)
        .min(layout.panel.h * 0.04)
        .max(1.0);
    let panel = layout.panel.inset(border / 2.0, border / 2.0);
    let panel_radius = layout.panel.h * 0.10;
    canvas.rounded(
        panel,
        Some(rgb(12, 7, 6)),
        rgb(105, 73, 54),
        border * 1.35,
        panel_radius,
    )?;

    // Use a restrained copper lip and a smooth, nearly-black brown interior.
    // The continuous gradients below model transmitted light through bottle
    // glass; broad flat bands made the previous version look like chocolate.
    let edge = (panel.h * 0.018).max(border).max(1.0);
    let rim = panel.inset(edge, edge);
    canvas.rounded(
        rim,
        Some(rgb(39, 22, 17)),
        rgb(157, 108, 77),
        (border * 0.75).max(1.0),
        (panel_radius - edge).max(1.0),
    )?;
    let glass = rim.inset(edge * 1.35, edge * 1.35);
    let glass_radius = (panel_radius - edge * 2.35).max(1.0);
    canvas.rounded(
        glass,
        Some(rgb(16, 9, 8)),
        rgb(65, 43, 33),
        1.0,
        glass_radius,
    )?;
    {
        let _clip = canvas.clip_rounded(glass, glass_radius)?;
        let upper = Rect {
            x: glass.x,
            y: glass.y,
            w: glass.w,
            h: glass.h * 0.52,
        };
        let lower = Rect {
            x: glass.x,
            y: upper.bottom(),
            w: glass.w,
            h: glass.h - upper.h,
        };
        canvas.gradient(upper, rgb(52, 31, 23), rgb(14, 8, 7), true)?;
        canvas.gradient(lower, rgb(14, 8, 7), rgb(35, 20, 15), true)?;

        let side_width = glass.w * 0.10;
        canvas.gradient(
            Rect {
                x: glass.x,
                y: glass.y,
                w: side_width,
                h: glass.h,
            },
            rgb(88, 52, 36),
            rgb(20, 12, 10),
            false,
        )?;
        canvas.gradient(
            Rect {
                x: glass.right() - side_width,
                y: glass.y,
                w: side_width,
                h: glass.h,
            },
            rgb(20, 12, 10),
            rgb(67, 39, 29),
            false,
        )?;

        canvas.gradient(
            Rect {
                x: glass.x + glass.w * 0.075,
                y: glass.y + glass.h * 0.085,
                w: glass.w * 0.20,
                h: (glass.h * 0.018).max(1.0),
            },
            rgb(139, 105, 82),
            rgb(48, 29, 23),
            false,
        )?;
    }
    canvas.rounded(
        glass,
        None,
        rgb(79, 54, 42),
        (border * 0.8).max(1.0),
        glass_radius,
    )?;
    let inner = layout.inner;
    let line_width = (3.0 * f64::from(dpi) / 96.0)
        .min(inner.w * 0.025)
        .min(inner.h * 0.12)
        .max(1.0);
    let shadow_width = (line_width * 2.0).min(inner.w).max(1.0);
    let line_x = (inner.x + ratio * inner.w).clamp(
        inner.x + shadow_width.min(inner.w) / 2.0,
        inner.right() - shadow_width.min(inner.w) / 2.0,
    );
    let end_inset = shadow_width.min(inner.h) / 2.0;
    canvas.line(
        &[
            (line_x, inner.y + end_inset),
            (line_x, inner.bottom() - end_inset),
        ],
        shadow_width,
        rgb(105, 32, 32),
    )?;
    canvas.line(
        &[
            (line_x, inner.y + end_inset),
            (line_x, inner.bottom() - end_inset),
        ],
        line_width,
        rgb(255, 32, 32),
    )?;
    let [hours, minutes, seconds] = hms(state.display_seconds);
    let label = format!("{hours:02}:{minutes:02}:{seconds:02}");
    let color = if state.dim { dim(color, 0.35) } else { color };
    if style.font == FontMode::SevenSegment || fonts.is_none() {
        digits(
            canvas,
            &label,
            inner.inset(inner.w * 0.02, inner.h * 0.02),
            color,
            style.outline(),
        )?;
    } else if let Some(fonts) = fonts {
        canvas.text(&fonts.countdown, &label, inner, color, style.outline())?;
    }
    if state.final_ten {
        canvas.rounded(
            panel,
            None,
            rgb(0, 210, 220),
            border.max(2.0),
            layout.panel.h * 0.07,
        )?;
    }
    Ok(())
}

fn digits(
    canvas: &mut Canvas<'_>,
    label: &str,
    rect: Rect,
    color: u32,
    shadow: bool,
) -> Result<(), AppError> {
    let mut units = 0.0;
    for (i, ch) in label.bytes().enumerate() {
        units += if ch == b':' { 0.18 } else { 0.56 };
        if i > 0 {
            units += 0.025;
        }
    }
    if units == 0.0 {
        return Ok(());
    }
    let h = rect.h.min(rect.w / units).max(0.0);
    let mut x = rect.cx() - h * units / 2.0;
    let y = rect.cy() - h / 2.0;
    for ch in label.bytes() {
        if ch == b':' {
            for cy in [y + h * 0.30, y + h * 0.70] {
                if shadow {
                    canvas.ellipse(
                        centered(x + h * 0.09, cy, h * 0.105, h * 0.105),
                        rgb(32, 36, 32),
                    )?;
                }
                canvas.ellipse(centered(x + h * 0.09, cy, h * 0.075, h * 0.075), color)?;
            }
            x += h * (0.18 + 0.025);
        } else if let Some(mask) = ch
            .checked_sub(b'0')
            .and_then(|n| SEGMENTS.get(usize::from(n)))
            .copied()
        {
            segment_digit(canvas, x, y, h, mask, color, shadow)?;
            x += h * (0.56 + 0.025);
        }
    }
    Ok(())
}

fn segment_digit(
    canvas: &mut Canvas<'_>,
    x: f64,
    y: f64,
    h: f64,
    mask: u8,
    color: u32,
    shadow: bool,
) -> Result<(), AppError> {
    let (w, t) = (h * 0.56, h * 0.095);
    let gap = h * 0.013;
    for segment in 0..7 {
        if mask & (1 << segment) == 0 {
            continue;
        }
        let points = match segment {
            0 | 3 | 6 => {
                let yy = y + match segment {
                    0 => 0.0,
                    3 => h - t,
                    _ => (h - t) / 2.0,
                };
                let (left, right) = (x + t * 0.55, x + w - t * 0.55);
                vec![
                    (left, yy + t / 2.0),
                    (left + t / 2.0, yy),
                    (right - t / 2.0, yy),
                    (right, yy + t / 2.0),
                    (right - t / 2.0, yy + t),
                    (left + t / 2.0, yy + t),
                ]
            }
            _ => {
                let xx = x + if matches!(segment, 1 | 2) { w - t } else { 0.0 };
                let top = y + if matches!(segment, 1 | 5) {
                    t * 0.55 + gap
                } else {
                    h / 2.0 + gap
                };
                let bottom = y + if matches!(segment, 1 | 5) {
                    h / 2.0 - gap
                } else {
                    h - t * 0.55 - gap
                };
                vec![
                    (xx + t / 2.0, top),
                    (xx + t, top + t / 2.0),
                    (xx + t, bottom - t / 2.0),
                    (xx + t / 2.0, bottom),
                    (xx, bottom - t / 2.0),
                    (xx, top + t / 2.0),
                ]
            }
        };
        canvas.polygon(
            &points,
            color,
            shadow.then_some((rgb(32, 36, 32), (h * 0.008).max(1.0))),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Countdown, LocalTime, Timeline};
    use std::{env, fs, path::Path};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS,
    };

    struct Screen(HDC);
    impl Screen {
        fn new() -> Self {
            // SAFETY: Borrow a screen DC and release it in Drop.
            let dc = unsafe { GetDC(ptr::null_mut()) };
            assert!(!dc.is_null());
            Self(dc)
        }
    }
    impl Drop for Screen {
        fn drop(&mut self) {
            // SAFETY: Exactly matches this wrapper's GetDC(NULL).
            unsafe {
                ReleaseDC(ptr::null_mut(), self.0);
            }
        }
    }
    fn frame(now: u64) -> FrameSnapshot {
        Timeline::new(DisplayMode::Countdown, 600, 0)
            .unwrap()
            .sample(LocalTime::FIXTURE, now)
    }

    #[test]
    fn automatic_clock_palette_changes_every_two_minutes_and_repeats() {
        let style = Style {
            palette: ColorPreset::Auto,
            ..Style::default()
        };
        assert_eq!(style.color_at(0), rgb(139, 0, 0));
        assert_eq!(style.color_at(119_999), rgb(139, 0, 0));
        assert_eq!(style.color_at(120_000), rgb(255, 140, 0));
        assert_eq!(style.color_at(240_000), rgb(0, 255, 0));
        assert_eq!(style.color_at(600_000), rgb(255, 191, 0));
        assert_eq!(style.color_at(720_000), rgb(154, 160, 163));
        assert_eq!(style.color_at(840_000), rgb(139, 0, 0));
    }
    #[allow(clippy::too_many_arguments)]
    fn draw(
        renderer: &mut Renderer,
        screen: HDC,
        w: i32,
        h: i32,
        dpi: u32,
        mode: DisplayMode,
        style: Style,
        frame: FrameSnapshot,
    ) -> Buffer {
        renderer.prepare(w, h, dpi, mode, style).unwrap();
        let buffer = Buffer::new(screen, w, h).unwrap();
        renderer
            .draw_scene(buffer.dc, w, h, dpi, mode, frame, style, (0, 0), (0, 0))
            .unwrap();
        buffer
    }
    fn write_bmp(path: &Path, w: i32, h: i32, pixels: &[u8]) {
        let mut bytes = Vec::with_capacity(54 + pixels.len());
        bytes.extend_from_slice(b"BM");
        bytes.extend_from_slice(&((54 + pixels.len()) as u32).to_le_bytes());
        bytes.extend_from_slice(&[0u8; 4]);
        bytes.extend_from_slice(&54u32.to_le_bytes());
        bytes.extend_from_slice(&40u32.to_le_bytes());
        bytes.extend_from_slice(&w.to_le_bytes());
        bytes.extend_from_slice(&(-h).to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 24]);
        bytes.extend_from_slice(pixels);
        fs::write(path, bytes).unwrap();
    }

    fn assert_clock_numerals_clear_ticks(screen: HDC) {
        let mut custom = FontSpec::default_choice().logfont(1);
        custom.lfFaceName.fill(0);
        for (target, unit) in custom
            .lfFaceName
            .iter_mut()
            .zip("Arial Black".encode_utf16())
        {
            *target = unit;
        }
        custom.lfWeight = 900;
        custom.lfItalic = 1;
        let custom = FontSpec::from_logfont(custom, 2400).unwrap();
        let styles = [
            Style::default(),
            Style {
                font: FontMode::MingLiu,
                ..Style::default()
            },
            Style {
                font: FontMode::Custom,
                custom: Some(custom),
                point_size_tenth: custom.point_size_tenth(),
                ..Style::default()
            },
        ];
        for (w, h) in [(320, 180), (1920, 1080), (1080, 1920), (3840, 2160)] {
            let layout = Layout::new(w, h, DisplayMode::TimeDate).unwrap();
            let r = layout.clock.w * 0.45;
            for style in styles {
                let fonts = Fonts::new(screen, layout, style).unwrap();
                let _font = Selection::new(screen, fonts.clock.handle()).unwrap();
                for (label, degrees) in [("12", 0.0), ("3", 90.0), ("6", 180.0), ("9", 270.0)] {
                    let rect = clock_number_rect(layout.clock, degrees);
                    if style.font != FontMode::SevenSegment {
                        let measured = gdi::measure(screen, label).unwrap();
                        assert!(f64::from(measured.cx) <= rect.w);
                        assert!(f64::from(measured.cy) <= rect.h);
                    }
                    // Even the text-box corners must clear the innermost tick,
                    // its half-width and pixel rounding, at preview through 4K.
                    for x in [rect.x, rect.right()] {
                        for y in [rect.y, rect.bottom()] {
                            let distance = (x - layout.clock.cx()).hypot(y - layout.clock.cy());
                            assert!(distance + r * 0.006 + 1.5 < r * 0.82);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn gdi_small_sizes_fit_and_fifty_resource_cycles_release_objects() {
        let screen = Screen::new();
        let modes = [
            DisplayMode::TimeDate,
            DisplayMode::Countdown,
            DisplayMode::JapanTravel,
        ];
        for mode in modes {
            for (w, h) in [(1, 1), (120, 80), (320, 180)] {
                for dpi in [96, 144, 192, 288] {
                    let mut renderer = Renderer::default();
                    let buffer = draw(
                        &mut renderer,
                        screen.0,
                        w,
                        h,
                        dpi,
                        mode,
                        Style::default(),
                        frame(300000),
                    );
                    let pixels = buffer.pixels(screen.0).unwrap();
                    assert_eq!(pixels.len(), w as usize * h as usize * 4);
                    assert!(renderer.pens.len() <= 32);
                    if let Some(fonts) = renderer.fonts.as_ref() {
                        let layout = Layout::new(w, h, mode).unwrap();
                        if mode == DisplayMode::TimeDate {
                            assert!(
                                fonts.day_height * 1.55
                                    <= (layout.calendar.w / 7.0)
                                        .min(layout.calendar.h * 0.75 / 6.0)
                                        * 0.9
                                        + 1.0
                            );
                        }
                    }
                }
            }
        }
        for travel_style in [
            TravelStyle::FreeFlight,
            TravelStyle::TrainJourney,
            TravelStyle::JapaneseInn,
            TravelStyle::TrainCab,
            TravelStyle::Walking,
        ] {
            let mut renderer = Renderer::default();
            let buffer = draw(
                &mut renderer,
                screen.0,
                800,
                450,
                144,
                DisplayMode::JapanTravel,
                Style {
                    travel_style,
                    ..Style::default()
                },
                frame(300000),
            );
            assert_eq!(buffer.pixels(screen.0).unwrap().len(), 800 * 450 * 4);
            assert!(renderer.pens.len() <= 32);
        }
        assert_clock_numerals_clear_ticks(screen.0);
        // Warmed GDI baseline; no live renderer/cache remains above.
        // SAFETY: Read counters for this test process only.
        let before = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        for cycle in 0..50 {
            let mut renderer = Renderer::default();
            for mode in modes {
                let buffer = draw(
                    &mut renderer,
                    screen.0,
                    320 + cycle % 2,
                    180,
                    144,
                    mode,
                    Style::default(),
                    frame(300000),
                );
                for tick in [590000, 599999, 600000, 600420, 603360, 3600000] {
                    renderer
                        .draw_scene(
                            buffer.dc,
                            buffer.width,
                            buffer.height,
                            144,
                            mode,
                            frame(tick),
                            Style::default(),
                            (0, 0),
                            (0, 0),
                        )
                        .unwrap();
                }
                assert!(renderer.pens.len() <= 32);
            }
            renderer
                .prepare(0, 0, 96, DisplayMode::TimeDate, Style::default())
                .unwrap();
            assert!(renderer.buffer.is_none() && renderer.fonts.is_none());
        }
        // SAFETY: Same warmed process counter after all owned resources dropped.
        let after = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        println!("GDI objects across 50 renderer/cache cycles: {before} -> {after}");
        assert!(after <= before + 2, "GDI leak: {before} -> {after}");
    }

    #[test]
    #[ignore = "writes real GDI fixture BMP files to PHASE2_FIXTURES"]
    fn export_visual_fixtures() {
        let directory = env::var_os("PHASE2_FIXTURES").expect("set PHASE2_FIXTURES");
        let directory = Path::new(&directory);
        fs::create_dir_all(directory).unwrap();
        let screen = Screen::new();
        let mut cases = Vec::new();
        for mode in [
            DisplayMode::TimeDate,
            DisplayMode::Countdown,
            DisplayMode::JapanTravel,
        ] {
            for palette in [
                ColorPreset::DarkRed,
                ColorPreset::DarkOrange,
                ColorPreset::BrightGreen,
                ColorPreset::OffWhite,
                ColorPreset::MutedLightBlue,
                ColorPreset::Amber,
                ColorPreset::Auto,
                ColorPreset::IronGray,
            ] {
                cases.push((
                    mode,
                    800,
                    369,
                    96,
                    Style {
                        palette,
                        ..Style::default()
                    },
                    300000,
                    "palette",
                ));
            }
            for (w, h, dpi) in [
                (1920, 1080, 96),
                (3840, 2160, 144),
                (1080, 1920, 192),
                (320, 180, 144),
                (120, 80, 288),
            ] {
                cases.push((mode, w, h, dpi, Style::default(), 300000, "size"));
            }
            for font in [FontMode::Consolas, FontMode::MingLiu] {
                cases.push((
                    mode,
                    800,
                    369,
                    96,
                    Style {
                        font,
                        ..Style::default()
                    },
                    300000,
                    "font",
                ));
            }
        }
        for (tick, label) in [(590000, "last-ten"), (600420, "dim"), (603360, "complete")] {
            cases.push((
                DisplayMode::Countdown,
                800,
                369,
                96,
                Style::default(),
                tick,
                label,
            ));
        }
        cases.push((
            DisplayMode::JapanTravel,
            800,
            450,
            96,
            Style {
                travel_style: TravelStyle::TrainJourney,
                ..Style::default()
            },
            300000,
            "train-journey",
        ));
        cases.push((
            DisplayMode::JapanTravel,
            800,
            450,
            96,
            Style {
                travel_style: TravelStyle::JapaneseInn,
                ..Style::default()
            },
            300000,
            "japanese-inn",
        ));
        cases.push((
            DisplayMode::JapanTravel,
            800,
            450,
            96,
            Style {
                travel_style: TravelStyle::TrainCab,
                ..Style::default()
            },
            300000,
            "train-cab",
        ));
        cases.push((
            DisplayMode::JapanTravel,
            800,
            450,
            96,
            Style {
                travel_style: TravelStyle::Walking,
                ..Style::default()
            },
            300000,
            "walking",
        ));
        for label in ["travel-checking", "travel-playing", "travel-unavailable"] {
            cases.push((
                DisplayMode::JapanTravel,
                1920,
                1080,
                96,
                Style::default(),
                300000,
                label,
            ));
        }
        for (index, (mode, w, h, dpi, style, tick, label)) in cases.into_iter().enumerate() {
            let mut renderer = Renderer::default();
            match label {
                "travel-checking" => renderer.set_travel_caption(TravelCaption::live(
                    None,
                    crate::travel::NetworkState::Checking,
                    false,
                )),
                "travel-playing" => renderer.set_travel_caption(TravelCaption::live(
                    Some("京都・中京區"),
                    crate::travel::NetworkState::Playing,
                    false,
                )),
                "travel-unavailable" => renderer.set_travel_caption(TravelCaption::live(
                    None,
                    crate::travel::NetworkState::Offline,
                    true,
                )),
                _ => (),
            }
            let buffer = draw(&mut renderer, screen.0, w, h, dpi, mode, style, frame(tick));
            let pixels = buffer.pixels(screen.0).unwrap();
            let colored = pixels
                .chunks_exact(4)
                .filter(|pixel| pixel[..3] != [0, 0, 0])
                .count();
            assert!(colored > 0, "blank scene");
            let layout = Layout::new(w, h, mode).unwrap();
            let rect = gdi::win_rect(layout.group);
            for (index, pixel) in pixels.chunks_exact(4).enumerate() {
                let (x, y) = (index as i32 % w, index as i32 / w);
                if x < rect.left || x >= rect.right || y < rect.top || y >= rect.bottom {
                    assert_eq!(&pixel[..3], &[0, 0, 0]);
                }
            }
            let name = format!(
                "{index:02}-{mode:?}-{w}x{h}-dpi{dpi}-p{}-{:?}-{label}.bmp",
                style.palette.registry_value(),
                style.font
            );
            write_bmp(&directory.join(&name), w, h, &pixels);
            println!(
                "{name}: {colored} nonblack pixels; generation {}",
                frame(tick).generation
            );
        }
        assert_eq!(
            Countdown::new(600, 0)
                .unwrap()
                .frame(603360)
                .display_seconds,
            0
        );
    }
}
