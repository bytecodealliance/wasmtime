use crate::p3::bindings::http::types::{self, ErrorCode, Method, Scheme};
use crate::{Error, WasiHttpCtxView};
use core::convert::Infallible;
use core::error::Error as _;
use std::io::ErrorKind;

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

impl WasiHttpCtxView<'_> {
    pub(crate) fn error_to_p3(&mut self, e: &Error) -> ErrorCode {
        match e {
            Error::Hyper(err) => {
                // If there's a source, we might be able to extract a wasi-http
                // error from it.
                if let Some(cause) = err.source() {
                    if let Some(err) = cause.downcast_ref::<ErrorCode>() {
                        return err.clone();
                    }
                    if let Some(err) = cause.downcast_ref::<Error>() {
                        return self.error_to_p3(err);
                    }
                }

                self.hooks.p3_error_from_hyper(err)
            }
            Error::Connect(err) => {
                if err.kind() == ErrorKind::AddrNotAvailable {
                    return ErrorCode::DnsError(types::DnsErrorPayload {
                        rcode: Some("address not available".to_string()),
                        info_code: None,
                    });
                }

                if err
                    .to_string()
                    .starts_with("failed to lookup address information")
                {
                    return ErrorCode::DnsError(types::DnsErrorPayload {
                        rcode: Some("address not available".to_string()),
                        info_code: None,
                    });
                }

                self.hooks.p3_error_from_connect(err)
            }
            Error::Tls(err) => self.hooks.p3_error_from_tls(err),
            #[cfg(feature = "default-send-request")]
            Error::InvalidDnsNameError(err) => self.hooks.p3_error_from_dns(err),
            Error::DnsTimeout => ErrorCode::DnsTimeout,
            Error::DnsError { rcode, info_code } => ErrorCode::DnsError(types::DnsErrorPayload {
                rcode: rcode.clone(),
                info_code: *info_code,
            }),
            Error::DestinationNotFound => ErrorCode::DestinationNotFound,
            Error::DestinationUnavailable => ErrorCode::DestinationUnavailable,
            Error::DestinationIpProhibited => ErrorCode::DestinationIpProhibited,
            Error::DestinationIpUnroutable => ErrorCode::DestinationIpUnroutable,
            Error::ConnectionRefused => ErrorCode::ConnectionRefused,
            Error::ConnectionTerminated => ErrorCode::ConnectionTerminated,
            Error::ConnectionTimeout => ErrorCode::ConnectionTimeout,
            Error::ConnectionReadTimeout => ErrorCode::ConnectionReadTimeout,
            Error::ConnectionWriteTimeout => ErrorCode::ConnectionWriteTimeout,
            Error::ConnectionLimitReached => ErrorCode::ConnectionLimitReached,
            Error::TlsProtocolError => ErrorCode::TlsProtocolError,
            Error::TlsCertificateError => ErrorCode::TlsCertificateError,
            Error::TlsAlertReceived {
                alert_id,
                alert_message,
            } => ErrorCode::TlsAlertReceived(types::TlsAlertReceivedPayload {
                alert_id: *alert_id,
                alert_message: alert_message.clone(),
            }),
            Error::HttpRequestDenied => ErrorCode::HttpRequestDenied,
            Error::HttpRequestLengthRequired => ErrorCode::HttpRequestLengthRequired,
            Error::HttpRequestBodySize(payload) => ErrorCode::HttpRequestBodySize(*payload),
            Error::HttpRequestMethodInvalid => ErrorCode::HttpRequestMethodInvalid,
            Error::HttpRequestUriInvalid => ErrorCode::HttpRequestUriInvalid,
            Error::HttpRequestUriTooLong => ErrorCode::HttpRequestUriTooLong,
            Error::HttpRequestHeaderSectionSize(payload) => {
                ErrorCode::HttpRequestHeaderSectionSize(*payload)
            }
            Error::HttpRequestHeaderSize {
                field_name,
                field_size,
            } => ErrorCode::HttpRequestHeaderSize(Some(types::FieldSizePayload {
                field_name: field_name.clone(),
                field_size: *field_size,
            })),
            Error::HttpRequestTrailerSectionSize(payload) => {
                ErrorCode::HttpRequestTrailerSectionSize(*payload)
            }
            Error::HttpRequestTrailerSize {
                field_name,
                field_size,
            } => ErrorCode::HttpRequestTrailerSize(types::FieldSizePayload {
                field_name: field_name.clone(),
                field_size: *field_size,
            }),
            Error::HttpResponseIncomplete => ErrorCode::HttpResponseIncomplete,
            Error::HttpResponseHeaderSectionSize(payload) => {
                ErrorCode::HttpResponseHeaderSectionSize(*payload)
            }
            Error::HttpResponseHeaderSize {
                field_name,
                field_size,
            } => ErrorCode::HttpResponseHeaderSize(types::FieldSizePayload {
                field_name: field_name.clone(),
                field_size: *field_size,
            }),
            Error::HttpResponseBodySize(payload) => ErrorCode::HttpResponseBodySize(*payload),
            Error::HttpResponseTrailerSectionSize(payload) => {
                ErrorCode::HttpResponseTrailerSectionSize(*payload)
            }
            Error::HttpResponseTrailerSize {
                field_name,
                field_size,
            } => ErrorCode::HttpResponseTrailerSize(types::FieldSizePayload {
                field_name: field_name.clone(),
                field_size: *field_size,
            }),
            Error::HttpResponseTransferCoding(payload) => {
                ErrorCode::HttpResponseTransferCoding(payload.clone())
            }
            Error::HttpResponseContentCoding(payload) => {
                ErrorCode::HttpResponseContentCoding(payload.clone())
            }
            Error::HttpResponseTimeout => ErrorCode::HttpResponseTimeout,
            Error::HttpUpgradeFailed => ErrorCode::HttpUpgradeFailed,
            Error::HttpProtocolError => ErrorCode::HttpProtocolError,
            Error::LoopDetected => ErrorCode::LoopDetected,
            Error::ConfigurationError => ErrorCode::ConfigurationError,
            Error::InternalError(payload) => ErrorCode::InternalError(payload.clone()),
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
