pub use crate::types::{WasiHttpCtx, WasiHttpView};

pub mod body;
pub mod http_impl;
pub mod incoming_handler;
pub mod proxy;
pub mod types;
pub mod types_impl;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        interfaces: "
                import wasi:http/incoming-handler
                import wasi:http/outgoing-handler
                import wasi:http/types
            ",
        tracing: true,
        async: false,
        with: {
            "wasi:io/streams": wasmtime_wasi::preview2::bindings::io::streams,
            "wasi:io/poll": wasmtime_wasi::preview2::bindings::io::poll,
        }
    });

    pub use wasi::http;
}

impl From<wasmtime_wasi::preview2::TableError> for crate::bindings::http::types::Error {
    fn from(err: wasmtime_wasi::preview2::TableError) -> Self {
        Self::UnexpectedError(err.to_string())
    }
}

impl From<anyhow::Error> for crate::bindings::http::types::Error {
    fn from(err: anyhow::Error) -> Self {
        Self::UnexpectedError(err.to_string())
    }
}

impl From<std::io::Error> for crate::bindings::http::types::Error {
    fn from(err: std::io::Error) -> Self {
        let message = err.to_string();
        match err.kind() {
            std::io::ErrorKind::InvalidInput => Self::InvalidUrl(message),
            std::io::ErrorKind::AddrNotAvailable => Self::InvalidUrl(message),
            _ => {
                if message.starts_with("failed to lookup address information") {
                    Self::InvalidUrl("invalid dnsname".to_string())
                } else {
                    Self::ProtocolError(message)
                }
            }
        }
    }
}

impl From<http::Error> for crate::bindings::http::types::Error {
    fn from(err: http::Error) -> Self {
        Self::InvalidUrl(err.to_string())
    }
}

impl From<hyper::Error> for crate::bindings::http::types::Error {
    fn from(err: hyper::Error) -> Self {
        let message = err.message().to_string();
        if err.is_timeout() {
            Self::TimeoutError(message)
        } else if err.is_parse_status() || err.is_user() {
            Self::InvalidUrl(message)
        } else if err.is_body_write_aborted()
            || err.is_canceled()
            || err.is_closed()
            || err.is_incomplete_message()
            || err.is_parse()
        {
            Self::ProtocolError(message)
        } else {
            Self::UnexpectedError(message)
        }
    }
}

impl From<tokio::time::error::Elapsed> for crate::bindings::http::types::Error {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Self::TimeoutError(err.to_string())
    }
}

#[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
impl From<rustls::client::InvalidDnsNameError> for crate::bindings::http::types::Error {
    fn from(_err: rustls::client::InvalidDnsNameError) -> Self {
        Self::InvalidUrl("invalid dnsname".to_string())
    }
}
