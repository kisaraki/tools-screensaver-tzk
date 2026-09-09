//! Validated settings and transactional registry persistence.
use std::fmt;

use windows_sys::Win32::System::Registry::{REG_BINARY, REG_DWORD};

pub use crate::font::{FontSpec, DEFAULT_POINT_SIZE_TENTH};
use crate::model::{DisplayMode, FontMode, TravelStyle};

pub const SCHEMA_VERSION: u32 = 8;
pub const DEFAULT_COUNTDOWN_SECONDS: u32 = 300;
pub const DEFAULT_TRAVEL_SWITCH_MINUTES: u32 = 1;
pub const MAX_TRAVEL_SWITCH_MINUTES: u32 = 1440;
pub const REG_BINARY_KIND: u32 = REG_BINARY;
pub const REG_DWORD_KIND: u32 = REG_DWORD;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPreset {
    DarkRed,
    DarkOrange,
    BrightGreen,
    OffWhite,
    MutedLightBlue,
    Amber,
    Auto,
    IronGray,
}

impl ColorPreset {
    pub const fn from_registry(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::DarkRed),
            1 => Some(Self::DarkOrange),
            2 => Some(Self::BrightGreen),
            3 => Some(Self::OffWhite),
            4 => Some(Self::MutedLightBlue),
            5 => Some(Self::Amber),
            6 => Some(Self::Auto),
            7 => Some(Self::IronGray),
            _ => None,
        }
    }

    pub const fn registry_value(self) -> u32 {
        match self {
            Self::DarkRed => 0,
            Self::DarkOrange => 1,
            Self::BrightGreen => 2,
            Self::OffWhite => 3,
            Self::MutedLightBlue => 4,
            Self::Amber => 5,
            Self::Auto => 6,
            Self::IronGray => 7,
        }
    }
}

const SCHEMA: &str = "SchemaVersion";
const DISPLAY_MODE: &str = "DisplayMode";
const TRAVEL_STYLE: &str = "TravelStyle";
const TRAVEL_SWITCH_MINUTES: &str = "TravelSwitchMinutes";
const COLOR_PRESET: &str = "ColorPreset";
const FONT_MODE: &str = "FontMode";
const CUSTOM_LOGFONT: &str = "CustomLogFont";
const CUSTOM_POINT_SIZE: &str = "CustomPointSizeTenth";
const LAST_COUNTDOWN: &str = "LastCountdownDurationSeconds";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawValue {
    pub kind: u32,
    pub bytes: Vec<u8>,
}

impl RawValue {
    pub fn dword(value: u32) -> Self {
        Self {
            kind: REG_DWORD_KIND,
            bytes: value.to_le_bytes().to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreError {
    pub operation: &'static str,
    pub code: u32,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} failed (Win32/store error {})",
            self.operation, self.code
        )
    }
}

pub trait SettingsStore {
    fn get(&self, name: &str) -> Result<Option<RawValue>, StoreError>;
    fn set(&mut self, name: &str, value: &RawValue) -> Result<(), StoreError>;
    fn delete(&mut self, name: &str) -> Result<(), StoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppConfig {
    pub display_mode: DisplayMode,
    pub travel_style: TravelStyle,
    /// Zero disables scheduled source changes; errors still trigger recovery.
    pub travel_switch_minutes: u32,
    pub color_preset: ColorPreset,
    pub font_mode: FontMode,
    pub custom_font: Option<FontSpec>,
    pub last_countdown_seconds: u32,
    pub schema_version: u32,
    pub future_schema: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::TimeDate,
            travel_style: TravelStyle::FreeFlight,
            travel_switch_minutes: DEFAULT_TRAVEL_SWITCH_MINUTES,
            color_preset: ColorPreset::BrightGreen,
            font_mode: FontMode::SevenSegment,
            custom_font: None,
            last_countdown_seconds: DEFAULT_COUNTDOWN_SECONDS,
            schema_version: SCHEMA_VERSION,
            future_schema: false,
        }
    }
}

impl AppConfig {
    pub fn effective_font_mode(self) -> FontMode {
        if self.font_mode == FontMode::Custom && self.custom_font.is_none() {
            FontMode::SevenSegment
        } else {
            self.font_mode
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigDraft {
    pub display_mode: DisplayMode,
    pub travel_style: TravelStyle,
    pub travel_switch_minutes: u32,
    pub color_preset: ColorPreset,
    pub font_mode: FontMode,
    pub custom_font: Option<FontSpec>,
}

impl From<AppConfig> for ConfigDraft {
    fn from(value: AppConfig) -> Self {
        Self {
            display_mode: value.display_mode,
            travel_style: value.travel_style,
            travel_switch_minutes: value.travel_switch_minutes,
            color_preset: value.color_preset,
            font_mode: value.font_mode,
            custom_font: value.custom_font,
        }
    }
}

impl ConfigDraft {
    pub fn validate(self) -> Result<Self, SaveError> {
        if self.travel_switch_minutes > MAX_TRAVEL_SWITCH_MINUTES {
            Err(SaveError::InvalidTravelSwitchMinutes)
        } else if self.font_mode == FontMode::Custom && self.custom_font.is_none() {
            Err(SaveError::InvalidDraft)
        } else {
            Ok(self)
        }
    }
}

/// Parse the enabled minutes field without accepting signs, whitespace, fractions,
/// localized digits, or zero. The dialog's separate option represents no switching.
pub(crate) fn parse_travel_switch_minutes(text: &str) -> Option<u32> {
    if text.is_empty() || text.len() > 4 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u32>()
        .ok()
        .filter(|value| (1..=MAX_TRAVEL_SWITCH_MINUTES).contains(value))
}

fn dword(value: Option<RawValue>) -> Option<u32> {
    let value = value?;
    if value.kind != REG_DWORD_KIND {
        return None;
    }
    let bytes: [u8; 4] = value.bytes.as_slice().try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn get(store: &impl SettingsStore, name: &str) -> Option<RawValue> {
    store.get(name).ok().flatten()
}

pub fn load(store: &impl SettingsStore) -> AppConfig {
    let schema_value = get(store, SCHEMA);
    let schema = dword(schema_value.clone());
    if schema_value.is_some() && schema.is_none() || schema == Some(0) {
        return AppConfig::default();
    }
    let schema_version = schema.unwrap_or(SCHEMA_VERSION);
    let display_mode = dword(get(store, DISPLAY_MODE))
        .and_then(DisplayMode::from_registry)
        .filter(|mode| {
            *mode != DisplayMode::JapanTravel || schema.is_some_and(|version| version >= 3)
        })
        .unwrap_or(DisplayMode::TimeDate);
    let travel_style = dword(get(store, TRAVEL_STYLE))
        .and_then(TravelStyle::from_registry)
        .filter(|style| {
            schema.is_some_and(|version| {
                version >= 4
                    && (*style != TravelStyle::JapaneseInn || version >= 6)
                    && (!matches!(style, TravelStyle::TrainCab | TravelStyle::Walking)
                        || version >= 7)
            })
        })
        .unwrap_or(TravelStyle::FreeFlight);
    let travel_switch_minutes = dword(get(store, TRAVEL_SWITCH_MINUTES))
        .filter(|_| schema.is_some_and(|version| version >= 5))
        .filter(|value| *value <= MAX_TRAVEL_SWITCH_MINUTES)
        .unwrap_or(DEFAULT_TRAVEL_SWITCH_MINUTES);
    let color_preset = dword(get(store, COLOR_PRESET))
        .and_then(ColorPreset::from_registry)
        .filter(|preset| match preset {
            ColorPreset::MutedLightBlue | ColorPreset::Amber | ColorPreset::Auto => {
                schema.is_some_and(|version| version >= 6)
            }
            ColorPreset::IronGray => schema.is_some_and(|version| version >= 8),
            _ => true,
        })
        .unwrap_or(ColorPreset::BrightGreen);
    let font_mode = dword(get(store, FONT_MODE))
        .and_then(FontMode::from_registry)
        .unwrap_or(FontMode::SevenSegment);
    let custom_font = match (
        get(store, CUSTOM_LOGFONT),
        dword(get(store, CUSTOM_POINT_SIZE)),
    ) {
        (Some(raw), Some(points)) if raw.kind == REG_BINARY_KIND => {
            FontSpec::from_registry(&raw.bytes, points)
        }
        _ => None,
    };
    let last_countdown_seconds = dword(get(store, LAST_COUNTDOWN))
        .filter(|&value| (1..=359999).contains(&value))
        .unwrap_or(DEFAULT_COUNTDOWN_SECONDS);
    AppConfig {
        display_mode,
        travel_style,
        travel_switch_minutes,
        color_preset,
        font_mode,
        custom_font,
        last_countdown_seconds,
        schema_version,
        future_schema: schema_version > SCHEMA_VERSION,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    FutureSchema(u32),
    InvalidDraft,
    InvalidTravelSwitchMinutes,
    Store(StoreError),
    Rollback {
        write: StoreError,
        rollback: StoreError,
    },
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FutureSchema(version) => write!(f, "設定版本 {version} 較新，本版不能覆寫。"),
            Self::InvalidDraft => f.write_str("請先選擇有效的自訂系統字型。"),
            Self::InvalidTravelSwitchMinutes => {
                f.write_str("來源切換時間請選擇不切換，或輸入 1～1440 的整數分鐘。")
            }
            Self::Store(error) => write!(f, "無法保存設定：{error}"),
            Self::Rollback { write, rollback } => write!(
                f,
                "保存失敗且還原也失敗，部分設定可能已變更：{write}；{rollback}"
            ),
        }
    }
}

fn current_schema(store: &impl SettingsStore) -> Result<Option<u32>, SaveError> {
    let raw = store.get(SCHEMA).map_err(SaveError::Store)?;
    Ok(dword(raw))
}

fn transaction(
    store: &mut impl SettingsStore,
    updates: Vec<(&'static str, Option<RawValue>)>,
) -> Result<(), SaveError> {
    if let Some(version) = current_schema(store)? {
        if version > SCHEMA_VERSION {
            return Err(SaveError::FutureSchema(version));
        }
    }
    let mut snapshots = Vec::with_capacity(updates.len());
    for (name, _) in &updates {
        snapshots.push((*name, store.get(name).map_err(SaveError::Store)?));
    }
    for (applied, (name, value)) in updates.iter().enumerate() {
        let result = match value {
            Some(value) => store.set(name, value),
            None => store.delete(name),
        };
        if let Err(write) = result {
            let mut rollback_error = None;
            for (old_name, old_value) in snapshots[..applied].iter().rev() {
                let result = match old_value {
                    Some(value) => store.set(old_name, value),
                    None => store.delete(old_name),
                };
                if let Err(error) = result {
                    rollback_error.get_or_insert(error);
                }
            }
            return Err(match rollback_error {
                Some(rollback) => SaveError::Rollback { write, rollback },
                None => SaveError::Store(write),
            });
        }
    }
    Ok(())
}

pub fn save_draft(store: &mut impl SettingsStore, draft: ConfigDraft) -> Result<(), SaveError> {
    let draft = draft.validate()?;
    let mut updates = vec![
        (
            DISPLAY_MODE,
            Some(RawValue::dword(draft.display_mode.registry_value())),
        ),
        (
            TRAVEL_STYLE,
            Some(RawValue::dword(draft.travel_style.registry_value())),
        ),
        (
            TRAVEL_SWITCH_MINUTES,
            Some(RawValue::dword(draft.travel_switch_minutes)),
        ),
        (
            COLOR_PRESET,
            Some(RawValue::dword(draft.color_preset.registry_value())),
        ),
        (
            FONT_MODE,
            Some(RawValue::dword(draft.font_mode.registry_value())),
        ),
    ];
    if let Some(font) = draft.custom_font {
        updates.push((
            CUSTOM_LOGFONT,
            Some(RawValue {
                kind: REG_BINARY_KIND,
                bytes: font.registry_bytes(),
            }),
        ));
        updates.push((
            CUSTOM_POINT_SIZE,
            Some(RawValue::dword(font.point_size_tenth())),
        ));
    }
    updates.push((SCHEMA, Some(RawValue::dword(SCHEMA_VERSION))));
    transaction(store, updates)
}

pub fn save_countdown(store: &mut impl SettingsStore, seconds: u32) -> Result<(), SaveError> {
    if !(1..=359999).contains(&seconds) {
        return Err(SaveError::InvalidDraft);
    }
    let mut updates = vec![(LAST_COUNTDOWN, Some(RawValue::dword(seconds)))];
    if current_schema(store)?.is_none_or(|version| version < 5) {
        // A countdown-only save also advances the schema. Preserve the legacy
        // one-minute behavior instead of activating an old unrecognized value.
        updates.push((
            TRAVEL_SWITCH_MINUTES,
            Some(RawValue::dword(DEFAULT_TRAVEL_SWITCH_MINUTES)),
        ));
    }
    updates.push((SCHEMA, Some(RawValue::dword(SCHEMA_VERSION))));
    transaction(store, updates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RegistryStore;
    use std::collections::BTreeMap;
    use std::mem::size_of;
    use windows_sys::Win32::Graphics::Gdi::LOGFONTW;
    use windows_sys::Win32::System::Registry::{RegDeleteTreeW, HKEY_CURRENT_USER};

    #[derive(Default)]
    struct MemoryStore {
        values: BTreeMap<String, RawValue>,
        calls: usize,
        fail_at: Option<usize>,
        fail_rollback_at: Option<usize>,
        rolling_back: bool,
    }

    struct RegistryTestKey {
        path: String,
    }

    impl RegistryTestKey {
        fn new(path: String) -> Self {
            let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
            // SAFETY: The unique path is terminated; deleting an absent prior test key is fine.
            unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide.as_ptr()) };
            Self { path }
        }
    }

    impl Drop for RegistryTestKey {
        fn drop(&mut self) {
            let wide: Vec<u16> = self.path.encode_utf16().chain([0]).collect();
            // SAFETY: The guard deletes only its unique test subkey.
            unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide.as_ptr()) };
        }
    }
    impl SettingsStore for MemoryStore {
        fn get(&self, name: &str) -> Result<Option<RawValue>, StoreError> {
            Ok(self.values.get(name).cloned())
        }
        fn set(&mut self, name: &str, value: &RawValue) -> Result<(), StoreError> {
            self.calls += 1;
            let target = if self.rolling_back {
                self.fail_rollback_at
            } else {
                self.fail_at
            };
            if target == Some(self.calls) {
                self.rolling_back = true;
                return Err(StoreError {
                    operation: "injected set",
                    code: self.calls as u32,
                });
            }
            self.values.insert(name.to_owned(), value.clone());
            Ok(())
        }
        fn delete(&mut self, name: &str) -> Result<(), StoreError> {
            self.calls += 1;
            if self.rolling_back && self.fail_rollback_at == Some(self.calls) {
                return Err(StoreError {
                    operation: "injected delete",
                    code: self.calls as u32,
                });
            }
            self.values.remove(name);
            Ok(())
        }
    }

    fn valid_font() -> FontSpec {
        FontSpec::default_choice()
    }

    #[test]
    fn every_enum_registry_value_round_trips_and_rejects_out_of_range() {
        for (raw, expected) in [
            (0, DisplayMode::TimeDate),
            (1, DisplayMode::Countdown),
            (2, DisplayMode::JapanTravel),
        ] {
            assert_eq!(DisplayMode::from_registry(raw), Some(expected));
            assert_eq!(expected.registry_value(), raw);
        }
        assert_eq!(DisplayMode::from_registry(3), None);

        for (raw, expected) in [
            (0, TravelStyle::FreeFlight),
            (1, TravelStyle::TrainJourney),
            (2, TravelStyle::JapaneseInn),
            (3, TravelStyle::TrainCab),
            (4, TravelStyle::Walking),
        ] {
            assert_eq!(TravelStyle::from_registry(raw), Some(expected));
            assert_eq!(expected.registry_value(), raw);
        }
        assert_eq!(TravelStyle::from_registry(5), None);

        for (raw, expected) in [
            (0, ColorPreset::DarkRed),
            (1, ColorPreset::DarkOrange),
            (2, ColorPreset::BrightGreen),
            (3, ColorPreset::OffWhite),
            (4, ColorPreset::MutedLightBlue),
            (5, ColorPreset::Amber),
            (6, ColorPreset::Auto),
            (7, ColorPreset::IronGray),
        ] {
            assert_eq!(ColorPreset::from_registry(raw), Some(expected));
            assert_eq!(expected.registry_value(), raw);
        }
        assert_eq!(ColorPreset::from_registry(8), None);

        for (raw, expected) in [
            (0, FontMode::SevenSegment),
            (1, FontMode::Consolas),
            (2, FontMode::MingLiu),
            (3, FontMode::Custom),
        ] {
            assert_eq!(FontMode::from_registry(raw), Some(expected));
            assert_eq!(expected.registry_value(), raw);
        }
        assert_eq!(FontMode::from_registry(4), None);
    }

    #[test]
    fn malformed_values_fall_back_per_field() {
        let mut store = MemoryStore::default();
        store.values.insert(SCHEMA.into(), RawValue::dword(2));
        store.values.insert(
            DISPLAY_MODE.into(),
            RawValue {
                kind: REG_BINARY_KIND,
                bytes: 1u32.to_le_bytes().to_vec(),
            },
        );
        store.values.insert(
            COLOR_PRESET.into(),
            RawValue {
                kind: REG_DWORD_KIND,
                bytes: vec![2, 0, 0],
            },
        );
        store.values.insert(FONT_MODE.into(), RawValue::dword(2));
        store.values.insert(
            CUSTOM_LOGFONT.into(),
            RawValue {
                kind: REG_BINARY_KIND,
                bytes: vec![0; 91],
            },
        );
        store
            .values
            .insert(CUSTOM_POINT_SIZE.into(), RawValue::dword(480));
        store
            .values
            .insert(LAST_COUNTDOWN.into(), RawValue::dword(360000));
        let config = load(&store);
        assert_eq!(config.display_mode, DisplayMode::TimeDate);
        assert_eq!(config.color_preset, ColorPreset::BrightGreen);
        assert_eq!(config.font_mode, FontMode::MingLiu);
        assert!(config.custom_font.is_none());
        assert_eq!(config.last_countdown_seconds, 300);
    }

    #[test]
    fn schema_states_and_future_write_protection() {
        for schema in [
            None,
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            Some(8),
        ] {
            let mut store = MemoryStore::default();
            if let Some(schema) = schema {
                store.values.insert(SCHEMA.into(), RawValue::dword(schema));
            }
            store.values.insert(DISPLAY_MODE.into(), RawValue::dword(1));
            assert_eq!(load(&store).display_mode, DisplayMode::Countdown);
        }
        let mut legacy = MemoryStore::default();
        legacy.values.insert(SCHEMA.into(), RawValue::dword(2));
        legacy
            .values
            .insert(DISPLAY_MODE.into(), RawValue::dword(2));
        assert_eq!(load(&legacy).display_mode, DisplayMode::TimeDate);
        legacy.values.insert(SCHEMA.into(), RawValue::dword(3));
        assert_eq!(load(&legacy).display_mode, DisplayMode::JapanTravel);
        legacy
            .values
            .insert(TRAVEL_STYLE.into(), RawValue::dword(1));
        assert_eq!(load(&legacy).travel_style, TravelStyle::FreeFlight);
        legacy.values.insert(SCHEMA.into(), RawValue::dword(4));
        assert_eq!(load(&legacy).travel_style, TravelStyle::TrainJourney);
        legacy
            .values
            .insert(TRAVEL_STYLE.into(), RawValue::dword(2));
        assert_eq!(load(&legacy).travel_style, TravelStyle::FreeFlight);
        legacy
            .values
            .insert(COLOR_PRESET.into(), RawValue::dword(6));
        assert_eq!(load(&legacy).color_preset, ColorPreset::BrightGreen);
        legacy.values.insert(SCHEMA.into(), RawValue::dword(6));
        assert_eq!(load(&legacy).travel_style, TravelStyle::JapaneseInn);
        assert_eq!(load(&legacy).color_preset, ColorPreset::Auto);
        legacy
            .values
            .insert(TRAVEL_STYLE.into(), RawValue::dword(4));
        assert_eq!(load(&legacy).travel_style, TravelStyle::FreeFlight);
        legacy.values.insert(SCHEMA.into(), RawValue::dword(7));
        assert_eq!(load(&legacy).travel_style, TravelStyle::Walking);
        legacy
            .values
            .insert(COLOR_PRESET.into(), RawValue::dword(7));
        assert_eq!(load(&legacy).color_preset, ColorPreset::BrightGreen);
        legacy.values.insert(SCHEMA.into(), RawValue::dword(8));
        assert_eq!(load(&legacy).color_preset, ColorPreset::IronGray);
        let mut future = MemoryStore::default();
        future.values.insert(SCHEMA.into(), RawValue::dword(9));
        future
            .values
            .insert(COLOR_PRESET.into(), RawValue::dword(1));
        assert!(load(&future).future_schema);
        assert_eq!(load(&future).color_preset, ColorPreset::DarkOrange);
        assert_eq!(
            save_countdown(&mut future, 5),
            Err(SaveError::FutureSchema(9))
        );
        assert_eq!(dword(future.values.get(SCHEMA).cloned()), Some(9));

        for broken_schema in [
            RawValue::dword(0),
            RawValue {
                kind: REG_BINARY_KIND,
                bytes: 2u32.to_le_bytes().to_vec(),
            },
        ] {
            let mut broken = MemoryStore::default();
            broken.values.insert(SCHEMA.into(), broken_schema);
            broken
                .values
                .insert(DISPLAY_MODE.into(), RawValue::dword(1));
            broken
                .values
                .insert(COLOR_PRESET.into(), RawValue::dword(1));
            assert_eq!(load(&broken), AppConfig::default());
        }
    }

    #[test]
    fn custom_font_binary_and_fields_are_validated_and_normalized() {
        assert_eq!(size_of::<LOGFONTW>(), 92);
        let font = valid_font();
        assert_eq!(
            FontSpec::from_registry(&font.registry_bytes(), 180),
            FontSpec::from_logfont(font.logfont(1), 180)
        );
        assert!(FontSpec::from_registry(&font.registry_bytes()[..91], 480).is_none());
        assert!(FontSpec::from_registry(&font.registry_bytes(), 179).is_none());
        assert!(FontSpec::from_registry(&font.registry_bytes(), 2400).is_some());
        assert!(FontSpec::from_registry(&font.registry_bytes(), 2401).is_none());
        let mut no_nul = font.logfont(1);
        no_nul.lfFaceName.fill(b'A' as u16);
        assert!(FontSpec::from_logfont(no_nul, 480).is_none());
        let mut invalid_utf16 = font.logfont(1);
        invalid_utf16.lfFaceName = [0; 32];
        invalid_utf16.lfFaceName[0] = 0xd800;
        invalid_utf16.lfFaceName[1] = 0;
        assert!(FontSpec::from_logfont(invalid_utf16, 480).is_none());
        let mut unsafe_height = font.logfont(1);
        unsafe_height.lfHeight = i32::MIN;
        assert!(FontSpec::from_logfont(unsafe_height, 480).is_none());
        let normalized = font.logfont(123);
        assert_eq!(normalized.lfHeight, -123);
        assert_eq!(
            (
                normalized.lfWidth,
                normalized.lfEscapement,
                normalized.lfOrientation
            ),
            (0, 0, 0)
        );
        let mut unknown_sdk_fields = font.logfont(1);
        unknown_sdk_fields.lfCharSet = 0xfe;
        unknown_sdk_fields.lfOutPrecision = 0xfe;
        unknown_sdk_fields.lfClipPrecision = 0x0f;
        unknown_sdk_fields.lfQuality = 0xfe;
        unknown_sdk_fields.lfPitchAndFamily = 0xfe;
        let normalized = FontSpec::from_logfont(unknown_sdk_fields, 480)
            .expect("unknown SDK fields use safe defaults")
            .logfont(10);
        assert_eq!(
            normalized.lfCharSet,
            windows_sys::Win32::Graphics::Gdi::DEFAULT_CHARSET
        );
        assert_eq!(normalized.lfOutPrecision, 0);
        assert_eq!(normalized.lfClipPrecision, 0);
        assert_eq!(normalized.lfQuality, 0);
        assert_eq!(normalized.lfPitchAndFamily, 0);

        let missing = AppConfig {
            font_mode: FontMode::Custom,
            custom_font: None,
            ..AppConfig::default()
        };
        assert_eq!(missing.effective_font_mode(), FontMode::SevenSegment);
        let valid = AppConfig {
            font_mode: FontMode::Custom,
            custom_font: Some(font),
            ..AppConfig::default()
        };
        assert_eq!(valid.effective_font_mode(), FontMode::Custom);
    }

    #[test]
    fn travel_switch_migration_and_malformed_values_keep_the_one_minute_default() {
        for schema in [None, Some(1), Some(2), Some(3), Some(4)] {
            let mut store = MemoryStore::default();
            if let Some(version) = schema {
                store.values.insert(SCHEMA.into(), RawValue::dword(version));
            }
            // A value under a legacy schema must not change that schema's behavior.
            store
                .values
                .insert(TRAVEL_SWITCH_MINUTES.into(), RawValue::dword(0));
            let before = store.values.clone();
            assert_eq!(load(&store).travel_switch_minutes, 1);
            assert_eq!(store.values, before);
            save_countdown(&mut store, 90).unwrap();
            assert_eq!(load(&store).travel_switch_minutes, 1);
            assert_eq!(load(&store).schema_version, SCHEMA_VERSION);
        }
        for value in [
            None,
            Some(RawValue::dword(1441)),
            Some(RawValue::dword(u32::MAX)),
            Some(RawValue {
                kind: REG_BINARY_KIND,
                bytes: vec![0, 0, 0, 0],
            }),
            Some(RawValue {
                kind: REG_DWORD_KIND,
                bytes: vec![0, 0, 0],
            }),
        ] {
            let mut store = MemoryStore::default();
            store
                .values
                .insert(SCHEMA.into(), RawValue::dword(SCHEMA_VERSION));
            if let Some(value) = value {
                store.values.insert(TRAVEL_SWITCH_MINUTES.into(), value);
            }
            assert_eq!(load(&store).travel_switch_minutes, 1);
        }
    }

    #[test]
    fn travel_switch_boundaries_round_trip_and_invalid_drafts_do_not_write() {
        for minutes in [0, 1, 15, 1440] {
            let mut store = MemoryStore::default();
            let mut draft = ConfigDraft::from(AppConfig::default());
            draft.travel_switch_minutes = minutes;
            save_draft(&mut store, draft).unwrap();
            assert_eq!(load(&store).travel_switch_minutes, minutes);
            assert_eq!(
                dword(store.values.get(SCHEMA).cloned()),
                Some(SCHEMA_VERSION)
            );
            save_countdown(&mut store, 90).unwrap();
            assert_eq!(load(&store).travel_switch_minutes, minutes);
        }
        for minutes in [1441, u32::MAX] {
            let mut store = MemoryStore::default();
            let mut draft = ConfigDraft::from(AppConfig::default());
            draft.travel_switch_minutes = minutes;
            assert_eq!(
                save_draft(&mut store, draft),
                Err(SaveError::InvalidTravelSwitchMinutes)
            );
            assert_eq!(store.calls, 0);
            assert!(store.values.is_empty());
        }
    }

    #[test]
    fn travel_switch_text_accepts_only_bounded_whole_minutes() {
        for (text, minutes) in [("1", 1), ("01", 1), ("30", 30), ("1440", 1440)] {
            assert_eq!(parse_travel_switch_minutes(text), Some(minutes));
        }
        for text in [
            "", "0", "0000", "1441", "9999", "12345", "1.5", "-1", "+1", " 1", "1 ", "１", "1\n",
        ] {
            assert_eq!(parse_travel_switch_minutes(text), None, "{text:?}");
        }
    }

    #[test]
    fn transactions_preserve_unknown_values_and_restore_on_failure() {
        let mut store = MemoryStore::default();
        store.values.insert("Unknown".into(), RawValue::dword(77));
        store.values.insert(COLOR_PRESET.into(), RawValue::dword(1));
        let before = store.values.clone();
        store.fail_at = Some(3);
        let draft = ConfigDraft {
            display_mode: DisplayMode::Countdown,
            travel_style: TravelStyle::TrainJourney,
            travel_switch_minutes: 15,
            color_preset: ColorPreset::OffWhite,
            font_mode: FontMode::Consolas,
            custom_font: Some(valid_font()),
        };
        assert!(matches!(
            save_draft(&mut store, draft),
            Err(SaveError::Store(_))
        ));
        assert_eq!(store.values, before);
        assert_eq!(dword(store.values.get("Unknown").cloned()), Some(77));
    }

    #[test]
    fn rollback_failure_is_distinct_and_countdown_preserves_unrelated_fields() {
        let mut store = MemoryStore::default();
        store.values.insert(DISPLAY_MODE.into(), RawValue::dword(1));
        store.values.insert(COLOR_PRESET.into(), RawValue::dword(3));
        save_countdown(&mut store, 5).unwrap();
        assert_eq!(dword(store.values.get(LAST_COUNTDOWN).cloned()), Some(5));
        assert_eq!(dword(store.values.get(DISPLAY_MODE).cloned()), Some(1));
        assert_eq!(dword(store.values.get(COLOR_PRESET).cloned()), Some(3));

        let mut failing = MemoryStore::default();
        failing
            .values
            .insert(DISPLAY_MODE.into(), RawValue::dword(0));
        failing.fail_at = Some(3);
        failing.fail_rollback_at = Some(4);
        let error = save_draft(
            &mut failing,
            ConfigDraft {
                display_mode: DisplayMode::Countdown,
                travel_style: TravelStyle::FreeFlight,
                travel_switch_minutes: 1,
                color_preset: ColorPreset::DarkOrange,
                font_mode: FontMode::MingLiu,
                custom_font: None,
            },
        )
        .unwrap_err();
        assert!(matches!(error, SaveError::Rollback { .. }));
    }

    #[test]
    fn invalid_or_cancelled_drafts_do_not_write() {
        let mut store = MemoryStore::default();
        let invalid = ConfigDraft {
            display_mode: DisplayMode::TimeDate,
            travel_style: TravelStyle::FreeFlight,
            travel_switch_minutes: 1,
            color_preset: ColorPreset::BrightGreen,
            font_mode: FontMode::Custom,
            custom_font: None,
        };
        assert_eq!(
            save_draft(&mut store, invalid),
            Err(SaveError::InvalidDraft)
        );
        assert_eq!(store.calls, 0);
        let draft = ConfigDraft::from(AppConfig::default());
        let cancelled_choose_font: Option<FontSpec> = None;
        assert!(cancelled_choose_font.is_none());
        assert_eq!(draft, ConfigDraft::from(AppConfig::default()));
        assert_eq!(store.calls, 0);
    }

    #[test]
    fn actual_registry_adapter_uses_only_a_dedicated_test_subkey() {
        let path = format!(
            "Software\\Classes\\tools-screensaver-tzk.Test.{}",
            std::process::id()
        );
        {
            let cleanup = RegistryTestKey::new(path.clone());
            let mut store = RegistryStore::at(&cleanup.path);
            let draft = ConfigDraft {
                display_mode: DisplayMode::Countdown,
                travel_style: TravelStyle::TrainJourney,
                travel_switch_minutes: 30,
                color_preset: ColorPreset::DarkRed,
                font_mode: FontMode::Consolas,
                custom_font: Some(valid_font()),
            };
            save_draft(&mut store, draft).unwrap();
            save_countdown(&mut store, 359999).unwrap();
            let loaded = load(&store);
            assert_eq!(loaded.display_mode, DisplayMode::Countdown);
            assert_eq!(loaded.travel_style, TravelStyle::TrainJourney);
            assert_eq!(loaded.travel_switch_minutes, 30);
            assert_eq!(loaded.color_preset, ColorPreset::DarkRed);
            assert_eq!(loaded.font_mode, FontMode::Consolas);
            assert_eq!(loaded.last_countdown_seconds, 359999);
        }
        assert!(RegistryStore::at(&path).get(SCHEMA).unwrap().is_none());
    }
}
