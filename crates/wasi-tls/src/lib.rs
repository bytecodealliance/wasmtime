//! # Wasmtime's [wasi-tls] (Transport Layer Security) Implementation
//!
//! This crate provides the Wasmtime host implementation for the [wasi-tls] API.
//! The [wasi-tls] world allows WebAssembly modules to perform SSL/TLS operations,
//! such as establishing secure connections to servers. TLS often relies on other wasi networking systems
//! to provide the stream so it will be common to enable the [wasi:cli] world as well with the networking features enabled.
//!
//! # An example of how to configure [wasi-tls] is the following:
//!
//! ```rust
//! use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
//! use wasmtime::{
//!     component::{Linker, ResourceTable},
//!     Store, Engine, Result,
//! };
//! use wasmtime_wasi_tls::{WasiTlsCtx, WasiTlsCtxBuilder};
//! use wasmtime_wasi_tls::p2::{LinkOptions, WasiTls};
//!
//! struct Ctx {
//!     table: ResourceTable,
//!     wasi_ctx: WasiCtx,
//!     wasi_tls_ctx: WasiTlsCtx,
//! }
//!
//! impl WasiView for Ctx {
//!     fn ctx(&mut self) -> WasiCtxView<'_> {
//!         WasiCtxView { ctx: &mut self.wasi_ctx, table: &mut self.table }
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let ctx = Ctx {
//!         table: ResourceTable::new(),
//!         wasi_ctx: WasiCtx::builder()
//!             .inherit_stderr()
//!             .inherit_network()
//!             .allow_ip_name_lookup(true)
//!             .build(),
//!         wasi_tls_ctx: WasiTlsCtxBuilder::new()
//!             // Optionally, configure a specific TLS provider:
//!             // .provider(Box::new(wasmtime_wasi_tls::RustlsProvider::default()))
//!             // .provider(Box::new(wasmtime_wasi_tls::NativeTlsProvider::default()))
//!             // .provider(Box::new(wasmtime_wasi_tls::OpenSslProvider::default()))
//!             .build(),
//!     };
//!
//!     let engine = Engine::default();
//!
//!     // Set up wasi-cli
//!     let mut store = Store::new(&engine, ctx);
//!     let mut linker = Linker::new(&engine);
//!     wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
//!
//!     // Add wasi-tls types and turn on the feature in linker
//!     let mut opts = LinkOptions::default();
//!     opts.tls(true);
//!     wasmtime_wasi_tls::p2::add_to_linker(&mut linker, &mut opts, |h: &mut Ctx| {
//!         WasiTls::new(&h.wasi_tls_ctx, &mut h.table)
//!     })?;
//!
//!     // ... use `linker` to instantiate within `store` ...
//!     Ok(())
//! }
//!
//! ```
//! [wasi-tls]: https://github.com/WebAssembly/wasi-tls
//! [wasi:cli]: https://docs.rs/wasmtime-wasi/latest

#![deny(missing_docs)]
#![doc(test(attr(deny(warnings))))]
#![doc(test(attr(allow(dead_code, unused_variables, unused_mut))))]

use tokio::io::{AsyncRead, AsyncWrite};

mod error;
mod providers;

/// WASIp2 (`wasi:tls@0.2.0-draft`) host implementation.
#[cfg(feature = "p2")]
pub mod p2;
/// WASIp3 (`wasi:tls@0.3.0-draft`) host implementation.
#[cfg(feature = "p3")]
pub mod p3;

pub use error::Error;
pub use providers::*;

/// Builder-style structure used to create a [`WasiTlsCtx`].
pub struct WasiTlsCtxBuilder {
    provider: Box<dyn TlsProvider>,
}

impl WasiTlsCtxBuilder {
    /// Creates a builder for a new context with default parameters set.
    pub fn new() -> Self {
        Default::default()
    }

    /// Configure the TLS provider to use for this context.
    ///
    /// By default, this is set to the [`DefaultProvider`] which is picked at
    /// compile time based on feature flags. If this crate is compiled with
    /// multiple TLS providers, this method can be used to specify the provider
    /// at runtime.
    pub fn provider(mut self, provider: Box<dyn TlsProvider>) -> Self {
        self.provider = provider;
        self
    }

    /// Uses the configured context so far to construct the final [`WasiTlsCtx`].
    pub fn build(self) -> WasiTlsCtx {
        WasiTlsCtx {
            provider: self.provider,
        }
    }
}
impl Default for WasiTlsCtxBuilder {
    fn default() -> Self {
        Self {
            provider: Box::new(DefaultProvider::default()),
        }
    }
}

/// Wasi TLS context needed for internal `wasi-tls` state.
pub struct WasiTlsCtx {
    pub(crate) provider: Box<dyn TlsProvider>,
}

/// The data stream that carries the encrypted TLS data.
/// Typically this is a TCP stream.
pub trait TlsTransport: AsyncRead + AsyncWrite + Send + Unpin + 'static {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin + ?Sized + 'static> TlsTransport for T {}

/// A TLS connection.
pub trait TlsStream: AsyncRead + AsyncWrite + Send + Unpin + 'static {}

/// A TLS implementation.
pub trait TlsProvider: Send + Sync + 'static {
    /// Set up a client TLS connection using the provided `server_name` and `transport`.
    fn connect(&self, server_name: String, transport: Box<dyn TlsTransport>) -> BoxFutureTlStream;
}

pub(crate) type BoxFutureTlStream =
    std::pin::Pin<Box<dyn Future<Output = Result<Box<dyn TlsStream>, Error>> + Send>>;
