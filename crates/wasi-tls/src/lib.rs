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
//! use wasmtime_wasi_tls::{LinkOptions, WasiTlsCtx};
//!
//! struct Ctx {
//!     table: ResourceTable,
//!     wasi_ctx: WasiCtx,
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
//!         WasiTlsCtx::new(&mut h.table)
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

use wasmtime::component::{HasData, ResourceTable};

pub mod bindings;
mod host;
mod io;

cfg_if::cfg_if! {
    if #[cfg(feature = "native-tls")] {
        mod client_nativetls;
        pub(crate) use client_nativetls as client;
    } else if #[cfg(feature = "rustls")] {
        mod client_rustls;
        pub(crate) use client_rustls as client;
    } else {
        compile_error!("Either the `rustls` or `native-tls` feature must be enabled.");
    }
}

pub use bindings::types::LinkOptions;
pub use host::{HostClientConnection, HostClientHandshake, HostFutureClientStreams};

/// Wasi TLS context needed for internal `wasi-tls` state
pub struct WasiTlsCtx<'a> {
    table: &'a mut ResourceTable,
}

impl<'a> WasiTlsCtx<'a> {
    /// Create a new Wasi TLS context
    pub fn new(table: &'a mut ResourceTable) -> Self {
        Self { table }
    }
}

/// Add the `wasi-tls` world's types to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T: Send + 'static>(
    l: &mut wasmtime::component::Linker<T>,
    opts: &mut LinkOptions,
    f: fn(&mut T) -> WasiTlsCtx<'_>,
) -> anyhow::Result<()> {
    bindings::types::add_to_linker::<_, WasiTls>(l, &opts, f)?;
    Ok(())
}

struct WasiTls;

impl HasData for WasiTls {
    type Data<'a> = WasiTlsCtx<'a>;
}
