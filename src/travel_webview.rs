//! A narrowly scoped WebView2 host for the Japan travel player.
//!
//! COM objects stay on the existing UI STA. Network catalog work is performed
//! elsewhere; this module only hosts a local shell and the official YouTube
//! iframe player that shell creates.

use std::{cell::Cell, mem, path::Path, rc::Rc, sync::mpsc};

use webview2_com::{
    CoTaskMemPWSTR, CoreWebView2EnvironmentOptions, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, DownloadStartingEventHandler,
    ExecuteScriptCompletedHandler, Microsoft::Web::WebView2::Win32::*,
    NavigationCompletedEventHandler, NavigationStartingEventHandler,
    NewWindowRequestedEventHandler, PermissionRequestedEventHandler, ProcessFailedEventHandler,
    WebMessageReceivedEventHandler,
};
use windows::{
    core::{Interface, PCWSTR, PWSTR},
    Win32::{
        Foundation::{E_POINTER, HWND, RECT},
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
    },
};
use windows_sys::Win32::{
    Foundation::HWND as SysHwnd,
    UI::WindowsAndMessaging::{IsWindow, PostMessageW, WM_APP},
};

pub(crate) const WEBVIEW_STARTUP_CHANGED: u32 = WM_APP + 38;
pub(crate) const SHELL_READY: u32 = WM_APP + 39;
pub(crate) const PLAYER_EVENT: u32 = WM_APP + 40;
pub(crate) const BROWSER_FAILED: u32 = WM_APP + 45;
pub(crate) const SHELL_NAVIGATED: u32 = WM_APP + 46;

const CLOSE_ALL: u32 = WM_APP + 1;
const SHELL_URL: &str = "https://travel.screensaver.local/index.html";
const SHELL_HOST: &str = "travel.screensaver.local";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerEventKind {
    Ready,
    Playing,
    Failed,
    Stalled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerNotification {
    pub kind: PlayerEventKind,
    pub token: u32,
}

struct ComApartment;

struct ControllerCloseGuard(Option<ICoreWebView2Controller>);

impl ControllerCloseGuard {
    fn disarm(&mut self) {
        self.0.take();
    }
}

impl Drop for ControllerCloseGuard {
    fn drop(&mut self) {
        if let Some(controller) = self.0.take() {
            // SAFETY: A controller that cannot be committed to TravelWebView
            // must be closed while its COM apartment is still initialized.
            let _ = unsafe { controller.Close() };
        }
    }
}

impl ComApartment {
    fn initialize() -> Result<Self, String> {
        // SAFETY: The screensaver owns its UI thread. Every successful call is
        // balanced by this value's Drop after all WebView2 COM fields are gone.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .map_err(|error| format!("CoInitializeEx failed: {error}"))?;
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        // SAFETY: Paired with the successful CoInitializeEx on this UI thread.
        unsafe { CoUninitialize() };
    }
}

pub(crate) struct TravelWebView {
    webview: ICoreWebView2,
    controller: ICoreWebView2Controller,
    _environment: ICoreWebView2Environment,
    notifications: mpsc::Receiver<PlayerNotification>,
    _apartment: ComApartment,
}

enum StartupStage {
    Environment {
        receiver: mpsc::Receiver<windows::core::Result<ICoreWebView2Environment>>,
        _handler: ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler,
    },
    Controller {
        environment: ICoreWebView2Environment,
        receiver: mpsc::Receiver<windows::core::Result<ICoreWebView2Controller>>,
        _handler: ICoreWebView2CreateCoreWebView2ControllerCompletedHandler,
    },
    Finished,
}

pub(crate) enum StartupPoll {
    Pending,
    Ready(TravelWebView),
    Failed(String),
}

pub(crate) struct TravelWebViewStartup {
    parent: SysHwnd,
    coordinator: SysHwnd,
    content_dir: Vec<u16>,
    apartment: Option<ComApartment>,
    stage: StartupStage,
}

impl TravelWebViewStartup {
    pub(crate) fn start(
        parent: SysHwnd,
        coordinator: SysHwnd,
        user_data_dir: &Path,
        content_dir: &Path,
    ) -> Result<Self, String> {
        if parent.is_null() || coordinator.is_null() {
            return Err("WebView2 parent window is unavailable".into());
        }
        // Probe the per-machine/per-user Evergreen Runtime before creating COM
        // objects. A missing runtime is a normal fallback case; the screensaver
        // never launches an installer or elevation prompt at runtime.
        let _runtime_version = available_runtime_version()?;
        let apartment = ComApartment::initialize()?;
        let user_data = wide_path(user_data_dir)?;
        let content_dir = wide_path(content_dir)?;
        let options: ICoreWebView2EnvironmentOptions =
            CoreWebView2EnvironmentOptions::default().into();
        let (sender, receiver) = mpsc::channel();
        let completion_coordinator = coordinator as usize;
        let completion_surface = parent as usize;
        let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |error_code, environment| {
                let result = error_code.and_then(|()| {
                    environment.ok_or_else(|| windows::core::Error::from(E_POINTER))
                });
                if sender.send(result).is_ok() {
                    // SAFETY: The opaque HWND is only used to wake its owning UI
                    // thread; a destroyed window makes PostMessage fail safely.
                    unsafe {
                        PostMessageW(
                            completion_coordinator as SysHwnd,
                            WEBVIEW_STARTUP_CHANGED,
                            completion_surface,
                            0,
                        );
                    }
                }
                Ok(())
            },
        ));
        // SAFETY: Input strings/options live through this initiating call and
        // WebView2 retains the handler until the asynchronous completion.
        unsafe {
            CreateCoreWebView2EnvironmentWithOptions(
                PCWSTR::null(),
                PCWSTR(user_data.as_ptr()),
                &options,
                &handler,
            )
        }
        .map_err(|error| format!("WebView2 Runtime unavailable: {error}"))?;
        Ok(Self {
            parent,
            coordinator,
            content_dir,
            apartment: Some(apartment),
            stage: StartupStage::Environment {
                receiver,
                _handler: handler,
            },
        })
    }

    pub(crate) fn poll(&mut self) -> StartupPoll {
        let stage = mem::replace(&mut self.stage, StartupStage::Finished);
        match stage {
            StartupStage::Environment { receiver, _handler } => match receiver.try_recv() {
                Ok(Ok(environment)) => self.start_controller(environment),
                Ok(Err(error)) => {
                    StartupPoll::Failed(format!("WebView2 environment failed: {error}"))
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    StartupPoll::Failed("WebView2 environment callback was canceled".into())
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.stage = StartupStage::Environment { receiver, _handler };
                    StartupPoll::Pending
                }
            },
            StartupStage::Controller {
                environment,
                receiver,
                _handler,
            } => match receiver.try_recv() {
                Ok(Ok(controller)) => self.finish(environment, controller),
                Ok(Err(error)) => {
                    StartupPoll::Failed(format!("WebView2 controller failed: {error}"))
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    StartupPoll::Failed("WebView2 controller callback was canceled".into())
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.stage = StartupStage::Controller {
                        environment,
                        receiver,
                        _handler,
                    };
                    StartupPoll::Pending
                }
            },
            StartupStage::Finished => StartupPoll::Pending,
        }
    }

    fn start_controller(&mut self, environment: ICoreWebView2Environment) -> StartupPoll {
        // SAFETY: IsWindow only validates opaque handles owned by this UI thread.
        if unsafe { IsWindow(self.parent) } == 0 || unsafe { IsWindow(self.coordinator) } == 0 {
            return StartupPoll::Failed("WebView2 startup canceled during shutdown".into());
        }
        let (sender, receiver) = mpsc::channel();
        let completion_coordinator = self.coordinator as usize;
        let completion_surface = self.parent as usize;
        let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |error_code, controller| {
                let result = error_code
                    .and_then(|()| controller.ok_or_else(|| windows::core::Error::from(E_POINTER)));
                if sender.send(result).is_ok() {
                    // SAFETY: See the environment completion callback above.
                    unsafe {
                        PostMessageW(
                            completion_coordinator as SysHwnd,
                            WEBVIEW_STARTUP_CHANGED,
                            completion_surface,
                            0,
                        );
                    }
                }
                Ok(())
            },
        ));
        // SAFETY: The environment/parent are live on this STA and WebView2
        // retains the completion handler until the result is delivered.
        if let Err(error) =
            unsafe { environment.CreateCoreWebView2Controller(HWND(self.parent), &handler) }
        {
            return StartupPoll::Failed(format!("WebView2 controller startup failed: {error}"));
        }
        self.stage = StartupStage::Controller {
            environment,
            receiver,
            _handler: handler,
        };
        StartupPoll::Pending
    }

    fn finish(
        &mut self,
        environment: ICoreWebView2Environment,
        controller: ICoreWebView2Controller,
    ) -> StartupPoll {
        let mut close_on_error = ControllerCloseGuard(Some(controller.clone()));
        // SAFETY: IsWindow only validates opaque handles owned by this UI thread.
        if unsafe { IsWindow(self.parent) } == 0 || unsafe { IsWindow(self.coordinator) } == 0 {
            return StartupPoll::Failed("WebView2 startup canceled during shutdown".into());
        }
        let webview = unsafe { controller.CoreWebView2() }
            .map_err(|error| format!("CoreWebView2 failed: {error}"));
        let webview = match webview {
            Ok(value) => value,
            Err(error) => return StartupPoll::Failed(error),
        };

        let notifications = match configure(
            &controller,
            &webview,
            self.coordinator,
            self.parent as usize,
        ) {
            Ok(value) => value,
            Err(error) => return StartupPoll::Failed(error),
        };
        let mapped: ICoreWebView2_3 = match webview.cast() {
            Ok(value) => value,
            Err(error) => {
                return StartupPoll::Failed(format!(
                    "WebView2 virtual host mapping unavailable: {error}"
                ));
            }
        };
        // SAFETY: Both strings remain live for this synchronous COM call. The
        // content directory contains only the application-owned shell.
        if let Err(error) = unsafe {
            mapped.SetVirtualHostNameToFolderMapping(
                PCWSTR(wide(SHELL_HOST).as_ptr()),
                PCWSTR(self.content_dir.as_ptr()),
                COREWEBVIEW2_HOST_RESOURCE_ACCESS_KIND_DENY_CORS,
            )
        } {
            return StartupPoll::Failed(format!(
                "SetVirtualHostNameToFolderMapping failed: {error}"
            ));
        }

        let mut result = TravelWebView {
            webview,
            controller,
            _environment: environment,
            notifications,
            _apartment: self
                .apartment
                .take()
                .expect("startup owns the initialized COM apartment"),
        };
        close_on_error.disarm();
        if let Err(error) = result
            .resize_to_parent(self.parent)
            .and_then(|()| result.navigate_shell())
        {
            return StartupPoll::Failed(error);
        }
        StartupPoll::Ready(result)
    }
}

impl Drop for TravelWebViewStartup {
    fn drop(&mut self) {
        if !matches!(self.stage, StartupStage::Finished) {
            // WebView2 creation has no cancellation API. During application
            // shutdown retain the STA initialization until process teardown so
            // a late completion cannot run after CoUninitialize.
            if let Some(apartment) = self.apartment.take() {
                mem::forget(apartment);
            }
        }
    }
}

impl TravelWebView {
    pub(crate) fn resize(&self, width: i32, height: i32) -> Result<(), String> {
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        // SAFETY: The controller is live on its owning STA. Bounds use parent
        // client coordinates and do not move or activate the parent.
        unsafe {
            self.controller.SetBounds(RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            })
        }
        .map_err(|error| format!("WebView2 SetBounds failed: {error}"))
    }

    fn resize_to_parent(&mut self, parent: SysHwnd) -> Result<(), String> {
        let mut rect = windows_sys::Win32::Foundation::RECT::default();
        // SAFETY: parent is a live surface HWND supplied by the caller.
        if unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(parent, &mut rect) }
            == 0
        {
            return Err("GetClientRect(WebView2 parent) failed".into());
        }
        self.resize(rect.right - rect.left, rect.bottom - rect.top)?;
        Ok(())
    }

    pub(crate) fn navigate_shell(&self) -> Result<(), String> {
        let url = wide(SHELL_URL);
        // SAFETY: URL is a NUL-terminated HTTPS virtual-host URL and stays live
        // through the synchronous COM call.
        unsafe { self.webview.Navigate(PCWSTR(url.as_ptr())) }
            .map_err(|error| format!("WebView2 Navigate failed: {error}"))
    }

    pub(crate) fn execute_script(&self, script: &str) -> Result<(), String> {
        let script = wide(script);
        let handler = ExecuteScriptCompletedHandler::create(Box::new(|result, _json| result));
        // SAFETY: Script text and handler are live for the call; WebView2 retains
        // the handler until the asynchronous operation completes.
        unsafe {
            self.webview
                .ExecuteScript(PCWSTR(script.as_ptr()), &handler)
        }
        .map_err(|error| format!("WebView2 ExecuteScript failed: {error}"))
    }

    pub(crate) fn take_player_notification(&self) -> Option<PlayerNotification> {
        self.notifications.try_recv().ok()
    }

    pub(crate) fn show(&self) -> Result<(), String> {
        // SAFETY: The verified local shell completed navigation and this method
        // runs on the controller's creating STA.
        unsafe { self.controller.SetIsVisible(true) }
            .map_err(|error| format!("WebView2 visibility failed: {error}"))
    }
}

pub(crate) fn available_runtime_version() -> Result<String, String> {
    let mut value = PWSTR::null();
    // SAFETY: A null browser folder asks the loader for the installed Evergreen
    // Runtime. On success it returns a CoTaskMem-owned, NUL-terminated string.
    unsafe { GetAvailableCoreWebView2BrowserVersionString(PCWSTR::null(), &mut value) }
        .map_err(|error| format!("WebView2 Runtime unavailable: {error}"))?;
    if value.is_null() {
        return Err("WebView2 Runtime returned no version".into());
    }
    let version = CoTaskMemPWSTR::from(value).to_string();
    if version.trim().is_empty() {
        Err("WebView2 Runtime returned an empty version".into())
    } else {
        Ok(version)
    }
}

impl Drop for TravelWebView {
    fn drop(&mut self) {
        // SAFETY: Close is idempotent for this live controller and runs before
        // COM is uninitialized by the final struct field.
        let _ = unsafe { self.controller.Close() };
    }
}

fn configure(
    controller: &ICoreWebView2Controller,
    webview: &ICoreWebView2,
    coordinator: SysHwnd,
    surface: usize,
) -> Result<mpsc::Receiver<PlayerNotification>, String> {
    let (notification_sender, notification_receiver) = mpsc::channel();
    // SAFETY: All COM objects live on their creating STA. Event handlers retain
    // only an opaque HWND and use PostMessage, so they cannot form an Rc cycle.
    unsafe {
        let settings = webview
            .Settings()
            .map_err(|error| format!("WebView2 Settings failed: {error}"))?;
        settings
            .SetAreDefaultContextMenusEnabled(false)
            .map_err(|error| format!("disable context menu failed: {error}"))?;
        settings
            .SetAreDevToolsEnabled(false)
            .map_err(|error| format!("disable devtools failed: {error}"))?;
        settings
            .SetIsStatusBarEnabled(false)
            .map_err(|error| format!("disable status bar failed: {error}"))?;
        settings
            .SetAreDefaultScriptDialogsEnabled(false)
            .map_err(|error| format!("disable script dialogs failed: {error}"))?;
        settings
            .SetIsZoomControlEnabled(false)
            .map_err(|error| format!("disable zoom failed: {error}"))?;
        if let Ok(settings3) = settings.cast::<ICoreWebView2Settings3>() {
            let _ = settings3.SetAreBrowserAcceleratorKeysEnabled(false);
        }
        if let Ok(settings4) = settings.cast::<ICoreWebView2Settings4>() {
            let _ = settings4.SetIsPasswordAutosaveEnabled(false);
            let _ = settings4.SetIsGeneralAutofillEnabled(false);
        }
        if let Ok(webview8) = webview.cast::<ICoreWebView2_8>() {
            let _ = webview8.SetIsMuted(true);
        }
        controller
            .SetIsVisible(false)
            .map_err(|error| format!("hide WebView2 during shell startup failed: {error}"))?;

        let mut token = 0;
        webview
            .add_WebMessageReceived(
                &WebMessageReceivedEventHandler::create(Box::new(move |_sender, args| {
                    if let Some(args) = args {
                        let mut source = PWSTR::null();
                        let source_result = args.Source(&mut source);
                        let source = CoTaskMemPWSTR::from(source).to_string();
                        if source_result.is_err() || source != SHELL_URL {
                            return Ok(());
                        }
                        let mut value = PWSTR::null();
                        let value_result = args.TryGetWebMessageAsString(&mut value);
                        let value = CoTaskMemPWSTR::from(value).to_string();
                        if value_result.is_ok() {
                            if value == "shell-ready" {
                                PostMessageW(coordinator, SHELL_READY, surface, 0);
                            } else if let Some(notification) = parse_player_notification(&value) {
                                if notification_sender.send(notification).is_ok() {
                                    PostMessageW(coordinator, PLAYER_EVENT, surface, 0);
                                }
                            }
                        }
                    }
                    Ok(())
                })),
                &mut token,
            )
            .map_err(|error| format!("WebMessageReceived handler failed: {error}"))?;

        let shell_navigation = Rc::new(Cell::new(false));
        let starting_shell = Rc::clone(&shell_navigation);
        webview
            .add_NavigationStarting(
                &NavigationStartingEventHandler::create(Box::new(move |_sender, args| {
                    if let Some(args) = args {
                        let mut uri = PWSTR::null();
                        let uri_result = args.Uri(&mut uri);
                        let uri = CoTaskMemPWSTR::from(uri).to_string();
                        uri_result?;
                        if uri != SHELL_URL {
                            args.SetCancel(true)?;
                        } else {
                            starting_shell.set(true);
                        }
                    }
                    Ok(())
                })),
                &mut token,
            )
            .map_err(|error| format!("NavigationStarting handler failed: {error}"))?;

        let completed_shell = Rc::clone(&shell_navigation);
        webview
            .add_NavigationCompleted(
                &NavigationCompletedEventHandler::create(Box::new(move |_sender, args| {
                    if completed_shell.replace(false) {
                        let Some(args) = args else {
                            PostMessageW(coordinator, BROWSER_FAILED, surface, 0);
                            return Ok(());
                        };
                        let mut succeeded = windows::core::BOOL::default();
                        if args.IsSuccess(&mut succeeded).is_err() || !succeeded.as_bool() {
                            PostMessageW(coordinator, BROWSER_FAILED, surface, 0);
                        } else {
                            PostMessageW(coordinator, SHELL_NAVIGATED, surface, 0);
                        }
                    }
                    Ok(())
                })),
                &mut token,
            )
            .map_err(|error| format!("NavigationCompleted handler failed: {error}"))?;

        webview
            .add_ProcessFailed(
                &ProcessFailedEventHandler::create(Box::new(move |_sender, _args| {
                    PostMessageW(coordinator, BROWSER_FAILED, surface, 0);
                    Ok(())
                })),
                &mut token,
            )
            .map_err(|error| format!("ProcessFailed handler failed: {error}"))?;

        webview
            .add_NewWindowRequested(
                &NewWindowRequestedEventHandler::create(Box::new(|_sender, args| {
                    if let Some(args) = args {
                        args.SetHandled(true)?;
                    }
                    Ok(())
                })),
                &mut token,
            )
            .map_err(|error| format!("NewWindowRequested handler failed: {error}"))?;

        webview
            .add_PermissionRequested(
                &PermissionRequestedEventHandler::create(Box::new(|_sender, args| {
                    if let Some(args) = args {
                        args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
                    }
                    Ok(())
                })),
                &mut token,
            )
            .map_err(|error| format!("PermissionRequested handler failed: {error}"))?;

        if let Ok(webview4) = webview.cast::<ICoreWebView2_4>() {
            let _ = webview4.add_DownloadStarting(
                &DownloadStartingEventHandler::create(Box::new(|_sender, args| {
                    if let Some(args) = args {
                        args.SetCancel(true)?;
                    }
                    Ok(())
                })),
                &mut token,
            );
        }

        controller
            .add_AcceleratorKeyPressed(
                &webview2_com::AcceleratorKeyPressedEventHandler::create(Box::new(
                    move |_sender, args| {
                        if let Some(args) = args {
                            let _ = args.SetHandled(true);
                        }
                        PostMessageW(coordinator, CLOSE_ALL, 0, 0);
                        Ok(())
                    },
                )),
                &mut token,
            )
            .map_err(|error| format!("AcceleratorKeyPressed handler failed: {error}"))?;
    }
    Ok(notification_receiver)
}

fn parse_player_notification(value: &str) -> Option<PlayerNotification> {
    let (kind, token) = value.split_once(':')?;
    let token = token.parse::<u32>().ok().filter(|&value| value != 0)?;
    let kind = match kind {
        "ready" => PlayerEventKind::Ready,
        "playing" => PlayerEventKind::Playing,
        "error" => PlayerEventKind::Failed,
        "stalled" => PlayerEventKind::Stalled,
        _ => return None,
    };
    Some(PlayerNotification { kind, token })
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

fn wide_path(path: &Path) -> Result<Vec<u16>, String> {
    let value = path
        .to_str()
        .ok_or_else(|| "WebView2 path is not valid Unicode".to_owned())?;
    if value.contains('\0') {
        return Err("WebView2 path contains NUL".into());
    }
    Ok(wide(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_identity_is_https_and_paths_are_terminated() {
        assert!(SHELL_URL.starts_with("https://"));
        assert!(SHELL_URL.contains(SHELL_HOST));
        let value = wide_path(Path::new(r"C:\Users\test\Travel")).expect("ordinary local path");
        assert_eq!(value.last(), Some(&0));
    }

    #[test]
    fn player_notifications_require_known_events_and_nonzero_tokens() {
        assert_eq!(
            parse_player_notification("playing:42"),
            Some(PlayerNotification {
                kind: PlayerEventKind::Playing,
                token: 42,
            })
        );
        for invalid in [
            "playing",
            "playing:0",
            "playing:-1",
            "playing:1:2",
            "unknown:2",
            "shell-ready",
        ] {
            assert_eq!(parse_player_notification(invalid), None, "{invalid}");
        }
    }

    #[test]
    #[ignore = "host prerequisite probe; does not create a window or install software"]
    fn installed_webview2_runtime_is_detectable_without_ui() {
        let version = available_runtime_version().expect("WebView2 Evergreen Runtime");
        assert!(version
            .chars()
            .next()
            .is_some_and(|value| value.is_ascii_digit()));
        eprintln!("WebView2 Runtime: {version}");
    }
}
