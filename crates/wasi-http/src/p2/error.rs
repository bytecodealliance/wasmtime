use crate::p2::bindings::http::types::{self, ErrorCode};
use crate::{Error, FieldMapError};
use std::error::Error as _;
use std::fmt;
use std::io::ErrorKind;
use tracing::warn;
use wasmtime::component::ResourceTableError;

/// A [`Result`] type where the error type defaults to [`HttpError`].
pub type HttpResult<T, E = HttpError> = Result<T, E>;

/// A `wasi:http`-specific error type used to represent either a trap or an
/// [`ErrorCode`].
///
/// Modeled after [`TrappableError`](wasmtime_wasi::TrappableError).
#[repr(transparent)]
pub struct HttpError {
    err: wasmtime::Error,
}

impl HttpError {
    /// Create a new `HttpError` that represents a trap.
    pub fn trap(err: impl Into<wasmtime::Error>) -> HttpError {
        HttpError { err: err.into() }
    }

    /// Downcast this error to an [`ErrorCode`].
    pub fn downcast(self) -> wasmtime::Result<ErrorCode> {
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

impl std::error::Error for HttpError {}

/// A [`Result`] type where the error type defaults to [`HeaderError`].
pub type HeaderResult<T, E = HeaderError> = Result<T, E>;

/// A `wasi:http`-specific error type used to represent either a trap or an
/// [`types::HeaderError`].
///
/// Modeled after [`TrappableError`](wasmtime_wasi::TrappableError).
#[repr(transparent)]
pub struct HeaderError {
    err: wasmtime::Error,
}

impl HeaderError {
    /// Create a new `HeaderError` that represents a trap.
    pub fn trap(err: impl Into<wasmtime::Error>) -> HeaderError {
        HeaderError { err: err.into() }
    }

    /// Downcast this error to an [`ErrorCode`].
    pub fn downcast(self) -> wasmtime::Result<types::HeaderError> {
        self.err.downcast()
    }

    /// Downcast this error to a reference to an [`ErrorCode`]
    pub fn downcast_ref(&self) -> Option<&types::HeaderError> {
        self.err.downcast_ref()
    }
}

impl From<types::HeaderError> for HeaderError {
    fn from(error: types::HeaderError) -> Self {
        Self { err: error.into() }
    }
}

impl From<ResourceTableError> for HeaderError {
    fn from(error: ResourceTableError) -> Self {
        HeaderError::trap(error)
    }
}

impl From<http::header::InvalidHeaderName> for HeaderError {
    fn from(_: http::header::InvalidHeaderName) -> Self {
        HeaderError::from(types::HeaderError::InvalidSyntax)
    }
}

impl From<http::header::InvalidHeaderValue> for HeaderError {
    fn from(_: http::header::InvalidHeaderValue) -> Self {
        HeaderError::from(types::HeaderError::InvalidSyntax)
    }
}

impl From<FieldMapError> for HeaderError {
    fn from(err: FieldMapError) -> Self {
        match err {
            FieldMapError::Immutable => types::HeaderError::Immutable.into(),
            FieldMapError::InvalidHeaderName => types::HeaderError::InvalidSyntax.into(),
            FieldMapError::TooManyFields | FieldMapError::TotalSizeTooBig => HeaderError::trap(err),
        }
    }
}

impl fmt::Debug for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.err.fmt(f)
    }
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.err.fmt(f)
    }
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

impl From<hyper::Error> for ErrorCode {
    fn from(err: hyper::Error) -> Self {
        hyper_response_error(err)
    }
}

impl From<ErrorCode> for Error {
    fn from(e: ErrorCode) -> Self {
        match e {
            ErrorCode::DnsTimeout => Self::DnsTimeout,
            ErrorCode::DnsError(payload) => Self::DnsError {
                rcode: payload.rcode,
                info_code: payload.info_code,
            },
            ErrorCode::DestinationNotFound => Self::DestinationNotFound,
            ErrorCode::DestinationUnavailable => Self::DestinationUnavailable,
            ErrorCode::DestinationIpProhibited => Self::DestinationIpProhibited,
            ErrorCode::DestinationIpUnroutable => Self::DestinationIpUnroutable,
            ErrorCode::ConnectionRefused => Self::ConnectionRefused,
            ErrorCode::ConnectionTerminated => Self::ConnectionTerminated,
            ErrorCode::ConnectionTimeout => Self::ConnectionTimeout,
            ErrorCode::ConnectionReadTimeout => Self::ConnectionReadTimeout,
            ErrorCode::ConnectionWriteTimeout => Self::ConnectionWriteTimeout,
            ErrorCode::ConnectionLimitReached => Self::ConnectionLimitReached,
            ErrorCode::TlsProtocolError => Self::TlsProtocolError,
            ErrorCode::TlsCertificateError => Self::TlsCertificateError,
            ErrorCode::TlsAlertReceived(payload) => Self::TlsAlertReceived {
                alert_id: payload.alert_id,
                alert_message: payload.alert_message,
            },
            ErrorCode::HttpRequestDenied => Self::HttpRequestDenied,
            ErrorCode::HttpRequestLengthRequired => Self::HttpRequestLengthRequired,
            ErrorCode::HttpRequestBodySize(payload) => Self::HttpRequestBodySize(payload),
            ErrorCode::HttpRequestMethodInvalid => Self::HttpRequestMethodInvalid,
            ErrorCode::HttpRequestUriInvalid => Self::HttpRequestUriInvalid,
            ErrorCode::HttpRequestUriTooLong => Self::HttpRequestUriTooLong,
            ErrorCode::HttpRequestHeaderSectionSize(payload) => {
                Self::HttpRequestHeaderSectionSize(payload)
            }
            ErrorCode::HttpRequestHeaderSize(payload) => {
                let (field_name, field_size) = match payload {
                    Some(p) => (p.field_name, p.field_size),
                    None => (None, None),
                };
                Self::HttpRequestHeaderSize {
                    field_name,
                    field_size,
                }
            }
            ErrorCode::HttpRequestTrailerSectionSize(payload) => {
                Self::HttpRequestTrailerSectionSize(payload)
            }
            ErrorCode::HttpRequestTrailerSize(payload) => Self::HttpRequestTrailerSize {
                field_name: payload.field_name,
                field_size: payload.field_size,
            },
            ErrorCode::HttpResponseIncomplete => Self::HttpResponseIncomplete,
            ErrorCode::HttpResponseHeaderSectionSize(payload) => {
                Self::HttpResponseHeaderSectionSize(payload)
            }
            ErrorCode::HttpResponseHeaderSize(payload) => Self::HttpRequestHeaderSize {
                field_name: payload.field_name,
                field_size: payload.field_size,
            },
            ErrorCode::HttpResponseBodySize(payload) => Self::HttpResponseBodySize(payload),
            ErrorCode::HttpResponseTrailerSectionSize(payload) => {
                Self::HttpResponseTrailerSectionSize(payload)
            }
            ErrorCode::HttpResponseTrailerSize(payload) => Self::HttpResponseTrailerSize {
                field_name: payload.field_name,
                field_size: payload.field_size,
            },
            ErrorCode::HttpResponseTransferCoding(payload) => {
                Self::HttpResponseTransferCoding(payload)
            }
            ErrorCode::HttpResponseContentCoding(payload) => {
                Self::HttpResponseContentCoding(payload)
            }
            ErrorCode::HttpResponseTimeout => Self::HttpResponseTimeout,
            ErrorCode::HttpUpgradeFailed => Self::HttpUpgradeFailed,
            ErrorCode::HttpProtocolError => Self::HttpProtocolError,
            ErrorCode::LoopDetected => Self::LoopDetected,
            ErrorCode::ConfigurationError => Self::ConfigurationError,
            ErrorCode::InternalError(payload) => Self::InternalError(payload),
        }
    }
}

impl From<Error> for ErrorCode {
    fn from(e: Error) -> Self {
        match e {
            Error::Hyper(err) => {
                // If there's a source, we might be able to extract a wasi-http error from it.
                if let Some(cause) = err.source() {
                    if let Some(err) = cause.downcast_ref::<Self>() {
                        return err.clone();
                    }
                }

                warn!("hyper error: {err:?}");

                Self::HttpProtocolError
            }
            Error::Connect(err) => {
                if err.kind() == ErrorKind::AddrNotAvailable {
                    return Self::DnsError(types::DnsErrorPayload {
                        rcode: Some("address not available".to_string()),
                        info_code: None,
                    });
                }

                if err
                    .to_string()
                    .starts_with("failed to lookup address information")
                {
                    return Self::DnsError(types::DnsErrorPayload {
                        rcode: Some("address not available".to_string()),
                        info_code: None,
                    });
                }

                warn!("connect error: {err:?}");
                Self::ConnectionRefused
            }
            Error::Tls(err) => {
                warn!("tls protocol error: {err:?}");
                Self::TlsProtocolError
            }
            Error::InvalidDnsNameError(err) => {
                warn!("dns lookup error: {err:?}");
                Self::DnsError(types::DnsErrorPayload {
                    rcode: Some("invalid dns name".to_string()),
                    info_code: None,
                })
            }
            Error::DnsTimeout => Self::DnsTimeout,
            Error::DnsError { rcode, info_code } => {
                Self::DnsError(types::DnsErrorPayload { rcode, info_code })
            }
            Error::DestinationNotFound => Self::DestinationNotFound,
            Error::DestinationUnavailable => Self::DestinationUnavailable,
            Error::DestinationIpProhibited => Self::DestinationIpProhibited,
            Error::DestinationIpUnroutable => Self::DestinationIpUnroutable,
            Error::ConnectionRefused => Self::ConnectionRefused,
            Error::ConnectionTerminated => Self::ConnectionTerminated,
            Error::ConnectionTimeout => Self::ConnectionTimeout,
            Error::ConnectionReadTimeout => Self::ConnectionReadTimeout,
            Error::ConnectionWriteTimeout => Self::ConnectionWriteTimeout,
            Error::ConnectionLimitReached => Self::ConnectionLimitReached,
            Error::TlsProtocolError => Self::TlsProtocolError,
            Error::TlsCertificateError => Self::TlsCertificateError,
            Error::TlsAlertReceived {
                alert_id,
                alert_message,
            } => Self::TlsAlertReceived(types::TlsAlertReceivedPayload {
                alert_id,
                alert_message,
            }),
            Error::HttpRequestDenied => Self::HttpRequestDenied,
            Error::HttpRequestLengthRequired => Self::HttpRequestLengthRequired,
            Error::HttpRequestBodySize(payload) => Self::HttpRequestBodySize(payload),
            Error::HttpRequestMethodInvalid => Self::HttpRequestMethodInvalid,
            Error::HttpRequestUriInvalid => Self::HttpRequestUriInvalid,
            Error::HttpRequestUriTooLong => Self::HttpRequestUriTooLong,
            Error::HttpRequestHeaderSectionSize(payload) => {
                Self::HttpRequestHeaderSectionSize(payload)
            }
            Error::HttpRequestHeaderSize {
                field_name,
                field_size,
            } => Self::HttpRequestHeaderSize(Some(types::FieldSizePayload {
                field_name,
                field_size,
            })),
            Error::HttpRequestTrailerSectionSize(payload) => {
                Self::HttpRequestTrailerSectionSize(payload)
            }
            Error::HttpRequestTrailerSize {
                field_name,
                field_size,
            } => Self::HttpRequestTrailerSize(types::FieldSizePayload {
                field_name,
                field_size,
            }),
            Error::HttpResponseIncomplete => Self::HttpResponseIncomplete,
            Error::HttpResponseHeaderSectionSize(payload) => {
                Self::HttpResponseHeaderSectionSize(payload)
            }
            Error::HttpResponseHeaderSize {
                field_name,
                field_size,
            } => Self::HttpResponseHeaderSize(types::FieldSizePayload {
                field_name,
                field_size,
            }),
            Error::HttpResponseBodySize(payload) => Self::HttpResponseBodySize(payload),
            Error::HttpResponseTrailerSectionSize(payload) => {
                Self::HttpResponseTrailerSectionSize(payload)
            }
            Error::HttpResponseTrailerSize {
                field_name,
                field_size,
            } => Self::HttpResponseTrailerSize(types::FieldSizePayload {
                field_name,
                field_size,
            }),
            Error::HttpResponseTransferCoding(payload) => Self::HttpResponseTransferCoding(payload),
            Error::HttpResponseContentCoding(payload) => Self::HttpResponseContentCoding(payload),
            Error::HttpResponseTimeout => Self::HttpResponseTimeout,
            Error::HttpUpgradeFailed => Self::HttpUpgradeFailed,
            Error::HttpProtocolError => Self::HttpProtocolError,
            Error::LoopDetected => Self::LoopDetected,
            Error::ConfigurationError => Self::ConfigurationError,
            Error::InternalError(payload) => Self::InternalError(payload),
        }
    }
}
