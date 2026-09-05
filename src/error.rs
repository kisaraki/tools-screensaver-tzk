use std::{error::Error, fmt};

use windows_sys::Win32::Foundation::GetLastError;

use crate::cli::ParseError;

#[derive(Debug, Clone, Copy)]
pub enum AppError {
    Arguments(ParseError),
    InvalidParent,
    Win32 {
        operation: &'static str,
        code: u32,
    },
    InvalidResource(&'static str),
    OperationFailed(&'static str),
    InstallHelper {
        operation: &'static str,
        code: Option<u32>,
    },
}

impl AppError {
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Arguments(_) | Self::InvalidParent => 2,
            Self::InstallHelper { .. } => 4,
            _ => 3,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arguments(error) => write!(f, "Invalid command line: {error}"),
            Self::InvalidParent => f.write_str("The preview parent window is not valid"),
            Self::Win32 { operation, code } => write!(f, "{operation} failed (Win32 error {code})"),
            Self::InvalidResource(name) => write!(f, "Missing or invalid resource: {name}"),
            Self::OperationFailed(operation) => write!(f, "{operation} failed"),
            Self::InstallHelper { operation, code } => match code {
                Some(code) => write!(
                    f,
                    "Set-current helper: {operation} failed (Win32 error {code})"
                ),
                None => write!(f, "Set-current helper refused: {operation}"),
            },
        }
    }
}

impl Error for AppError {}

pub(crate) fn last_error(operation: &'static str) -> AppError {
    // SAFETY: No preconditions; callers invoke this immediately after an API failure.
    let code = unsafe { GetLastError() };
    AppError::Win32 { operation, code }
}
