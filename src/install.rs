use std::{mem::size_of, ptr};

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, SMTO_BLOCK, WM_SETTINGCHANGE,
};

use crate::{error::AppError, utf16};

const INSTALLED_FILENAME: &str = "tools-screensaver-tzk.scr";
const MAX_WINDOWS_PATH: usize = 32_768;

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: This handle is returned by OpenProcessToken and is owned here.
            unsafe { CloseHandle(self.0) };
        }
    }
}

struct OwnedKey(HKEY);

impl Drop for OwnedKey {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: This key is returned by RegOpenKeyExW and is owned here.
            unsafe { RegCloseKey(self.0) };
        }
    }
}

fn install_error(operation: &'static str) -> AppError {
    // SAFETY: No preconditions; this is called immediately after a failed Win32 API.
    let code = unsafe { GetLastError() };
    AppError::InstallHelper {
        operation,
        code: Some(code),
    }
}

fn registry_error(operation: &'static str, code: u32) -> AppError {
    AppError::InstallHelper {
        operation,
        code: Some(code),
    }
}

fn refused(operation: &'static str) -> AppError {
    AppError::InstallHelper {
        operation,
        code: None,
    }
}

fn current_module_path() -> Result<String, AppError> {
    let mut buffer = vec![0u16; MAX_WINDOWS_PATH];
    // SAFETY: NULL requests the current module and buffer is writable for its full capacity.
    let length =
        unsafe { GetModuleFileNameW(ptr::null_mut(), buffer.as_mut_ptr(), buffer.len() as u32) };
    let length = usize::try_from(length).map_err(|_| refused("module path length overflow"))?;
    if length == 0 {
        return Err(install_error("GetModuleFileNameW"));
    }
    if length >= buffer.len() {
        return Err(refused("module path exceeds the bounded buffer"));
    }
    String::from_utf16(&buffer[..length]).map_err(|_| refused("module path is invalid UTF-16"))
}

fn expected_installed_path() -> Result<String, AppError> {
    let mut buffer = vec![0u16; MAX_WINDOWS_PATH];
    // SAFETY: The buffer is writable for the supplied capacity.
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
    let length = usize::try_from(length).map_err(|_| refused("System32 path length overflow"))?;
    if length == 0 {
        return Err(install_error("GetSystemDirectoryW"));
    }
    if length >= buffer.len() {
        return Err(refused("System32 path exceeds the bounded buffer"));
    }
    let mut system = String::from_utf16(&buffer[..length])
        .map_err(|_| refused("System32 path is invalid UTF-16"))?;
    if !system.ends_with(['\\', '/']) {
        system.push('\\');
    }
    system.push_str(INSTALLED_FILENAME);
    Ok(system)
}

fn require_non_elevated_token() -> Result<(), AppError> {
    let mut raw_token: HANDLE = ptr::null_mut();
    // SAFETY: The pseudo process handle is valid and raw_token is writable.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw_token) } == 0 {
        return Err(install_error("OpenProcessToken"));
    }
    let token = OwnedHandle(raw_token);
    let mut elevation = TOKEN_ELEVATION::default();
    let mut returned = 0u32;
    // SAFETY: token is live; elevation and returned are correctly sized writable outputs.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    } == 0
    {
        return Err(install_error("GetTokenInformation(TokenElevation)"));
    }
    if returned < size_of::<TOKEN_ELEVATION>() as u32 {
        return Err(refused("token elevation response is incomplete"));
    }
    if elevation.TokenIsElevated != 0 {
        return Err(refused("the process token is elevated"));
    }
    Ok(())
}

fn write_and_verify_current(path: &str) -> Result<(), AppError> {
    let subkey = utf16::nul_terminated("Control Panel\\Desktop")
        .map_err(|_| refused("registry subkey contains an embedded NUL"))?;
    let value_name = utf16::nul_terminated("SCRNSAVE.EXE")
        .map_err(|_| refused("registry value name contains an embedded NUL"))?;
    let value = utf16::nul_terminated(path)
        .map_err(|_| refused("installed path contains an embedded NUL"))?;
    let value_bytes = u32::try_from(value.len() * size_of::<u16>())
        .map_err(|_| refused("installed path is too large for the registry"))?;

    let mut raw_key: HKEY = ptr::null_mut();
    // SAFETY: Inputs are terminated UTF-16 and raw_key is writable.
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            &mut raw_key,
        )
    };
    if status != 0 {
        return Err(registry_error("RegOpenKeyExW(HKCU Desktop)", status));
    }
    let key = OwnedKey(raw_key);
    // SAFETY: key is open with KEY_SET_VALUE; pointers and byte length describe value.
    let status = unsafe {
        RegSetValueExW(
            key.0,
            value_name.as_ptr(),
            0,
            REG_SZ,
            value.as_ptr().cast(),
            value_bytes,
        )
    };
    if status != 0 {
        return Err(registry_error("RegSetValueExW(SCRNSAVE.EXE)", status));
    }

    let mut kind = 0u32;
    let mut actual_bytes = 0u32;
    // SAFETY: key is open with query access and output fields are writable.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            value_name.as_ptr(),
            ptr::null(),
            &mut kind,
            ptr::null_mut(),
            &mut actual_bytes,
        )
    };
    if status != 0 {
        return Err(registry_error(
            "RegQueryValueExW(SCRNSAVE.EXE size)",
            status,
        ));
    }
    if kind != REG_SZ || actual_bytes != value_bytes {
        return Err(refused("SCRNSAVE.EXE verification type or length differs"));
    }
    let mut actual = vec![0u16; actual_bytes as usize / size_of::<u16>()];
    // SAFETY: actual is sized from the preceding query and is writable for actual_bytes.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            value_name.as_ptr(),
            ptr::null(),
            &mut kind,
            actual.as_mut_ptr().cast(),
            &mut actual_bytes,
        )
    };
    if status != 0 {
        return Err(registry_error(
            "RegQueryValueExW(SCRNSAVE.EXE data)",
            status,
        ));
    }
    if kind != REG_SZ || actual != value {
        return Err(refused("SCRNSAVE.EXE verification data differs"));
    }
    Ok(())
}

fn notify_shell() -> Result<(), AppError> {
    let section = utf16::nul_terminated("Control Panel\\Desktop")
        .map_err(|_| refused("setting-change section contains an embedded NUL"))?;
    let mut result = 0usize;
    // SAFETY: section remains alive during this bounded synchronous broadcast.
    if unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            section.as_ptr() as isize,
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            5_000,
            &mut result,
        )
    } == 0
    {
        return Err(install_error("SendMessageTimeoutW(WM_SETTINGCHANGE)"));
    }
    Ok(())
}

pub(crate) fn set_current() -> Result<(), AppError> {
    let current = current_module_path()?;
    let expected = expected_installed_path()?;
    if !current.eq_ignore_ascii_case(&expected) {
        return Err(refused(
            "the executable is not the expected System32 installation",
        ));
    }
    require_non_elevated_token()?;
    write_and_verify_current(&current)?;
    notify_shell()
}
