use std::{
    cell::{Cell, RefCell},
    env, fs,
    mem::size_of,
    path::PathBuf,
    ptr,
    rc::Rc,
    sync::mpsc,
    thread,
};

use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, SYSTEMTIME, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, FillRect, GetStockObject, BLACK_BRUSH, PAINTSTRUCT,
};
use windows_sys::Win32::System::SystemInformation::{GetLocalTime, GetTickCount64};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, SetFocus, LASTINPUTINFO};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::config::AppConfig;
use crate::error::{last_error, AppError};
use crate::layout::{Drift, Layout};
use crate::lifecycle::{InputBaseline, Shutdown};
use crate::model::{DisplayMode, FrameSnapshot, LocalTime, Timeline};
use crate::monitor::{self, Bounds};
use crate::native::{client_size, set_pointer, DpiScope, WindowIdentity};
use crate::render::{Renderer, Style};
use crate::travel::{self, NetworkEvent, NetworkState, TravelRotation, TravelSource};
use crate::travel_webview::{
    PlayerEventKind, StartupPoll, TravelWebView, TravelWebViewStartup, BROWSER_FAILED,
    PLAYER_EVENT, SHELL_NAVIGATED, SHELL_READY, WEBVIEW_STARTUP_CHANGED,
};

const CLASS_NAME: *const u16 = windows_sys::w!("tools-screensaver-tzk.Window");
const WINDOW_TITLE: *const u16 = windows_sys::w!("tools-screensaver-tzk");
const CLOSE_ALL: u32 = WM_APP + 1;
const CHECK_FOREGROUND: u32 = WM_APP + 2;
const TRAVEL_FETCH_DONE: u32 = WM_APP + 44;
#[cfg(debug_assertions)]
const TEST_GET_CONFIG_SNAPSHOT: u32 = WM_APP + 32;
const MAINTENANCE_TIMER: usize = 1;
const TRAVEL_INPUT_TIMER: usize = 2;
const TRAVEL_RETRY_MS: u64 = 30_000;
const PLAYER_START_TIMEOUT_MS: u64 = 20_000;
const MAX_CANDIDATES_PER_ATTEMPT: usize = 3;

#[derive(Clone, Copy)]
enum Mode {
    Fullscreen,
    Preview(WindowIdentity),
    #[cfg(debug_assertions)]
    Developer(DisplayMode),
}

impl Mode {
    fn display(self, config: AppConfig) -> DisplayMode {
        match self {
            #[cfg(debug_assertions)]
            Self::Developer(mode) => mode,
            _ => config.display_mode,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Coordinator,
    Surface,
}

struct TravelFetchCompletion {
    generation: u64,
    purpose: FetchPurpose,
    catalog_healthy: bool,
    source: Result<TravelSource, String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FetchPurpose {
    Current,
    Prefetch,
}

struct TravelSession {
    rotation: TravelRotation,
    sender: mpsc::Sender<TravelFetchCompletion>,
    receiver: mpsc::Receiver<TravelFetchCompletion>,
    generation: u64,
    fetching: bool,
    catalog_healthy: bool,
    state: NetworkState,
    source: Option<TravelSource>,
    prefetched_source: Option<TravelSource>,
    shell_ready: bool,
    awaiting_playback: bool,
    playback_token: u32,
    load_started: u64,
    last_playing: u64,
    retry_at: u64,
    prefetch_retry_at: u64,
    browser_startup: Option<TravelWebViewStartup>,
    browser: Option<TravelWebView>,
    browser_error: Option<String>,
}

impl TravelSession {
    fn new(seed: u32) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            rotation: TravelRotation::new(seed),
            sender,
            receiver,
            generation: 0,
            fetching: false,
            catalog_healthy: false,
            state: NetworkState::Offline,
            source: None,
            prefetched_source: None,
            shell_ready: false,
            awaiting_playback: false,
            playback_token: 0,
            load_started: 0,
            last_playing: 0,
            retry_at: 0,
            prefetch_retry_at: 0,
            browser_startup: None,
            browser: None,
            browser_error: None,
        }
    }

    fn activate_source(&mut self, source: TravelSource, now: u64) {
        self.playback_token = self.playback_token.wrapping_add(1).max(1);
        self.source = Some(source);
        self.awaiting_playback = true;
        self.load_started = now;
        self.state = self.state.transition(NetworkEvent::SourceResolved);
    }
}

struct Session {
    mode: Mode,
    config: Cell<AppConfig>,
    countdown_seconds: Cell<u32>,
    windows: RefCell<Vec<HWND>>,
    coordinator: Cell<HWND>,
    lifecycle: Cell<Shutdown>,
    baseline: Cell<Option<InputBaseline>>,
    last_input_tick: Cell<Option<u32>>,
    timer: Cell<usize>,
    input_timer: Cell<usize>,
    error: Cell<Option<AppError>>,
    in_loop: Cell<bool>,
    closing: Cell<bool>,
    old_cursor: Cell<HCURSOR>,
    timeline: Cell<Option<Timeline>>,
    frame: Cell<Option<FrameSnapshot>>,
    interval: Cell<u32>,
    travel: RefCell<Option<TravelSession>>,
    travel_host: Cell<HWND>,
}

impl Session {
    fn new(mode: Mode, config: AppConfig, countdown_seconds: u32) -> Self {
        // SAFETY: This is only a seed read and has no lifetime or ownership effect.
        let seed_tick = unsafe { GetTickCount64() } as u32;
        let travel = (matches!(mode, Mode::Fullscreen)
            && mode.display(config) == DisplayMode::JapanTravel)
            .then(|| {
                TravelSession::new(seed_tick ^ unsafe { GetCurrentProcessId() }.rotate_left(13))
            });
        Self {
            mode,
            config: Cell::new(config),
            countdown_seconds: Cell::new(countdown_seconds),
            windows: RefCell::new(Vec::new()),
            coordinator: Cell::new(ptr::null_mut()),
            lifecycle: Cell::new(Shutdown::default()),
            baseline: Cell::new(None),
            last_input_tick: Cell::new(None),
            timer: Cell::new(0),
            input_timer: Cell::new(0),
            error: Cell::new(None),
            in_loop: Cell::new(false),
            closing: Cell::new(false),
            old_cursor: Cell::new(ptr::null_mut()),
            timeline: Cell::new(None),
            frame: Cell::new(None),
            interval: Cell::new(1000),
            travel: RefCell::new(travel),
            travel_host: Cell::new(ptr::null_mut()),
        }
    }

    fn display(&self) -> DisplayMode {
        self.mode.display(self.config.get())
    }

    fn style(&self) -> Style {
        match self.mode {
            #[cfg(debug_assertions)]
            Mode::Developer(_) => Style::default(),
            _ => Style::from_config(self.config.get()),
        }
    }

    fn initialize_travel(&self) {
        if self.travel.borrow().is_none() || self.lifecycle.get().stopping {
            return;
        }
        let host = self.travel_host.get();
        let coordinator = self.coordinator.get();
        if host.is_null() || coordinator.is_null() {
            return;
        }

        let shell = travel::travel_html_shell(self.config.get().travel_style);
        let startup = prepare_travel_storage(&shell).and_then(|(user_data, content)| {
            TravelWebViewStartup::start(host, coordinator, &user_data, &content)
        });
        if self.lifecycle.get().stopping {
            drop(startup);
            return;
        }
        if let Some(travel) = self.travel.borrow_mut().as_mut() {
            match startup {
                Ok(startup) => travel.browser_startup = Some(startup),
                Err(error) => {
                    travel.browser_error = Some(error);
                    travel.state = NetworkState::Offline;
                }
            }
        }
        // SAFETY: GetTickCount64 is a monotonic, process-independent clock read.
        self.begin_travel_fetch(unsafe { GetTickCount64() }, FetchPurpose::Current);
    }

    fn begin_travel_fetch(&self, now: u64, purpose: FetchPurpose) {
        let coordinator = self.coordinator.get();
        if coordinator.is_null() || self.lifecycle.get().stopping {
            return;
        }
        let (generation, purpose, candidates, sender, check_catalog) = {
            let mut travel_slot = self.travel.borrow_mut();
            let Some(travel) = travel_slot.as_mut() else {
                return;
            };
            if travel.fetching || travel.browser_error.is_some() && travel.source.is_some() {
                return;
            }
            travel.generation = travel.generation.wrapping_add(1);
            travel.fetching = true;
            if purpose == FetchPurpose::Current {
                travel.state = travel.state.transition(NetworkEvent::FetchStarted);
            }
            let current = travel
                .source
                .as_ref()
                .map(|source| source.camera_id.as_str());
            let mut candidates = travel.rotation.candidates();
            candidates.retain(|candidate| Some(candidate.camera_id) != current);
            candidates.truncate(MAX_CANDIDATES_PER_ATTEMPT);
            (
                travel.generation,
                purpose,
                candidates,
                travel.sender.clone(),
                !travel.catalog_healthy,
            )
        };
        if purpose == FetchPurpose::Current {
            self.update_travel_status();
        }

        let coordinator_value = coordinator as usize;
        let spawn = thread::Builder::new()
            .name("japan-travel-source-check".into())
            .spawn(move || {
                let catalog = if check_catalog {
                    travel::check_catalog().map(|()| true)
                } else {
                    Ok(true)
                };
                let completion = match catalog {
                    Err(error) => TravelFetchCompletion {
                        generation,
                        purpose,
                        catalog_healthy: false,
                        source: Err(error.to_string()),
                    },
                    Ok(catalog_healthy) => {
                        let mut last_error = "沒有可用的日本即時影像候選".to_owned();
                        let mut source = None;
                        for candidate in candidates {
                            match travel::fetch_source(candidate) {
                                Ok(resolved) => {
                                    source = Some(resolved);
                                    break;
                                }
                                Err(error) => last_error = error.to_string(),
                            }
                        }
                        TravelFetchCompletion {
                            generation,
                            purpose,
                            catalog_healthy,
                            source: source.ok_or(last_error),
                        }
                    }
                };
                if sender.send(completion).is_ok() {
                    // SAFETY: The HWND is an opaque copied value. Failure means
                    // shutdown won the race; receiver teardown reclaims the value.
                    unsafe {
                        PostMessageW(coordinator_value as HWND, TRAVEL_FETCH_DONE, 0, 0);
                    }
                }
            });
        if let Err(_error) = spawn {
            if let Some(travel) = self.travel.borrow_mut().as_mut() {
                travel.fetching = false;
                if purpose == FetchPurpose::Current {
                    travel.state = travel.state.transition(NetworkEvent::FetchFailed);
                    travel.retry_at = now.saturating_add(TRAVEL_RETRY_MS);
                } else {
                    travel.prefetch_retry_at = now.saturating_add(TRAVEL_RETRY_MS);
                }
            }
            self.update_travel_status();
        }
    }

    fn receive_travel_fetch(&self, now: u64) {
        let completions = {
            let travel_slot = self.travel.borrow();
            let Some(travel) = travel_slot.as_ref() else {
                return;
            };
            let mut completions = Vec::new();
            while let Ok(completion) = travel.receiver.try_recv() {
                completions.push(completion);
            }
            completions
        };
        for completion in completions {
            let mut should_load = false;
            {
                let mut travel_slot = self.travel.borrow_mut();
                let Some(travel) = travel_slot.as_mut() else {
                    return;
                };
                if completion.generation != travel.generation {
                    continue;
                }
                travel.fetching = false;
                travel.catalog_healthy |= completion.catalog_healthy;
                match completion.source {
                    Ok(source) => {
                        debug_assert!(source
                            .embed_url()
                            .starts_with("https://www.youtube-nocookie.com/"));
                        let keep_prefetched = completion.purpose == FetchPurpose::Prefetch
                            && travel.state == NetworkState::Playing
                            && !TravelRotation::due(travel.last_playing, now);
                        if keep_prefetched {
                            travel.prefetched_source = Some(source);
                        } else {
                            travel.activate_source(source, now);
                            should_load = travel.shell_ready && travel.browser.is_some();
                        }
                        if travel.browser.is_none() && travel.browser_startup.is_none() {
                            travel.awaiting_playback = false;
                            travel.state = NetworkState::Offline;
                            travel.retry_at = u64::MAX;
                        }
                    }
                    Err(_error) => {
                        if completion.purpose == FetchPurpose::Prefetch
                            && travel.state == NetworkState::Playing
                        {
                            travel.prefetch_retry_at = now.saturating_add(TRAVEL_RETRY_MS);
                        } else {
                            travel.awaiting_playback = false;
                            travel.state = travel.state.transition(NetworkEvent::FetchFailed);
                            travel.retry_at = now.saturating_add(TRAVEL_RETRY_MS);
                        }
                    }
                }
            }
            self.update_travel_status();
            if should_load {
                self.load_current_travel_source(now);
            }
        }
    }

    fn travel_shell_ready(&self, now: u64) {
        let should_load = {
            let mut travel_slot = self.travel.borrow_mut();
            let Some(travel) = travel_slot.as_mut() else {
                return;
            };
            travel.shell_ready = true;
            travel.source.is_some() && travel.awaiting_playback
        };
        self.update_travel_status();
        if should_load {
            self.load_current_travel_source(now);
        }
    }

    fn load_current_travel_source(&self, now: u64) {
        let script = self.travel.borrow().as_ref().and_then(|travel| {
            travel
                .source
                .as_ref()
                .map(|source| travel::load_source_script(source, travel.playback_token))
        });
        let Some(script) = script else {
            return;
        };
        match self.with_travel_browser(|browser| browser.execute_script(&script)) {
            Ok(()) => {
                if let Some(travel) = self.travel.borrow_mut().as_mut() {
                    travel.load_started = now;
                    travel.state = NetworkState::LoadingPlayer;
                }
            }
            Err(error) => {
                if let Some(travel) = self.travel.borrow_mut().as_mut() {
                    travel.awaiting_playback = false;
                    travel.state = NetworkState::Offline;
                    travel.retry_at = u64::MAX;
                    travel.browser_error = Some(error);
                }
            }
        }
        self.update_travel_status();
    }

    fn receive_travel_player_event(&self, now: u64) {
        let notification = self
            .travel
            .borrow()
            .as_ref()
            .and_then(|travel| travel.browser.as_ref())
            .and_then(TravelWebView::take_player_notification);
        let Some(notification) = notification else {
            return;
        };
        let event = match notification.kind {
            PlayerEventKind::Ready => NetworkEvent::PlayerReady,
            PlayerEventKind::Playing => NetworkEvent::PlayerPlaying,
            PlayerEventKind::Failed => NetworkEvent::PlayerError,
            PlayerEventKind::Stalled => NetworkEvent::PlayerStalled,
        };
        let mut start_prefetch = false;
        let mut load_prefetched = false;
        {
            let mut travel_slot = self.travel.borrow_mut();
            let Some(travel) = travel_slot.as_mut() else {
                return;
            };
            if notification.token != travel.playback_token {
                return;
            }
            match event {
                NetworkEvent::PlayerReady if travel.awaiting_playback => {
                    travel.state = travel.state.transition(event);
                }
                NetworkEvent::PlayerPlaying if travel.awaiting_playback => {
                    travel.state = travel.state.transition(event);
                    travel.awaiting_playback = false;
                    travel.last_playing = now;
                    if let Some(source) = travel.source.as_ref() {
                        travel.rotation.mark_playing(&source.camera_id);
                    }
                    travel.prefetch_retry_at = now;
                    start_prefetch = true;
                }
                NetworkEvent::PlayerError | NetworkEvent::PlayerStalled
                    if travel.awaiting_playback || travel.state == NetworkState::Playing =>
                {
                    if let Some(source) = travel.prefetched_source.take() {
                        travel.activate_source(source, now);
                        load_prefetched = travel.shell_ready && travel.browser.is_some();
                    } else {
                        travel.state = travel.state.transition(event);
                        travel.awaiting_playback = false;
                        travel.retry_at = now;
                    }
                }
                _ => return,
            }
        }
        self.update_travel_status();
        if load_prefetched {
            self.load_current_travel_source(now);
        }
        if start_prefetch {
            self.begin_travel_fetch(now, FetchPurpose::Prefetch);
        }
    }

    fn maintain_travel(&self, now: u64) {
        self.receive_travel_webview_startup();
        self.receive_travel_fetch(now);
        let mut should_load = false;
        let fetch_purpose = {
            let mut travel_slot = self.travel.borrow_mut();
            let Some(travel) = travel_slot.as_mut() else {
                return;
            };
            if travel.browser_error.is_some() {
                None
            } else if matches!(
                travel.state,
                NetworkState::LoadingPlayer | NetworkState::PlayerReady
            ) && travel.browser.is_some()
                && travel.shell_ready
                && travel.awaiting_playback
                && now.saturating_sub(travel.load_started) >= PLAYER_START_TIMEOUT_MS
            {
                travel.state = NetworkState::SourceFailed;
                travel.awaiting_playback = false;
                travel.retry_at = now;
                Some(FetchPurpose::Current)
            } else if travel.state == NetworkState::Playing {
                if TravelRotation::due(travel.last_playing, now) {
                    if let Some(source) = travel.prefetched_source.take() {
                        travel.activate_source(source, now);
                        should_load = travel.shell_ready && travel.browser.is_some();
                        None
                    } else if !travel.fetching {
                        Some(FetchPurpose::Current)
                    } else {
                        None
                    }
                } else if travel.prefetched_source.is_none()
                    && !travel.fetching
                    && now >= travel.prefetch_retry_at
                {
                    Some(FetchPurpose::Prefetch)
                } else {
                    None
                }
            } else {
                (matches!(
                    travel.state,
                    NetworkState::Offline | NetworkState::SourceFailed | NetworkState::Stalled
                ) && now >= travel.retry_at
                    && !travel.fetching)
                    .then_some(FetchPurpose::Current)
            }
        };
        if should_load {
            self.load_current_travel_source(now);
        }
        if let Some(purpose) = fetch_purpose {
            self.begin_travel_fetch(now, purpose);
        }
    }

    fn receive_travel_webview_startup(&self) {
        let outcome = self
            .travel
            .borrow_mut()
            .as_mut()
            .and_then(|travel| travel.browser_startup.as_mut())
            .map(TravelWebViewStartup::poll);
        match outcome {
            Some(StartupPoll::Ready(browser)) => {
                if let Some(travel) = self.travel.borrow_mut().as_mut() {
                    travel.browser_startup.take();
                    travel.browser = Some(browser);
                    travel.browser_error = None;
                }
            }
            Some(StartupPoll::Failed(error)) => {
                if let Some(travel) = self.travel.borrow_mut().as_mut() {
                    travel.browser_startup.take();
                    travel.browser_error = Some(error);
                    travel.awaiting_playback = false;
                    travel.state = NetworkState::Offline;
                    travel.retry_at = u64::MAX;
                }
            }
            Some(StartupPoll::Pending) | None => {}
        }
        self.update_travel_status();
    }

    fn travel_browser_failed(&self) {
        let browser = {
            let mut travel_slot = self.travel.borrow_mut();
            let Some(travel) = travel_slot.as_mut() else {
                return;
            };
            travel.browser_error = Some("WebView2 navigation or renderer failed".into());
            travel.shell_ready = false;
            travel.awaiting_playback = false;
            travel.state = NetworkState::Offline;
            travel.retry_at = u64::MAX;
            travel.browser.take()
        };
        drop(browser);
        let host = self.travel_host.get();
        if !host.is_null() {
            // SAFETY: The surface is owned by this UI thread. After the child
            // controller closes, repaint exposes the GDI fallback immediately.
            unsafe { windows_sys::Win32::Graphics::Gdi::InvalidateRect(host, ptr::null(), 0) };
        }
    }

    fn travel_shell_navigated(&self) {
        if self.with_travel_browser(TravelWebView::show).is_err() {
            self.travel_browser_failed();
        }
    }

    fn update_travel_status(&self) {
        let script = {
            let travel_slot = self.travel.borrow();
            let Some(travel) = travel_slot.as_ref() else {
                return;
            };
            if !travel.shell_ready || travel.browser.is_none() {
                return;
            }
            travel::set_status_script(travel.state.label())
        };
        let _ = self.with_travel_browser(|browser| browser.execute_script(&script));
    }

    fn resize_travel(&self, hwnd: HWND, width: i32, height: i32) {
        if hwnd == self.travel_host.get() {
            let _ = self.with_travel_browser(|browser| browser.resize(width, height));
        }
    }

    fn with_travel_browser(
        &self,
        operation: impl FnOnce(&TravelWebView) -> Result<(), String>,
    ) -> Result<(), String> {
        let browser = self
            .travel
            .borrow_mut()
            .as_mut()
            .and_then(|travel| travel.browser.take())
            .ok_or_else(|| "WebView2 player is unavailable".to_owned())?;
        let result = operation(&browser);
        if !self.lifecycle.get().stopping {
            if let Some(travel) = self.travel.borrow_mut().as_mut() {
                travel.browser = Some(browser);
                return result;
            }
        }
        drop(browser);
        result
    }

    fn close_travel(&self) {
        self.travel_host.set(ptr::null_mut());
        let travel = self.travel.borrow_mut().take();
        drop(travel);
    }

    fn register(&self, hwnd: HWND) {
        // No FFI or callback-producing operation occurs while borrowing this Vec.
        self.windows.borrow_mut().push(hwnd);
        let mut lifecycle = self.lifecycle.get();
        lifecycle.created();
        self.lifecycle.set(lifecycle);
    }

    fn destroyed(&self, hwnd: HWND, role: Role) {
        self.windows.borrow_mut().retain(|&entry| entry != hwnd);
        let mut lifecycle = self.lifecycle.get();
        lifecycle.destroyed();
        self.lifecycle.set(lifecycle);
        if role == Role::Coordinator {
            self.coordinator.set(ptr::null_mut());
        }
        self.request_shutdown();
        if role == Role::Coordinator {
            self.close_all();
        }
        if self.lifecycle.get().live == 0 && self.in_loop.get() {
            // SAFETY: This posts WM_QUIT on the current UI thread only, after
            // all registered window states have been detached from their HWNDs.
            unsafe { PostQuitMessage(0) };
        }
    }

    fn fail(&self, error: AppError) {
        if self.error.get().is_none() {
            self.error.set(Some(error));
        }
        self.request_shutdown();
    }

    fn request_shutdown(&self) {
        let mut lifecycle = self.lifecycle.get();
        let first = lifecycle.begin();
        self.lifecycle.set(lifecycle);
        if first {
            let coordinator = self.coordinator.get();
            // SAFETY: Posting to an expired/null HWND fails safely. The caller
            // holds an Rc<Session>, never a borrowed WindowState across cleanup.
            if coordinator.is_null() || unsafe { PostMessageW(coordinator, CLOSE_ALL, 0, 0) } == 0 {
                self.close_all();
            }
        }
    }

    fn close_all(&self) {
        if self.closing.replace(true) {
            return;
        }
        let mut lifecycle = self.lifecycle.get();
        lifecycle.begin();
        self.lifecycle.set(lifecycle);
        let coordinator = self.coordinator.get();
        let timer = self.timer.replace(0);
        if timer != 0 && !coordinator.is_null() {
            // SAFETY: This timer belongs to this UI thread's coordinator.
            unsafe { KillTimer(coordinator, timer) };
        }
        let input_timer = self.input_timer.replace(0);
        if input_timer != 0 && !coordinator.is_null() {
            // SAFETY: This input poll timer belongs to this coordinator.
            unsafe { KillTimer(coordinator, input_timer) };
        }
        // Close WebView2 before destroying its parent surface. This also drops
        // the completion receiver, so late worker results are reclaimed safely.
        self.close_travel();
        // Clone and release the RefCell borrow BEFORE any reentrant DestroyWindow.
        let windows = self.windows.borrow().clone();
        for hwnd in windows.into_iter().filter(|&hwnd| hwnd != coordinator) {
            // SAFETY: All entries are owned by this UI thread; destruction can
            // synchronously reenter, but the snapshot holds no borrowed state.
            unsafe { DestroyWindow(hwnd) };
        }
        if !coordinator.is_null() {
            // SAFETY: Destroy the message-only coordinator after every surface.
            unsafe { DestroyWindow(coordinator) };
        }
        if matches!(self.mode, Mode::Fullscreen) {
            let previous = self.old_cursor.replace(ptr::null_mut());
            if !previous.is_null() {
                // SAFETY: SetCursor returned this borrowed cursor; no ShowCursor
                // counter was changed and we never own/delete the cursor.
                unsafe { SetCursor(previous) };
            }
        }
        self.closing.set(false);
    }

    fn hide_cursor(&self) {
        // SAFETY: NULL selects no cursor; retain the previous borrowed handle.
        let previous = unsafe { SetCursor(ptr::null_mut()) };
        if self.old_cursor.get().is_null() && !previous.is_null() {
            self.old_cursor.set(previous);
        }
    }

    fn check_mouse(&self) {
        if let Some(baseline) = self.baseline.get() {
            let mut point = POINT::default();
            // SAFETY: point is writable; coordinates match the PMv2 screen baseline.
            if unsafe { GetCursorPos(&mut point) } == 0 {
                self.fail(last_error("GetCursorPos"));
                return;
            }
            // SAFETY: GetTickCount64 has no preconditions.
            if baseline.moved(unsafe { GetTickCount64() }, point.x, point.y) {
                self.request_shutdown();
            }
        }
    }

    fn check_travel_input(&self) {
        if self.travel.borrow().is_none() || !matches!(self.mode, Mode::Fullscreen) {
            return;
        }
        let (Some(started), Some(baseline)) = (self.last_input_tick.get(), self.baseline.get())
        else {
            return;
        };
        // Preserve the same startup grace used by cursor movement, then compare
        // the session-wide last-input tick so even short events consumed by the
        // WebView2 child are observed.
        if !baseline.ready(unsafe { GetTickCount64() }) {
            return;
        }
        let mut info = LASTINPUTINFO {
            cbSize: size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        // SAFETY: info has the documented size and remains writable for the call.
        if unsafe { GetLastInputInfo(&mut info) } == 0 {
            self.fail(last_error("GetLastInputInfo"));
        } else if info.dwTime != started {
            self.request_shutdown();
        }
    }

    fn check_foreground(&self) {
        let Some(baseline) = self.baseline.get() else {
            return;
        };
        // SAFETY: All calls use OS-owned opaque handles or writable stack outputs.
        unsafe {
            if baseline.ready(GetTickCount64()) {
                let foreground = GetForegroundWindow();
                let mut process = 0;
                GetWindowThreadProcessId(foreground, &mut process);
                if process != GetCurrentProcessId() {
                    self.request_shutdown();
                }
            }
        }
    }

    fn maintain(&self) {
        if self.lifecycle.get().stopping {
            return;
        }
        let mut restart = false;
        match self.mode {
            Mode::Fullscreen => {
                self.check_foreground();
                self.check_mouse();
            }
            Mode::Preview(parent) => {
                if let Err(error) = self.resize_preview(parent) {
                    if matches!(error, AppError::InvalidParent)
                        || !parent.alive()
                        || self.lifecycle.get().stopping
                    {
                        self.request_shutdown();
                    } else {
                        self.fail(error);
                    }
                }
                let latest = crate::registry::load_registry();
                if latest != self.config.get() {
                    self.config.set(latest);
                    self.countdown_seconds.set(latest.last_countdown_seconds);
                    restart = true;
                }
            }
            #[cfg(debug_assertions)]
            Mode::Developer(_) => (),
        }
        if self.travel.borrow().is_some() && !self.lifecycle.get().stopping {
            // SAFETY: Read the same monotonic clock used for rotation deadlines.
            self.maintain_travel(unsafe { GetTickCount64() });
        }
        if !self.lifecycle.get().stopping {
            if let Err(error) = self.sample_frame(restart) {
                self.fail(error);
            }
        }
    }

    fn sample_frame(&self, restart: bool) -> Result<(), AppError> {
        let mut local = SYSTEMTIME::default();
        // SAFETY: The coordinator samples wall time and the monotonic clock once
        // for the entire generation; renderers never read either clock.
        let tick = unsafe {
            GetLocalTime(&mut local);
            GetTickCount64()
        };
        let local = LocalTime {
            year: local.wYear,
            month: local.wMonth,
            day: local.wDay,
            hour: local.wHour,
            minute: local.wMinute,
            second: local.wSecond,
        };
        if !local.valid() {
            return Err(AppError::OperationFailed("GetLocalTime snapshot"));
        }
        let preview_static =
            matches!(self.mode, Mode::Preview(_)) && self.display() == DisplayMode::Countdown;
        let mut timeline = if restart || preview_static || self.timeline.get().is_none() {
            Timeline::new(self.display(), self.countdown_seconds.get(), tick)
                .map_err(|_| AppError::OperationFailed("countdown deadline"))?
        } else {
            self.timeline
                .get()
                .ok_or(AppError::OperationFailed("timeline"))?
        };
        let frame = timeline.sample(local, tick);
        self.frame.set(Some(frame));
        self.timeline.set(Some(timeline));
        let interval = if matches!(self.mode, Mode::Preview(_)) {
            1000
        } else {
            timeline.interval(frame)
        };
        if self.interval.replace(interval) != interval && self.timer.get() != 0 {
            // SAFETY: Replace the existing coordinator timer without adding another timer.
            let timer =
                unsafe { SetTimer(self.coordinator.get(), MAINTENANCE_TIMER, interval, None) };
            if timer == 0 {
                return Err(last_error("SetTimer(cadence)"));
            }
            self.timer.set(timer);
        }
        let windows = self.windows.borrow().clone();
        for hwnd in windows {
            if let Some(surface) = surface_for(hwnd) {
                if matches!(self.mode, Mode::Fullscreen) {
                    let (width, height) = client_size(hwnd)?;
                    surface.update_drift(tick, width, height, self.display());
                }
                // SAFETY: Invalidation does not erase; WM_PAINT uses the same saved generation.
                unsafe {
                    windows_sys::Win32::Graphics::Gdi::InvalidateRect(hwnd, ptr::null(), 0);
                }
            }
        }
        Ok(())
    }

    fn resize_preview(&self, parent: WindowIdentity) -> Result<(), AppError> {
        if !parent.alive() {
            return Err(AppError::InvalidParent);
        }
        let surface = self
            .windows
            .borrow()
            .iter()
            .copied()
            .find(|&hwnd| hwnd != self.coordinator.get());
        let Some(surface) = surface else {
            return Err(AppError::InvalidParent);
        };
        // SAFETY: A disappearing cross-process parent is checked again on each poll.
        if unsafe { GetParent(surface) } != parent.hwnd {
            return Err(AppError::InvalidParent);
        }
        let _dpi = DpiScope::for_window(parent.hwnd)?;
        let (width, height) = client_size(parent.hwnd)?;
        if client_size(surface)? != (width, height) {
            // SAFETY: Same UI thread owns surface, coordinates are in the parent
            // context. No borrow of the window list is retained during reentry.
            if unsafe {
                SetWindowPos(
                    surface,
                    ptr::null_mut(),
                    0,
                    0,
                    width,
                    height,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )
            } == 0
            {
                return Err(last_error("SetWindowPos(preview)"));
            }
        }
        Ok(())
    }
}

fn prepare_travel_storage(shell_html: &str) -> Result<(PathBuf, PathBuf), String> {
    let local_app_data = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "LOCALAPPDATA is unavailable for WebView2 data".to_owned())?;
    let base = local_app_data.join("KOMSMOS").join("tools-screensaver-tzk");
    let user_data = base.join("WebView2");
    let content = base.join("TravelContent");
    fs::create_dir_all(&user_data)
        .map_err(|error| format!("cannot create WebView2 user data directory: {error}"))?;
    fs::create_dir_all(&content)
        .map_err(|error| format!("cannot create travel content directory: {error}"))?;
    let shell = content.join("index.html");
    let needs_write = fs::read(&shell)
        .map(|bytes| bytes != shell_html.as_bytes())
        .unwrap_or(true);
    if needs_write {
        fs::write(&shell, shell_html.as_bytes())
            .map_err(|error| format!("cannot write travel player shell: {error}"))?;
    }
    Ok((user_data, content))
}

struct WindowState {
    session: Rc<Session>,
    role: Role,
    dpi: Cell<u32>,
    surface: Option<Rc<Surface>>,
    #[cfg(test)]
    fault: u8,
    #[cfg(test)]
    drops: Rc<Cell<usize>>,
}

impl WindowState {
    fn new(session: Rc<Session>, role: Role) -> Self {
        let tick = session.frame.get().map_or(0, |frame| frame.tick);
        let seed = tick as u32 ^ (session.lifecycle.get().live as u32).wrapping_mul(0x9e3779b9);
        Self {
            session,
            role,
            dpi: Cell::new(96),
            surface: (role == Role::Surface).then(|| {
                Rc::new(Surface {
                    renderer: RefCell::new(Some(Renderer::default())),
                    drift: Cell::new(Drift::new(seed, tick)),
                })
            }),
            #[cfg(test)]
            fault: 0,
            #[cfg(test)]
            drops: Rc::new(Cell::new(0)),
        }
    }
}

struct Surface {
    renderer: RefCell<Option<Renderer>>,
    drift: Cell<Drift>,
}
impl Surface {
    fn update_drift(&self, tick: u64, width: i32, height: i32, mode: DisplayMode) {
        if let Some(layout) = Layout::new(width, height, mode) {
            let mut drift = self.drift.get();
            drift.update(tick, width, height, layout.group);
            self.drift.set(drift);
        }
    }
}

fn surface_for(hwnd: HWND) -> Option<Rc<Surface>> {
    // SAFETY: Called only for this UI thread's known class windows. Copy the Rc
    // before any call that can reenter and free WindowState.
    unsafe {
        let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
        raw.as_ref().and_then(|state| state.surface.clone())
    }
}

#[cfg(test)]
impl Drop for WindowState {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

// Caller owns the Box until WM_NCCREATE installs it successfully. From that
// point WM_NCDESTROY is its sole owner, including WM_CREATE failure cleanup.
struct PendingWindow {
    state: Option<Box<WindowState>>,
}

struct WindowClass {
    instance: HINSTANCE,
}

impl WindowClass {
    fn register(instance: HINSTANCE, icon: HICON) -> Result<Self, AppError> {
        // SAFETY: Borrowed system cursor/brush; none are deleted by this class.
        let (cursor, brush) = unsafe {
            (
                LoadCursorW(ptr::null_mut(), IDC_ARROW),
                GetStockObject(BLACK_BRUSH),
            )
        };
        if cursor.is_null() {
            return Err(last_error("LoadCursorW"));
        }
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            hIcon: icon,
            hCursor: cursor,
            hbrBackground: brush,
            ..Default::default()
        };
        // SAFETY: class and its static name stay valid during registration; the
        // callback has the exact Win32 ABI and all class handles are borrowed.
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(last_error("RegisterClassW"));
        }
        Ok(Self { instance })
    }

    fn create(
        &self,
        state: WindowState,
        bounds: Bounds,
        parent: HWND,
        style: u32,
        ex_style: u32,
    ) -> Result<HWND, AppError> {
        let mut pending = PendingWindow {
            state: Some(Box::new(state)),
        };
        // SAFETY: pending stays pinned on this stack throughout synchronous
        // creation. The callback transfers only its contained Box, not pending.
        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                CLASS_NAME,
                WINDOW_TITLE,
                style,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                parent,
                ptr::null_mut(),
                self.instance,
                &mut pending as *mut _ as *const _,
            )
        };
        if hwnd.is_null() {
            Err(last_error("CreateWindowExW"))
        } else {
            Ok(hwnd)
        }
    }
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        // SAFETY: The session guard is dropped first, so no windows remain.
        unsafe { UnregisterClassW(CLASS_NAME, self.instance) };
    }
}

struct SessionGuard(Rc<Session>);
impl Drop for SessionGuard {
    fn drop(&mut self) {
        self.0.in_loop.set(false);
        self.0.close_all();
    }
}

pub(crate) fn fullscreen(
    instance: HINSTANCE,
    icon: HICON,
    config: AppConfig,
    countdown_seconds: u32,
) -> Result<(), AppError> {
    run(instance, icon, Mode::Fullscreen, config, countdown_seconds)
}

#[cfg(debug_assertions)]
pub(crate) fn developer(
    instance: HINSTANCE,
    icon: HICON,
    mode: DisplayMode,
) -> Result<(), AppError> {
    run(
        instance,
        icon,
        Mode::Developer(mode),
        AppConfig::default(),
        300,
    )
}

pub(crate) fn preview(instance: HINSTANCE, icon: HICON, parent: usize) -> Result<(), AppError> {
    let parent = WindowIdentity::capture(parent as HWND).ok_or(AppError::InvalidParent)?;
    let config = crate::registry::load_registry();
    let result = run(
        instance,
        icon,
        Mode::Preview(parent),
        config,
        config.last_countdown_seconds,
    );
    if result.is_err() && !parent.alive() {
        Err(AppError::InvalidParent)
    } else {
        result
    }
}

fn run(
    instance: HINSTANCE,
    icon: HICON,
    mode: Mode,
    config: AppConfig,
    countdown_seconds: u32,
) -> Result<(), AppError> {
    let class = WindowClass::register(instance, icon)?;
    let session = Rc::new(Session::new(mode, config, countdown_seconds));
    let _guard = SessionGuard(Rc::clone(&session));
    let coordinator = class.create(
        WindowState::new(Rc::clone(&session), Role::Coordinator),
        Bounds {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
        HWND_MESSAGE,
        0,
        0,
    )?;
    session.coordinator.set(coordinator);
    match mode {
        Mode::Fullscreen => {
            let mut surfaces = Vec::new();
            for bounds in monitor::enumerate()? {
                surfaces.push(class.create(
                    WindowState::new(Rc::clone(&session), Role::Surface),
                    bounds,
                    ptr::null_mut(),
                    WS_POPUP,
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                )?);
            }
            session.sample_frame(true)?;
            // SAFETY: Every surface is fully initialized and belongs to this UI
            // thread. Show without activating each monitor in turn, then request
            // the foreground only once; do not circumvent foreground policies.
            unsafe {
                for &hwnd in &surfaces {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
                if let Some(&first) = surfaces.first() {
                    SetForegroundWindow(first);
                    SetFocus(first);
                }
            }
            let mut point = POINT::default();
            // SAFETY: Writable point; take the baseline only after all windows show.
            if unsafe { GetCursorPos(&mut point) } == 0 {
                return Err(last_error("GetCursorPos"));
            }
            // SAFETY: GetTickCount64 is an OS monotonic tick read.
            session.baseline.set(Some(InputBaseline {
                tick: unsafe { GetTickCount64() },
                x: point.x,
                y: point.y,
            }));
            let mut last_input = LASTINPUTINFO {
                cbSize: size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            // SAFETY: The initialized struct has the documented size.
            if unsafe { GetLastInputInfo(&mut last_input) } == 0 {
                return Err(last_error("GetLastInputInfo"));
            }
            session.last_input_tick.set(Some(last_input.dwTime));
            session.hide_cursor();
            if session.display() == DisplayMode::JapanTravel {
                if let Some(&primary) = surfaces.first() {
                    session.travel_host.set(primary);
                    // The already-visible GDI surface remains responsive while
                    // WebView2 performs its bounded, message-pumping startup.
                    session.initialize_travel();
                }
            }
        }
        Mode::Preview(parent) => {
            session.sample_frame(true)?;
            let _dpi = DpiScope::for_window(parent.hwnd)?;
            let (width, height) = client_size(parent.hwnd)?;
            if !parent.alive() {
                return Err(AppError::InvalidParent);
            }
            class.create(
                WindowState::new(Rc::clone(&session), Role::Surface),
                Bounds {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
                parent.hwnd,
                WS_CHILD | WS_VISIBLE,
                0,
            )?;
        }
        #[cfg(debug_assertions)]
        Mode::Developer(_) => {
            let hwnd = class.create(
                WindowState::new(Rc::clone(&session), Role::Surface),
                Bounds {
                    x: CW_USEDEFAULT,
                    y: CW_USEDEFAULT,
                    width: 960,
                    height: 600,
                },
                ptr::null_mut(),
                WS_OVERLAPPEDWINDOW,
                0,
            )?;
            session.sample_frame(true)?;
            let title = match session.display() {
                DisplayMode::TimeDate => {
                    windows_sys::w!("tools-screensaver-tzk — Phase 3 / TimeDate (Debug)")
                }
                DisplayMode::Countdown => {
                    windows_sys::w!("tools-screensaver-tzk — Phase 3 / Countdown (Debug)")
                }
                DisplayMode::JapanTravel => {
                    windows_sys::w!("tools-screensaver-tzk — Japan Travel (Debug)")
                }
            };
            // SAFETY: Show the fully initialized ordinary developer window.
            unsafe {
                SetWindowTextW(hwnd, title);
                ShowWindow(hwnd, SW_SHOWNORMAL);
            }
        }
    }
    if session.lifecycle.get().stopping {
        return Ok(());
    }
    // SAFETY: Timer belongs to the live message-only coordinator; no timer callback.
    let timer = unsafe { SetTimer(coordinator, MAINTENANCE_TIMER, session.interval.get(), None) };
    if timer == 0 {
        return Err(last_error("SetTimer"));
    }
    session.timer.set(timer);
    if session.travel.borrow().is_some() {
        // SAFETY: A dedicated short timer catches mouse clicks consumed by the
        // WebView2 child without increasing the renderer's one-second cadence.
        let input_timer = unsafe { SetTimer(coordinator, TRAVEL_INPUT_TIMER, 100, None) };
        if input_timer == 0 {
            return Err(last_error("SetTimer(travel input)"));
        }
        session.input_timer.set(input_timer);
    }
    session.in_loop.set(true);
    let mut message = MSG::default();
    loop {
        // SAFETY: message is writable; NULL receives all this UI thread's messages.
        let status = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
        if status == -1 {
            return Err(last_error("GetMessageW"));
        }
        if status == 0 {
            break;
        }
        // SAFETY: The message was supplied by GetMessageW, not synthesized pointers.
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    session.in_loop.set(false);
    match session.error.get() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        if lparam == 0 {
            return 0;
        }
        // SAFETY: WM_NCCREATE lParam points to the CREATESTRUCT supplied by Win32.
        let pending =
            unsafe { (*(lparam as *const CREATESTRUCTW)).lpCreateParams as *mut PendingWindow };
        if pending.is_null() {
            return 0;
        }
        #[cfg(test)]
        // SAFETY: PendingWindow belongs to the synchronous class.create call.
        if unsafe {
            (*pending)
                .state
                .as_ref()
                .is_some_and(|state| state.fault == 1)
        } {
            return 0;
        }
        // SAFETY: This is the sole transfer point; the temporary borrow ends here.
        let Some(state) = (unsafe { (*pending).state.take() }) else {
            return 0;
        };
        let raw = Box::into_raw(state);
        if set_pointer(hwnd, GWLP_USERDATA, raw as isize).is_err() {
            // SAFETY: Installation failed; no callback owns raw. Restore caller ownership.
            unsafe {
                (*pending).state = Some(Box::from_raw(raw));
            }
            return 0;
        }
        // SAFETY: raw is now owned by WM_NCDESTROY. register performs no Win32 calls.
        unsafe {
            (*raw).session.register(hwnd);
        }
        return 1;
    }

    // SAFETY: Only this class's creation/destruction handlers modify GWLP_USERDATA.
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
    if raw.is_null() {
        // SAFETY: Win32 supplied this message; no application state exists yet/anymore.
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    if message == WM_NCDESTROY {
        let _ = set_pointer(hwnd, GWLP_USERDATA, 0);
        // SAFETY: The pointer is cleared before any reentrant work, reclaimed once.
        let state = unsafe { Box::from_raw(raw) };
        let session = Rc::clone(&state.session);
        let role = state.role;
        drop(state);
        session.destroyed(hwnd, role);
        // SAFETY: DefWindowProc releases remaining Win32 nonclient bookkeeping.
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    // SAFETY: Clone/copy all necessary state BEFORE any potentially reentrant FFI.
    // No &WindowState survives DispatchMessage, SetWindowPos or DestroyWindow.
    let (session, role) = unsafe { (Rc::clone(&(*raw).session), (*raw).role) };
    match message {
        #[cfg(debug_assertions)]
        TEST_GET_CONFIG_SNAPSHOT => {
            let config = session.config.get();
            let display = match session.display() {
                DisplayMode::TimeDate => 0,
                DisplayMode::Countdown => 1,
                DisplayMode::JapanTravel => 2,
            };
            let font = match config.font_mode {
                crate::model::FontMode::SevenSegment => 0,
                crate::model::FontMode::Consolas => 1,
                crate::model::FontMode::MingLiu => 2,
                crate::model::FontMode::Custom => 3,
            };
            display | ((config.color_preset.registry_value() as isize) << 8) | (font << 16)
        }
        WM_CREATE => {
            #[cfg(test)]
            // SAFETY: No calls since loading raw; it is still this window's state.
            if unsafe { (*raw).fault == 2 } {
                return -1;
            }
            // SAFETY: hwnd is created and raw stays valid for this non-reentrant read.
            unsafe {
                (*raw).dpi.set(GetDpiForWindow(hwnd).max(96));
            }
            if let Some(surface) = surface_for(hwnd) {
                let renderer = surface.renderer.borrow_mut().take();
                if let Some(mut renderer) = renderer {
                    // SAFETY: This window has reached WM_CREATE on the owning thread.
                    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
                    let result = renderer.warm(hwnd, dpi, session.display(), session.style());
                    surface.renderer.borrow_mut().replace(renderer);
                    if let Err(error) = result {
                        session.error.set(Some(error));
                        return -1;
                    }
                }
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            if let (Some(surface), Some(frame)) = (surface_for(hwnd), session.frame.get()) {
                // Move the renderer out of its slot before BeginPaint (which may
                // reenter). The Rc survives WM_NCDESTROY, with no WindowState
                // reference or RefCell borrow held across the paint operation.
                let renderer = surface.renderer.borrow_mut().take();
                if let Some(mut renderer) = renderer {
                    // SAFETY: GetDpiForWindow validates the opaque handle.
                    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
                    let offset = if matches!(session.mode, Mode::Fullscreen) {
                        surface.drift.get().offset
                    } else {
                        (0, 0)
                    };
                    let result = renderer.paint(
                        hwnd,
                        dpi,
                        session.display(),
                        frame,
                        session.style(),
                        offset,
                    );
                    surface.renderer.borrow_mut().replace(renderer);
                    if let Err(error) = result {
                        session.fail(error);
                    }
                } else if let Err(error) = paint_black(hwnd) {
                    session.fail(error);
                }
            } else if let Err(error) = paint_black(hwnd) {
                session.fail(error);
            }
            0
        }
        WM_CLOSE => {
            session.request_shutdown();
            0
        }
        CLOSE_ALL if role == Role::Coordinator => {
            session.close_all();
            0
        }
        TRAVEL_FETCH_DONE if role == Role::Coordinator => {
            // SAFETY: The worker only posts an event; the UI thread drains owned
            // channel values and applies the latest generation.
            session.receive_travel_fetch(unsafe { GetTickCount64() });
            0
        }
        WEBVIEW_STARTUP_CHANGED if role == Role::Coordinator => {
            session.receive_travel_webview_startup();
            0
        }
        SHELL_READY if role == Role::Coordinator => {
            // SAFETY: Monotonic timestamp for the pending player deadline.
            session.travel_shell_ready(unsafe { GetTickCount64() });
            0
        }
        PLAYER_EVENT if role == Role::Coordinator => {
            // SAFETY: Monotonic timestamp validates the tokened player event and
            // starts the one-minute window only for the current source.
            session.receive_travel_player_event(unsafe { GetTickCount64() });
            0
        }
        BROWSER_FAILED if role == Role::Coordinator => {
            session.travel_browser_failed();
            0
        }
        SHELL_NAVIGATED if role == Role::Coordinator => {
            session.travel_shell_navigated();
            0
        }
        WM_TIMER if role == Role::Coordinator && wparam == session.timer.get() => {
            session.maintain();
            0
        }
        WM_TIMER if role == Role::Coordinator && wparam == session.input_timer.get() => {
            session.check_travel_input();
            0
        }
        CHECK_FOREGROUND => {
            session.check_foreground();
            0
        }
        WM_SETCURSOR if matches!(session.mode, Mode::Fullscreen) => {
            session.hide_cursor();
            1
        }
        WM_MOUSEMOVE if matches!(session.mode, Mode::Fullscreen) => {
            session.check_mouse();
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_LBUTTONDOWN | WM_MBUTTONDOWN | WM_RBUTTONDOWN
        | WM_XBUTTONDOWN | WM_MOUSEWHEEL | WM_MOUSEHWHEEL
            if matches!(session.mode, Mode::Fullscreen) =>
        {
            session.request_shutdown();
            0
        }
        WM_ACTIVATE | WM_ACTIVATEAPP if matches!(session.mode, Mode::Fullscreen) => {
            // SAFETY: Deferred checking sees the final foreground after internal
            // activation transitions, instead of treating every WA_INACTIVE as exit.
            unsafe {
                PostMessageW(session.coordinator.get(), CHECK_FOREGROUND, 0, 0);
            }
            0
        }
        WM_DISPLAYCHANGE if matches!(session.mode, Mode::Fullscreen) => {
            session.request_shutdown();
            0
        }
        WM_DPICHANGED => {
            // SAFETY: Store before calls which may destroy the state through reentry.
            unsafe {
                (*raw).dpi.set(GetDpiForWindow(hwnd).max(96));
            }
            if role == Role::Surface && matches!(session.mode, Mode::Fullscreen) {
                match monitor::info_for_window(hwnd)
                    .ok()
                    .and_then(|info| Bounds::from_rect(info.rcMonitor))
                {
                    Some(bounds) => {
                        // SAFETY: Refit the owned popup to current monitor bounds,
                        // rather than using a normal window's suggested rectangle.
                        if unsafe {
                            SetWindowPos(
                                hwnd,
                                HWND_TOPMOST,
                                bounds.x,
                                bounds.y,
                                bounds.width,
                                bounds.height,
                                SWP_NOACTIVATE,
                            )
                        } == 0
                        {
                            session.fail(last_error("SetWindowPos(DPI)"));
                        }
                    }
                    None => {
                        session.fail(AppError::OperationFailed("monitor bounds after DPI change"))
                    }
                }
            }
            0
        }
        WM_SIZE => {
            if let (Some(surface), Ok((width, height))) = (surface_for(hwnd), client_size(hwnd)) {
                if width == 0 || height == 0 {
                    // Move resources out before deletion; a nested paint owns
                    // its renderer independently and will recheck size next time.
                    let renderer = surface.renderer.borrow_mut().take();
                    if let Some(renderer) = renderer {
                        drop(renderer);
                        surface.renderer.borrow_mut().replace(Renderer::default());
                    }
                }
                session.resize_travel(hwnd, width, height);
            }
            if matches!(session.mode, Mode::Fullscreen) {
                if let (Some(surface), Some(frame), Ok((width, height))) =
                    (surface_for(hwnd), session.frame.get(), client_size(hwnd))
                {
                    surface.update_drift(frame.tick, width, height, session.display());
                }
            }
            // SAFETY: Invalidate only; actual black painting happens in WM_PAINT.
            unsafe {
                windows_sys::Win32::Graphics::Gdi::InvalidateRect(hwnd, ptr::null(), 0);
            }
            0
        }
        WM_QUERYENDSESSION => 1,
        WM_TIMECHANGE | WM_POWERBROADCAST => {
            if let Err(error) = session.sample_frame(false) {
                session.fail(error);
            }
            1
        }
        WM_ENDSESSION if wparam != 0 => {
            session.request_shutdown();
            0
        }
        _ => {
            // SAFETY: Preserve default behavior for unhandled messages.
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
    }
}

fn paint_black(hwnd: HWND) -> Result<(), AppError> {
    let mut paint = PAINTSTRUCT::default();
    // SAFETY: hwnd is currently handling WM_PAINT and paint is a writable struct.
    let dc = unsafe { BeginPaint(hwnd, &mut paint) };
    let mut result = Ok(());
    if dc.is_null() {
        result = Err(AppError::OperationFailed("BeginPaint"));
    } else if paint.rcPaint.right > paint.rcPaint.left && paint.rcPaint.bottom > paint.rcPaint.top {
        // SAFETY: The paint DC/rectangle are borrowed from BeginPaint. The stock
        // black brush is borrowed and never deleted. No allocation for zero sizes.
        if unsafe { FillRect(dc, &paint.rcPaint, GetStockObject(BLACK_BRUSH)) } == 0 {
            result = Err(AppError::OperationFailed("FillRect"));
        }
    }
    // SAFETY: Always pair the BeginPaint invocation, including the failure path.
    unsafe {
        EndPaint(hwnd, &paint);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

    #[test]
    fn activating_a_source_advances_a_nonzero_playback_token() {
        let mut travel = TravelSession::new(1);
        let source = |camera: &str| TravelSource {
            camera_id: camera.into(),
            place: "日本".into(),
            youtube_id: "Ee27soLzJ5c".into(),
        };
        travel.activate_source(source("first"), 100);
        assert_eq!(travel.playback_token, 1);
        assert!(travel.awaiting_playback);
        travel.activate_source(source("second"), 200);
        assert_eq!(travel.playback_token, 2);
        assert_eq!(travel.load_started, 200);
        travel.playback_token = u32::MAX;
        travel.activate_source(source("wrapped"), 300);
        assert_eq!(travel.playback_token, 1);
    }

    #[test]
    fn real_creation_failures_and_partial_group_cleanup_drop_each_state_once() {
        // SAFETY: This test creates only hidden message-only windows on its own
        // test thread. It never activates UI, changes settings, or runs an idle saver.
        let instance = unsafe { GetModuleHandleW(ptr::null()) };
        let class = WindowClass::register(instance, ptr::null_mut()).expect("register test class");
        let session = Rc::new(Session::new(Mode::Fullscreen, AppConfig::default(), 300));
        let drops = Rc::new(Cell::new(0));
        let bounds = Bounds {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
        for fault in [1, 2] {
            let mut state = WindowState::new(Rc::clone(&session), Role::Surface);
            state.fault = fault;
            state.drops = Rc::clone(&drops);
            assert!(class.create(state, bounds, HWND_MESSAGE, 0, 0).is_err());
            assert_eq!(session.lifecycle.get().live, 0);
        }
        assert_eq!(drops.get(), 2);
        let session = Rc::new(Session::new(Mode::Fullscreen, AppConfig::default(), 300));
        for _ in 0..3 {
            let mut state = WindowState::new(Rc::clone(&session), Role::Surface);
            state.drops = Rc::clone(&drops);
            class
                .create(state, bounds, HWND_MESSAGE, 0, 0)
                .expect("create hidden surface");
        }
        session.close_all();
        session.close_all();
        assert_eq!(session.lifecycle.get().live, 0);
        assert!(session.windows.borrow().is_empty());
        assert_eq!(drops.get(), 5);

        // Reproduce run() returning early when a later window fails WM_CREATE:
        // the guard must also reclaim the coordinator and prior hidden surfaces.
        let session = Rc::new(Session::new(Mode::Fullscreen, AppConfig::default(), 300));
        let guard = SessionGuard(Rc::clone(&session));
        let mut handles = Vec::new();
        for role in [Role::Coordinator, Role::Surface, Role::Surface] {
            let mut state = WindowState::new(Rc::clone(&session), role);
            state.drops = Rc::clone(&drops);
            let hwnd = class
                .create(state, bounds, HWND_MESSAGE, 0, 0)
                .expect("create partial group");
            if role == Role::Coordinator {
                session.coordinator.set(hwnd);
            }
            handles.push(hwnd);
        }
        let mut failed = WindowState::new(Rc::clone(&session), Role::Surface);
        failed.fault = 2;
        failed.drops = Rc::clone(&drops);
        assert!(class.create(failed, bounds, HWND_MESSAGE, 0, 0).is_err());
        drop(guard);
        assert_eq!(session.lifecycle.get().live, 0);
        assert!(session.windows.borrow().is_empty());
        assert_eq!(drops.get(), 9);
        for hwnd in handles {
            // SAFETY: IsWindow permits querying a stale opaque handle.
            assert_eq!(unsafe { IsWindow(hwnd) }, 0);
        }
    }
}
