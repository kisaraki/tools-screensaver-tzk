//! Bounded HTTPS GETs for fixed product services; never follows redirects.
use std::{
    ptr,
    time::{Duration, Instant},
};
use windows_sys::Win32::Networking::WinHttp::*;

struct Handle(*mut core::ffi::c_void);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            WinHttpCloseHandle(self.0);
        }
    }
}
fn handle(value: *mut core::ffi::c_void) -> Result<Handle, String> {
    if value.is_null() {
        Err("無法建立 HTTPS 連線".into())
    } else {
        Ok(Handle(value))
    }
}
fn checked(value: i32) -> Result<(), String> {
    if value == 0 {
        Err(format!("HTTPS 連線失敗或逾時（WinHTTP {}）", unsafe {
            windows_sys::Win32::Foundation::GetLastError()
        }))
    } else {
        Ok(())
    }
}
pub(crate) fn get(host: &str, path: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
    if !matches!(
        host,
        "api.github.com"
            | "kisaraki.github.io"
            | "www.cwa.gov.tw"
            | "api.open-meteo.com"
            | "geocoding-api.open-meteo.com"
            | "ipwho.is"
    ) || !path.starts_with('/')
        || path.chars().any(char::is_control)
    {
        return Err("來源不在 HTTPS 白名單".into());
    }
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<_>>();
    let agent = wide(concat!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) tools-screensaver-tzk/",
        env!("CARGO_PKG_VERSION")
    ));
    let host = wide(host);
    let path = wide(path);
    let timeout = if max_bytes > 1024 * 1024 { 15000 } else { 5000 };
    // SAFETY: Owned parent handles and terminated strings outlive each child and call.
    unsafe {
        let session = handle(WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        ))?;
        checked(WinHttpSetTimeouts(
            session.0, timeout, timeout, timeout, timeout,
        ))?;
        let protocols = WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2;
        checked(WinHttpSetOption(
            session.0,
            WINHTTP_OPTION_SECURE_PROTOCOLS,
            std::ptr::from_ref(&protocols).cast(),
            size_of::<u32>() as u32,
        ))?;
        let connection = handle(WinHttpConnect(
            session.0,
            host.as_ptr(),
            INTERNET_DEFAULT_HTTPS_PORT,
            0,
        ))?;
        let request = handle(WinHttpOpenRequest(
            connection.0,
            windows_sys::w!("GET"),
            path.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        let policy = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
        checked(WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_REDIRECT_POLICY,
            std::ptr::from_ref(&policy).cast(),
            size_of::<u32>() as u32,
        ))?;
        // Request identity encoding: the CWA CDN's compressed body can fail native
        // WinHTTP decompression on Windows 10. Limits still apply to received bytes.
        let headers = wide("Accept: application/json, text/html, application/octet-stream\r\nAccept-Encoding: identity\r\nCache-Control: no-cache\r\n");
        checked(WinHttpSendRequest(
            request.0,
            headers.as_ptr(),
            (headers.len() - 1) as u32,
            ptr::null(),
            0,
            0,
            0,
        ))
        .map_err(|e| format!("送出請求：{e}"))?;
        checked(WinHttpReceiveResponse(request.0, ptr::null_mut()))
            .map_err(|e| format!("接收回應：{e}"))?;
        let mut status = 0u32;
        let mut length = size_of::<u32>() as u32;
        checked(WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            std::ptr::from_mut(&mut status).cast(),
            &mut length,
            ptr::null_mut(),
        ))?;
        if status != 200 {
            return Err(format!("來源 HTTP {status}"));
        }
        let started = Instant::now();
        let total = Duration::from_secs(if max_bytes > 1024 * 1024 { 180 } else { 20 });
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 16384];
        loop {
            if started.elapsed() > total {
                return Err("來源總讀取時間逾時".into());
            }
            let mut read = 0u32;
            checked(WinHttpReadData(
                request.0,
                chunk.as_mut_ptr().cast(),
                chunk.len() as u32,
                &mut read,
            ))
            .map_err(|e| format!("讀取回應：{e}"))?;
            if read == 0 {
                break;
            }
            if bytes.len().saturating_add(read as usize) > max_bytes {
                return Err("來源資料超過大小上限".into());
            }
            bytes.extend_from_slice(&chunk[..read as usize]);
        }
        Ok(bytes)
    }
}

pub(crate) fn json(host: &str, path: &str) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&get(host, path, 512 * 1024)?).map_err(|_| "來源 JSON 格式不正確".into())
}
