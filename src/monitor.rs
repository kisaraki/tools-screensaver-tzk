use std::{mem::size_of, ptr};

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, MONITORINFOF_PRIMARY, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::error::{last_error, AppError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Bounds {
    pub fn from_rect(rect: RECT) -> Option<Self> {
        let width = rect.right.checked_sub(rect.left)?;
        let height = rect.bottom.checked_sub(rect.top)?;
        (width > 0 && height > 0).then_some(Self {
            x: rect.left,
            y: rect.top,
            width,
            height,
        })
    }
}

pub(crate) fn enumerate() -> Result<Vec<Bounds>, AppError> {
    let mut monitors: Vec<(bool, Bounds)> = Vec::new();
    // SAFETY: The callback is synchronous and receives an exclusive pointer to
    // this stack Vec. No references to it are held across the API call.
    let success = unsafe {
        EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null(),
            Some(collect),
            &mut monitors as *mut _ as LPARAM,
        )
    };
    if success != 0 && !monitors.is_empty() {
        monitors.sort_by_key(|(primary, bounds)| (!*primary, bounds.x, bounds.y));
        let mut unique = Vec::new();
        for (_, bounds) in monitors {
            if !unique.contains(&bounds) {
                unique.push(bounds);
            }
        }
        return Ok(unique);
    }
    // SAFETY: These metrics require no pointers and are queried on the PMv2 UI thread.
    let bounds = unsafe {
        Bounds {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
            height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
        }
    };
    if bounds.width <= 0
        || bounds.height <= 0
        || bounds.x.checked_add(bounds.width).is_none()
        || bounds.y.checked_add(bounds.height).is_none()
    {
        return Err(AppError::OperationFailed("virtual screen bounds"));
    }
    Ok(vec![bounds])
}

unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
    if data == 0 {
        return 0;
    }
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: monitor is supplied by EnumDisplayMonitors and info is correctly sized.
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return 0;
    }
    if let Some(bounds) = Bounds::from_rect(info.rcMonitor) {
        // SAFETY: enumerate supplied this live, exclusively accessed Vec for the callback.
        unsafe { &mut *(data as *mut Vec<(bool, Bounds)>) }
            .push((info.dwFlags & MONITORINFOF_PRIMARY != 0, bounds));
    }
    1
}

pub(crate) fn info_for_window(hwnd: HWND) -> Result<MONITORINFO, AppError> {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: hwnd is an opaque OS handle; info is a writable, correctly sized struct.
    let success =
        unsafe { GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info) };
    if success == 0 {
        Err(last_error("GetMonitorInfoW"))
    } else {
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_preserve_negative_coordinates_and_reject_overflow() {
        assert_eq!(
            Bounds::from_rect(RECT {
                left: -1920,
                top: -1080,
                right: 0,
                bottom: 0
            }),
            Some(Bounds {
                x: -1920,
                y: -1080,
                width: 1920,
                height: 1080
            })
        );
        assert!(Bounds::from_rect(RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 1
        })
        .is_none());
        assert!(Bounds::from_rect(RECT {
            left: i32::MIN,
            top: 0,
            right: i32::MAX,
            bottom: 1
        })
        .is_none());
    }
}
