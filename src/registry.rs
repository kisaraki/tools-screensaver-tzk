//! Bounded, current-user Windows Registry adapter for application settings.
use std::ptr;

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::*;

use crate::config::{self, AppConfig, RawValue, SettingsStore, StoreError};

const KEY_PATH: &str = "Software\\MyDateTimeScreensaver";
const MAX_VALUE_BYTES: u32 = 4096;

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: This wrapper owns only keys returned by RegOpen/CreateKeyExW.
        unsafe { RegCloseKey(self.0) };
    }
}

pub struct RegistryStore {
    path: Vec<u16>,
}

impl RegistryStore {
    pub fn new() -> Self {
        let path = registry_path();
        Self {
            path: path.encode_utf16().chain([0]).collect(),
        }
    }

    #[cfg(test)]
    pub(crate) fn at(path: &str) -> Self {
        Self {
            path: path.encode_utf16().chain([0]).collect(),
        }
    }

    fn open(&self, access: u32) -> Result<Option<Key>, StoreError> {
        let mut key = ptr::null_mut();
        // SAFETY: path is NUL-terminated; output is writable and owned on success.
        let code =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, self.path.as_ptr(), 0, access, &mut key) };
        match code {
            ERROR_SUCCESS => Ok(Some(Key(key))),
            ERROR_FILE_NOT_FOUND => Ok(None),
            code => Err(StoreError {
                operation: "RegOpenKeyExW",
                code,
            }),
        }
    }

    fn create(&self) -> Result<Key, StoreError> {
        let mut key = ptr::null_mut();
        let mut disposition = 0;
        // SAFETY: path is terminated; outputs are writable. No security descriptor.
        let code = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                self.path.as_ptr(),
                0,
                ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_QUERY_VALUE | KEY_SET_VALUE,
                ptr::null(),
                &mut key,
                &mut disposition,
            )
        };
        if code == ERROR_SUCCESS {
            Ok(Key(key))
        } else {
            Err(StoreError {
                operation: "RegCreateKeyExW",
                code,
            })
        }
    }
}

impl Default for RegistryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsStore for RegistryStore {
    fn get(&self, name: &str) -> Result<Option<RawValue>, StoreError> {
        let Some(key) = self.open(KEY_QUERY_VALUE)? else {
            return Ok(None);
        };
        let name: Vec<u16> = name.encode_utf16().chain([0]).collect();
        for _ in 0..3 {
            let mut kind = 0;
            let mut length = 0;
            // SAFETY: First query requests only type/size for a terminated name.
            let code = unsafe {
                RegQueryValueExW(
                    key.0,
                    name.as_ptr(),
                    ptr::null(),
                    &mut kind,
                    ptr::null_mut(),
                    &mut length,
                )
            };
            if code == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            if code != ERROR_SUCCESS && code != ERROR_MORE_DATA {
                return Err(StoreError {
                    operation: "RegQueryValueExW(size)",
                    code,
                });
            }
            if length > MAX_VALUE_BYTES {
                return Ok(None);
            }
            let mut bytes = vec![0u8; length as usize];
            let mut actual = length;
            let data = if bytes.is_empty() {
                ptr::null_mut()
            } else {
                bytes.as_mut_ptr()
            };
            // SAFETY: Data is NULL for zero length or points to `length` writable bytes.
            let code = unsafe {
                RegQueryValueExW(
                    key.0,
                    name.as_ptr(),
                    ptr::null(),
                    &mut kind,
                    data,
                    &mut actual,
                )
            };
            if code == ERROR_MORE_DATA {
                continue;
            }
            if code == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            if code != ERROR_SUCCESS {
                return Err(StoreError {
                    operation: "RegQueryValueExW(data)",
                    code,
                });
            }
            bytes.truncate(actual as usize);
            return Ok(Some(RawValue { kind, bytes }));
        }
        Ok(None)
    }

    fn set(&mut self, name: &str, value: &RawValue) -> Result<(), StoreError> {
        let key = self.create()?;
        let name: Vec<u16> = name.encode_utf16().chain([0]).collect();
        let length = u32::try_from(value.bytes.len()).map_err(|_| StoreError {
            operation: "RegSetValueExW(length)",
            code: ERROR_MORE_DATA,
        })?;
        // SAFETY: Name and value buffers remain live for the synchronous call.
        let code = unsafe {
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                value.kind,
                value.bytes.as_ptr(),
                length,
            )
        };
        if code == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(StoreError {
                operation: "RegSetValueExW",
                code,
            })
        }
    }

    fn delete(&mut self, name: &str) -> Result<(), StoreError> {
        let Some(key) = self.open(KEY_SET_VALUE)? else {
            return Ok(());
        };
        let name: Vec<u16> = name.encode_utf16().chain([0]).collect();
        // SAFETY: Name is terminated and key is owned for this call.
        let code = unsafe { RegDeleteValueW(key.0, name.as_ptr()) };
        if code == ERROR_SUCCESS || code == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(StoreError {
                operation: "RegDeleteValueW",
                code,
            })
        }
    }
}

pub fn load_registry() -> AppConfig {
    config::load(&RegistryStore::new())
}

#[cfg(debug_assertions)]
fn registry_path() -> String {
    const PREFIX: &str = "Software\\MyDateTimeScreensaver\\Tests\\";
    std::env::var("MYDATETIME_SCREENSAVER_TEST_KEY")
        .ok()
        .filter(|path| path.starts_with(PREFIX) && !path.contains(".."))
        .unwrap_or_else(|| KEY_PATH.to_owned())
}

#[cfg(not(debug_assertions))]
fn registry_path() -> String {
    KEY_PATH.to_owned()
}
