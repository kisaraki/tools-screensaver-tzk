use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteriorNulError;

impl fmt::Display for InteriorNulError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Win32 strings must not contain an interior NUL")
    }
}

impl Error for InteriorNulError {}

/// Encode a Rust string for a NUL-terminated wide Win32 API.
///
/// Keep the returned buffer alive until the API has finished using its pointer.
pub fn nul_terminated(value: &str) -> Result<Vec<u16>, InteriorNulError> {
    if value.contains('\0') {
        return Err(InteriorNulError);
    }
    Ok(value.encode_utf16().chain(std::iter::once(0)).collect())
}

#[cfg(test)]
mod tests {
    use super::{nul_terminated, InteriorNulError};

    #[test]
    fn preserves_chinese_and_surrogate_pairs() {
        let actual = nul_terminated("時鐘🕒").expect("valid Unicode");
        assert_eq!(actual, [0x6642, 0x9418, 0xD83D, 0xDD52, 0]);
    }

    #[test]
    fn empty_string_is_a_valid_terminated_buffer() {
        assert_eq!(nul_terminated(""), Ok(vec![0]));
    }

    #[test]
    fn rejects_nul_including_a_caller_supplied_terminator() {
        for value in ["\0", "before\0after", "already terminated\0"] {
            assert_eq!(nul_terminated(value), Err(InteriorNulError));
        }
    }
}
