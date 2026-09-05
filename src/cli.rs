use std::{error::Error, fmt, ptr};

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::System::Environment::GetCommandLineW;
use windows_sys::Win32::UI::Shell::CommandLineToArgvW;

use crate::error::{last_error, AppError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Fullscreen,
    Preview(usize),
    Configure(Option<usize>),
    InstallSetCurrent,
    #[cfg(debug_assertions)]
    Developer(crate::model::DisplayMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    UnknownMode,
    MissingHandle,
    InvalidHandle,
    ExtraArguments,
    InvalidUnicode,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownMode => "expected /s, /p <HWND>, or /c [HWND]",
            Self::MissingHandle => "preview requires a parent HWND",
            Self::InvalidHandle => "HWND must be an unsigned decimal pointer-width integer",
            Self::ExtraArguments => "unexpected additional arguments",
            Self::InvalidUnicode => "arguments contain invalid UTF-16",
        })
    }
}

impl Error for ParseError {}

/// Parse arguments AFTER argv[0]. HWND existence is a separate Win32 check.
pub fn parse(args: &[&str]) -> Result<RunMode, ParseError> {
    if args == ["--install-set-current"] {
        return Ok(RunMode::InstallSetCurrent);
    }
    #[cfg(debug_assertions)]
    if let [mode] = args {
        match *mode {
            "--dev-render=time-date" => {
                return Ok(RunMode::Developer(crate::model::DisplayMode::TimeDate))
            }
            "--dev-render=countdown" => {
                return Ok(RunMode::Developer(crate::model::DisplayMode::Countdown))
            }
            _ => (),
        }
    }
    let Some(first) = args.first() else {
        return Ok(RunMode::Configure(None));
    };
    if args.len() > 2 {
        return Err(ParseError::ExtraArguments);
    }
    let body = first
        .strip_prefix('/')
        .or_else(|| first.strip_prefix('-'))
        .ok_or(ParseError::UnknownMode)?;
    let (mode, inline) = match body.split_once(':') {
        Some((mode, handle)) => (mode, Some(handle)),
        None => (body, None),
    };
    if mode.eq_ignore_ascii_case("s") {
        return if inline.is_none() && args.len() == 1 {
            Ok(RunMode::Fullscreen)
        } else {
            Err(ParseError::ExtraArguments)
        };
    }
    if !mode.eq_ignore_ascii_case("p") && !mode.eq_ignore_ascii_case("c") {
        return Err(ParseError::UnknownMode);
    }
    if inline.is_some() && args.len() != 1 {
        return Err(ParseError::ExtraArguments);
    }
    let handle = inline.or_else(|| args.get(1).copied());
    let parsed = handle.map(parse_handle).transpose()?;
    if mode.eq_ignore_ascii_case("p") {
        match parsed {
            Some(0) => Err(ParseError::InvalidHandle),
            Some(value) => Ok(RunMode::Preview(value)),
            None => Err(ParseError::MissingHandle),
        }
    } else {
        Ok(RunMode::Configure(parsed.filter(|&value| value != 0)))
    }
}

fn parse_handle(value: &str) -> Result<usize, ParseError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ParseError::InvalidHandle);
    }
    value.parse().map_err(|_| ParseError::InvalidHandle)
}

pub(crate) fn from_process() -> Result<RunMode, AppError> {
    let mut count = 0;
    // SAFETY: GetCommandLineW returns the OS-owned, terminated command line;
    // count is a valid writable i32. CommandLineToArgvW owns its returned block.
    let argv = unsafe { CommandLineToArgvW(GetCommandLineW(), &mut count) };
    if argv.is_null() {
        return Err(last_error("CommandLineToArgvW"));
    }
    struct OwnedArgv(*mut *mut u16);
    impl Drop for OwnedArgv {
        fn drop(&mut self) {
            // SAFETY: This is exactly the allocation returned by CommandLineToArgvW.
            unsafe { LocalFree(self.0.cast()) };
        }
    }
    let owned = OwnedArgv(argv);
    if count < 1 {
        return Err(AppError::OperationFailed(
            "CommandLineToArgvW argument count",
        ));
    }
    if count > 3 {
        return Err(AppError::Arguments(ParseError::ExtraArguments));
    }
    // SAFETY: The successful API call guarantees count valid pointers into owned.
    let pointers = unsafe { std::slice::from_raw_parts(owned.0, count as usize) };
    let mut values = Vec::new();
    for &pointer in pointers.iter().skip(1) {
        if pointer.is_null() {
            return Err(AppError::OperationFailed(
                "CommandLineToArgvW null argument",
            ));
        }
        let mut length = 0;
        // SAFETY: Each API-created argument is NUL terminated within the same
        // still-live allocation; none of these pointers escape this function.
        unsafe {
            while ptr::read(pointer.add(length)) != 0 {
                length += 1;
            }
            let wide = std::slice::from_raw_parts(pointer, length);
            values.push(
                String::from_utf16(wide)
                    .map_err(|_| AppError::Arguments(ParseError::InvalidUnicode))?,
            );
        }
    }
    let arguments: Vec<_> = values.iter().map(String::as_str).collect();
    parse(&arguments).map_err(AppError::Arguments)
}
