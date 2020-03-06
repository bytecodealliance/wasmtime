//! Result and error types representing the outcome of compiling a function.

use crate::verifier::VerifierErrors;
use thiserror::Error;

/// A compilation error.
///
/// When Cranelift fails to compile a function, it will return one of these error codes.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum CodegenError {
    /// A list of IR verifier errors.
    ///
    /// This always represents a bug, either in the code that generated IR for Cranelift, or a bug
    /// in Cranelift itself.
    #[error("Verifier errors")]
    Verifier(#[from] VerifierErrors),

    /// An implementation limit was exceeded.
    ///
    /// Cranelift can compile very large and complicated functions, but the [implementation has
    /// limits][limits] that cause compilation to fail when they are exceeded.
    ///
    /// [limits]: https://github.com/bytecodealliance/wasmtime/blob/master/cranelift/docs/ir.md#implementation-limits
    #[error("Implementation limit exceeded")]
    ImplLimitExceeded,

    /// The code size for the function is too large.
    ///
    /// Different target ISAs may impose a limit on the size of a compiled function. If that limit
    /// is exceeded, compilation fails.
    #[error("Code for function is too large")]
    CodeTooLarge,
}

/// A convenient alias for a `Result` that uses `CodegenError` as the error type.
pub type CodegenResult<T> = Result<T, CodegenError>;
