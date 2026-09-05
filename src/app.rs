use std::ptr;

use windows_sys::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows_sys::Win32::System::LibraryLoader::{FindResourceW, GetModuleHandleW};
use windows_sys::Win32::UI::WindowsAndMessaging::{LoadIconW, LoadStringW, RT_MANIFEST};

use crate::cli::{self, RunMode};
use crate::error::last_error;
pub use crate::error::AppError;
use crate::{dialog, install, model::DisplayMode, registry, resource_ids, utf16, window};

/// Parse the real Windows command line, validate resources, and run one mode.
pub fn run() -> Result<(), AppError> {
    let mode = cli::from_process()?;
    if mode == RunMode::InstallSetCurrent {
        return install::set_current();
    }
    // SAFETY: NULL requests the current executable. This is a borrowed module
    // handle that remains valid until process termination; do not FreeLibrary it.
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    if instance.is_null() {
        return Err(last_error("GetModuleHandleW"));
    }

    let mut name = [0u16; 256];
    // SAFETY: The current executable stays loaded. The buffer is writable for
    // all 256 UTF-16 code units and remains alive for the synchronous call.
    let count = unsafe {
        LoadStringW(
            instance,
            u32::from(resource_ids::IDS_APP_NAME),
            name.as_mut_ptr(),
            256,
        )
    };
    let count = usize::try_from(count)
        .ok()
        .filter(|&length| length > 0 && length < name.len())
        .ok_or(AppError::InvalidResource("IDS_APP_NAME"))?;
    let product_name = String::from_utf16(&name[..count])
        .map_err(|_| AppError::InvalidResource("IDS_APP_NAME UTF-16"))?;

    // SAFETY: An integer in the low 16 bits is the Win32 MAKEINTRESOURCEW
    // convention, not a dereferenced Rust pointer. LoadIconW returns a shared
    // resource icon, so this handle must not be passed to DestroyIcon.
    let icon = unsafe { LoadIconW(instance, resource_ids::IDI_APP as usize as *const u16) };
    if icon.is_null() {
        return Err(last_error("LoadIconW"));
    }

    // SAFETY: Both resource identifiers use documented integer-resource
    // pointers. FindResourceW borrows data from the still-loaded executable.
    let manifest = unsafe {
        FindResourceW(
            instance,
            resource_ids::IDR_MANIFEST as usize as *const u16,
            RT_MANIFEST,
        )
    };
    if manifest.is_null() {
        return Err(last_error("FindResourceW(RT_MANIFEST)"));
    }

    if cfg!(debug_assertions) {
        report_diagnostic(&format!("{product_name}: native bootstrap OK"));
    }
    match mode {
        RunMode::Fullscreen => {
            let config = registry::load_registry();
            let duration = if config.display_mode == DisplayMode::Countdown {
                let Some(seconds) =
                    dialog::countdown(instance, None, config.last_countdown_seconds)?
                else {
                    return Ok(());
                };
                seconds
            } else {
                config.last_countdown_seconds
            };
            window::fullscreen(instance, icon, config, duration)
        }
        RunMode::Preview(parent) => window::preview(instance, icon, parent),
        RunMode::Configure(owner) => dialog::configure(instance, owner),
        RunMode::InstallSetCurrent => unreachable!("install helper returned before UI bootstrap"),
        #[cfg(debug_assertions)]
        RunMode::Developer(mode) => window::developer(instance, icon, mode),
    }
}

/// Send a short diagnostic to an attached debugger without creating a console.
pub fn report_diagnostic(message: &str) {
    if let Ok(wide) = utf16::nul_terminated(message) {
        // SAFETY: wide owns a NUL-terminated UTF-16 string for the entire call.
        unsafe { OutputDebugStringW(wide.as_ptr()) };
    }
}
