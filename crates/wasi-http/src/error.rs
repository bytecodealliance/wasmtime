use crate::bindings::http::types::ErrorCode;
use std::error::Error;
use std::fmt;
use wasmtime_wasi::ResourceTableError;

/// A [`Result`] type where the error type defaults to [`HttpError`].
pub type HttpResult<T, E = HttpError> = Result<T, E>;

/// A `wasi:http`-specific error type used to represent either a trap or an
/// [`ErrorCode`].
///
/// Modeled after [`TrappableError`](wasmtime_wasi::TrappableError).
#[repr(transparent)]
pub struct HttpError {
    err: anyhow::Error,
}

impl HttpError {
    /// Create a new `HttpError` that represents a trap.
    pub fn trap(err: impl Into<anyhow::Error>) -> HttpError {
        HttpError { err: err.into() }
    }

    /// Downcast this error to an [`ErrorCode`].
    pub fn downcast(self) -> anyhow::Result<ErrorCode> {
        self.err.downcast()
    }

    /// Downcast this error to a reference to an [`ErrorCode`]
    pub fn downcast_ref(&self) -> Option<&ErrorCode> {
        self.err.downcast_ref()
    }
}

impl From<ErrorCode> for HttpError {
    fn from(error: ErrorCode) -> Self {
        Self { err: error.into() }
    }
}

impl From<ResourceTableError> for HttpError {
    fn from(error: ResourceTableError) -> Self {
        HttpError::trap(error)
    }
}

impl fmt::Debug for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.err.fmt(f)
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.err.fmt(f)
    }
}

impl Error for HttpError {}
