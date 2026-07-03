use std::fmt;
use std::io;

/// Error type for `wasi:http` embedder interactions.
///
/// This error type is a superset of the wasip2/wasip3 error types for example
/// and is the error that the embedder interacts with for all operations.
#[derive(Debug)]
#[non_exhaustive]
#[expect(missing_docs, reason = "too cluttered")]
pub enum Error {
    Hyper(hyper::Error),
    Connect(io::Error),
    Tls(io::Error),
    #[cfg(feature = "default-send-request")]
    InvalidDnsNameError(rustls::pki_types::InvalidDnsNameError),
    HttpRequestBodySize(Option<u64>),

    DnsTimeout,
    DnsError {
        rcode: Option<String>,
        info_code: Option<u16>,
    },
    DestinationNotFound,
    DestinationUnavailable,
    DestinationIpProhibited,
    DestinationIpUnroutable,
    ConnectionRefused,
    ConnectionTerminated,
    ConnectionTimeout,
    ConnectionReadTimeout,
    ConnectionWriteTimeout,
    ConnectionLimitReached,
    TlsProtocolError,
    TlsCertificateError,
    TlsAlertReceived {
        alert_id: Option<u8>,
        alert_message: Option<String>,
    },
    HttpRequestDenied,
    HttpRequestLengthRequired,
    HttpRequestMethodInvalid,
    HttpRequestUriInvalid,
    HttpRequestUriTooLong,
    HttpRequestHeaderSectionSize(Option<u32>),
    HttpRequestHeaderSize {
        field_name: Option<String>,
        field_size: Option<u32>,
    },
    HttpRequestTrailerSectionSize(Option<u32>),
    HttpRequestTrailerSize {
        field_name: Option<String>,
        field_size: Option<u32>,
    },
    HttpResponseIncomplete,
    HttpResponseHeaderSectionSize(Option<u32>),
    HttpResponseHeaderSize {
        field_name: Option<String>,
        field_size: Option<u32>,
    },
    HttpResponseBodySize(Option<u64>),
    HttpResponseTrailerSectionSize(Option<u32>),
    HttpResponseTrailerSize {
        field_name: Option<String>,
        field_size: Option<u32>,
    },
    HttpResponseTransferCoding(Option<String>),
    HttpResponseContentCoding(Option<String>),
    HttpResponseTimeout,
    HttpUpgradeFailed,
    HttpProtocolError,
    LoopDetected,
    ConfigurationError,
    InternalError(Option<String>),
}

/// Convenience type for a result with [`Error`]
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Self::Hyper(err)
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(e: std::convert::Infallible) -> Self {
        match e {}
    }
}

#[cfg(feature = "default-send-request")]
impl From<rustls::pki_types::InvalidDnsNameError> for Error {
    fn from(err: rustls::pki_types::InvalidDnsNameError) -> Self {
        Self::InvalidDnsNameError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hyper(err) => write!(f, "hyper error: {err}"),
            Self::Connect(err) => write!(f, "connection error: {err}"),
            Self::Tls(err) => write!(f, "TLS error: {err}"),
            #[cfg(feature = "default-send-request")]
            Self::InvalidDnsNameError(err) => write!(f, "invalid DNS name: {err}"),
            Self::HttpRequestBodySize(None) => {
                write!(f, "HTTP request body size exceeds limit")
            }
            Self::HttpRequestBodySize(Some(size)) => {
                write!(f, "HTTP request body size exceeds limit: {size} bytes")
            }
            Self::DnsTimeout => write!(f, "DNS lookup timed out"),
            Self::DnsError { rcode, info_code } => {
                write!(f, "DNS error")?;
                if let Some(rcode) = rcode {
                    write!(f, ": rcode {rcode}")?;
                }
                if let Some(info_code) = info_code {
                    write!(f, " (info code {info_code})")?;
                }
                Ok(())
            }
            Self::DestinationNotFound => write!(f, "destination not found"),
            Self::DestinationUnavailable => write!(f, "destination unavailable"),
            Self::DestinationIpProhibited => write!(f, "destination IP address is prohibited"),
            Self::DestinationIpUnroutable => write!(f, "destination IP address is unroutable"),
            Self::ConnectionRefused => write!(f, "connection refused"),
            Self::ConnectionTerminated => write!(f, "connection terminated"),
            Self::ConnectionTimeout => write!(f, "connection timeout"),
            Self::ConnectionReadTimeout => write!(f, "connection read timeout"),
            Self::ConnectionWriteTimeout => write!(f, "connection write timeout"),
            Self::ConnectionLimitReached => write!(f, "connection limit reached"),
            Self::TlsProtocolError => write!(f, "TLS protocol error"),
            Self::TlsCertificateError => write!(f, "TLS certificate error"),
            Self::TlsAlertReceived {
                alert_id,
                alert_message,
            } => {
                write!(f, "TLS alert received")?;
                if let Some(id) = alert_id {
                    write!(f, ": alert id {id}")?;
                }
                if let Some(msg) = alert_message {
                    write!(f, ": {msg}")?;
                }
                Ok(())
            }
            Self::HttpRequestDenied => write!(f, "HTTP request denied"),
            Self::HttpRequestLengthRequired => write!(f, "HTTP request length required"),
            Self::HttpRequestMethodInvalid => write!(f, "invalid HTTP request method"),
            Self::HttpRequestUriInvalid => write!(f, "invalid HTTP request URI"),
            Self::HttpRequestUriTooLong => write!(f, "HTTP request URI too long"),
            Self::HttpRequestHeaderSectionSize(size) => {
                write!(f, "HTTP request header section size exceeds limit")?;
                if let Some(size) = size {
                    write!(f, ": {size} bytes")?;
                }
                Ok(())
            }
            Self::HttpRequestHeaderSize {
                field_name,
                field_size,
            } => {
                write!(f, "HTTP request header size exceeds limit")?;
                if let Some(name) = field_name {
                    write!(f, ": field {name:?}")?;
                }
                if let Some(size) = field_size {
                    write!(f, " ({size} bytes)")?;
                }
                Ok(())
            }
            Self::HttpRequestTrailerSectionSize(size) => {
                write!(f, "HTTP request trailer section size exceeds limit")?;
                if let Some(size) = size {
                    write!(f, ": {size} bytes")?;
                }
                Ok(())
            }
            Self::HttpRequestTrailerSize {
                field_name,
                field_size,
            } => {
                write!(f, "HTTP request trailer size exceeds limit")?;
                if let Some(name) = field_name {
                    write!(f, ": field {name:?}")?;
                }
                if let Some(size) = field_size {
                    write!(f, " ({size} bytes)")?;
                }
                Ok(())
            }
            Self::HttpResponseIncomplete => write!(f, "incomplete HTTP response"),
            Self::HttpResponseHeaderSectionSize(size) => {
                write!(f, "HTTP response header section size exceeds limit")?;
                if let Some(size) = size {
                    write!(f, ": {size} bytes")?;
                }
                Ok(())
            }
            Self::HttpResponseHeaderSize {
                field_name,
                field_size,
            } => {
                write!(f, "HTTP response header size exceeds limit")?;
                if let Some(name) = field_name {
                    write!(f, ": field {name:?}")?;
                }
                if let Some(size) = field_size {
                    write!(f, " ({size} bytes)")?;
                }
                Ok(())
            }
            Self::HttpResponseBodySize(size) => {
                write!(f, "HTTP response body size exceeds limit")?;
                if let Some(size) = size {
                    write!(f, ": {size} bytes")?;
                }
                Ok(())
            }
            Self::HttpResponseTrailerSectionSize(size) => {
                write!(f, "HTTP response trailer section size exceeds limit")?;
                if let Some(size) = size {
                    write!(f, ": {size} bytes")?;
                }
                Ok(())
            }
            Self::HttpResponseTrailerSize {
                field_name,
                field_size,
            } => {
                write!(f, "HTTP response trailer size exceeds limit")?;
                if let Some(name) = field_name {
                    write!(f, ": field {name:?}")?;
                }
                if let Some(size) = field_size {
                    write!(f, " ({size} bytes)")?;
                }
                Ok(())
            }
            Self::HttpResponseTransferCoding(coding) => {
                write!(f, "unsupported HTTP response transfer coding")?;
                if let Some(coding) = coding {
                    write!(f, ": {coding:?}")?;
                }
                Ok(())
            }
            Self::HttpResponseContentCoding(coding) => {
                write!(f, "unsupported HTTP response content coding")?;
                if let Some(coding) = coding {
                    write!(f, ": {coding:?}")?;
                }
                Ok(())
            }
            Self::HttpResponseTimeout => write!(f, "HTTP response timeout"),
            Self::HttpUpgradeFailed => write!(f, "HTTP upgrade failed"),
            Self::HttpProtocolError => write!(f, "HTTP protocol error"),
            Self::LoopDetected => write!(f, "request loop detected"),
            Self::ConfigurationError => write!(f, "configuration error"),
            Self::InternalError(None) => write!(f, "internal error"),
            Self::InternalError(Some(msg)) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
