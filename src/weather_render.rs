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
    let side = (w * 0.30).min(background(width, height).h * 0.84).max(1.0);
    Rect {
        x: (w - side) / 2.0,
        y: (h - side) / 2.0,
        w: side,
        h: side,
    }
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
            let nearest_x = xf.clamp(panel.x + radius, panel.right() - radius);
            let nearest_y = yf.clamp(panel.y + radius, panel.bottom() - radius);
            let outside = ((xf - nearest_x).powi(2) + (yf - nearest_y).powi(2)).sqrt() - radius;
            if outside <= 0.0 {
                let blur = (panel.w * 0.012).max(1.0) as i32;
                let samples = [
                    sample(x - blur, y - blur),
                    sample(x + blur, y - blur),
                    sample(x - blur, y + blur),
                    sample(x + blur, y + blur),
                ];
                let edge = (1.0 - (-outside / 3.0).clamp(0.0, 1.0)) * 0.36;
                let shine = (1.0 - (yf - panel.y) / panel.h).powi(4) * 0.18 + edge;
                for c in 0..3 {
                    let average = samples.iter().map(|p| f64::from(p[c])).sum::<f64>() / 4.0;
                    pixel[c] =
                        (average * 0.60 + 24.0 + (255.0 - average) * shine).clamp(0.0, 255.0) as u8;
                }
            } else if outside < 8.0 && yf > panel.y {
                for channel in &mut pixel[..3] {
                    *channel =
                        (f64::from(*channel) * (0.75 + outside / 32.0)).clamp(0.0, 255.0) as u8;
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
            FontMode::MingLiu,
            None,
            &text,
            rect.w,
            rect.h,
            rect.h * 0.90,
            true,
        )?;
        canvas.text(&font, &text, rect, rgb(248, 250, 255), true)?;
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
            FontMode::MingLiu,
            None,
            &caption,
            rect.w,
            rect.h,
            rect.h * 0.90,
            true,
        )?;
        canvas.text(&font, &caption, rect, rgb(240, 245, 255), true)?;
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
