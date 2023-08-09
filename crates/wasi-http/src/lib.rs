use crate::component_impl::add_component_to_linker;
pub use crate::http_impl::WasiHttpViewExt;
pub use crate::r#struct::{WasiHttpCtx, WasiHttpView};
use core::fmt::Formatter;
use std::fmt::{self, Display};

wasmtime::component::bindgen!({
    path: "wasi-http/wit",
    world: "proxy",
    with: {
        "wasi:io/streams": wasmtime_wasi::preview2::bindings::io::streams,
        "wasi:poll/poll": wasmtime_wasi::preview2::bindings::poll::poll,
    }
});

pub mod component_impl;
pub mod http_impl;
pub mod r#struct;
pub mod types_impl;

pub fn add_to_component_linker<T>(linker: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView
        + WasiHttpViewExt
        + crate::wasi::http::outgoing_handler::Host
        + crate::wasi::http::types::Host,
{
    crate::wasi::http::outgoing_handler::add_to_linker(linker, |t| t)?;
    crate::wasi::http::types::add_to_linker(linker, |t| t)?;
    Ok(())
}

pub fn add_to_linker<T>(linker: &mut wasmtime::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView
        + WasiHttpViewExt
        + crate::wasi::http::outgoing_handler::Host
        + crate::wasi::http::types::Host
        + wasmtime_wasi::preview2::bindings::io::streams::Host
        + wasmtime_wasi::preview2::bindings::poll::poll::Host,
{
    add_component_to_linker::<T>(linker, |t| t)
}

impl std::error::Error for crate::wasi::http::types::Error {}

impl Display for crate::wasi::http::types::Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            crate::wasi::http::types::Error::InvalidUrl(m) => {
                write!(f, "[InvalidUrl] {}", m)
            }
            crate::wasi::http::types::Error::ProtocolError(m) => {
                write!(f, "[ProtocolError] {}", m)
            }
            crate::wasi::http::types::Error::TimeoutError(m) => {
                write!(f, "[TimeoutError] {}", m)
            }
            crate::wasi::http::types::Error::UnexpectedError(m) => {
                write!(f, "[UnexpectedError] {}", m)
            }
        }
    }
}

impl From<wasmtime_wasi::preview2::TableError> for crate::wasi::http::types::Error {
    fn from(err: wasmtime_wasi::preview2::TableError) -> Self {
        Self::UnexpectedError(err.to_string())
    }
}

impl From<anyhow::Error> for crate::wasi::http::types::Error {
    fn from(err: anyhow::Error) -> Self {
        Self::UnexpectedError(err.to_string())
    }
}

impl From<std::io::Error> for crate::wasi::http::types::Error {
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

impl From<http::Error> for crate::wasi::http::types::Error {
    fn from(err: http::Error) -> Self {
        Self::InvalidUrl(err.to_string())
    }
}

impl From<hyper::Error> for crate::wasi::http::types::Error {
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
        {
            Self::ProtocolError(message)
        } else {
            Self::UnexpectedError(message)
        }
    }
}

impl From<tokio::time::error::Elapsed> for crate::wasi::http::types::Error {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Self::TimeoutError(err.to_string())
    }
}

#[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
impl From<rustls::client::InvalidDnsNameError> for crate::wasi::http::types::Error {
    fn from(_err: rustls::client::InvalidDnsNameError) -> Self {
        Self::InvalidUrl("invalid dnsname".to_string())
    }
}
