//! Result and error types representing the outcome of compiling a function.

use verifier;
use std::error::Error as StdError;
use std::fmt;

/// A compilation error.
///
/// When Cretonne fails to compile a function, it will return one of these error codes.
#[derive(Debug, PartialEq, Eq)]
pub enum CtonError {
    /// The input is invalid.
    ///
    /// This error code is used by a WebAssembly translator when it encounters invalid WebAssembly
    /// code. This should never happen for validated WebAssembly code.
    InvalidInput,

    /// An IL verifier error.
    ///
    /// This always represents a bug, either in the code that generated IL for Cretonne, or a bug
    /// in Cretonne itself.
    Verifier(verifier::Error),

    /// An implementation limit was exceeded.
    ///
    /// Cretonne can compile very large and complicated functions, but the implementation has
    /// limits that cause compilation to fail when they are exceeded.
    ///
    /// See http://cretonne.readthedocs.io/en/latest/langref.html#implementation-limits
    ImplLimitExceeded,

    /// The code size for the function is too large.
    ///
    /// Different target ISAs may impose a limit on the size of a compiled function. If that limit
    /// is exceeded, compilation fails.
    CodeTooLarge,
}

/// A Cretonne compilation result.
pub type CtonResult = Result<(), CtonError>;

impl fmt::Display for CtonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CtonError::Verifier(ref e) => write!(f, "Verifier error: {}", e),
            CtonError::InvalidInput |
            CtonError::ImplLimitExceeded |
            CtonError::CodeTooLarge => f.write_str(self.description()),
        }
    }
}

impl StdError for CtonError {
    fn description(&self) -> &str {
        match *self {
            CtonError::InvalidInput => "Invalid input code",
            CtonError::Verifier(ref e) => &e.message,
            CtonError::ImplLimitExceeded => "Implementation limit exceeded",
            CtonError::CodeTooLarge => "Code for function is too large",
        }
    }
    fn cause(&self) -> Option<&StdError> {
        match *self {
            CtonError::Verifier(ref e) => Some(e),
            CtonError::InvalidInput |
            CtonError::ImplLimitExceeded |
            CtonError::CodeTooLarge => None,
        }
    }
}

impl From<verifier::Error> for CtonError {
    fn from(e: verifier::Error) -> CtonError {
        CtonError::Verifier(e)
    }
}
