//! Offline PNG surroundings and CPU-composited translucent glass card.
use crate::{
    error::AppError,
    gdi::{rgb, Canvas, Font, FontMode},
    layout::Rect,
    model::FrameSnapshot,
    travel_art::{self, Pixels},
    weather::{Condition, Snapshot},
};
use std::sync::OnceLock;
static BACKGROUNDS: [OnceLock<Result<Pixels, AppError>>; 8] = [const { OnceLock::new() }; 8];
pub(crate) struct Scene {
    condition: Condition,
    pixels: Pixels,
}
fn background(width: i32, height: i32) -> Rect {
    // All eight supplied backgrounds share this aspect ratio. Keep the complete
    // image within the same central area used by the clock and countdown.
    let w = f64::from(width);
    let h = f64::from(height);
    let scale = (w * 0.64 / 1983.0).min(h * 0.60 / 793.0);
    let bw = (1983.0 * scale).max(1.0);
    let bh = (793.0 * scale).max(1.0);
    Rect {
        x: (w - bw) / 2.0,
        y: (h - bh) / 2.0,
        w: bw,
        h: bh,
    }
}
pub(crate) fn card(width: i32, height: i32) -> Rect {
    let w = f64::from(width);
    let h = f64::from(height);
    let side = (w * 0.26).min(background(width, height).h * 0.70).max(1.0);
    Rect {
        x: (w - side) / 2.0,
        y: (h - side) / 2.0,
        w: side,
        h: side,
    }
}
struct Glass {
    x: i32,
    y: i32,
    width: usize,
    pixels: Vec<[u8; 3]>,
}
impl Glass {
    fn new(panel: Rect, sample: impl Fn(i32, i32) -> [u8; 4]) -> Result<Self, AppError> {
        let radius = (panel.w * 0.035).round().clamp(2.0, 48.0) as i32;
        let padding = radius * 4;
        let x = panel.x.floor() as i32 - padding;
        let y = panel.y.floor() as i32 - padding;
        let width = panel.w.ceil() as usize + padding as usize * 2 + 2;
        let count = width
            .checked_mul(width)
            .filter(|n| {
                n.checked_mul(6)
                    .is_some_and(|bytes| bytes <= 16 * 1024 * 1024)
            })
            .ok_or(AppError::OperationFailed("weather glass pixel limit"))?;
        let mut pixels = Vec::new();
        let mut scratch = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| AppError::OperationFailed("weather glass allocation"))?;
        scratch
            .try_reserve_exact(count)
            .map_err(|_| AppError::OperationFailed("weather glass scratch allocation"))?;
        scratch.resize(count, [0; 3]);
        for row in 0..width {
            for column in 0..width {
                let value = sample(x + column as i32, y + row as i32);
                pixels.push([value[0], value[1], value[2]]);
            }
        }
        // Three separable box passes approximate Gaussian diffusion without a
        // sparse sampling grid that leaves the underlying pixel blocks visible.
        for _ in 0..3 {
            blur(&pixels, &mut scratch, width, radius, true);
            blur(&scratch, &mut pixels, width, radius, false);
        }
        Ok(Self {
            x,
            y,
            width,
            pixels,
        })
    }
    fn sample(&self, x: f64, y: f64, channel: usize) -> f64 {
        let x = (x - f64::from(self.x)).clamp(0.0, (self.width - 1) as f64);
        let y = (y - f64::from(self.y)).clamp(0.0, (self.width - 1) as f64);
        let (ix, iy) = (x.floor() as usize, y.floor() as usize);
        let (next_x, next_y) = ((ix + 1).min(self.width - 1), (iy + 1).min(self.width - 1));
        let (fx, fy) = (x.fract(), y.fract());
        let p = |px: usize, py: usize| f64::from(self.pixels[py * self.width + px][channel]);
        (p(ix, iy) * (1.0 - fx) + p(next_x, iy) * fx) * (1.0 - fy)
            + (p(ix, next_y) * (1.0 - fx) + p(next_x, next_y) * fx) * fy
    }
}
fn blur(input: &[[u8; 3]], output: &mut [[u8; 3]], width: usize, radius: i32, horizontal: bool) {
    let index = |line: usize, position: i32| {
        let p = position.clamp(0, width as i32 - 1) as usize;
        if horizontal {
            line * width + p
        } else {
            p * width + line
        }
    };
    let divisor = (radius * 2 + 1) as u32;
    for line in 0..width {
        let mut sum = [0u32; 3];
        for position in -radius..=radius {
            for (c, total) in sum.iter_mut().enumerate() {
                *total += u32::from(input[index(line, position)][c]);
            }
        }
        for position in 0..width as i32 {
            for (c, total) in sum.iter_mut().enumerate() {
                output[index(line, position)][c] = (*total / divisor) as u8;
                *total -= u32::from(input[index(line, position - radius)][c]);
                *total += u32::from(input[index(line, position + radius + 1)][c]);
            }
        }
    }
}
fn surface(panel: Rect, radius: f64, x: f64, y: f64) -> (f64, f64, f64) {
    let qx = (x - panel.cx()).abs() - panel.w / 2.0 + radius;
    let qy = (y - panel.cy()).abs() - panel.h / 2.0 + radius;
    let (dx, dy) = (qx.max(0.0), qy.max(0.0));
    let length = dx.hypot(dy);
    let distance = length + qx.max(qy).min(0.0) - radius;
    let (nx, ny) = if length > 0.0001 {
        (
            dx / length * (x - panel.cx()).signum(),
            dy / length * (y - panel.cy()).signum(),
        )
    } else if qx > qy {
        ((x - panel.cx()).signum(), 0.0)
    } else {
        (0.0, (y - panel.cy()).signum())
    };
    (distance, nx, ny)
}
fn compose(width: i32, height: i32, condition: Condition) -> Result<Scene, AppError> {
    let bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .filter(|n| *n <= 64 * 1024 * 1024)
        .ok_or(AppError::OperationFailed("weather pixel limit"))?;
    let source = BACKGROUNDS[condition as usize]
        .get_or_init(|| travel_art::decode(condition.png()))
        .as_ref()
        .map_err(|e| *e)?;
    let region = background(width, height);
    let sample = |x: i32, y: i32| {
        let sx = ((f64::from(x) - region.x) / region.w * f64::from(source.width))
            .floor()
            .clamp(0.0, f64::from(source.width - 1)) as usize;
        let sy = ((f64::from(y) - region.y) / region.h * f64::from(source.height))
            .floor()
            .clamp(0.0, f64::from(source.height - 1)) as usize;
        let index = (sy * source.width as usize + sx) * 4;
        [
            source.bytes[index],
            source.bytes[index + 1],
            source.bytes[index + 2],
            0,
        ]
    };
    let panel = card(width, height);
    let radius = panel.w * 0.13;
    let glass = Glass::new(panel, sample)?;
    let bevel = (panel.w * 0.038).max(1.5);
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(bytes)
        .map_err(|_| AppError::OperationFailed("weather pixel allocation"))?;
    for y in 0..height {
        for x in 0..width {
            if f64::from(x) < region.x
                || f64::from(x) >= region.right()
                || f64::from(y) < region.y
                || f64::from(y) >= region.bottom()
            {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let mut pixel = sample(x, y);
            let xf = f64::from(x);
            let yf = f64::from(y);
            let (outside, nx, ny) = surface(panel, radius, xf, yf);
            let shadow_distance =
                surface(panel, radius, xf - panel.w * 0.012, yf - panel.w * 0.023).0;
            let shadow =
                (-((shadow_distance.max(0.0) / (panel.w * 0.032).max(1.0)).powi(2))).exp() * 0.22;
            if outside > -0.5 {
                for channel in &mut pixel[..3] {
                    *channel = (f64::from(*channel) * (1.0 - shadow)) as u8;
                }
            }
            let coverage = (0.5 - outside).clamp(0.0, 1.0);
            if coverage > 0.0 {
                let t = (-outside / bevel).clamp(0.0, 1.0);
                let bend = (std::f64::consts::PI * t).sin() * bevel * 1.2;
                let lighting = (-nx * 0.6 - ny * 0.8).max(0.0);
                let rim = (1.0 - t).powi(3) * (0.20 + lighting * 0.56);
                let polish = (-outside.abs() / 0.85).exp() * (0.12 + lighting * 0.38);
                let u = (xf - panel.x) / panel.w;
                let v = (yf - panel.y) / panel.h;
                let reflection = (-((v - 0.08 - u * 0.12) / 0.22).powi(2)).exp() * 0.065;
                let caustic = (-((t - 0.72) / 0.16).powi(2)).exp() * (1.0 - lighting) * 0.14;
                let shine = (reflection + rim + polish + caustic).clamp(0.0, 0.85);
                for (c, channel) in pixel[..3].iter_mut().enumerate() {
                    // Subpixel displacement and mild dispersion follow the curved
                    // bevel normal, instead of drawing a flat bright outline.
                    let displacement = bend * (1.0 + (c as f64 - 1.0) * 0.07);
                    let transmitted =
                        glass.sample(xf + nx * displacement, yf + ny * displacement, c);
                    // Halve the body tint opacity (30% -> 15%) while keeping
                    // the blurred transmission and curved optical rim intact.
                    let base = transmitted * 0.85 + [10.0, 8.5, 7.5][c];
                    let shaded = base * (1.0 - shine) + 250.0 * shine;
                    *channel = (f64::from(*channel) * (1.0 - coverage) + shaded * coverage)
                        .clamp(0.0, 255.0) as u8;
                }
            }
            pixels.extend_from_slice(&pixel);
        }
    }
    Ok(Scene {
        condition,
        pixels: Pixels {
            width,
            height,
            bytes: pixels,
        },
    })
}
pub(crate) fn draw(
    canvas: &mut Canvas<'_>,
    width: i32,
    height: i32,
    frame: FrameSnapshot,
    snapshot: &Snapshot,
    cache: &mut Option<Scene>,
) -> Result<(), AppError> {
    if width <= 0 || height <= 0 {
        return Ok(());
    }
    if cache.as_ref().is_none_or(|s| {
        s.condition != snapshot.condition || s.pixels.width != width || s.pixels.height != height
    }) {
        *cache = Some(compose(width, height, snapshot.condition)?);
    }
    travel_art::draw_pixels(
        canvas.dc,
        Rect {
            x: 0.0,
            y: 0.0,
            w: f64::from(width),
            h: f64::from(height),
        },
        &cache
            .as_ref()
            .ok_or(AppError::OperationFailed("weather scene"))?
            .pixels,
    )?;
    if width < 120 || height < 80 {
        return Ok(());
    }
    let panel = card(width, height);
    let time = format!(
        "{:02}:{:02}:{:02}",
        frame.local.hour, frame.local.minute, frame.local.second
    );
    let temperature = snapshot
        .temperature
        .map_or("—°C".into(), |t| format!("{t:.1}°C"));
    let metric = |n: Option<f64>, unit: &str| n.map_or("—".into(), |n| format!("{n:.0}{unit}"));
    let lines = [
        (snapshot.place.clone(), 0.08, 0.10),
        (temperature, 0.18, 0.28),
        (
            format!(
                "{} · {}",
                if snapshot.temperature.is_some() {
                    snapshot.condition.label()
                } else {
                    "即時氣象"
                },
                time
            ),
            0.47,
            0.13,
        ),
        (
            format!(
                "{:04}-{:02}-{:02}",
                frame.local.year, frame.local.month, frame.local.day
            ),
            0.60,
            0.08,
        ),
        (
            format!(
                "濕度 {}  風速 {}",
                metric(snapshot.humidity, "%"),
                snapshot.wind.map_or("—".into(), |w| format!("{w:.1}m/s"))
            ),
            0.70,
            0.07,
        ),
        (snapshot.status.clone(), 0.80, 0.08),
    ];
    for (text, y, h) in lines {
        let rect = Rect {
            x: panel.x + panel.w * 0.06,
            y: panel.y + panel.h * y,
            w: panel.w * 0.88,
            h: panel.h * h,
        };
        let font = Font::fit(
            canvas.dc,
            FontMode::Consolas,
            None,
            &text,
            rect.w,
            rect.h,
            rect.h * 0.90,
            true,
        )?;
        canvas.text(&font, &text, rect, rgb(248, 250, 255), false)?;
    }
    if width >= 320 && height >= 180 {
        let caption = format!(
            "{} · {} · {}",
            snapshot.source, snapshot.location, snapshot.observed
        );
        let region = background(width, height);
        let rect = Rect {
            x: region.x,
            y: region.bottom() + f64::from(height) * 0.015,
            w: region.w,
            h: (f64::from(height) * 0.025).min(region.h * 0.065).max(1.0),
        };
        let font = Font::fit(
            canvas.dc,
            FontMode::Consolas,
            None,
            &caption,
            rect.w,
            rect.h,
            rect.h * 0.90,
            true,
        )?;
        canvas.text(&font, &caption, rect, rgb(240, 245, 255), false)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_weather_backgrounds_decode_and_glass_stays_central() {
        for condition in Condition::ALL {
            let scene = compose(320, 180, condition).unwrap();
            assert_eq!(scene.pixels.bytes.len(), 320 * 180 * 4);
            let p = card(320, 180);
            assert!(p.x > 0.0 && p.y > 0.0 && p.right() < 320.0 && p.bottom() < 180.0);
        }
    }
}
