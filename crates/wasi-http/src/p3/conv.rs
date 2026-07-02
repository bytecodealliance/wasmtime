use crate::Error;
use crate::p3::bindings::http::types::{self, ErrorCode, Method, Scheme};
use core::convert::Infallible;
use core::error::Error as _;
use std::io::ErrorKind;
use tracing::warn;

impl From<Infallible> for ErrorCode {
    fn from(x: Infallible) -> Self {
        match x {}
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

impl From<http::Method> for Method {
    fn from(method: http::Method) -> Self {
        Self::from(&method)
    }
}

impl From<&http::Method> for Method {
    fn from(method: &http::Method) -> Self {
        if method == http::Method::GET {
            Self::Get
        } else if method == http::Method::HEAD {
            Self::Head
        } else if method == http::Method::POST {
            Self::Post
        } else if method == http::Method::PUT {
            Self::Put
        } else if method == http::Method::DELETE {
            Self::Delete
        } else if method == http::Method::CONNECT {
            Self::Connect
        } else if method == http::Method::OPTIONS {
            Self::Options
        } else if method == http::Method::TRACE {
            Self::Trace
        } else if method == http::Method::PATCH {
            Self::Patch
        } else {
            Self::Other(method.as_str().into())
        }
    }
}

impl TryFrom<Method> for http::Method {
    type Error = http::method::InvalidMethod;

    fn try_from(method: Method) -> Result<Self, Self::Error> {
        Self::try_from(&method)
    }
}

impl TryFrom<&Method> for http::Method {
    type Error = http::method::InvalidMethod;

    fn try_from(method: &Method) -> Result<Self, Self::Error> {
        match method {
            Method::Get => Ok(Self::GET),
            Method::Head => Ok(Self::HEAD),
            Method::Post => Ok(Self::POST),
            Method::Put => Ok(Self::PUT),
            Method::Delete => Ok(Self::DELETE),
            Method::Connect => Ok(Self::CONNECT),
            Method::Options => Ok(Self::OPTIONS),
            Method::Trace => Ok(Self::TRACE),
            Method::Patch => Ok(Self::PATCH),
            Method::Other(s) => s.parse(),
        }
    }
}

impl From<http::uri::Scheme> for Scheme {
    fn from(scheme: http::uri::Scheme) -> Self {
        Self::from(&scheme)
    }
}

impl From<&http::uri::Scheme> for Scheme {
    fn from(scheme: &http::uri::Scheme) -> Self {
        if *scheme == http::uri::Scheme::HTTP {
            Self::Http
        } else if *scheme == http::uri::Scheme::HTTPS {
            Self::Https
        } else {
            Self::Other(scheme.as_str().into())
        }
    }
}

impl TryFrom<Scheme> for http::uri::Scheme {
    type Error = http::uri::InvalidUri;

    fn try_from(scheme: Scheme) -> Result<Self, Self::Error> {
        Self::try_from(&scheme)
    }
}

impl TryFrom<&Scheme> for http::uri::Scheme {
    type Error = http::uri::InvalidUri;

    fn try_from(scheme: &Scheme) -> Result<Self, Self::Error> {
        match scheme {
            Scheme::Http => Ok(Self::HTTP),
            Scheme::Https => Ok(Self::HTTPS),
            Scheme::Other(s) => s.parse(),
        }
    }
}
