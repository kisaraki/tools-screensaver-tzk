//! Pure calendar and deadline calculations. No OS calls or timer-count arithmetic.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    TimeDate,
    Countdown,
    JapanTravel,
}

impl DisplayMode {
    pub const fn from_registry(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::TimeDate),
            1 => Some(Self::Countdown),
            2 => Some(Self::JapanTravel),
            _ => None,
        }
    }

    pub const fn registry_value(self) -> u32 {
        match self {
            Self::TimeDate => 0,
            Self::Countdown => 1,
            Self::JapanTravel => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontMode {
    SevenSegment,
    Consolas,
    MingLiu,
    Custom,
}

impl FontMode {
    pub const fn from_registry(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::SevenSegment),
            1 => Some(Self::Consolas),
            2 => Some(Self::MingLiu),
            3 => Some(Self::Custom),
            _ => None,
        }
    }

    pub const fn registry_value(self) -> u32 {
        match self {
            Self::SevenSegment => 0,
            Self::Consolas => 1,
            Self::MingLiu => 2,
            Self::Custom => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
}

impl LocalTime {
    pub const FIXTURE: Self = Self {
        year: 2023,
        month: 12,
        day: 31,
        hour: 12,
        minute: 15,
        second: 40,
    };
    pub fn valid(self) -> bool {
        (1..=9999).contains(&self.year)
            && self.day > 0
            && self.day <= days_in_month(self.year, self.month)
            && self.hour < 24
            && self.minute < 60
            && self.second < 60
    }
    pub fn hand_angles(self) -> [f64; 3] {
        [
            30.0 * (f64::from(self.hour % 12)
                + f64::from(self.minute) / 60.0
                + f64::from(self.second) / 3600.0),
            6.0 * (f64::from(self.minute) + f64::from(self.second) / 60.0),
            6.0 * f64::from(self.second),
        ]
    }
}

pub fn days_in_month(year: u16, month: u16) -> u16 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Calendar {
    pub days: u16,
    pub monday_offset: u16,
}
impl Calendar {
    pub fn new(year: u16, month: u16) -> Option<Self> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return None;
        }
        // Gregorian Sakamoto offsets; 0=Sunday, then rotate to Monday-first.
        const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let y = i32::from(year) - i32::from(month < 3);
        let sunday = (y + y / 4 - y / 100 + y / 400 + OFFSETS[usize::from(month - 1)] + 1) % 7;
        Some(Self {
            days: days_in_month(year, month),
            monday_offset: ((sunday + 6) % 7) as u16,
        })
    }
    pub fn cell(self, day: u16) -> Option<(u16, u16)> {
        if day == 0 || day > self.days {
            return None;
        }
        let index = self.monday_offset + day - 1;
        Some((index / 7, index % 7))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    Hours,
    Minutes,
    Seconds,
    Zero,
    Overflow,
}

pub fn parse_duration(hours: &str, minutes: &str, seconds: &str) -> Result<u32, InputError> {
    fn field(value: &str, max: u32, error: InputError) -> Result<u32, InputError> {
        if value.is_empty() || value.len() > 2 || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(error);
        }
        value
            .parse::<u32>()
            .ok()
            .filter(|&value| value <= max)
            .ok_or(error)
    }
    let total = field(hours, 99, InputError::Hours)? * 3600
        + field(minutes, 59, InputError::Minutes)? * 60
        + field(seconds, 59, InputError::Seconds)?;
    if total == 0 {
        Err(InputError::Zero)
    } else {
        Ok(total)
    }
}

pub fn hms(seconds: u32) -> [u32; 3] {
    [seconds / 3600, seconds % 3600 / 60, seconds % 60]
}

#[derive(Debug, Clone, Copy)]
pub struct Countdown {
    total_ms: u64,
    deadline_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CountdownFrame {
    pub remaining_ms: u64,
    pub display_seconds: u32,
    pub ratio: f64,
    pub final_ten: bool,
    pub dim: bool,
    pub animating: bool,
}

impl Countdown {
    pub fn new(seconds: u32, start: u64) -> Result<Self, InputError> {
        if seconds == 0 {
            return Err(InputError::Zero);
        }
        if seconds > 359999 {
            return Err(InputError::Overflow);
        }
        let total_ms = u64::from(seconds) * 1000;
        let deadline_ms = start.checked_add(total_ms).ok_or(InputError::Overflow)?;
        Ok(Self {
            total_ms,
            deadline_ms,
        })
    }
    pub fn frame(self, now: u64) -> CountdownFrame {
        let remaining_ms = self.deadline_ms.saturating_sub(now).min(self.total_ms);
        let elapsed = now.saturating_sub(self.deadline_ms);
        let phase = elapsed / 420;
        CountdownFrame {
            remaining_ms,
            display_seconds: remaining_ms.div_ceil(1000) as u32,
            ratio: remaining_ms as f64 / self.total_ms as f64,
            final_ten: remaining_ms > 0 && remaining_ms <= 10000,
            dim: remaining_ms == 0 && phase < 8 && phase % 2 == 1,
            animating: remaining_ms > 0 || elapsed < 3360,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameSnapshot {
    pub generation: u64,
    pub local: LocalTime,
    pub tick: u64,
    pub countdown: CountdownFrame,
}

#[derive(Debug, Clone, Copy)]
pub struct Timeline {
    mode: DisplayMode,
    countdown: Countdown,
    generation: u64,
}
impl Timeline {
    pub fn new(mode: DisplayMode, seconds: u32, start: u64) -> Result<Self, InputError> {
        Ok(Self {
            mode,
            countdown: Countdown::new(seconds, start)?,
            generation: 0,
        })
    }
    pub fn sample(&mut self, local: LocalTime, tick: u64) -> FrameSnapshot {
        self.generation = self.generation.wrapping_add(1);
        FrameSnapshot {
            generation: self.generation,
            local,
            tick,
            countdown: self.countdown.frame(tick),
        }
    }
    pub fn interval(self, frame: FrameSnapshot) -> u32 {
        if self.mode == DisplayMode::Countdown && frame.countdown.animating {
            100
        } else {
            1000
        }
    }
}

pub const SEGMENTS: [u8; 10] = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];

#[derive(Debug, Clone, Copy)]
pub struct Xorshift32(u32);
impl Xorshift32 {
    pub fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x6d2b79f5 } else { seed })
    }
    pub fn next_value(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}
