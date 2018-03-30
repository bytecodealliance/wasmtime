//! Result and error types representing the outcome of compiling a function.

use verifier;

/// A compilation error.
///
/// When Cretonne fails to compile a function, it will return one of these error codes.
#[derive(Fail, Debug, PartialEq, Eq)]
pub enum CtonError {
    /// The input is invalid.
    ///
    /// This error code is used by a WebAssembly translator when it encounters invalid WebAssembly
    /// code. This should never happen for validated WebAssembly code.
    #[fail(display = "Invalid input code")]
    InvalidInput,

    /// An IR verifier error.
    ///
    /// This always represents a bug, either in the code that generated IR for Cretonne, or a bug
    /// in Cretonne itself.
    #[fail(display = "Verifier error: {}", _0)]
    Verifier(
        #[cause]
        verifier::Error
    ),

    /// An implementation limit was exceeded.
    ///
    /// Cretonne can compile very large and complicated functions, but the [implementation has
    /// limits][limits] that cause compilation to fail when they are exceeded.
    ///
    /// [limits]: https://cretonne.readthedocs.io/en/latest/langref.html#implementation-limits
    #[fail(display = "Implementation limit exceeded")]
    ImplLimitExceeded,

    /// The code size for the function is too large.
    ///
    /// Different target ISAs may impose a limit on the size of a compiled function. If that limit
    /// is exceeded, compilation fails.
    #[fail(display = "Code for function is too large")]
    CodeTooLarge,
}

/// A Cretonne compilation result.
pub type CtonResult = Result<(), CtonError>;

impl From<verifier::Error> for CtonError {
    fn from(e: verifier::Error) -> CtonError {
        CtonError::Verifier(e)
    }
}
