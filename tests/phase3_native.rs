//! Explicit Phase 3 UI flow against a dedicated HKCU test subkey.
use std::{
    env,
    path::PathBuf,
    process::{Child, Command},
    ptr, thread,
    time::{Duration, Instant},
};

use my_datetime_screensaver::{
    config::{ColorPreset, SettingsStore},
    model::{DisplayMode, FontMode},
    registry::{load_registry, RegistryStore},
};
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::MapWindowPoints;
use windows_sys::Win32::System::Registry::{RegDeleteTreeW, HKEY_CURRENT_USER};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const TEST_KEY_ENV: &str = "MYDATETIME_SCREENSAVER_TEST_KEY";
const IDC_MODE_TIME_DATE: i32 = 1001;
const IDC_MODE_COUNTDOWN: i32 = 1002;
const IDC_COLOR_DARK_RED: i32 = 1101;
const IDC_COLOR_DARK_ORANGE: i32 = 1102;
const IDC_COLOR_BRIGHT_GREEN: i32 = 1103;
const IDC_COLOR_OFF_WHITE: i32 = 1104;
const IDC_FONT_COMBO: i32 = 1201;
const IDC_CHOOSE_FONT: i32 = 1202;
const IDC_PREVIEW: i32 = 1301;
const IDC_COUNTDOWN_HOURS: i32 = 1401;
const IDC_COUNTDOWN_ERROR: i32 = 1404;
const TEST_SET_COUNTDOWN_FIELDS: u32 = WM_APP + 30;
const TEST_GET_COUNTDOWN_FIELDS: u32 = WM_APP + 31;
const TEST_GET_CONFIG_SNAPSHOT: u32 = WM_APP + 32;
const TEST_GET_CONFIG_DRAFT: u32 = WM_APP + 33;

struct Sandbox {
    path: String,
}
impl Sandbox {
    fn new() -> Self {
        let path = format!(
            "Software\\MyDateTimeScreensaver\\Tests\\phase3-native-{}",
            std::process::id()
        );
        delete_tree(&path);
        // This ignored test is required to run serially; the child inherits only
        // this validated test-subkey override. Release builds ignore the override.
        env::set_var(TEST_KEY_ENV, &path);
        Self { path }
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        delete_tree(&self.path);
        env::remove_var(TEST_KEY_ENV);
    }
}

fn delete_tree(path: &str) {
    let path: Vec<u16> = path.encode_utf16().chain([0]).collect();
    // SAFETY: The path is unique to this test and under the prescribed Tests key.
    let code = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, path.as_ptr()) };
    assert!(code == ERROR_SUCCESS || code == ERROR_FILE_NOT_FOUND);
}

struct Saver(Child);
impl Saver {
    fn start(args: &[&str]) -> Self {
        let executable = env::var_os("SCREENSAVER_PHASE3_TEST_EXE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_my_datetime_screensaver")));
        Self(
            Command::new(executable)
                .args(args)
                .env(TEST_KEY_ENV, env::var(TEST_KEY_ENV).expect("sandbox key"))
                .spawn()
                .expect("start owned Phase 3 saver process"),
        )
    }
    fn pid(&self) -> u32 {
        self.0.id()
    }
    fn until(&mut self, seconds: u64, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(seconds);
        loop {
            pump();
            if condition() {
                return;
            }
            assert!(
                self.0.try_wait().expect("poll child").is_none(),
                "child exited before expected UI state"
            );
            if Instant::now() >= deadline {
                let details: Vec<_> = windows(self.pid())
                    .into_iter()
                    .map(|hwnd| {
                        (
                            hwnd as usize,
                            get_window_text(hwnd),
                            !unsafe { GetDlgItem(hwnd, IDC_COUNTDOWN_HOURS) }.is_null(),
                        )
                    })
                    .collect();
                panic!("Phase 3 UI condition timed out; windows={details:?}");
            }
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
                    windows(self.pid()).is_empty(),
                    "residual application windows"
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
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Host(HWND);
impl Host {
    fn new() -> Self {
        // SAFETY: STATIC is predefined; this process owns the cross-process host.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                windows_sys::w!("STATIC"),
                windows_sys::w!("Phase 3 registry-refresh preview host"),
                WS_POPUP | WS_CLIPCHILDREN,
                120,
                120,
                320,
                180,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            )
        };
        assert!(!hwnd.is_null());
        unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        Self(hwnd)
    }

    fn close(mut self) {
        assert_eq!(unsafe { DestroyWindow(self.0) }, 1);
        self.0 = ptr::null_mut();
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { DestroyWindow(self.0) };
        }
    }
}

fn pump() {
    let mut message = MSG::default();
    // SAFETY: Pump only this test thread's messages.
    unsafe {
        while PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn windows(process: u32) -> Vec<HWND> {
    let mut found = (process, Vec::new());
    unsafe { EnumWindows(Some(collect), &mut found as *mut _ as LPARAM) };
    found.1
}

fn child_windows(process: u32, parent: HWND) -> Vec<HWND> {
    let mut found = (process, Vec::new());
    unsafe { EnumChildWindows(parent, Some(collect), &mut found as *mut _ as LPARAM) };
    found.1
}

unsafe extern "system" fn collect(hwnd: HWND, parameter: LPARAM) -> i32 {
    let found = unsafe { &mut *(parameter as *mut (u32, Vec<HWND>)) };
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut process) };
    if process == found.0 {
        let mut class = [0u16; 64];
        let count = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) };
        let class = String::from_utf16_lossy(&class[..count.max(0) as usize]);
        if class == "#32770" || class == "MyDateTimeScreensaver.Window" {
            found.1.push(hwnd);
        }
    }
    1
}

fn find_dialog(process: u32, control: i32) -> Option<HWND> {
    windows(process)
        .into_iter()
        .find(|&hwnd| !unsafe { GetDlgItem(hwnd, control) }.is_null())
}

fn send(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> usize {
    let mut result = 0;
    // SAFETY: Messages carry only integers and target UI owned by this test's child.
    assert_ne!(
        unsafe {
            SendMessageTimeoutW(
                hwnd,
                message,
                wparam,
                lparam,
                SMTO_ABORTIFHUNG,
                3000,
                &mut result,
            )
        },
        0
    );
    result
}

fn click(dialog: HWND, control: i32) {
    let hwnd = unsafe { GetDlgItem(dialog, control) };
    assert!(!hwnd.is_null());
    send(hwnd, BM_CLICK, 0, 0);
}

fn select_combo(dialog: HWND, index: usize) {
    let combo = unsafe { GetDlgItem(dialog, IDC_FONT_COMBO) };
    assert!(!combo.is_null());
    assert_ne!(
        send(combo, CB_SETCURSEL, index, 0) as isize,
        CB_ERR as isize
    );
    send(
        dialog,
        WM_COMMAND,
        IDC_FONT_COMBO as usize | ((CBN_SELCHANGE as usize) << 16),
        combo as isize,
    );
}

fn combo_index(dialog: HWND) -> usize {
    let combo = unsafe { GetDlgItem(dialog, IDC_FONT_COMBO) };
    assert!(!combo.is_null());
    let index = send(combo, CB_GETCURSEL, 0, 0);
    assert_ne!(index as isize, CB_ERR as isize);
    index
}

fn set_countdown(dialog: HWND, hours: u8, minutes: u8, seconds: u8) {
    let packed = usize::from(hours) | (usize::from(minutes) << 8) | (usize::from(seconds) << 16);
    send(dialog, TEST_SET_COUNTDOWN_FIELDS, packed, 0);
}

fn countdown_fields(dialog: HWND) -> [u8; 3] {
    let packed = send(dialog, TEST_GET_COUNTDOWN_FIELDS, 0, 0);
    assert_ne!(packed, usize::MAX);
    [
        (packed & 0xff) as u8,
        ((packed >> 8) & 0xff) as u8,
        ((packed >> 16) & 0xff) as u8,
    ]
}

fn config_snapshot(hwnd: HWND) -> [u8; 3] {
    let packed = send(hwnd, TEST_GET_CONFIG_SNAPSHOT, 0, 0);
    [
        (packed & 0xff) as u8,
        ((packed >> 8) & 0xff) as u8,
        ((packed >> 16) & 0xff) as u8,
    ]
}

fn config_draft(hwnd: HWND) -> [u8; 3] {
    let packed = send(hwnd, TEST_GET_CONFIG_DRAFT, 0, 0);
    [
        (packed & 0xff) as u8,
        ((packed >> 8) & 0xff) as u8,
        ((packed >> 16) & 0xff) as u8,
    ]
}

fn get_window_text(hwnd: HWND) -> String {
    let mut text = [0u16; 256];
    let count = unsafe { GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32) };
    String::from_utf16_lossy(&text[..count.max(0) as usize])
}

fn assert_controls_fit(dialog: HWND, controls: &[i32]) {
    let mut client = RECT::default();
    assert_ne!(unsafe { GetClientRect(dialog, &mut client) }, 0);
    for &id in controls {
        let hwnd = unsafe { GetDlgItem(dialog, id) };
        assert!(!hwnd.is_null(), "missing control {id}");
        let mut rect = RECT::default();
        assert_ne!(unsafe { GetWindowRect(hwnd, &mut rect) }, 0);
        let mut points = [
            windows_sys::Win32::Foundation::POINT {
                x: rect.left,
                y: rect.top,
            },
            windows_sys::Win32::Foundation::POINT {
                x: rect.right,
                y: rect.bottom,
            },
        ];
        assert_ne!(
            unsafe { MapWindowPoints(ptr::null_mut(), dialog, points.as_mut_ptr(), 2) },
            0
        );
        assert!(points[0].x >= 0 && points[0].y >= 0);
        assert!(points[1].x <= client.right && points[1].y <= client.bottom);
    }
}

#[test]
#[ignore = "opens native config/font/countdown UI and fullscreen surfaces; run serially"]
fn phase3_configuration_preview_and_countdown_flow() {
    let _sandbox = Sandbox::new();
    let store = RegistryStore::new();
    assert!(store.get("SchemaVersion").unwrap().is_none());

    // Cancellation and ChooseFont cancellation do not create/write the key.
    let mut config = Saver::start(&["/c"]);
    let pid = config.pid();
    config.until(5, || find_dialog(pid, IDC_PREVIEW).is_some());
    let dialog = find_dialog(pid, IDC_PREVIEW).unwrap();
    assert_controls_fit(
        dialog,
        &[
            IDC_MODE_TIME_DATE,
            IDC_MODE_COUNTDOWN,
            IDC_COLOR_DARK_RED,
            IDC_COLOR_DARK_ORANGE,
            IDC_COLOR_BRIGHT_GREEN,
            IDC_COLOR_OFF_WHITE,
            IDC_FONT_COMBO,
            IDC_CHOOSE_FONT,
            IDC_PREVIEW,
            IDOK,
            IDCANCEL,
        ],
    );
    assert_eq!(config_draft(dialog), [0, 2, 0]);
    for (control, preset) in [
        (IDC_COLOR_DARK_RED, 0),
        (IDC_COLOR_DARK_ORANGE, 1),
        (IDC_COLOR_BRIGHT_GREEN, 2),
        (IDC_COLOR_OFF_WHITE, 3),
    ] {
        click(dialog, control);
        assert_eq!(config_draft(dialog), [0, preset, 0]);
    }
    click(dialog, IDC_MODE_COUNTDOWN);
    click(dialog, IDC_COLOR_DARK_ORANGE);
    select_combo(dialog, 2);
    let choose_button = unsafe { GetDlgItem(dialog, IDC_CHOOSE_FONT) };
    assert_ne!(
        unsafe {
            PostMessageW(
                dialog,
                WM_COMMAND,
                IDC_CHOOSE_FONT as usize,
                choose_button as isize,
            )
        },
        0
    );
    config.until(5, || windows(pid).len() >= 2);
    let font_dialog = windows(pid)
        .into_iter()
        .find(|&hwnd| hwnd != dialog)
        .unwrap();
    send(font_dialog, WM_COMMAND, IDCANCEL as usize, 0);
    config.until(5, || windows(pid).len() == 1);
    send(dialog, WM_COMMAND, IDCANCEL as usize, 0);
    config.exits(0);
    assert!(store.get("SchemaVersion").unwrap().is_none());

    // Accepting the system font picker selects and persists a validated custom font.
    let mut config = Saver::start(&["/c"]);
    let pid = config.pid();
    config.until(5, || find_dialog(pid, IDC_PREVIEW).is_some());
    let dialog = find_dialog(pid, IDC_PREVIEW).unwrap();
    let choose_button = unsafe { GetDlgItem(dialog, IDC_CHOOSE_FONT) };
    assert_ne!(
        unsafe {
            PostMessageW(
                dialog,
                WM_COMMAND,
                IDC_CHOOSE_FONT as usize,
                choose_button as isize,
            )
        },
        0
    );
    config.until(5, || windows(pid).len() >= 2);
    let font_dialog = windows(pid)
        .into_iter()
        .find(|&hwnd| hwnd != dialog)
        .unwrap();
    send(font_dialog, WM_COMMAND, IDOK as usize, 0);
    config.until(5, || windows(pid).len() == 1);
    assert_eq!(combo_index(dialog), 3);
    send(dialog, WM_COMMAND, IDOK as usize, 0);
    config.exits(0);
    let custom = load_registry();
    assert_eq!(custom.font_mode, FontMode::Custom);
    assert!(custom.custom_font.is_some());
    assert_eq!(store.get("CustomLogFont").unwrap().unwrap().bytes.len(), 92);

    // A running /p polls the saved settings and converges within two seconds.
    let host = Host::new();
    let mut preview = Saver::start(&["/p", &format!("{}", host.0 as usize)]);
    let preview_pid = preview.pid();
    preview.until(5, || child_windows(preview_pid, host.0).len() == 1);
    let preview_hwnd = child_windows(preview_pid, host.0)[0];
    thread::sleep(Duration::from_millis(200));
    assert_eq!(config_snapshot(preview_hwnd), [0, 2, 3]);

    let mut config = Saver::start(&["/c"]);
    let config_pid = config.pid();
    config.until(5, || find_dialog(config_pid, IDC_PREVIEW).is_some());
    let dialog = find_dialog(config_pid, IDC_PREVIEW).unwrap();
    click(dialog, IDC_MODE_COUNTDOWN);
    click(dialog, IDC_COLOR_DARK_ORANGE);
    select_combo(dialog, 1);
    send(dialog, WM_COMMAND, IDOK as usize, 0);
    config.exits(0);
    let saved = load_registry();
    assert_eq!(saved.display_mode, DisplayMode::Countdown);
    assert_eq!(saved.color_preset, ColorPreset::DarkOrange);
    assert_eq!(saved.font_mode, FontMode::Consolas);
    assert_eq!(saved.last_countdown_seconds, 300);
    let refresh_started = Instant::now();
    preview.until(3, || config_snapshot(preview_hwnd) == [1, 1, 1]);
    assert!(refresh_started.elapsed() <= Duration::from_secs(2));
    host.close();
    preview.exits(0);

    let host = Host::new();
    let mut preview = Saver::start(&["/p", &format!("{}", host.0 as usize)]);
    let preview_pid = preview.pid();
    preview.until(5, || child_windows(preview_pid, host.0).len() == 1);
    let preview_hwnd = child_windows(preview_pid, host.0)[0];
    thread::sleep(Duration::from_millis(200));
    assert_eq!(config_snapshot(preview_hwnd), [1, 1, 1]);
    host.close();
    preview.exits(0);

    // Invalid input remains in the dialog and does not update the last duration.
    let mut fullscreen = Saver::start(&["/s"]);
    let fullscreen_pid = fullscreen.pid();
    fullscreen.until(5, || {
        find_dialog(fullscreen_pid, IDC_COUNTDOWN_HOURS).is_some()
    });
    let dialog = find_dialog(fullscreen_pid, IDC_COUNTDOWN_HOURS).unwrap();
    thread::sleep(Duration::from_millis(100));
    assert_eq!(countdown_fields(dialog), [0, 5, 0]);
    set_countdown(dialog, 0, 60, 0);
    send(dialog, WM_COMMAND, IDOK as usize, 0);
    thread::sleep(Duration::from_millis(100));
    assert!(find_dialog(fullscreen_pid, IDC_COUNTDOWN_ERROR).is_some());
    assert_eq!(load_registry().last_countdown_seconds, 300);
    set_countdown(dialog, 0, 0, 1);
    send(dialog, WM_COMMAND, IDOK as usize, 0);
    fullscreen.until(5, || {
        windows(fullscreen_pid)
            .iter()
            .filter(|&&hwnd| unsafe { GetDlgItem(hwnd, IDC_COUNTDOWN_HOURS) }.is_null())
            .count()
            == 2
    });
    let surfaces = windows(fullscreen_pid);
    assert_eq!(surfaces.len(), 2);
    send(surfaces[0], WM_CLOSE, 0, 0);
    fullscreen.exits(0);
    assert_eq!(load_registry().last_countdown_seconds, 1);

    // Valid 5 s, 30 min, and maximum inputs start once and preserve other fields.
    for (h, m, s, expected) in [(0, 0, 5, 5), (0, 30, 0, 1800), (99, 59, 59, 359999)] {
        let mut saver = Saver::start(&["/s"]);
        let pid = saver.pid();
        saver.until(5, || find_dialog(pid, IDC_COUNTDOWN_HOURS).is_some());
        thread::sleep(Duration::from_millis(100));
        let dialog = find_dialog(pid, IDC_COUNTDOWN_HOURS).unwrap();
        assert_eq!(
            countdown_fields(dialog),
            crate_hms(load_registry().last_countdown_seconds)
        );
        set_countdown(dialog, h, m, s);
        send(dialog, WM_COMMAND, IDOK as usize, 0);
        saver.until(5, || windows(pid).len() == 2);
        let surfaces = windows(pid);
        send(surfaces[0], WM_CLOSE, 0, 0);
        saver.exits(0);
        let config = load_registry();
        assert_eq!(config.last_countdown_seconds, expected);
        assert_eq!(config.display_mode, DisplayMode::Countdown);
        assert_eq!(config.color_preset, ColorPreset::DarkOrange);
        assert_eq!(config.font_mode, FontMode::Consolas);
    }

    // Cancel ends before any fullscreen surface and preserves the last value.
    let mut cancelled = Saver::start(&["/s"]);
    let pid = cancelled.pid();
    cancelled.until(5, || find_dialog(pid, IDC_COUNTDOWN_HOURS).is_some());
    let dialog = find_dialog(pid, IDC_COUNTDOWN_HOURS).unwrap();
    send(dialog, WM_COMMAND, IDCANCEL as usize, 0);
    cancelled.exits(0);
    assert_eq!(load_registry().last_countdown_seconds, 359999);
    println!(
        "Phase 3: cancel/no-write, ChooseFont cancel/accept, /p <=2 s refresh, countdown prefill, invalid input, 1/5/1800/359999 s and field isolation passed"
    );
}

fn crate_hms(total: u32) -> [u8; 3] {
    [
        (total / 3600) as u8,
        ((total % 3600) / 60) as u8,
        (total % 60) as u8,
    ]
}
