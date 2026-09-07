//! Opt-in, ten-minute observation of two real Debug GUI processes.
#![cfg(debug_assertions)]
use std::{
    env,
    fs::{self, File},
    io::Write,
    os::windows::io::AsRawHandle,
    path::PathBuf,
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::Foundation::{FILETIME, HANDLE, HWND, LPARAM};
use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
use windows_sys::Win32::System::Threading::{
    GetGuiResources, GetProcessTimes, GR_GDIOBJECTS, GR_USEROBJECTS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn windows(process: u32) -> Vec<HWND> {
    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> i32 {
        let mut id = 0;
        let mut name = [0u16; 128];
        // SAFETY: Synchronous EnumWindows callback with live stack context.
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut id);
            let length = GetClassNameW(hwnd, name.as_mut_ptr(), 128);
            if id == (*(data as *const (u32, Vec<HWND>))).0
                && String::from_utf16_lossy(&name[..length.max(0) as usize])
                    == "tools-screensaver-tzk.Window"
            {
                (*(data as *mut (u32, Vec<HWND>))).1.push(hwnd);
            }
        }
        1
    }
    let mut result = (process, Vec::new());
    // SAFETY: EnumWindows only queries handles and completes before result drops.
    unsafe {
        EnumWindows(Some(collect), &mut result as *mut _ as LPARAM);
    }
    result.1
}

#[derive(Clone, Copy)]
struct Sample {
    cpu: f64,
    working: usize,
    private: usize,
    gdi: u32,
    user: u32,
}
fn sample(process: HANDLE) -> Sample {
    let mut memory = PROCESS_MEMORY_COUNTERS_EX {
        cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..Default::default()
    };
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: Only read counters from the live Child handle owned by this test.
    unsafe {
        assert_ne!(
            GetProcessMemoryInfo(process, &mut memory as *mut _ as *mut _, memory.cb),
            0
        );
        assert_ne!(
            GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user),
            0
        );
        let ticks =
            |time: FILETIME| (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
        Sample {
            cpu: (ticks(kernel) + ticks(user)) as f64 / 10_000_000.0,
            working: memory.WorkingSetSize,
            private: memory.PrivateUsage,
            gdi: GetGuiResources(process, GR_GDIOBJECTS),
            user: GetGuiResources(process, GR_USEROBJECTS),
        }
    }
}

#[test]
#[ignore = "two visible Debug windows, real 600-second observation; set PHASE2_OBSERVATION"]
fn observe_two_debug_modes_for_ten_minutes() {
    let output = PathBuf::from(env::var_os("PHASE2_OBSERVATION").expect("set PHASE2_OBSERVATION"));
    fs::create_dir_all(&output).unwrap();
    let executable = env!("CARGO_BIN_EXE_tools-screensaver-tzk");
    let cores = thread::available_parallelism().unwrap().get();
    let mut children = Vec::new();
    for mode in ["time-date", "countdown"] {
        let mut child = OwnedChild(
            Command::new(executable)
                .arg(format!("--dev-render={mode}"))
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "{mode} startup failed"
            );
            let handles = windows(child.0.id());
            // SAFETY: Read visibility of our own child process's windows.
            if handles
                .iter()
                .any(|&hwnd| unsafe { IsWindowVisible(hwnd) } != 0)
            {
                break;
            }
            assert!(Instant::now() < deadline, "{mode} has no visible window");
            thread::sleep(Duration::from_millis(20));
        }
        println!("{mode}: PID {}, executable {executable}", child.0.id());
        children.push((mode, child));
    }
    let start = Instant::now();
    let mut file = File::create(output.join("resource-observation.csv")).unwrap();
    writeln!(file,"elapsed_seconds,mode,pid,cpu_seconds,whole_machine_cpu_percent,working_set_bytes,private_bytes,gdi_objects,user_objects,logical_processors").unwrap();
    let mut previous: Vec<_> = children
        .iter()
        .map(|(_, child)| sample(child.0.as_raw_handle()))
        .collect();
    let mut previous_time = 0.0;
    let mut baseline = None;
    let mut final_samples = Vec::new();
    for minute in 0..=10 {
        let target = Duration::from_secs(minute * 60);
        while start.elapsed() < target {
            for (mode, child) in &mut children {
                assert!(child.0.try_wait().unwrap().is_none(), "{mode} exited early");
            }
            thread::sleep(Duration::from_millis(100));
        }
        let elapsed = start.elapsed().as_secs_f64();
        let mut current = Vec::new();
        for (index, (mode, child)) in children.iter().enumerate() {
            let now = sample(child.0.as_raw_handle());
            let cpu = if minute == 0 {
                0.0
            } else {
                100.0 * (now.cpu - previous[index].cpu) / ((elapsed - previous_time) * cores as f64)
            };
            writeln!(
                file,
                "{elapsed:.3},{mode},{},{:.6},{cpu:.4},{},{},{},{},{cores}",
                child.0.id(),
                now.cpu,
                now.working,
                now.private,
                now.gdi,
                now.user
            )
            .unwrap();
            println!(
                "{elapsed:.1}s {mode}: GDI {}, USER {}, private {}, CPU {cpu:.3}%",
                now.gdi, now.user, now.private
            );
            current.push(now);
        }
        file.flush().unwrap();
        if minute == 1 {
            baseline = Some(current.clone());
        }
        previous = current.clone();
        previous_time = elapsed;
        final_samples = current;
    }
    for (before, after) in baseline.unwrap().iter().zip(&final_samples) {
        assert!(
            after.gdi <= before.gdi + 10 && after.user <= before.user + 10,
            "GUI resource growth"
        );
        assert!(
            after.private <= before.private + (4 * 1024 * 1024).max(before.private / 10),
            "private memory growth"
        );
    }
    for (mode, child) in &mut children {
        for hwnd in windows(child.0.id()) {
            // SAFETY: Close only the exact developer windows created by this test.
            unsafe {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                assert_eq!(status.code(), Some(0));
                break;
            }
            assert!(Instant::now() < deadline, "{mode} cleanup timed out");
            thread::sleep(Duration::from_millis(20));
        }
        assert!(windows(child.0.id()).is_empty());
    }
    assert!(start.elapsed() >= Duration::from_secs(600));
}
