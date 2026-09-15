//! Opt-in daily update check, user-confirmed download, TLS and SHA-256 validation.
use crate::{
    config::{RawValue, SettingsStore},
    net,
    registry::RegistryStore,
    resource_ids,
};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    fs,
    io::Write,
    path::PathBuf,
    ptr,
    sync::mpsc::{self, Receiver},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HINSTANCE, HWND, LPARAM, WAIT_ABANDONED, WAIT_OBJECT_0, WPARAM},
    System::{
        StationsAndDesktops::*,
        SystemInformation::GetLocalTime,
        Threading::{CreateMutexW, GetCurrentThreadId, ReleaseMutex, WaitForSingleObject},
    },
    UI::{Shell::ShellExecuteW, WindowsAndMessaging::*},
};

#[derive(Debug, Clone)]
pub(crate) struct Release {
    tag: String,
}
fn version(text: &str) -> Option<[u32; 3]> {
    let mut parts = text.strip_prefix('v').unwrap_or(text).split('.');
    let mut out = [0; 3];
    for n in &mut out {
        let part = parts.next()?;
        if part.is_empty() || part.len() > 9 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        *n = part.parse().ok()?;
    }
    if parts.next().is_some() {
        None
    } else {
        Some(out)
    }
}
fn parse_release(value: &serde_json::Value, current: &str) -> Result<Option<Release>, String> {
    if value["draft"].as_bool() != Some(false) || value["prerelease"].as_bool() != Some(false) {
        return Err("發布資料不是正式版本".into());
    }
    let tag = value["tag_name"]
        .as_str()
        .filter(|s| s.starts_with('v'))
        .ok_or("版本標籤無效")?;
    let remote = version(tag).ok_or("版本格式無效")?;
    Ok((remote > version(current).ok_or("目前版本格式無效")?).then(|| Release { tag: tag.into() }))
}
fn check() -> Result<Option<Release>, String> {
    parse_release(
        &net::json(
            "api.github.com",
            "/repos/kisaraki/tools-screensaver-tzk/releases/latest",
        )?,
        env!("CARGO_PKG_VERSION"),
    )
}
pub(crate) fn interactive_desktop() -> bool {
    fn name(desk: HDESK) -> Option<String> {
        let mut buffer = [0u16; 128];
        let mut needed = 0u32;
        if unsafe {
            GetUserObjectInformationW(
                desk,
                UOI_NAME,
                buffer.as_mut_ptr().cast(),
                size_of_val(&buffer) as u32,
                &mut needed,
            )
        } == 0
        {
            return None;
        }
        Some(String::from_utf16_lossy(
            &buffer[..buffer.iter().position(|c| *c == 0)?],
        ))
    }
    // Do not display update prompts or launch installers on a locked/secure desktop.
    unsafe {
        let input = OpenInputDesktop(0, 0, DESKTOP_READOBJECTS);
        if input.is_null() {
            return false;
        }
        let result = name(input).as_deref() == Some("Default")
            && name(GetThreadDesktop(GetCurrentThreadId())).as_deref() == Some("Default");
        CloseDesktop(input);
        result
    }
}
fn daily_claim(store: &mut impl SettingsStore, day: u32) -> bool {
    let Ok(previous) = store.get("LastUpdateCheckDay") else {
        return false;
    };
    if previous
        .is_some_and(|v| v.kind == crate::config::REG_DWORD_KIND && v.bytes == day.to_le_bytes())
    {
        return false;
    }
    store
        .set("LastUpdateCheckDay", &RawValue::dword(day))
        .is_ok()
}
pub(crate) fn start_auto(
    config: crate::config::AppConfig,
) -> Option<Receiver<Result<Option<Release>, String>>> {
    if !config.auto_update || config.future_schema || !interactive_desktop() {
        return None;
    }
    // Mutex serializes date claims across simultaneous /s processes; no blocking wait.
    unsafe {
        let mutex = CreateMutexW(
            ptr::null(),
            0,
            windows_sys::w!("Local\\tools-screensaver-tzk.UpdateDay"),
        );
        if mutex.is_null() {
            return None;
        }
        let acquired = matches!(
            WaitForSingleObject(mutex, 0),
            WAIT_OBJECT_0 | WAIT_ABANDONED
        );
        let mut time = Default::default();
        GetLocalTime(&mut time);
        let day =
            u32::from(time.wYear) * 10000 + u32::from(time.wMonth) * 100 + u32::from(time.wDay);
        let claimed = acquired && daily_claim(&mut RegistryStore::new(), day);
        if acquired {
            ReleaseMutex(mutex);
        }
        CloseHandle(mutex);
        if !claimed {
            return None;
        }
    }
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(check());
    });
    Some(rx)
}
fn manifest_hash(bytes: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "更新雜湊格式無效")?;
    let matches: Vec<_> = text
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            (fields.len() == 2
                && fields[1].trim_start_matches('*') == "tools-screensaver-tzk-Setup.exe")
                .then(|| fields[0])
        })
        .collect();
    if matches.len() != 1
        || matches[0].len() != 64
        || !matches[0].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("找不到唯一有效的 Setup SHA-256".into());
    }
    Ok(matches[0].to_ascii_lowercase())
}
fn download(release: &Release) -> Result<PathBuf, String> {
    if version(&release.tag).is_none() {
        return Err("版本標籤無效".into());
    }
    let base = format!("/tools-screensaver-tzk/downloads/{}", release.tag);
    let expected = manifest_hash(&net::get(
        "kisaraki.github.io",
        &format!("{base}/SHA256SUMS.txt"),
        16384,
    )?)?;
    let bytes = net::get(
        "kisaraki.github.io",
        &format!("{base}/tools-screensaver-tzk-Setup.exe"),
        128 * 1024 * 1024,
    )?;
    if format!("{:x}", Sha256::digest(&bytes)) != expected || !bytes.starts_with(b"MZ") {
        return Err("下載檔案 SHA-256 或 PE 標頭驗證失敗；未啟動安裝".into());
    }
    let root = std::env::var_os("LOCALAPPDATA").ok_or("找不到 LOCALAPPDATA")?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "系統時間無效")?
        .as_nanos();
    let directory = PathBuf::from(root)
        .join("KOMSMOS/tools-screensaver-tzk/Updates")
        .join(format!("{}-{}-{nonce}", release.tag, std::process::id()));
    fs::create_dir_all(&directory).map_err(|_| "無法建立更新目錄")?;
    let path = directory.join("tools-screensaver-tzk-Setup.exe");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "無法建立更新檔案")?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "無法完整保存更新檔案")?;
    Ok(path)
}
enum Completion {
    Checked(Result<Option<Release>, String>),
    Downloaded(Result<PathBuf, String>),
}
struct State {
    receiver: RefCell<Receiver<Completion>>,
    launched: Cell<bool>,
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}
fn message(hwnd: HWND, text: &str, flags: u32) -> i32 {
    unsafe {
        MessageBoxW(
            hwnd,
            wide(text).as_ptr(),
            windows_sys::w!("tools-screensaver-tzk 更新"),
            flags,
        )
    }
}
fn confirm(hwnd: HWND, release: &Release) -> bool {
    interactive_desktop() && message(hwnd,&format!("目前版本 {}，已發現 {}。是否下載並啟動新版安裝程式？\n安裝將詢問移除既有版本，並可能需要 UAC。",env!("CARGO_PKG_VERSION"),release.tag),MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2)==IDYES
}
fn download_receiver(release: Release) -> Receiver<Completion> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(Completion::Downloaded(download(&release)));
    });
    rx
}
fn dialog(instance: HINSTANCE, owner: HWND, receiver: Receiver<Completion>) -> bool {
    let state = State {
        receiver: RefCell::new(receiver),
        launched: Cell::new(false),
    };
    let result = unsafe {
        DialogBoxParamW(
            instance,
            resource_ids::IDD_UPDATE as usize as *const u16,
            owner,
            Some(proc),
            std::ptr::from_ref(&state) as isize,
        )
    };
    if result == -1 {
        message(owner, "無法開啟更新視窗。", MB_OK | MB_ICONERROR);
    }
    state.launched.get()
}
pub(crate) fn manual(instance: HINSTANCE, owner: HWND) -> bool {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(Completion::Checked(check()));
    });
    dialog(instance, owner, rx)
}
pub(crate) fn offer(instance: HINSTANCE, release: Release) {
    if confirm(ptr::null_mut(), &release) {
        dialog(instance, ptr::null_mut(), download_receiver(release));
    }
}
pub(crate) fn finish_auto(
    instance: HINSTANCE,
    receiver: Receiver<Result<Option<Release>, String>>,
) {
    // The saver windows are already gone. Keep only this background check alive
    // so a short first /s run does not consume today's claim without its result.
    if let Ok(Ok(Some(release))) = receiver.recv_timeout(std::time::Duration::from_secs(50)) {
        offer(instance, release);
    }
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    const USER: i32 = (2 * size_of::<isize>()) as i32;
    if msg == WM_INITDIALOG {
        if crate::native::set_pointer(hwnd, USER, l).is_err() {
            unsafe {
                EndDialog(hwnd, 0);
            };
            return 1;
        }
        if unsafe { SetTimer(hwnd, 1, 100, None) } == 0 {
            unsafe {
                EndDialog(hwnd, 0);
            };
        }
        return 1;
    }
    let state = unsafe { (GetWindowLongPtrW(hwnd, USER) as *const State).as_ref() };
    let Some(state) = state else {
        return 0;
    };
    match msg {
        WM_TIMER => {
            let result = state.receiver.borrow().try_recv();
            if !matches!(result, Err(mpsc::TryRecvError::Empty)) {
                unsafe {
                    KillTimer(hwnd, 1);
                }
            }
            match result {
                Ok(Completion::Checked(Ok(Some(release)))) => {
                    if confirm(hwnd, &release) {
                        *state.receiver.borrow_mut() = download_receiver(release);
                        unsafe {
                            if SetTimer(hwnd, 1, 100, None) == 0 {
                                EndDialog(hwnd, 0);
                                return 1;
                            }
                            SetDlgItemTextW(
                                hwnd,
                                i32::from(resource_ids::IDC_UPDATE_STATUS),
                                windows_sys::w!("正在下載並驗證新版安裝程式…"),
                            );
                        }
                    } else {
                        unsafe {
                            EndDialog(hwnd, 0);
                        }
                    }
                }
                Ok(Completion::Checked(Ok(None))) => {
                    message(hwnd, "目前已是最新正式版本。", MB_OK | MB_ICONINFORMATION);
                    unsafe {
                        EndDialog(hwnd, 0);
                    }
                }
                Ok(Completion::Checked(Err(e))) | Ok(Completion::Downloaded(Err(e))) => {
                    message(
                        hwnd,
                        &format!("更新未完成：{e}\n目前版本可繼續使用。"),
                        MB_OK | MB_ICONERROR,
                    );
                    unsafe {
                        EndDialog(hwnd, 0);
                    }
                }
                Ok(Completion::Downloaded(Ok(path))) => {
                    if interactive_desktop() {
                        let path: Vec<u16> = path
                            .as_os_str()
                            .to_string_lossy()
                            .encode_utf16()
                            .chain([0])
                            .collect();
                        let result = unsafe {
                            ShellExecuteW(
                                hwnd,
                                windows_sys::w!("open"),
                                path.as_ptr(),
                                ptr::null(),
                                ptr::null(),
                                SW_SHOWNORMAL,
                            )
                        };
                        if result as isize <= 32 {
                            message(
                                hwnd,
                                "安裝程式未啟動，可能已取消 UAC。",
                                MB_OK | MB_ICONERROR,
                            );
                        } else {
                            state.launched.set(true);
                        }
                    }
                    unsafe {
                        EndDialog(hwnd, 0);
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    message(hwnd, "更新工作已中止。", MB_OK | MB_ICONERROR);
                    unsafe {
                        EndDialog(hwnd, 0);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
            1
        }
        WM_COMMAND if w & 0xffff == IDCANCEL as usize => {
            unsafe {
                EndDialog(hwnd, 0);
            }
            1
        }
        WM_CLOSE => {
            unsafe {
                EndDialog(hwnd, 0);
            }
            1
        }
        WM_DESTROY => {
            unsafe {
                KillTimer(hwnd, 1);
            }
            1
        }
        _ => 0,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn semantic_versions_never_compare_lexically_or_accept_paths() {
        assert!(version("v0.15.0") > version("v0.9.9"));
        for s in ["../1.0.0", "v1.0.0-beta", "v1.0", "v1.0.0/", "v1.0.0.0"] {
            assert!(version(s).is_none());
        }
    }
    #[test]
    fn only_new_stable_releases_are_offered() {
        let v = serde_json::json!({"tag_name":"v0.15.0","draft":false,"prerelease":false});
        assert!(parse_release(&v, "0.14.1").unwrap().is_some());
        assert!(parse_release(&v, "0.15.0").unwrap().is_none());
        assert!(parse_release(&v, "1.0.0").unwrap().is_none());
        let mut pre = v;
        pre["prerelease"] = true.into();
        assert!(parse_release(&pre, "0.14.1").is_err());
    }
    #[test]
    fn manifests_require_one_exact_filename_and_full_digest() {
        let h = "a".repeat(64);
        assert_eq!(
            manifest_hash(format!("{h}  tools-screensaver-tzk-Setup.exe\r\n").as_bytes()).unwrap(),
            h
        );
        assert!(manifest_hash(b"bad tools-screensaver-tzk-Setup.exe").is_err());
        assert!(
            manifest_hash(format!("{h} ../tools-screensaver-tzk-Setup.exe").as_bytes()).is_err()
        );
        assert!(manifest_hash(
            format!("{h} tools-screensaver-tzk-Setup.exe\n{h} tools-screensaver-tzk-Setup.exe")
                .as_bytes()
        )
        .is_err());
    }
    #[test]
    fn date_claim_runs_once_per_local_day_and_failure_never_claims() {
        #[derive(Default)]
        struct Store {
            value: Option<RawValue>,
            writes: u32,
            fail: bool,
        }
        impl SettingsStore for Store {
            fn get(&self, _: &str) -> Result<Option<RawValue>, crate::config::StoreError> {
                Ok(self.value.clone())
            }
            fn set(&mut self, _: &str, value: &RawValue) -> Result<(), crate::config::StoreError> {
                if self.fail {
                    return Err(crate::config::StoreError {
                        operation: "test",
                        code: 5,
                    });
                }
                self.value = Some(value.clone());
                self.writes += 1;
                Ok(())
            }
            fn delete(&mut self, _: &str) -> Result<(), crate::config::StoreError> {
                self.value = None;
                Ok(())
            }
        }
        let mut store = Store::default();
        assert!(daily_claim(&mut store, 20260915));
        assert!(!daily_claim(&mut store, 20260915));
        assert_eq!(store.writes, 1);
        assert!(daily_claim(&mut store, 20260916));
        store.fail = true;
        assert!(!daily_claim(&mut store, 20260917));
        assert_eq!(store.writes, 2);
    }
    #[test]
    #[ignore = "explicit public release/download/hash probe; never executes Setup"]
    fn public_update_network_probe() {
        let value = net::json(
            "api.github.com",
            "/repos/kisaraki/tools-screensaver-tzk/releases/latest",
        )
        .unwrap();
        let release = parse_release(&value, "0.0.0").unwrap().unwrap();
        let path = download(&release).unwrap();
        println!(
            "{} public HTTPS Setup SHA-256 verified (no execution)",
            release.tag
        );
        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }
}
