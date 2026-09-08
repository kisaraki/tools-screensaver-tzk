//! Runtime source discovery and the isolated WebView shell for Japan travel mode.
//!
//! Network calls in this module are synchronous by design. The window layer must
//! run them on a worker thread and must keep all WebView2 COM objects on the UI
//! thread.

use std::{
    fmt,
    mem::{size_of, size_of_val},
    ptr, str,
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Foundation::GetLastError,
    Networking::WinHttp::{
        WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders,
        WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetOption,
        WinHttpSetTimeouts, INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
        WINHTTP_DECOMPRESSION_FLAG_DEFLATE, WINHTTP_DECOMPRESSION_FLAG_GZIP, WINHTTP_FLAG_SECURE,
        WINHTTP_OPTION_DECOMPRESSION, WINHTTP_OPTION_REDIRECT_POLICY,
        WINHTTP_OPTION_REDIRECT_POLICY_NEVER, WINHTTP_QUERY_CONTENT_TYPE,
        WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
    },
};

use crate::model::{TravelStyle, Xorshift32};

const SOURCE_HOST: &str = "tw.live";
const CAMERA_PATH_PREFIX: &str = "/cam/?id=";
const USER_AGENT: &str = "tools-screensaver-tzk/0.4 (Windows 10; Japan travel mode)";
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const TOTAL_TIMEOUT: Duration = Duration::from_secs(15);
const IO_TIMEOUT_MS: i32 = 4_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CameraSeed {
    pub camera_id: &'static str,
    pub place_hint: &'static str,
}

/// Stable tw.live camera identifiers are seeds only. Their YouTube video IDs
/// are deliberately resolved again whenever the source is selected because a
/// live broadcast can be restarted under a new video ID.
pub(crate) const CAMERA_SEEDS: [CameraSeed; 8] = [
    CameraSeed {
        camera_id: "sapporostationhbc",
        place_hint: "北海道・札幌",
    },
    CameraSeed {
        camera_id: "okutamastationview",
        place_hint: "東京・奧多摩",
    },
    CameraSeed {
        camera_id: "jpkyotokarasumadorinakagyo",
        place_hint: "京都・中京區",
    },
    CameraSeed {
        camera_id: "osakahanatencam",
        place_hint: "大阪・JR 放出車站",
    },
    CameraSeed {
        camera_id: "miyajimacamera",
        place_hint: "廣島・嚴島宮島",
    },
    CameraSeed {
        camera_id: "kariyushibeachnago",
        place_hint: "沖繩・名護嘉利吉海灘",
    },
    CameraSeed {
        camera_id: "sakurajimatarumizu",
        place_hint: "鹿兒島・垂水櫻島",
    },
    CameraSeed {
        camera_id: "jpkamikochitaishoilakealps",
        place_hint: "長野・上高地大正池",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TravelSource {
    pub camera_id: String,
    pub place: String,
    pub youtube_id: String,
    pub playlist_id: Option<String>,
}

impl TravelSource {
    pub fn embed_url(&self) -> String {
        match self.playlist_id.as_deref() {
            Some(playlist) => format!(
                "https://www.youtube-nocookie.com/embed/videoseries?list={playlist}&autoplay=1&mute=1&playsinline=1&rel=0&controls=1&disablekb=1&fs=0&enablejsapi=1"
            ),
            None => format!(
                "https://www.youtube-nocookie.com/embed/{}?autoplay=1&mute=1&playsinline=1&rel=0&controls=1&disablekb=1&fs=0&enablejsapi=1",
                self.youtube_id
            ),
        }
    }
}

/// Playlist-backed scenes let the official iframe player retrieve the current
/// playlist at fullscreen startup. This avoids an API key and HTML scraping.
pub(crate) fn playlist_source(style: TravelStyle) -> Option<TravelSource> {
    let (playlist_id, place) = match style {
        TravelStyle::FreeFlight => ("PLdsqwBj2O1Nw", "自在飛行播放清單"),
        TravelStyle::TrainJourney => ("PLBH60D9AGfu0", "列車旅行播放清單"),
        TravelStyle::TrainCab => ("PLB-Fmt68BNm4", "列車駕駛前方播放清單"),
        TravelStyle::Walking => ("PLbYZr39owNGo", "散步播放清單"),
        TravelStyle::JapaneseInn => return None,
    };
    Some(TravelSource {
        camera_id: playlist_id.into(),
        place: place.into(),
        youtube_id: String::new(),
        playlist_id: Some(playlist_id.into()),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TravelError {
    InvalidCameraId,
    Http { operation: &'static str, code: u32 },
    HttpStatus(u32),
    UnexpectedContentType(String),
    ResponseTooLarge,
    ResponseTimedOut,
    InvalidUtf8,
    MissingPlace,
    MissingYouTubeSource,
    InvalidCatalog,
}

impl fmt::Display for TravelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCameraId => f.write_str("invalid tw.live camera identifier"),
            Self::Http { operation, code } => {
                write!(f, "{operation} failed (WinHTTP error {code})")
            }
            Self::HttpStatus(status) => write!(f, "tw.live returned HTTP status {status}"),
            Self::UnexpectedContentType(value) => {
                write!(f, "tw.live returned an unexpected content type: {value}")
            }
            Self::ResponseTooLarge => f.write_str("tw.live response exceeded the size limit"),
            Self::ResponseTimedOut => f.write_str("tw.live response exceeded the total timeout"),
            Self::InvalidUtf8 => f.write_str("tw.live response was not valid UTF-8"),
            Self::MissingPlace => f.write_str("tw.live camera page had no usable place name"),
            Self::MissingYouTubeSource => {
                f.write_str("tw.live camera page had no valid YouTube live source")
            }
            Self::InvalidCatalog => f.write_str("tw.live Japan catalog marker was not found"),
        }
    }
}

impl std::error::Error for TravelError {}

fn winhttp_error(operation: &'static str) -> TravelError {
    // SAFETY: Callers invoke this immediately after a WinHTTP BOOL/handle failure.
    TravelError::Http {
        operation,
        code: unsafe { GetLastError() },
    }
}

struct InternetHandle(*mut core::ffi::c_void);

impl InternetHandle {
    fn new(raw: *mut core::ffi::c_void, operation: &'static str) -> Result<Self, TravelError> {
        if raw.is_null() {
            Err(winhttp_error(operation))
        } else {
            Ok(Self(raw))
        }
    }
}

impl Drop for InternetHandle {
    fn drop(&mut self) {
        // SAFETY: Each wrapper owns exactly one non-null WinHTTP handle.
        unsafe {
            WinHttpCloseHandle(self.0);
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

fn checked(ok: i32, operation: &'static str) -> Result<(), TravelError> {
    if ok == 0 {
        Err(winhttp_error(operation))
    } else {
        Ok(())
    }
}

fn query_status(request: &InternetHandle) -> Result<u32, TravelError> {
    let mut status = 0u32;
    let mut bytes = size_of::<u32>() as u32;
    // SAFETY: The output points to one writable u32 and the number query requires
    // exactly that representation. The request remains owned by `request`.
    checked(
        unsafe {
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                ptr::null(),
                (&mut status as *mut u32).cast(),
                &mut bytes,
                ptr::null_mut(),
            )
        },
        "WinHttpQueryHeaders(status)",
    )?;
    Ok(status)
}

fn query_content_type(request: &InternetHandle) -> Result<String, TravelError> {
    let mut buffer = [0u16; 128];
    let mut bytes = size_of_val(&buffer) as u32;
    // SAFETY: The UTF-16 buffer and byte count describe the same writable array.
    checked(
        unsafe {
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_CONTENT_TYPE,
                ptr::null(),
                buffer.as_mut_ptr().cast(),
                &mut bytes,
                ptr::null_mut(),
            )
        },
        "WinHttpQueryHeaders(content-type)",
    )?;
    let units = usize::try_from(bytes / 2)
        .unwrap_or(buffer.len())
        .min(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..units])
        .trim_end_matches('\0')
        .trim()
        .to_owned())
}

/// Perform one bounded HTTPS GET against tw.live. Redirects are disabled so a
/// compromised or changed catalog cannot make the native client fetch another
/// origin. The caller must execute this blocking function on a worker thread.
fn get_tw_live(path: &str) -> Result<Vec<u8>, TravelError> {
    if !path.starts_with('/') || path.chars().any(char::is_control) {
        return Err(TravelError::InvalidCameraId);
    }
    let agent = wide(USER_AGENT);
    let host = wide(SOURCE_HOST);
    let verb = wide("GET");
    let path = wide(path);
    let headers = wide("Accept: text/html;charset=UTF-8\r\nAccept-Encoding: gzip, deflate\r\n");

    // SAFETY: Every PCWSTR below is NUL-terminated for the duration of its call;
    // the owned handles keep their parents alive until child handles are dropped.
    unsafe {
        let session = InternetHandle::new(
            WinHttpOpen(
                agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                ptr::null(),
                ptr::null(),
                0,
            ),
            "WinHttpOpen",
        )?;
        checked(
            WinHttpSetTimeouts(
                session.0,
                IO_TIMEOUT_MS,
                IO_TIMEOUT_MS,
                IO_TIMEOUT_MS,
                IO_TIMEOUT_MS,
            ),
            "WinHttpSetTimeouts",
        )?;

        let connection = InternetHandle::new(
            WinHttpConnect(session.0, host.as_ptr(), INTERNET_DEFAULT_HTTPS_PORT, 0),
            "WinHttpConnect",
        )?;
        let request = InternetHandle::new(
            WinHttpOpenRequest(
                connection.0,
                verb.as_ptr(),
                path.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                WINHTTP_FLAG_SECURE,
            ),
            "WinHttpOpenRequest",
        )?;

        let redirect_policy = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
        checked(
            WinHttpSetOption(
                request.0,
                WINHTTP_OPTION_REDIRECT_POLICY,
                (&redirect_policy as *const u32).cast(),
                size_of::<u32>() as u32,
            ),
            "WinHttpSetOption(redirect-policy)",
        )?;
        let decompression = WINHTTP_DECOMPRESSION_FLAG_GZIP | WINHTTP_DECOMPRESSION_FLAG_DEFLATE;
        checked(
            WinHttpSetOption(
                request.0,
                WINHTTP_OPTION_DECOMPRESSION,
                (&decompression as *const u32).cast(),
                size_of::<u32>() as u32,
            ),
            "WinHttpSetOption(decompression)",
        )?;

        checked(
            WinHttpSendRequest(
                request.0,
                headers.as_ptr(),
                u32::try_from(headers.len().saturating_sub(1))
                    .map_err(|_| TravelError::ResponseTooLarge)?,
                ptr::null(),
                0,
                0,
                0,
            ),
            "WinHttpSendRequest",
        )?;
        checked(
            WinHttpReceiveResponse(request.0, ptr::null_mut()),
            "WinHttpReceiveResponse",
        )?;

        let status = query_status(&request)?;
        if !(200..=299).contains(&status) {
            return Err(TravelError::HttpStatus(status));
        }
        let content_type = query_content_type(&request)?;
        if !content_type.to_ascii_lowercase().starts_with("text/html") {
            return Err(TravelError::UnexpectedContentType(content_type));
        }

        let started = Instant::now();
        let mut result = Vec::new();
        let mut chunk = [0u8; 16 * 1024];
        loop {
            if started.elapsed() > TOTAL_TIMEOUT {
                return Err(TravelError::ResponseTimedOut);
            }
            let mut read = 0u32;
            checked(
                WinHttpReadData(
                    request.0,
                    chunk.as_mut_ptr().cast(),
                    chunk.len() as u32,
                    &mut read,
                ),
                "WinHttpReadData",
            )?;
            if read == 0 {
                break;
            }
            let read = usize::try_from(read).map_err(|_| TravelError::ResponseTooLarge)?;
            if result.len().saturating_add(read) > MAX_RESPONSE_BYTES {
                return Err(TravelError::ResponseTooLarge);
            }
            result.extend_from_slice(&chunk[..read]);
        }
        Ok(result)
    }
}

pub(crate) fn fetch_source(seed: CameraSeed) -> Result<TravelSource, TravelError> {
    if !valid_camera_id(seed.camera_id) {
        return Err(TravelError::InvalidCameraId);
    }
    let path = format!("{CAMERA_PATH_PREFIX}{}", seed.camera_id);
    let bytes = get_tw_live(&path)?;
    let html = str::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes))
        .map_err(|_| TravelError::InvalidUtf8)?;
    parse_camera_detail(seed.camera_id, seed.place_hint, html)
}

/// Verify that the primary Japan catalog is reachable and still identifies its
/// Japan/YouTube source structure. This is intentionally separate from camera
/// resolution so callers can report catalog health distinctly.
pub(crate) fn check_catalog() -> Result<(), TravelError> {
    let bytes = get_tw_live("/japan/")?;
    let html = str::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes))
        .map_err(|_| TravelError::InvalidUtf8)?;
    if find_ascii_case_insensitive(html, "href=\"/japan/").is_some()
        && find_ascii_case_insensitive(html, "youtube").is_some()
    {
        Ok(())
    } else {
        Err(TravelError::InvalidCatalog)
    }
}

pub(crate) fn parse_camera_detail(
    camera_id: &str,
    place_hint: &str,
    html: &str,
) -> Result<TravelSource, TravelError> {
    if !valid_camera_id(camera_id) {
        return Err(TravelError::InvalidCameraId);
    }
    let parsed_place = extract_element_text(html, "h1")
        .map(|value| normalize_place(&value))
        .filter(|value| !value.is_empty());
    let place = parsed_place
        .or_else(|| {
            let hint = normalize_place(place_hint);
            (!hint.is_empty()).then_some(hint)
        })
        .ok_or(TravelError::MissingPlace)?;

    let mut cursor = 0;
    let youtube_id = loop {
        let Some(relative) = find_ascii_case_insensitive(&html[cursor..], "<iframe") else {
            break None;
        };
        let start = cursor + relative;
        let Some(end_relative) = html[start..].find('>') else {
            break None;
        };
        let end = start + end_relative + 1;
        if let Some(src) = attribute(&html[start..end], "src") {
            let decoded = decode_html_entities(src);
            if let Some(video_id) = youtube_video_id(&decoded) {
                break Some(video_id.to_owned());
            }
        }
        cursor = end;
    }
    .ok_or(TravelError::MissingYouTubeSource)?;

    Ok(TravelSource {
        camera_id: camera_id.to_owned(),
        place,
        youtube_id,
        playlist_id: None,
    })
}

fn valid_camera_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_youtube_id(value: &str) -> bool {
    value.len() == 11
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn youtube_video_id(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/')?;
    if !matches!(
        host.to_ascii_lowercase().as_str(),
        "www.youtube.com" | "youtube.com" | "www.youtube-nocookie.com" | "youtube-nocookie.com"
    ) {
        return None;
    }
    let path = path.split(['?', '#']).next()?;
    let id = path.strip_prefix("embed/")?;
    valid_youtube_id(id).then_some(id)
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn attribute<'a>(tag: &'a str, wanted: &str) -> Option<&'a str> {
    let bytes = tag.as_bytes();
    let mut index = tag.find(char::is_whitespace)?;
    while index < bytes.len() {
        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b'/') {
            index += 1;
        }
        if bytes.get(index) == Some(&b'>') || index >= bytes.len() {
            break;
        }
        let name_start = index;
        while index < bytes.len()
            && !bytes[index].is_ascii_whitespace()
            && !matches!(bytes[index], b'=' | b'>' | b'/')
        {
            index += 1;
        }
        let name = &tag[name_start..index];
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            // Boolean attributes have no value. Keep `index` at the next
            // attribute (or terminator) so malformed/untrusted tags always
            // make forward progress instead of looping at `>`.
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let (value_start, value_end, quoted) = match bytes.get(index).copied() {
            Some(quote @ (b'\'' | b'"')) => {
                index += 1;
                let start = index;
                while index < bytes.len() && bytes[index] != quote {
                    index += 1;
                }
                (start, index, true)
            }
            Some(_) => {
                let start = index;
                while index < bytes.len()
                    && !bytes[index].is_ascii_whitespace()
                    && bytes[index] != b'>'
                {
                    index += 1;
                }
                (start, index, false)
            }
            None => return None,
        };
        if name.eq_ignore_ascii_case(wanted) {
            return Some(&tag[value_start..value_end]);
        }
        index = value_end;
        if quoted && index < bytes.len() {
            index += 1;
        }
    }
    None
}

fn extract_element_text(html: &str, element: &str) -> Option<String> {
    let open = format!("<{element}");
    let close = format!("</{element}>");
    let start = find_ascii_case_insensitive(html, &open)?;
    let body_start = start + html[start..].find('>')? + 1;
    let body_end = body_start + find_ascii_case_insensitive(&html[body_start..], &close)?;
    Some(decode_html_entities(&strip_tags(
        &html[body_start..body_end],
    )))
}

fn strip_tags(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut inside = false;
    for ch in value.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => result.push(ch),
            _ => (),
        }
    }
    result
}

fn decode_html_entities(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        result.push_str(&rest[..index]);
        rest = &rest[index..];
        let Some(end) = rest.find(';').filter(|&end| end <= 12) else {
            result.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "nbsp" => Some(' '),
            value if value.starts_with("#x") || value.starts_with("#X") => {
                u32::from_str_radix(&value[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            value if value.starts_with('#') => {
                value[1..].parse::<u32>().ok().and_then(char::from_u32)
            }
            _ => None,
        };
        if let Some(ch) = decoded {
            result.push(ch);
        } else {
            result.push_str(&rest[..=end]);
        }
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    result
}

fn normalize_place(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .trim_end_matches("即時影像畫面")
        .trim_end_matches("即時影像")
        .trim()
        .chars()
        .take(96)
        .collect()
}

#[derive(Debug, Clone)]
pub(crate) struct TravelRotation {
    random: Xorshift32,
    last_camera_id: Option<&'static str>,
    interval_ms: Option<u64>,
}

impl TravelRotation {
    pub fn new(seed: u32, interval_minutes: u32) -> Self {
        let minutes = if interval_minutes <= crate::config::MAX_TRAVEL_SWITCH_MINUTES {
            interval_minutes
        } else {
            crate::config::DEFAULT_TRAVEL_SWITCH_MINUTES
        };
        Self {
            random: Xorshift32::new(seed),
            last_camera_id: None,
            interval_ms: (minutes != 0).then_some(u64::from(minutes) * 60_000),
        }
    }

    /// Return every candidate exactly once in random order, excluding the last
    /// successful source. The caller may try candidates in order until one loads.
    pub fn candidates(&mut self) -> Vec<CameraSeed> {
        let mut result: Vec<_> = CAMERA_SEEDS
            .into_iter()
            .filter(|seed| Some(seed.camera_id) != self.last_camera_id)
            .collect();
        for end in (1..result.len()).rev() {
            let index = (self.random.next_value() as usize) % (end + 1);
            result.swap(end, index);
        }
        result
    }

    pub fn mark_playing(&mut self, camera_id: &str) {
        self.last_camera_id = CAMERA_SEEDS
            .iter()
            .find(|seed| seed.camera_id == camera_id)
            .map(|seed| seed.camera_id);
    }

    pub fn due(&self, last_switch_ms: u64, now_ms: u64) -> bool {
        self.interval_ms
            .is_some_and(|interval| now_ms.saturating_sub(last_switch_ms) >= interval)
    }

    /// Resolve the next live URL within the final minute of the current source.
    /// Long configured intervals must not keep an hours-old prefetched live ID.
    pub fn should_prefetch(&self, last_switch_ms: u64, now_ms: u64) -> bool {
        self.interval_ms.is_some_and(|interval| {
            now_ms.saturating_sub(last_switch_ms) >= interval.saturating_sub(60_000)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkState {
    Checking,
    LoadingPlayer,
    PlayerReady,
    Playing,
    Offline,
    SourceFailed,
    Stalled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkEvent {
    FetchStarted,
    SourceResolved,
    FetchFailed,
    PlayerReady,
    PlayerPlaying,
    PlayerError,
    PlayerStalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TravelCaption {
    pub place: String,
    pub status: String,
}

impl Default for TravelCaption {
    fn default() -> Self {
        Self {
            place: "日本旅行模式".into(),
            status: "靜態預覽・全螢幕啟動後顯示即時影像".into(),
        }
    }
}

impl TravelCaption {
    pub fn live(place: Option<&str>, state: NetworkState, player_unavailable: bool) -> Self {
        Self {
            place: place.unwrap_or("日本旅行模式").to_owned(),
            status: if player_unavailable {
                "播放器無法啟動或已停止・請檢查 WebView2 Runtime".into()
            } else {
                state.label().into()
            },
        }
    }
}

impl NetworkState {
    pub fn transition(self, event: NetworkEvent) -> Self {
        match event {
            NetworkEvent::FetchStarted => Self::Checking,
            NetworkEvent::SourceResolved => Self::LoadingPlayer,
            NetworkEvent::FetchFailed => Self::Offline,
            NetworkEvent::PlayerReady => Self::PlayerReady,
            NetworkEvent::PlayerPlaying => Self::Playing,
            NetworkEvent::PlayerError => Self::SourceFailed,
            NetworkEvent::PlayerStalled => Self::Stalled,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Checking => "正在檢查來源網路…",
            Self::LoadingPlayer => "正在載入日本即時影像…",
            Self::PlayerReady => "影像來源已連線",
            Self::Playing => "日本即時影像播放中",
            Self::Offline => "來源網路暫時無法連線，稍後重試",
            Self::SourceFailed => "目前影像來源無法播放，正在準備切換",
            Self::Stalled => "影像已停滯，正在準備切換",
        }
    }
}

/// One visible 16:9 player is surrounded by an embedded photorealistic
/// travel frame. Location and health text occupy their own row below the player;
/// no element is layered over or clipped into the YouTube player rectangle.
const TRAVEL_HTML_TEMPLATE: &str = r#"<!doctype html>
<html lang="zh-Hant">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'self' https://www.youtube.com https://www.youtube-nocookie.com; script-src 'unsafe-inline' https://www.youtube.com https://www.youtube-nocookie.com; style-src 'unsafe-inline'; frame-src https://www.youtube.com https://www.youtube-nocookie.com; connect-src https://www.youtube.com https://www.youtube-nocookie.com;">
<style>
html,body{width:100%;height:100%;margin:0;overflow:hidden;background:#071019;color:#f4f7fa;font-family:"Microsoft JhengHei UI",sans-serif}
body{display:grid;place-items:center}
.cabin{box-sizing:border-box;width:min(94vw,135vh)}
.scene{position:relative;width:100%;aspect-ratio:1586/992;background-size:100% 100%;background-repeat:no-repeat;box-shadow:0 12px 40px #000}
.free-flight .scene{background-image:url('free-flight.png')}
.train-journey .scene{background-image:url('train-journey.png')}
.japanese-inn .scene{background-image:url('japanese-inn.png')}
.train-cab .scene{background-image:url('train-cab.png')}
.walking .scene{background-image:url('walking.png')}
.window{position:absolute;left:11%;top:21%;width:78.5%;height:56%;display:grid;place-items:center;background:#000;border-radius:2%}
.train-journey .window{left:17.8%;top:20%;width:64.4%;height:54.5%}
.japanese-inn .window{left:16.5%;top:18.5%;width:68%;height:57%;border-radius:0}
.train-cab .window{left:12.6%;top:23.5%;width:76%;height:40%;border-radius:0}
.walking .window{left:20%;top:22%;width:60%;height:54%;border-radius:12%/18%;overflow:hidden}
.blink{display:none;position:absolute;inset:0;z-index:5;pointer-events:none;overflow:hidden}
.walking .blink{display:block}
.blink::before,.blink::after{content:"";position:absolute;left:0;width:100%;height:51%;background:#000}
.blink::before{top:0;transform:translateY(-100%)}
.blink::after{bottom:0;transform:translateY(100%)}
.walking.blinking .blink::before{animation:blinkTop 700ms ease-in-out}
.walking.blinking .blink::after{animation:blinkBottom 700ms ease-in-out}
@keyframes blinkTop{0%,100%{transform:translateY(-100%)}42%,58%{transform:translateY(0)}}
@keyframes blinkBottom{0%,100%{transform:translateY(100%)}42%,58%{transform:translateY(0)}}
.screen{height:100%;aspect-ratio:16/9;max-width:100%;background:#000}
#player,#player iframe{display:block;width:100%;height:100%;border:0}
.caption{display:flex;flex-wrap:wrap;justify-content:space-between;gap:.35em 1em;align-items:center;padding:clamp(9px,1.2vw,18px) 0;font-weight:700;letter-spacing:.04em;overflow-wrap:anywhere}
#place{font-size:clamp(16px,2.2vw,34px);color:#fff}
#status{font-size:clamp(12px,1.2vw,18px);color:#c8f3ff;text-align:right}
.train-journey #place{color:#ffe1a6}.train-journey #status{color:#ffd7a1}
.japanese-inn #place{color:#ffe0a3}.japanese-inn #status{color:#dbe9d1}
.train-cab #place{color:#d7ecf6}.train-cab #status{color:#b9d9e8}
.walking #place{color:#e6f5dc}.walking #status{color:#cde4bf}
</style>
</head>
<body>
<main class="cabin __TRAVEL_SCENE_CLASS__" aria-label="__TRAVEL_SCENE_LABEL__">
  <div class="scene">
    <section class="window">
      <div class="screen"><div id="player"></div></div>
    </section>
    <div class="blink" aria-hidden="true"></div>
  </div>
  <div class="caption"><span id="place">日本旅行模式</span><span id="status">正在檢查來源網路…</span></div>
</main>
<script>
(() => {
  'use strict';
  const send = (state, token) => {
    if (window.chrome && window.chrome.webview) {
      window.chrome.webview.postMessage(`${state}:${token}`);
    }
  };
  let player = null;
  let pending = null;
  let currentToken = 0;
  let lastTime = -1;
  let unchanged = 0;
  let stalled = false;
  let playlistNeedsShuffle = false;
  let acceptPlayerEvents = false;
  let selectedVideoId = '';
  let playlistProbe = 0;
  const cabin = document.querySelector('.cabin');
  const place = document.getElementById('place');
  const status = document.getElementById('status');
  const updateStatus = text => { status.textContent = String(text).slice(0, 96); };
  const randomIndex = length => {
    if (window.crypto && typeof window.crypto.getRandomValues === 'function') {
      const value = new Uint32Array(1); window.crypto.getRandomValues(value); return value[0] % length;
    }
    return Math.floor(Math.random() * length);
  };
  const shuffleAndSelect = target => {
    if (!playlistNeedsShuffle || typeof target.getPlaylist !== 'function') return false;
    const loaded = target.getPlaylist();
    if (!Array.isArray(loaded) || loaded.length === 0) return false;
    target.setShuffle(true);
    const shuffled = target.getPlaylist();
    const choices = Array.isArray(shuffled) && shuffled.length ? shuffled : loaded;
    const selected = choices[randomIndex(choices.length)];
    if (typeof selected !== 'string' || !/^[A-Za-z0-9_-]{11}$/.test(selected)) return false;
    playlistNeedsShuffle = false;
    selectedVideoId = selected;
    target.loadVideoById(selected);
    return true;
  };
  const waitForPlaylist = (target, token, attempt) => {
    if (currentToken !== token || !playlistNeedsShuffle) return;
    if (shuffleAndSelect(target)) return;
    if (attempt >= 20) {
      playlistNeedsShuffle = false;
      updateStatus('YouTube 播放清單無法讀取');
      send('error', token);
      return;
    }
    clearTimeout(playlistProbe);
    playlistProbe = setTimeout(() => waitForPlaylist(target, token, attempt + 1), 250);
  };
  const apply = source => {
    if (!source || !Number.isInteger(source.token) || source.token <= 0) return;
    const hasVideo = typeof source.videoId === 'string' && /^[A-Za-z0-9_-]{11}$/.test(source.videoId);
    const hasPlaylist = typeof source.playlistId === 'string' && /^[A-Za-z0-9_-]{10,64}$/.test(source.playlistId);
    if (hasVideo === hasPlaylist) return;
    const switching = currentToken !== 0;
    clearTimeout(playlistProbe);
    currentToken = source.token;
    acceptPlayerEvents = false;
    place.textContent = String(source.place || '日本').slice(0, 96);
    updateStatus(hasPlaylist ? '正在讀取並隨機排列 YouTube 播放清單…' : '正在載入日本即時影像…');
    lastTime = -1; unchanged = 0; stalled = false;
    const startLoad = () => {
      if (currentToken !== source.token) return;
      playlistNeedsShuffle = hasPlaylist;
      acceptPlayerEvents = true;
      selectedVideoId = hasVideo ? source.videoId : '';
      if (!window.YT || !window.YT.Player) { pending = source; return; }
      if (player) {
        if (hasPlaylist && typeof player.loadPlaylist === 'function') {
          player.loadPlaylist({listType:'playlist',list:source.playlistId,index:0,startSeconds:0});
          waitForPlaylist(player, source.token, 0);
        } else if (hasVideo && typeof player.loadVideoById === 'function') {
          player.loadVideoById(source.videoId);
        }
        if (typeof player.mute === 'function') player.mute();
        return;
      }
      const options = {
      host:'https://www.youtube-nocookie.com',
      playerVars:{autoplay:1,mute:1,playsinline:1,rel:0,controls:1,disablekb:1,fs:0,origin:'https://travel.screensaver.local'},
      events:{
        onReady:event => {
          event.target.mute();
          updateStatus('影像來源已連線');
          send('ready',currentToken);
          if (playlistNeedsShuffle) waitForPlaylist(event.target, currentToken, 0);
          else event.target.playVideo();
        },
        onStateChange:event => {
          if (!acceptPlayerEvents) return;
          if (playlistNeedsShuffle) return;
          if (event.data === YT.PlayerState.PLAYING) {
            updateStatus(hasPlaylist ? '播放清單隨機影片播放中' : '日本即時影像播放中');
            send('playing',currentToken);
          } else if (event.data === YT.PlayerState.ENDED && selectedVideoId) {
            event.target.loadVideoById(selectedVideoId);
          }
        },
        onError:event => { if (acceptPlayerEvents) { updateStatus('目前影像來源無法播放'); send('error',currentToken); } }
      }
      };
      if (hasPlaylist) {
        options.playerVars.listType='playlist'; options.playerVars.list=source.playlistId;
      } else {
        options.videoId=source.videoId;
      }
      player = new YT.Player('player', options);
    };
    if (switching && cabin.classList.contains('walking')) {
      cabin.classList.remove('blinking');
      void cabin.offsetWidth;
      cabin.classList.add('blinking');
      setTimeout(startLoad, 300);
      setTimeout(() => cabin.classList.remove('blinking'), 720);
    } else {
      startLoad();
    }
  };
  window.onYouTubeIframeAPIReady = () => { if (pending) { const source=pending; pending=null; currentToken=0; apply(source); } };
  window.travel = {load: apply, setStatus:updateStatus};
  if (window.chrome && window.chrome.webview) window.chrome.webview.postMessage('shell-ready');
  setInterval(() => {
    if (!player || typeof player.getPlayerState !== 'function' || player.getPlayerState() !== YT.PlayerState.PLAYING) return;
    const current = Number(player.getCurrentTime());
    if (!Number.isFinite(current)) return;
    if (current <= lastTime + .05) unchanged += 1; else unchanged = 0;
    lastTime = current;
    if (unchanged >= 3 && !stalled) { stalled=true; updateStatus('影像已停滯，正在準備切換'); send('stalled',currentToken); }
  },5000);
  const api=document.createElement('script'); api.src='https://www.youtube.com/iframe_api'; document.head.appendChild(api);
})();
</script>
</body>
</html>"#;

pub(crate) fn travel_html_shell(style: TravelStyle) -> String {
    let (class, label) = match style {
        TravelStyle::FreeFlight => ("free-flight", "自在飛行客艙窗景"),
        TravelStyle::TrainJourney => ("train-journey", "列車旅行車廂窗景"),
        TravelStyle::JapaneseInn => ("japanese-inn", "日式旅館庭園窗景"),
        TravelStyle::TrainCab => ("train-cab", "列車駕駛前方視角"),
        TravelStyle::Walking => ("walking", "散步第一人稱人眼視角"),
    };
    TRAVEL_HTML_TEMPLATE
        .replace("__TRAVEL_SCENE_CLASS__", class)
        .replace("__TRAVEL_SCENE_LABEL__", label)
}

pub(crate) fn load_source_script(source: &TravelSource, token: u32) -> String {
    format!(
        "window.travel.load({{videoId:{},playlistId:{},place:{},token:{token}}});",
        json_string(&source.youtube_id),
        source
            .playlist_id
            .as_deref()
            .map_or_else(|| "null".to_owned(), json_string),
        json_string(&source.place)
    )
}

pub(crate) fn set_status_script(status: &str) -> String {
    format!("window.travel.setStatus({});", json_string(status))
}

fn json_string(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 2);
    result.push('"');
    for ch in value.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\u{08}' => result.push_str("\\b"),
            '\u{0c}' => result.push_str("\\f"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '<' => result.push_str("\\u003c"),
            '>' => result.push_str("\\u003e"),
            '&' => result.push_str("\\u0026"),
            '\u{2028}' => result.push_str("\\u2028"),
            '\u{2029}' => result.push_str("\\u2029"),
            value if value < '\u{20}' => {
                use fmt::Write;
                let _ = write!(result, "\\u{:04x}", u32::from(value));
            }
            value => result.push(value),
        }
    }
    result.push('"');
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captions_distinguish_preview_playback_and_browser_failure() {
        let preview = TravelCaption::default();
        assert!(preview.status.contains("靜態預覽"));
        assert!(!preview.status.contains("無法"));
        let playing = TravelCaption::live(Some("京都・中京區"), NetworkState::Playing, false);
        assert_eq!(playing.place, "京都・中京區");
        assert_eq!(playing.status, NetworkState::Playing.label());
        let offline = TravelCaption::live(Some("札幌"), NetworkState::Offline, false);
        assert_eq!(offline.status, NetworkState::Offline.label());
        let unavailable = TravelCaption::live(None, NetworkState::Offline, true);
        assert!(unavailable.status.contains("播放器"));
        assert_ne!(unavailable.status, offline.status);
    }

    const DETAIL_FIXTURE: &str = r#"
      <html><body>
        <h1 class="entry-title">北海道・札幌 &amp; 車站即時影像</h1>
        <div class="embed-container">
          <iframe allow="autoplay" src="https://www.youtube-nocookie.com/embed/Ee27soLzJ5c?mute=1&amp;autoplay=1"></iframe>
        </div>
      </body></html>"#;

    #[test]
    fn camera_detail_extracts_normalized_place_and_strict_youtube_id() {
        let source = parse_camera_detail("sapporostationhbc", "備援地點", DETAIL_FIXTURE).unwrap();
        assert_eq!(source.camera_id, "sapporostationhbc");
        assert_eq!(source.place, "北海道・札幌 & 車站");
        assert_eq!(source.youtube_id, "Ee27soLzJ5c");
        assert_eq!(
            source.embed_url(),
            "https://www.youtube-nocookie.com/embed/Ee27soLzJ5c?autoplay=1&mute=1&playsinline=1&rel=0&controls=1&disablekb=1&fs=0&enablejsapi=1"
        );
    }

    #[test]
    fn parser_uses_hint_but_rejects_untrusted_hosts_ids_and_camera_paths() {
        let no_heading = DETAIL_FIXTURE.replace(
            "<h1 class=\"entry-title\">北海道・札幌 &amp; 車站即時影像</h1>",
            "",
        );
        assert_eq!(
            parse_camera_detail("sapporostationhbc", "北海道・札幌", &no_heading)
                .unwrap()
                .place,
            "北海道・札幌"
        );
        for bad in [
            DETAIL_FIXTURE.replace("www.youtube-nocookie.com", "evil.example"),
            DETAIL_FIXTURE.replace("Ee27soLzJ5c", "too-short"),
            DETAIL_FIXTURE.replace("https://", "http://"),
        ] {
            assert_eq!(
                parse_camera_detail("safe-id", "日本", &bad),
                Err(TravelError::MissingYouTubeSource)
            );
        }
        assert_eq!(
            parse_camera_detail("../escape", "日本", DETAIL_FIXTURE),
            Err(TravelError::InvalidCameraId)
        );
    }

    #[test]
    fn attribute_parser_handles_boolean_missing_and_malformed_attributes() {
        let with_boolean = r#"<iframe allowfullscreen class="camera" src="https://www.youtube.com/embed/Ee27soLzJ5c"></iframe>"#;
        assert_eq!(
            attribute(with_boolean, "src"),
            Some("https://www.youtube.com/embed/Ee27soLzJ5c")
        );
        assert_eq!(attribute(r#"<iframe class="camera">"#, "src"), None);
        assert_eq!(attribute("<iframe allowfullscreen/>", "src"), None);
        assert_eq!(attribute("<iframe broken", "src"), None);
    }

    #[test]
    fn candidate_shuffle_is_unique_and_excludes_last_success() {
        let mut rotation = TravelRotation::new(7, 1);
        let first = rotation.candidates();
        assert_eq!(first.len(), CAMERA_SEEDS.len());
        for seed in CAMERA_SEEDS {
            assert_eq!(first.iter().filter(|item| **item == seed).count(), 1);
        }
        rotation.mark_playing(first[0].camera_id);
        let second = rotation.candidates();
        assert_eq!(second.len(), CAMERA_SEEDS.len() - 1);
        assert!(second
            .iter()
            .all(|item| item.camera_id != first[0].camera_id));
        assert!(!rotation.due(10, 60_009));
        assert!(rotation.due(10, 60_010));
        assert!(!rotation.due(u64::MAX - 10, 5));
    }

    #[test]
    fn rotation_intervals_and_prefetch_respect_disabled_and_minute_boundaries() {
        let disabled = TravelRotation::new(1, 0);
        for now in [0, 60_000, 86_400_000, u64::MAX] {
            assert!(!disabled.due(0, now));
            assert!(!disabled.should_prefetch(0, now));
        }
        for minutes in [1, 2, 30, 1440] {
            let rotation = TravelRotation::new(1, minutes);
            let interval = u64::from(minutes) * 60_000;
            assert!(!rotation.due(25, 25 + interval - 1));
            assert!(rotation.due(25, 25 + interval));
            assert!(rotation.should_prefetch(25, 25 + interval - 60_000));
            if minutes > 1 {
                assert!(!rotation.should_prefetch(25, 25 + interval - 60_001));
            }
            assert!(!rotation.due(100, 99));
        }
        let invalid = TravelRotation::new(1, u32::MAX);
        assert!(!invalid.due(0, 59_999));
        assert!(invalid.due(0, 60_000));
    }

    #[test]
    fn network_state_distinguishes_fetch_player_error_and_stall() {
        let mut state = NetworkState::Offline;
        state = state.transition(NetworkEvent::FetchStarted);
        assert_eq!(state, NetworkState::Checking);
        state = state.transition(NetworkEvent::SourceResolved);
        assert_eq!(state, NetworkState::LoadingPlayer);
        state = state.transition(NetworkEvent::PlayerReady);
        assert_eq!(state, NetworkState::PlayerReady);
        state = state.transition(NetworkEvent::PlayerPlaying);
        assert_eq!(state, NetworkState::Playing);
        assert_eq!(
            state.transition(NetworkEvent::PlayerStalled),
            NetworkState::Stalled
        );
        assert_eq!(
            state.transition(NetworkEvent::PlayerError),
            NetworkState::SourceFailed
        );
        assert_eq!(
            state.transition(NetworkEvent::FetchFailed),
            NetworkState::Offline
        );
        assert!(NetworkState::Offline.label().contains("稍後重試"));
    }

    #[test]
    fn html_shell_has_one_player_and_keeps_caption_outside_it() {
        for (style, class, label) in [
            (TravelStyle::FreeFlight, "free-flight", "自在飛行客艙窗景"),
            (
                TravelStyle::TrainJourney,
                "train-journey",
                "列車旅行車廂窗景",
            ),
            (TravelStyle::JapaneseInn, "japanese-inn", "日式旅館庭園窗景"),
            (TravelStyle::TrainCab, "train-cab", "列車駕駛前方視角"),
            (TravelStyle::Walking, "walking", "散步第一人稱人眼視角"),
        ] {
            let shell = travel_html_shell(style);
            assert_eq!(shell.matches("id=\"player\"").count(), 1);
            let screen_end = shell.find("</div></div>").unwrap();
            let caption = shell.find("class=\"caption\"").unwrap();
            assert!(caption > screen_end);
            assert!(shell.contains(&format!("class=\"cabin {class}\"")));
            assert!(shell.contains(label));
            assert!(shell.contains("background-image:url('free-flight.png')"));
            assert!(shell.contains("background-image:url('train-journey.png')"));
            assert!(shell.contains("background-image:url('japanese-inn.png')"));
            assert!(shell.contains("background-image:url('train-cab.png')"));
            assert!(shell.contains("background-image:url('walking.png')"));
            assert!(shell.contains("aspect-ratio:1586/992"));
            assert!(shell.contains("aspect-ratio:16/9"));
            assert!(!shell.contains("__TRAVEL_SCENE_"));
            for event in ["'ready'", "'playing'", "'error'", "'stalled'"] {
                assert!(shell.contains(event));
            }
            assert!(shell.contains("youtube-nocookie.com"));
            assert!(!shell.contains("iframe{position:absolute"));
        }
    }

    #[test]
    fn load_script_escapes_untrusted_place_text_as_data() {
        let source = TravelSource {
            camera_id: "safe".into(),
            youtube_id: "Ee27soLzJ5c".into(),
            playlist_id: None,
            place: "東京 </script> & \"測試\"\n下一行".into(),
        };
        let script = load_source_script(&source, 7);
        assert!(script
            .starts_with("window.travel.load({videoId:\"Ee27soLzJ5c\",playlistId:null,place:"));
        assert!(!script.contains("</script>"));
        assert!(script.contains("\\u003c/script\\u003e"));
        assert!(script.contains("\\u0026"));
        assert!(script.contains("\\n"));
        assert!(script.contains("token:7"));
        assert_eq!(
            set_status_script("離線 <重試>"),
            "window.travel.setStatus(\"離線 \\u003c重試\\u003e\");"
        );
    }

    #[test]
    fn playlist_scenes_use_the_requested_youtube_lists_and_shuffle_in_the_player() {
        for (style, expected) in [
            (TravelStyle::FreeFlight, "PLdsqwBj2O1Nw"),
            (TravelStyle::TrainJourney, "PLBH60D9AGfu0"),
            (TravelStyle::TrainCab, "PLB-Fmt68BNm4"),
            (TravelStyle::Walking, "PLbYZr39owNGo"),
        ] {
            let source = playlist_source(style).unwrap();
            assert_eq!(source.playlist_id.as_deref(), Some(expected));
            assert_eq!(source.camera_id, expected);
            assert!(source.youtube_id.is_empty());
            assert!(source.embed_url().contains("/embed/videoseries?list="));
            let script = load_source_script(&source, 9);
            assert!(script.contains(&format!("playlistId:\"{expected}\"")));
            assert!(script.contains("token:9"));
        }
        assert!(playlist_source(TravelStyle::JapaneseInn).is_none());
        let shell = travel_html_shell(TravelStyle::Walking);
        for behavior in [
            "loadPlaylist({listType:'playlist'",
            "setShuffle(true)",
            "getPlaylist()",
            "target.loadVideoById(selected)",
            "@keyframes blinkTop",
            "setTimeout(startLoad, 300)",
        ] {
            assert!(shell.contains(behavior), "missing {behavior}");
        }
    }

    #[test]
    #[ignore = "live bounded HTTPS probe; run explicitly without opening UI"]
    fn live_catalog_and_candidate_parser_have_usable_sources() {
        check_catalog().expect("tw.live Japan catalog");
        let mut usable = 0;
        for seed in CAMERA_SEEDS {
            match fetch_source(seed) {
                Ok(source) => {
                    usable += 1;
                    eprintln!(
                        "{} => {} ({})",
                        source.camera_id, source.youtube_id, source.place
                    );
                }
                Err(error) => eprintln!("{} unavailable: {error}", seed.camera_id),
            }
        }
        assert!(usable >= 3, "only {usable} usable tw.live candidates");
    }
}
