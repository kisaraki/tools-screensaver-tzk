//! Layout works entirely in client pixels; DPI is never applied to the canvas twice.
use crate::model::{DisplayMode, Xorshift32};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
impl Rect {
    pub fn right(self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(self) -> f64 {
        self.y + self.h
    }
    pub fn cx(self) -> f64 {
        self.x + self.w / 2.0
    }
    pub fn cy(self) -> f64 {
        self.y + self.h / 2.0
    }
    pub fn inset(self, x: f64, y: f64) -> Self {
        let x = x.max(0.0).min(self.w / 2.0);
        let y = y.max(0.0).min(self.h / 2.0);
        Self {
            x: self.x + x,
            y: self.y + y,
            w: self.w - 2.0 * x,
            h: self.h - 2.0 * y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    Full,
    Compact,
    Tiny,
}

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub group: Rect,
    pub clock: Rect,
    pub calendar: Rect,
    pub hourglass: Rect,
    pub panel: Rect,
    pub inner: Rect,
    pub horizontal: bool,
    pub detail: Detail,
}

impl Layout {
    pub fn new(width: i32, height: i32, mode: DisplayMode) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let (w, h) = (f64::from(width), f64::from(height));
        let group = Rect {
            x: w * 0.06,
            y: h * 0.09,
            w: w * 0.88,
            h: h * 0.82,
        };
        let empty = Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };
        let horizontal = w / h >= 1.35;
        let detail = if width >= 320 && height >= 180 {
            Detail::Full
        } else if width >= 120 && height >= 80 {
            Detail::Compact
        } else {
            Detail::Tiny
        };
        let mut result = Self {
            group,
            clock: empty,
            calendar: empty,
            hourglass: empty,
            panel: empty,
            inner: empty,
            horizontal,
            detail,
        };
        match mode {
            DisplayMode::TimeDate if horizontal => {
                let side = (group.w * 0.48).min(group.h);
                let gap = group.w * 0.08;
                let cw = group.w * 0.44;
                let left = group.cx() - (side + gap + cw) / 2.0;
                result.clock = Rect {
                    x: left,
                    y: group.cy() - side / 2.0,
                    w: side,
                    h: side,
                };
                result.calendar = Rect {
                    x: left + side + gap,
                    y: group.cy() - side / 2.0,
                    w: cw,
                    h: side,
                };
            }
            DisplayMode::TimeDate => {
                let side = group.w.min(group.h * 0.48);
                let gap = group.h * 0.06;
                let ch = group.h * 0.46;
                let cw = group.w.min(ch * 1.15);
                let top = group.cy() - (side + gap + ch) / 2.0;
                result.clock = Rect {
                    x: group.cx() - side / 2.0,
                    y: top,
                    w: side,
                    h: side,
                };
                result.calendar = Rect {
                    x: group.cx() - cw / 2.0,
                    y: top + side + gap,
                    w: cw,
                    h: ch,
                };
            }
            DisplayMode::Countdown => {
                let short = w.min(h);
                let scale = (group.h / (short * 0.24 + group.w * 0.27)).min(1.0);
                let (pw, ph, hw, hh, gap) = (
                    group.w * scale,
                    group.w * 0.27 * scale,
                    short * 0.12 * scale,
                    short * 0.18 * scale,
                    short * 0.06 * scale,
                );
                let top = group.cy() - (hh + gap + ph) / 2.0;
                result.hourglass = Rect {
                    x: group.cx() - hw / 2.0,
                    y: top,
                    w: hw,
                    h: hh,
                };
                result.panel = Rect {
                    x: group.cx() - pw / 2.0,
                    y: top + hh + gap,
                    w: pw,
                    h: ph,
                };
                result.inner = result.panel.inset(pw * 0.04, ph * 0.12);
            }
            DisplayMode::JapanTravel => {
                // Reserve a complete, unobscured 16:9 player rectangle. The
                // surrounding panel and the lower plaque form the aircraft-window
                // suggestion without clipping third-party video or its controls.
                let plaque = group.h * 0.14;
                let available_h = (group.h - plaque).max(f64::EPSILON);
                let player_w = (group.w / 1.12).min((available_h / 1.16) * 16.0 / 9.0);
                let player_h = player_w * 9.0 / 16.0;
                let frame = (player_w * 0.055)
                    .min((available_h - player_h) / 2.0)
                    .min((group.w - player_w) / 2.0)
                    .max(0.0);
                result.inner = Rect {
                    x: group.cx() - player_w / 2.0,
                    y: group.y + (available_h - player_h) / 2.0,
                    w: player_w,
                    h: player_h,
                };
                result.panel = Rect {
                    x: result.inner.x - frame,
                    y: result.inner.y - frame,
                    w: result.inner.w + frame * 2.0,
                    h: result.inner.h + frame * 2.0,
                };
                result.calendar = Rect {
                    x: group.x,
                    y: group.y + available_h,
                    w: group.w,
                    h: plaque,
                };
            }
        }
        Some(result)
    }
}

pub fn offset_range(size: i32, start: f64, end: f64) -> (i32, i32) {
    if size < 120 || !start.is_finite() || !end.is_finite() {
        return (0, 0);
    }
    let size = f64::from(size);
    let low = (-0.05 * size).max(0.02 * size - start).ceil() as i32;
    let high = (0.05 * size).min(0.98 * size - end).floor() as i32;
    if low > high || low > 0 || high < 0 {
        (0, 0)
    } else {
        (low, high)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Drift {
    rng: Xorshift32,
    last: u64,
    pub offset: (i32, i32),
}
impl Drift {
    pub fn new(seed: u32, tick: u64) -> Self {
        Self {
            rng: Xorshift32::new(seed),
            last: tick,
            offset: (0, 0),
        }
    }
    pub fn update(&mut self, now: u64, width: i32, height: i32, bounds: Rect) {
        let xr = offset_range(width, bounds.x, bounds.right());
        let yr = offset_range(height, bounds.y, bounds.bottom());
        if now.saturating_sub(self.last) >= 60000 {
            self.last = now;
            fn choose(rng: &mut Xorshift32, (low, high): (i32, i32)) -> i32 {
                let span = (i64::from(high) - i64::from(low) + 1) as u64;
                (i64::from(low) + (u64::from(rng.next_value()) % span) as i64) as i32
            }
            self.offset = (choose(&mut self.rng, xr), choose(&mut self.rng, yr));
        }
        self.offset.0 = self.offset.0.clamp(xr.0, xr.1);
        self.offset.1 = self.offset.1.clamp(yr.0, yr.1);
    }
}

/// Hard cap is per surface (256 MiB at 32 bpp), not a false 20 MiB process budget.
pub fn buffer_bytes(width: i32, height: i32) -> Option<usize> {
    let (w, h) = (usize::try_from(width).ok()?, usize::try_from(height).ok()?);
    if w == 0 || h == 0 {
        return Some(0);
    }
    w.checked_mul(h)?
        .checked_mul(4)
        .filter(|&bytes| bytes <= 256 * 1024 * 1024)
}
