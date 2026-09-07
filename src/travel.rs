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

pub(crate) const ROTATION_INTERVAL_MS: u64 = 60_000;

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
}

impl TravelSource {
    pub fn embed_url(&self) -> String {
        format!(
            "https://www.youtube-nocookie.com/embed/{}?autoplay=1&mute=1&playsinline=1&rel=0&controls=1&disablekb=1&fs=0&enablejsapi=1",
            self.youtube_id
        )
    }
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
}

impl TravelRotation {
    pub fn new(seed: u32) -> Self {
        Self {
            random: Xorshift32::new(seed),
            last_camera_id: None,
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

    pub fn due(last_switch_ms: u64, now_ms: u64) -> bool {
        now_ms.saturating_sub(last_switch_ms) >= ROTATION_INTERVAL_MS
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

/// One visible 16:9 player is surrounded by a user-selected, project-drawn
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
.cabin{box-sizing:border-box;width:min(94vw,135vh);padding:clamp(12px,2.6vw,40px);box-shadow:0 20px 60px #000}
.free-flight{border:clamp(8px,1.4vw,24px) solid #ccd5dc;border-radius:clamp(42px,8vw,128px);background:linear-gradient(145deg,#f3f6f8,#8696a3 46%,#d9e0e5 65%,#6d7b86);box-shadow:inset 0 0 0 clamp(5px,.7vw,12px) #3f4c56,0 20px 60px #000}
.train-journey{border:clamp(9px,1.5vw,25px) solid #5d2f19;border-radius:clamp(24px,3vw,52px) clamp(24px,3vw,52px) clamp(10px,1.3vw,24px) clamp(10px,1.3vw,24px);background:repeating-linear-gradient(90deg,#3d1d10 0,#3d1d10 5%,#a45b2c 5.6%,#5e2e17 7.2%,#32170d 12%);box-shadow:inset 0 0 0 clamp(5px,.7vw,12px) #d28743,inset 0 clamp(18px,3vw,46px) 0 #492414,0 20px 60px #000}
.window{box-sizing:border-box;padding:clamp(8px,1vw,16px);border:clamp(5px,.7vw,11px) solid #283640;border-radius:clamp(24px,4vw,60px);background:#101b24}
.train-journey .window{border-color:#d79b5d;border-radius:clamp(10px,1.6vw,25px);background:linear-gradient(90deg,#3a1d11,#8c4a27 9%,#35190e 16%,#35190e 84%,#8c4a27 91%,#3a1d11);box-shadow:inset 0 0 0 clamp(3px,.45vw,8px) #2b140b}
.screen{width:100%;aspect-ratio:16/9;background:#000}
#player,#player iframe{display:block;width:100%;height:100%;border:0}
.caption{display:flex;justify-content:space-between;gap:1em;align-items:center;padding:clamp(9px,1.2vw,18px) clamp(4px,.8vw,12px) 0;font-weight:700;letter-spacing:.04em}
.train-journey .caption{margin-top:clamp(5px,.7vw,11px);padding:clamp(8px,1vw,15px);border-radius:clamp(4px,.6vw,10px);background:linear-gradient(90deg,#2d160d,#75401f,#2d160d);box-shadow:inset 0 0 0 1px #ca8242}
#place{font-size:clamp(16px,2.2vw,34px);color:#fff}
#status{font-size:clamp(12px,1.2vw,18px);color:#c8f3ff;text-align:right}
.train-journey #place{color:#ffe1a6}.train-journey #status{color:#ffd7a1}
</style>
</head>
<body>
<main class="cabin __TRAVEL_SCENE_CLASS__" aria-label="__TRAVEL_SCENE_LABEL__">
  <section class="window">
    <div class="screen"><div id="player"></div></div>
    <div class="caption"><span id="place">日本旅行模式</span><span id="status">正在檢查來源網路…</span></div>
  </section>
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
  const place = document.getElementById('place');
  const status = document.getElementById('status');
  const updateStatus = text => { status.textContent = String(text).slice(0, 96); };
  const apply = source => {
    if (!source || !/^[A-Za-z0-9_-]{11}$/.test(source.videoId) || !Number.isInteger(source.token) || source.token <= 0) return;
    currentToken = source.token;
    place.textContent = String(source.place || '日本').slice(0, 96);
    updateStatus('正在載入日本即時影像…');
    lastTime = -1; unchanged = 0; stalled = false;
    if (!window.YT || !window.YT.Player) { pending = source; return; }
    if (player && typeof player.loadVideoById === 'function') {
      player.loadVideoById(source.videoId);
      if (typeof player.mute === 'function') player.mute();
      return;
    }
    player = new YT.Player('player', {
      host:'https://www.youtube-nocookie.com',
      videoId:source.videoId,
      playerVars:{autoplay:1,mute:1,playsinline:1,rel:0,controls:1,disablekb:1,fs:0,origin:'https://travel.screensaver.local'},
      events:{
        onReady:event => { event.target.mute(); event.target.playVideo(); updateStatus('影像來源已連線'); send('ready',currentToken); },
        onStateChange:event => { if (event.data === YT.PlayerState.PLAYING) { updateStatus('日本即時影像播放中'); send('playing',currentToken); } },
        onError:event => { updateStatus('目前影像來源無法播放'); send('error',currentToken); }
      }
    });
  };
  window.onYouTubeIframeAPIReady = () => { if (pending) { const source=pending; pending=null; apply(source); } };
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
    };
    TRAVEL_HTML_TEMPLATE
        .replace("__TRAVEL_SCENE_CLASS__", class)
        .replace("__TRAVEL_SCENE_LABEL__", label)
}

pub(crate) fn load_source_script(source: &TravelSource, token: u32) -> String {
    format!(
        "window.travel.load({{videoId:{},place:{},token:{token}}});",
        json_string(&source.youtube_id),
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
        let mut rotation = TravelRotation::new(7);
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
        assert!(!TravelRotation::due(10, 60_009));
        assert!(TravelRotation::due(10, 60_010));
        assert!(!TravelRotation::due(u64::MAX - 10, 5));
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
        ] {
            let shell = travel_html_shell(style);
            assert_eq!(shell.matches("id=\"player\"").count(), 1);
            let screen_end = shell.find("</div></div>").unwrap();
            let caption = shell.find("class=\"caption\"").unwrap();
            assert!(caption > screen_end);
            assert!(shell.contains(&format!("class=\"cabin {class}\"")));
            assert!(shell.contains(label));
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
            place: "東京 </script> & \"測試\"\n下一行".into(),
        };
        let script = load_source_script(&source, 7);
        assert!(script.starts_with("window.travel.load({videoId:\"Ee27soLzJ5c\",place:"));
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
