//! # Wasmtime's [wasi-runtime-config] Implementation
//!
//! This crate provides a Wasmtime host implementation of the [wasi-runtime-config]
//! API. With this crate, the runtime can run components that call APIs in
//! [wasi-runtime-config] and provide configuration variables for the component.
//!
//! # Examples
//!
//! The usage of this crate is very similar to other WASI API implementations
//! such as [wasi:cli] and [wasi:http].
//!
//! A common scenario is getting runtime-passed configurations in a [wasi:cli]
//! component. A standalone example of doing all this looks like:
//!
//! ```
//! use wasmtime::{
//!     component::{Linker, ResourceTable},
//!     Config, Engine, Result, Store,
//! };
//! use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
//! use wasmtime_wasi_runtime_config::WasiRuntimeConfig;
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
//!     });
//!
//!     let mut linker = Linker::<Ctx>::new(&engine);
//!     wasmtime_wasi::add_to_linker_async(&mut linker)?;
//!     // add `wasi-runtime-config` world's interfaces to the linker
//!     wasmtime_wasi_runtime_config::add_to_linker(&mut linker, |_| {
//!         // These configuration items can come from many places,
//!         // and if necessary, `Ctx` can also be accessed in this closure
//!         WasiRuntimeConfig::from_iter(vec![
//!             ("config_key1", "value1"),
//!             ("config_key2", "value2"),
//!         ])
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
//! }
//!
//! impl WasiView for Ctx {
//!     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
//!     fn ctx(&mut self) -> &mut WasiCtx { &mut self.wasi_ctx }
//! }
//! ```
//!
//! [wasi-runtime-config]: https://github.com/WebAssembly/wasi-runtime-config
//! [wasi:cli]: https://docs.rs/wasmtime-wasi/latest
//! [wasi:http]: https://docs.rs/wasmtime-wasi-http/latest

#![deny(missing_docs)]

use anyhow::Result;
use std::collections::HashMap;

mod gen_ {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "wasi:config/imports",
        trappable_imports: true,
    });
}
use self::gen_::wasi::config::runtime as generated;

/// Capture the state necessary for use in the `wasi-runtime-config` API implementation.
#[derive(Default)]
pub struct WasiRuntimeConfig(HashMap<String, String>);

impl<S: Into<String>> FromIterator<(S, S)> for WasiRuntimeConfig {
    fn from_iter<I: IntoIterator<Item = (S, S)>>(iter: I) -> Self {
        Self(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl WasiRuntimeConfig {
    /// Create a new struct into the `wasi-runtime-config` state.
    pub fn new() -> Self {
        Default::default()
    }

    /// Insert a key-value pair into the state.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.0.insert(key.into(), value.into());
        self
    }
}

impl generated::Host for WasiRuntimeConfig {
    fn get(&mut self, key: String) -> Result<Result<Option<String>, generated::ConfigError>> {
        Ok(Ok(self.0.get(&key).map(|s| s.to_owned())))
    }

    fn get_all(&mut self) -> Result<Result<Vec<(String, String)>, generated::ConfigError>> {
        Ok(Ok(self
            .0
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()))
    }
}

/// Add all the `wasi-runtime-config` world's interfaces to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl Fn(&mut T) -> WasiRuntimeConfig + Send + Sync + Copy + 'static,
) -> Result<()> {
    generated::add_to_linker_get_host(l, f)?;
    Ok(())
}
