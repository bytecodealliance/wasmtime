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
//! use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};
//! use wasmtime::{
//!     component::{Linker, ResourceTable},
//!     Store, Engine, Result, Config
//! };
//! use wasmtime_wasi_tls::{LinkOptions, WasiTls, WasiTlsCtx, WasiTlsCtxBuilder};
//!
//! struct Ctx {
//!     table: ResourceTable,
//!     wasi_ctx: WasiCtx,
//!     wasi_tls_ctx: WasiTlsCtx,
//! }
//!
//! impl IoView for Ctx {
//!     fn table(&mut self) -> &mut ResourceTable {
//!         &mut self.table
//!     }
//! }
//!
//! impl WasiView for Ctx {
//!     fn ctx(&mut self) -> &mut WasiCtx {
//!         &mut self.wasi_ctx
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let ctx = Ctx {
//!         table: ResourceTable::new(),
//!         wasi_ctx: WasiCtxBuilder::new()
//!             .inherit_stderr()
//!             .inherit_network()
//!             .allow_ip_name_lookup(true)
//!             .build(),
//!         wasi_tls_ctx: WasiTlsCtxBuilder::new()
//!             // Optionally, configure a different TLS provider:
//!             // .provider(Box::new(wasmtime_wasi_tls_nativetls::NativeTlsProvider::default()))
//!             .build(),
//!     };
//!
//!     let mut config = Config::new();
//!     config.async_support(true);
//!     let engine = Engine::new(&config)?;
//!
//!     // Set up wasi-cli
//!     let mut store = Store::new(&engine, ctx);
//!     let mut linker = Linker::new(&engine);
//!     wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
//!
//!     // Add wasi-tls types and turn on the feature in linker
//!     let mut opts = LinkOptions::default();
//!     opts.tls(true);
//!     wasmtime_wasi_tls::add_to_linker(&mut linker, &mut opts, |h: &mut Ctx| {
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
use wasmtime::component::{HasData, ResourceTable};

pub mod bindings;
mod host;
mod io;
mod rustls;

pub use bindings::types::LinkOptions;
pub use host::{HostClientConnection, HostClientHandshake, HostFutureClientStreams};
pub use rustls::RustlsProvider;

/// Capture the state necessary for use in the `wasi-tls` API implementation.
pub struct WasiTls<'a> {
    ctx: &'a WasiTlsCtx,
    table: &'a mut ResourceTable,
}

impl<'a> WasiTls<'a> {
    /// Create a new Wasi TLS context
    pub fn new(ctx: &'a WasiTlsCtx, table: &'a mut ResourceTable) -> Self {
        Self { ctx, table }
    }
}

/// Add the `wasi-tls` world's types to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T: Send + 'static>(
    l: &mut wasmtime::component::Linker<T>,
    opts: &mut LinkOptions,
    f: fn(&mut T) -> WasiTls<'_>,
) -> anyhow::Result<()> {
    bindings::types::add_to_linker::<_, HasWasiTls>(l, &opts, f)?;
    Ok(())
}

struct HasWasiTls;
impl HasData for HasWasiTls {
    type Data<'a> = WasiTls<'a>;
}

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
    /// By default, this is set to the [`RustlsProvider`].
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
            provider: Box::new(RustlsProvider::default()),
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
    fn connect(
        &self,
        server_name: String,
        transport: Box<dyn TlsTransport>,
    ) -> BoxFuture<std::io::Result<Box<dyn TlsStream>>>;
}

pub(crate) type BoxFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send>>;
