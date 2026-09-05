//! Explicit GUI integration tests. Run serially with --ignored --nocapture.
//! Each child process is owned by a guard with bounded cleanup. No system settings change.
use std::{
    env,
    path::PathBuf,
    process::{Child, Command},
    ptr, thread,
    time::{Duration, Instant},
};

use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

struct Saver(Child);
impl Saver {
    fn start(args: &[&str]) -> Self {
        let executable = env::var_os("SCREENSAVER_TEST_EXE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_my_datetime_screensaver")));
        Self(
            Command::new(executable)
                .args(args)
                .spawn()
                .expect("start owned saver process"),
        )
    }
    fn until(&mut self, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            pump();
            if condition() {
                return;
            }
            let status = self.0.try_wait().expect("poll child");
            assert!(
                status.is_none(),
                "saver exited before expected condition: {status:?}"
            );
            assert!(
                Instant::now() < deadline,
                "condition timed out for PID {}",
                self.0.id()
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn stays_alive(&mut self, milliseconds: u64) {
        let deadline = Instant::now() + Duration::from_millis(milliseconds);
        while Instant::now() < deadline {
            pump();
            assert!(
                self.0.try_wait().expect("poll child").is_none(),
                "saver exited unexpectedly"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn exits(&mut self, code: i32) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            pump();
            if let Some(status) = self.0.try_wait().expect("poll child") {
                assert_eq!(status.code(), Some(code));
                assert!(
                    windows(self.0.id(), ptr::null_mut()).is_empty(),
                    "residual top-level windows"
                );
                return;
            }
            assert!(Instant::now() < deadline, "child did not exit");
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Saver {
    fn drop(&mut self) {
        // Only this exact Child handle is terminated on failure; never match process names.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Dpi(DPI_AWARENESS_CONTEXT);
impl Dpi {
    fn set(context: DPI_AWARENESS_CONTEXT) -> Self {
        // SAFETY: The test uses documented context constants on its own thread.
        let old = unsafe { SetThreadDpiAwarenessContext(context) };
        assert!(!old.is_null());
        Self(old)
    }
}
impl Drop for Dpi {
    fn drop(&mut self) {
        // SAFETY: Restore the previous valid context on the same thread.
        unsafe {
            SetThreadDpiAwarenessContext(self.0);
        }
    }
}

struct Host(HWND);
impl Host {
    fn new() -> Self {
        // SAFETY: STATIC is a predefined class. The host belongs to this test
        // process/thread, making the saver child genuinely cross-process.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                windows_sys::w!("STATIC"),
                windows_sys::w!("Phase 3 test preview host"),
                WS_POPUP | WS_CLIPCHILDREN,
                160,
                160,
                320,
                180,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            )
        };
        assert!(!hwnd.is_null());
        // SAFETY: Show this small host without stealing keyboard focus.
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        Self(hwnd)
    }
    fn resize(&self, width: i32, height: i32) {
        // SAFETY: Resizes only the test's owned host; does not touch monitor configuration.
        assert_ne!(
            unsafe {
                SetWindowPos(
                    self.0,
                    ptr::null_mut(),
                    0,
                    0,
                    width,
                    height,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
            },
            0
        );
    }
    fn close(&mut self) {
        if !self.0.is_null() {
            // SAFETY: The test UI thread owns this window.
            unsafe {
                DestroyWindow(self.0);
            }
            self.0 = ptr::null_mut();
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.close();
    }
}

fn pump() {
    let mut message = MSG::default();
    // SAFETY: Pump only this test thread's queue, allowing cross-process Win32
    // parenting/modal messages to complete while waiting for the child.
    unsafe {
        while PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn windows(process: u32, parent: HWND) -> Vec<HWND> {
    let mut found = (process, Vec::new());
    // SAFETY: Win32 calls collect synchronously with the live stack context.
    unsafe {
        if parent.is_null() {
            EnumWindows(Some(collect), &mut found as *mut _ as LPARAM);
        } else {
            EnumChildWindows(parent, Some(collect), &mut found as *mut _ as LPARAM);
        }
    }
    found.1
}

unsafe extern "system" fn collect(hwnd: HWND, parameter: LPARAM) -> i32 {
    let mut process = 0;
    // SAFETY: Writable output; callback context is the synchronous windows call.
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process);
        let expected = (*(parameter as *const (u32, Vec<HWND>))).0;
        if process == expected {
            let mut name = [0u16; 128];
            let length = GetClassNameW(hwnd, name.as_mut_ptr(), name.len() as i32);
            let name = String::from_utf16_lossy(&name[..length.max(0) as usize]);
            // Exclude OS-created IME helper windows; they are not saver surfaces.
            if name == "MyDateTimeScreensaver.Window" || name == "#32770" {
                (*(parameter as *mut (u32, Vec<HWND>))).1.push(hwnd);
            }
        }
    }
    1
}

fn client(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    // SAFETY: Win32 validates the handle; rect is writable.
    assert_ne!(unsafe { GetClientRect(hwnd, &mut rect) }, 0);
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn bounds(hwnd: HWND) -> (i32, i32, i32, i32) {
    let mut rect = RECT::default();
    // SAFETY: Live window and writable output.
    assert_ne!(unsafe { GetWindowRect(hwnd, &mut rect) }, 0);
    (rect.left, rect.top, rect.right, rect.bottom)
}

fn send(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> usize {
    let mut result = 0;
    // SAFETY: These are system messages (< WM_USER) to our own child process.
    // Win32 marshals WM_DPICHANGED's RECT; that caller keeps its input alive.
    // Allow sent-message reentry so the foreign child can call back into its host.
    assert_ne!(
        unsafe {
            SendMessageTimeoutW(
                hwnd,
                message,
                wparam,
                lparam,
                SMTO_ABORTIFHUNG,
                2000,
                &mut result,
            )
        },
        0,
        "message {message:#x}, error {}",
        unsafe { windows_sys::Win32::Foundation::GetLastError() }
    );
    result
}

fn rendered_frame(hwnd: HWND) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        // SAFETY: Read the safety-margin corner on our own child, then release
        // the borrowed DC. The render/layout tests cover the content pixels.
        let corner = unsafe {
            let dc = GetDC(hwnd);
            assert!(!dc.is_null());
            let corner = GetPixel(dc, 1, 1);
            ReleaseDC(hwnd, dc);
            corner
        };
        if corner == 0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "black safety margin not observed: corner={corner:#x}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn invalid_real_command_lines_exit_two_without_windows() {
    for args in [
        vec!["/p"],
        vec!["/p:0"],
        vec!["/p:1"],
        vec!["/p", "0x20"],
        vec!["/p:18446744073709551616"],
        vec!["/c:"],
        vec!["/s", "/c"],
        vec!["--dev-render=unknown"],
        vec!["/c", "１２"],
    ] {
        let mut saver = Saver::start(&args);
        saver.exits(2);
    }
}

#[test]
fn install_helper_refuses_a_non_system32_copy() {
    let mut saver = Saver::start(&["--install-set-current"]);
    saver.exits(4);
}

#[test]
#[ignore = "shows a small native host; run explicitly and serially"]
fn preview_embeds_resizes_and_exits_in_three_dpi_contexts() {
    for (name, context) in [
        ("unaware", DPI_AWARENESS_CONTEXT_UNAWARE),
        ("system", DPI_AWARENESS_CONTEXT_SYSTEM_AWARE),
        ("PMv2", DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2),
    ] {
        let _dpi = Dpi::set(context);
        let mut host = Host::new();
        // SAFETY: Read-only desktop foreground observation.
        let foreground = unsafe { GetForegroundWindow() };
        let mut saver = Saver::start(&["-P", &format!("{:010}", host.0 as usize)]);
        let process = saver.0.id();
        saver.until(|| windows(process, host.0).len() == 1);
        let child = windows(process, host.0)[0];
        saver.stays_alive(150);
        // SAFETY: Read-only queries against our live host and saver child.
        unsafe {
            assert_eq!(GetParent(child), host.0);
            assert_eq!(
                GetWindowLongPtrW(child, GWL_STYLE) as u32 & (WS_CHILD | WS_VISIBLE),
                WS_CHILD | WS_VISIBLE
            );
            assert_eq!(
                GetWindowLongPtrW(child, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST,
                0
            );
            assert_eq!(GetForegroundWindow(), foreground, "preview stole focus");
            assert_ne!(
                AreDpiAwarenessContextsEqual(
                    GetWindowDpiAwarenessContext(child),
                    GetWindowDpiAwarenessContext(host.0)
                ),
                0
            );
            let mut origin = POINT::default();
            ClientToScreen(child, &mut origin);
            ScreenToClient(host.0, &mut origin);
            assert_eq!((origin.x, origin.y), (0, 0));
            println!(
                "preview {name}: host DPI {}, child DPI {}",
                GetDpiForWindow(host.0),
                GetDpiForWindow(child)
            );
        }
        assert_eq!(client(child), (320, 180));
        rendered_frame(child);
        for message in [WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_KEYDOWN, WM_MOUSEWHEEL] {
            send(child, message, 0, 0);
        }
        saver.stays_alive(100);
        for (width, height) in [(480, 270), (0, 0), (1, 1), (320, 180)] {
            host.resize(width, height);
            saver.until(|| client(child) == (width, height));
            println!("preview {name}: resized to {width}x{height}");
        }
        host.close();
        saver.exits(0);
        // SAFETY: Win32 accepts a stale opaque handle for this existence query.
        assert_eq!(unsafe { IsWindow(child) }, 0);
    }
}

#[test]
#[ignore = "shows the Phase 3 configuration dialog; run explicitly and serially"]
fn configure_default_invalid_and_real_owners() {
    let _dpi = Dpi::set(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    for args in [vec![], vec!["/c"], vec!["/c:0"], vec!["/c", "1"]] {
        let mut saver = Saver::start(&args);
        let process = saver.0.id();
        saver.until(|| !windows(process, ptr::null_mut()).is_empty());
        let dialog = windows(process, ptr::null_mut())[0];
        // SAFETY: Read the dialog's owner relationship.
        assert!(unsafe { GetWindow(dialog, GW_OWNER) }.is_null());
        send(dialog, WM_COMMAND, IDCANCEL as usize, 0);
        saver.exits(0);
    }
    for destroy_owner in [false, true] {
        let mut host = Host::new();
        let mut saver = Saver::start(&[&format!("/C:{}", host.0 as usize)]);
        let process = saver.0.id();
        saver.until(|| !windows(process, ptr::null_mut()).is_empty());
        let dialog = windows(process, ptr::null_mut())[0];
        saver.stays_alive(100);
        // SAFETY: Query only the windows created by this test and its child.
        unsafe {
            assert_eq!(GetWindow(dialog, GW_OWNER), host.0);
            assert_eq!(
                IsWindowEnabled(host.0),
                0,
                "owner must be disabled by modal dialog"
            );
            let mut info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            assert_ne!(
                GetMonitorInfoW(
                    MonitorFromWindow(host.0, MONITOR_DEFAULTTONEAREST),
                    &mut info
                ),
                0
            );
            let (left, top, right, bottom) = bounds(dialog);
            assert!((left + right - info.rcWork.left - info.rcWork.right).abs() <= 2);
            assert!((top + bottom - info.rcWork.top - info.rcWork.bottom).abs() <= 2);
        }
        if destroy_owner {
            host.close();
        } else {
            send(dialog, WM_COMMAND, IDCANCEL as usize, 0);
        }
        saver.exits(0);
        if !destroy_owner {
            // SAFETY: The owned host is still valid and must be re-enabled.
            assert_ne!(unsafe { IsWindowEnabled(host.0) }, 0);
        }
    }
    println!("configure: default/zero/stale/live owners and cancellation passed without writes");
}

fn monitor_rectangles() -> Vec<(i32, i32, i32, i32)> {
    unsafe extern "system" fn callback(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: OS callback monitor and live synchronous stack vector.
        unsafe {
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return 0;
            }
            let rect = info.rcMonitor;
            (*(data as *mut Vec<(i32, i32, i32, i32)>)).push((
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
            ));
        }
        1
    }
    let mut rectangles: Vec<(i32, i32, i32, i32)> = Vec::new();
    // SAFETY: Callback receives a writable stack vector for the synchronous enumeration.
    assert_ne!(
        unsafe {
            EnumDisplayMonitors(
                ptr::null_mut(),
                ptr::null(),
                Some(callback),
                &mut rectangles as *mut _ as LPARAM,
            )
        },
        0
    );
    rectangles.sort_unstable();
    rectangles.dedup();
    rectangles
}

#[test]
#[ignore = "briefly covers all displays in black; run explicitly and serially"]
fn fullscreen_covers_monitors_and_handles_exit_messages() {
    let _dpi = Dpi::set(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    let expected = monitor_rectangles();
    println!("physical monitor rectangles: {expected:?}");
    let messages = [
        WM_KEYDOWN,
        WM_SYSKEYDOWN,
        WM_LBUTTONDOWN,
        WM_MBUTTONDOWN,
        WM_RBUTTONDOWN,
        WM_XBUTTONDOWN,
        WM_MOUSEWHEEL,
        WM_MOUSEHWHEEL,
        WM_DISPLAYCHANGE,
        WM_ENDSESSION,
        WM_CLOSE,
    ];
    for (index, message) in messages.into_iter().enumerate() {
        println!("starting fullscreen case {index}, message {message:#06x}");
        let mut saver = Saver::start(&[if index == 0 { "-S" } else { "/s" }]);
        let process = saver.0.id();
        saver.until(|| windows(process, ptr::null_mut()).len() == expected.len());
        let surfaces = windows(process, ptr::null_mut());
        saver.until(|| {
            surfaces.iter().all(|&hwnd| {
                // SAFETY: Query the child's known windows.
                unsafe { IsWindowVisible(hwnd) != 0 }
            })
        });
        let mut actual: Vec<_> = surfaces.iter().map(|&hwnd| bounds(hwnd)).collect();
        actual.sort_unstable();
        assert_eq!(actual, expected);
        for &surface in &surfaces {
            // SAFETY: Read styles from known saver windows.
            unsafe {
                assert_eq!(
                    GetWindowLongPtrW(surface, GWL_STYLE) as u32 & (WS_POPUP | WS_CHILD),
                    WS_POPUP
                );
                assert_eq!(
                    GetWindowLongPtrW(surface, GWL_EXSTYLE) as u32
                        & (WS_EX_TOPMOST | WS_EX_TOOLWINDOW),
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW
                );
            }
        }
        if index == 0 {
            saver.stays_alive(150);
            for &surface in &surfaces {
                // SAFETY: Read DPI from the live saver surface.
                println!("fullscreen surface {:?}, DPI {}", bounds(surface), unsafe {
                    GetDpiForWindow(surface)
                });
            }
            send(surfaces[0], WM_MOUSEMOVE, 0, 0x7fff7fff);
            saver.stays_alive(100);
            // Synthetic DPI event checks the current-monitor sizing policy; no actual DPI changes.
            let suggested = RECT {
                left: 160,
                top: 160,
                right: 480,
                bottom: 340,
            };
            send(
                surfaces[0],
                WM_DPICHANGED,
                96 | (96 << 16),
                &suggested as *const RECT as isize,
            );
            assert!(expected.contains(&bounds(surfaces[0])));
            assert_eq!(send(surfaces[0], WM_QUERYENDSESSION, 0, 0), 1);
        }
        send(surfaces[0], message, 1, 0);
        saver.exits(0);
        println!(
            "fullscreen: exit message {message:#06x}, all {} surfaces removed",
            surfaces.len()
        );
    }
}

#[test]
#[ignore = "temporarily switches foreground between owned test windows"]
fn fullscreen_foreground_transitions_when_windows_allows_activation() {
    let _dpi = Dpi::set(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    let host = Host::new();
    let expected = monitor_rectangles().len();
    let mut saver = Saver::start(&["/s"]);
    let process = saver.0.id();
    saver.until(|| windows(process, ptr::null_mut()).len() == expected);
    let surfaces = windows(process, ptr::null_mut());
    saver.stays_alive(650);
    for &surface in &surfaces {
        // SAFETY: Request ordinary activation; never override foreground policy.
        if unsafe { SetForegroundWindow(surface) } == 0 {
            println!("NOT TESTED: foreground activation denied by Windows policy");
            send(surfaces[0], WM_CLOSE, 0, 0);
            saver.exits(0);
            return;
        }
        saver.stays_alive(150);
    }
    println!("foreground: same-process activation retained all {expected} surfaces after grace");
    // SAFETY: The test process owns this host; request ordinary external activation.
    if unsafe { SetForegroundWindow(host.0) } == 0 {
        println!("NOT TESTED: external foreground activation denied by Windows policy");
        send(surfaces[0], WM_CLOSE, 0, 0);
    } else {
        println!("foreground: external host activated, expecting saver exit");
    }
    saver.exits(0);
}
