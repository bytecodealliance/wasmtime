use crate::bindings::http::types::ErrorCode;
use std::error::Error;
use std::fmt;
use wasmtime::component::ResourceTableError;

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

pub(crate) fn dns_error(rcode: String, info_code: u16) -> ErrorCode {
    ErrorCode::DnsError(crate::bindings::http::types::DnsErrorPayload {
        rcode: Some(rcode),
        info_code: Some(info_code),
    })
}

pub(crate) fn internal_error(msg: String) -> ErrorCode {
    ErrorCode::InternalError(Some(msg))
}

/// Translate a [`http::Error`] to a wasi-http `ErrorCode` in the context of a request.
pub fn http_request_error(err: http::Error) -> ErrorCode {
    if err.is::<http::uri::InvalidUri>() {
        return ErrorCode::HttpRequestUriInvalid;
    }

    tracing::warn!("http request error: {err:?}");

    ErrorCode::HttpProtocolError
}

/// Translate a [`hyper::Error`] to a wasi-http `ErrorCode` in the context of a request.
pub fn hyper_request_error(err: hyper::Error) -> ErrorCode {
    // If there's a source, we might be able to extract a wasi-http error from it.
    if let Some(cause) = err.source() {
        if let Some(err) = cause.downcast_ref::<ErrorCode>() {
            return err.clone();
        }
    }

    tracing::warn!("hyper request error: {err:?}");

    ErrorCode::HttpProtocolError
}

/// Translate a [`hyper::Error`] to a wasi-http `ErrorCode` in the context of a response.
pub fn hyper_response_error(err: hyper::Error) -> ErrorCode {
    if err.is_timeout() {
        return ErrorCode::HttpResponseTimeout;
    }

    // If there's a source, we might be able to extract a wasi-http error from it.
    if let Some(cause) = err.source() {
        if let Some(err) = cause.downcast_ref::<ErrorCode>() {
            return err.clone();
        }
    }

    tracing::warn!("hyper response error: {err:?}");

    ErrorCode::HttpProtocolError
}
