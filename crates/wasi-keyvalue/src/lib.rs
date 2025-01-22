//! # Wasmtime's [wasi-keyvalue] Implementation
//!
//! This crate provides a Wasmtime host implementation of the [wasi-keyvalue]
//! API. With this crate, the runtime can run components that call APIs in
//! [wasi-keyvalue] and provide components with access to key-value storages.
//!
//! Currently supported storage backends:
//! * In-Memory (empty identifier)
//!
//! # Examples
//!
//! The usage of this crate is very similar to other WASI API implementations
//! such as [wasi:cli] and [wasi:http].
//!
//! A common scenario is accessing KV store in a [wasi:cli] component.
//! A standalone example of doing all this looks like:
//!
//! ```
//! use wasmtime::{
//!     component::{Linker, ResourceTable},
//!     Config, Engine, Result, Store,
//! };
//! use wasmtime_wasi::{IoView, WasiCtx, WasiCtxBuilder, WasiView};
//! use wasmtime_wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtx, WasiKeyValueCtxBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut config = Config::new();
//!     config.async_support(true);
//!     let engine = Engine::new(&config)?;
//!
//!     let mut store = Store::new(&engine, Ctx {
//!         table: ResourceTable::new(),
//!         wasi_ctx: WasiCtxBuilder::new().build(),
//!         wasi_keyvalue_ctx: WasiKeyValueCtxBuilder::new().build(),
//!     });
//!
//!     let mut linker = Linker::<Ctx>::new(&engine);
//!     wasmtime_wasi::add_to_linker_async(&mut linker)?;
//!     // add `wasi-keyvalue` world's interfaces to the linker
//!     wasmtime_wasi_keyvalue::add_to_linker(&mut linker, |h: &mut Ctx| {
//!         WasiKeyValue::new(&h.wasi_keyvalue_ctx, &mut h.table)
//!     })?;
//!
//!     // ... use `linker` to instantiate within `store` ...
//!
//!     Ok(())
//! }
//!
//! struct Ctx {
//!     table: ResourceTable,
//!     wasi_ctx: WasiCtx,
//!     wasi_keyvalue_ctx: WasiKeyValueCtx,
//! }
//!
//! impl IoView for Ctx {
//!     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
//! }
//! impl WasiView for Ctx {
//!     fn ctx(&mut self) -> &mut WasiCtx { &mut self.wasi_ctx }
//! }
//! ```
//!
//! [wasi-keyvalue]: https://github.com/WebAssembly/wasi-keyvalue
//! [wasi:cli]: https://docs.rs/wasmtime-wasi/latest
//! [wasi:http]: https://docs.rs/wasmtime-wasi-http/latest

#![deny(missing_docs)]

use std::collections::HashMap;
use wasmtime::component::{ResourceTable, ResourceTableError};

pub mod p2;
#[doc(inline)]
pub use p2::*;

#[doc(hidden)]
pub enum Error {
    NoSuchStore,
    AccessDenied,
    Other(String),
}

impl From<ResourceTableError> for Error {
    fn from(err: ResourceTableError) -> Self {
        Self::Other(err.to_string())
    }
}

#[doc(hidden)]
pub struct Bucket {
    in_memory_data: HashMap<String, Vec<u8>>,
}

/// Builder-style structure used to create a [`WasiKeyValueCtx`].
#[derive(Default)]
pub struct WasiKeyValueCtxBuilder {
    in_memory_data: HashMap<String, Vec<u8>>,
}

impl WasiKeyValueCtxBuilder {
    /// Creates a builder for a new context with default parameters set.
    pub fn new() -> Self {
        Default::default()
    }

    /// Preset data for the In-Memory provider.
    pub fn in_memory_data<I, K, V>(mut self, data: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.in_memory_data = data
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    /// Uses the configured context so far to construct the final [`WasiKeyValueCtx`].
    pub fn build(self) -> WasiKeyValueCtx {
        WasiKeyValueCtx {
            in_memory_data: self.in_memory_data,
        }
    }
}

/// Capture the state necessary for use in the `wasi-keyvalue` API implementation.
pub struct WasiKeyValueCtx {
    in_memory_data: HashMap<String, Vec<u8>>,
}

impl WasiKeyValueCtx {
    /// Convenience function for calling [`WasiKeyValueCtxBuilder::new`].
    pub fn builder() -> WasiKeyValueCtxBuilder {
        WasiKeyValueCtxBuilder::new()
    }
}

/// A wrapper capturing the needed internal `wasi-keyvalue` state.
pub struct WasiKeyValue<'a> {
    ctx: &'a WasiKeyValueCtx,
    table: &'a mut ResourceTable,
}

impl<'a> WasiKeyValue<'a> {
    /// Create a new view into the `wasi-keyvalue` state.
    pub fn new(ctx: &'a WasiKeyValueCtx, table: &'a mut ResourceTable) -> Self {
        Self { ctx, table }
    }
}
