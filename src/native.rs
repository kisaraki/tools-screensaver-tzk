use windows_sys::Win32::Foundation::{GetLastError, SetLastError, HWND, RECT};
use windows_sys::Win32::UI::HiDpi::{
    GetWindowDpiAwarenessContext, SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowThreadProcessId, IsWindow, SetWindowLongPtrW,
};

use crate::error::{last_error, AppError};

#[derive(Clone, Copy)]
pub(crate) struct WindowIdentity {
    pub hwnd: HWND,
    process: u32,
    thread: u32,
}

impl WindowIdentity {
    pub fn capture(hwnd: HWND) -> Option<Self> {
        if hwnd.is_null() {
            return None;
        }
        let mut process = 0;
        // SAFETY: Opaque handles are checked by Win32; process is writable.
        let thread = unsafe { GetWindowThreadProcessId(hwnd, &mut process) };
        // SAFETY: IsWindow accepts any opaque handle, including an expired one.
        (thread != 0 && process != 0 && unsafe { IsWindow(hwnd) } != 0).then_some(Self {
            hwnd,
            process,
            thread,
        })
    }

    pub fn alive(self) -> bool {
        Self::capture(self.hwnd)
            .is_some_and(|current| current.process == self.process && current.thread == self.thread)
    }
}

pub(crate) struct DpiScope(DPI_AWARENESS_CONTEXT);

impl DpiScope {
    pub fn for_window(hwnd: HWND) -> Result<Self, AppError> {
        // SAFETY: This API checks hwnd; an invalid context is rejected below.
        let context = unsafe { GetWindowDpiAwarenessContext(hwnd) };
        if context.is_null() {
            return Err(last_error("GetWindowDpiAwarenessContext"));
        }
        // SAFETY: Use the valid context returned by the OS on this same thread.
        let old = unsafe { SetThreadDpiAwarenessContext(context) };
        if old.is_null() {
            Err(last_error("SetThreadDpiAwarenessContext"))
        } else {
            Ok(Self(old))
        }
    }
}

impl Drop for DpiScope {
    fn drop(&mut self) {
        // SAFETY: old is the valid context captured on this UI thread; the raw
        // pointer field makes this guard !Send, so it cannot migrate threads.
        unsafe { SetThreadDpiAwarenessContext(self.0) };
    }
}

pub(crate) fn client_size(hwnd: HWND) -> Result<(i32, i32), AppError> {
    let mut rect = RECT::default();
    // SAFETY: rect is a valid output; Win32 validates the opaque HWND.
    if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
        return Err(last_error("GetClientRect"));
    }
    match (
        rect.right.checked_sub(rect.left),
        rect.bottom.checked_sub(rect.top),
    ) {
        (Some(width), Some(height)) if width >= 0 && height >= 0 => Ok((width, height)),
        _ => Err(AppError::OperationFailed("client rectangle dimensions")),
    }
}

pub(crate) fn set_pointer(hwnd: HWND, index: i32, value: isize) -> Result<(), AppError> {
    // SAFETY: Callers use a live owned window and a valid userdata index.
    // Zero is also a successful previous value; clear/capture the error explicitly.
    let (previous, code) = unsafe {
        SetLastError(0);
        let previous = SetWindowLongPtrW(hwnd, index, value);
        (previous, GetLastError())
    };
    if previous == 0 && code != 0 {
        Err(AppError::Win32 {
            operation: "SetWindowLongPtrW",
            code,
        })
    } else {
        Ok(())
    }
}
