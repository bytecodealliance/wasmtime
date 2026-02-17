use crate::p3::bindings::http::types::{ErrorCode, Method, Scheme};
use core::convert::Infallible;
use core::error::Error as _;
use tracing::warn;

impl From<Infallible> for ErrorCode {
    fn from(x: Infallible) -> Self {
        match x {}
    }
}

impl ErrorCode {
    /// Translate a [`hyper::Error`] to a wasi-http [ErrorCode] in the context of a request.
    pub fn from_hyper_request_error(err: hyper::Error) -> Self {
        // If there's a source, we might be able to extract a wasi-http error from it.
        if let Some(cause) = err.source() {
            if let Some(err) = cause.downcast_ref::<Self>() {
                return err.clone();
            }
        }

        warn!("hyper request error: {err:?}");

        Self::HttpProtocolError
    }

    /// Translate a [`hyper::Error`] to a wasi-http [ErrorCode] in the context of a response.
    #[cfg(feature = "default-send-request")]
    pub(crate) fn from_hyper_response_error(err: hyper::Error) -> Self {
        if err.is_timeout() {
            return ErrorCode::HttpResponseTimeout;
        }

        // If there's a source, we might be able to extract a wasi-http error from it.
        if let Some(cause) = err.source() {
            if let Some(err) = cause.downcast_ref::<Self>() {
                return err.clone();
            }
        }

        warn!("hyper response error: {err:?}");

        ErrorCode::HttpProtocolError
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
