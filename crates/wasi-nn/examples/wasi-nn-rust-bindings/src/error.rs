use super::NnErrno;
use core::fmt;
use core::num::NonZeroU16;

/// A raw error returned by wasi-nn APIs, internally containing a 16-bit error
/// code.
#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct Error {
    code: NonZeroU16,
}

impl Error {
    /// Constructs a new error from a raw error code, returning `None` if the
    /// error code is zero (which means success).
    pub fn from_raw_error(error: NnErrno) -> Option<Error> {
        Some(Error {
            code: NonZeroU16::new(error)?,
        })
    }

    /// Returns the raw error code that this error represents.
    pub fn raw_error(&self) -> u16 {
        self.code.get()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (error {})", strerror(self.code.get()), self.code)?;
        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("code", &self.code)
            .field("message", &strerror(self.code.get()))
            .finish()
    }
}

/// This should be generated automatically by witx-bindgen but is not yet for enums other than
/// `Errno` (this API uses `NnErrno` to avoid naming conflicts). TODO: https://github.com/bytecodealliance/wasi/issues/52.
fn strerror(code: u16) -> &'static str {
    match code {
        super::NN_ERRNO_SUCCESS => "No error occurred.",
        super::NN_ERRNO_INVALID_ARGUMENT => "Caller module passed an invalid argument.",
        super::NN_ERRNO_MISSING_MEMORY => "Caller module is missing a memory export.",
        super::NN_ERRNO_BUSY => "Device or resource busy.",
        _ => "Unknown error.",
    }
}

#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_from_success_code() {
        assert_eq!(None, Error::from_raw_error(0));
    }

    #[test]
    fn error_from_invalid_argument_code() {
        assert_eq!(
            "Caller module passed an invalid argument. (error 1)",
            Error::from_raw_error(1).unwrap().to_string()
        );
    }
}
