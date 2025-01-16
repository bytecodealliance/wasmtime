//! # Wasmtime's [wasi-config] Implementation
//!
//! This crate provides a Wasmtime host implementation of the [wasi-config]
//! API. With this crate, the runtime can run components that call APIs in
//! [wasi-config] and provide configuration variables for the component.
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
//! use wasmtime_wasi::{IoView, WasiCtx, WasiCtxBuilder, WasiView};
//! use wasmtime_wasi_config::{WasiConfig, WasiConfigVariables};
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
//!         wasi_config_vars: WasiConfigVariables::from_iter(vec![
//!             ("config_key1", "value1"),
//!             ("config_key2", "value2"),
//!         ]),
//!     });
//!
//!     let mut linker = Linker::<Ctx>::new(&engine);
//!     wasmtime_wasi::add_to_linker_async(&mut linker)?;
//!     // add `wasi-config` world's interfaces to the linker
//!     wasmtime_wasi_config::add_to_linker(&mut linker, |h: &mut Ctx| {
//!         WasiConfig::from(&h.wasi_config_vars)
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
//!     wasi_config_vars: WasiConfigVariables,
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
//! [wasi-config]: https://github.com/WebAssembly/wasi-config
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
use self::gen_::wasi::config::store as generated;

/// Capture the state necessary for use in the `wasi-config` API implementation.
#[derive(Default)]
pub struct WasiConfigVariables(HashMap<String, String>);

impl<S: Into<String>> FromIterator<(S, S)> for WasiConfigVariables {
    fn from_iter<I: IntoIterator<Item = (S, S)>>(iter: I) -> Self {
        Self(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl WasiConfigVariables {
    /// Create a new runtime configuration.
    pub fn new() -> Self {
        Default::default()
    }

    /// Insert a key-value pair into the configuration map.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.0.insert(key.into(), value.into());
        self
    }
}

/// A wrapper capturing the needed internal `wasi-config` state.
pub struct WasiConfig<'a> {
    vars: &'a WasiConfigVariables,
}

impl<'a> From<&'a WasiConfigVariables> for WasiConfig<'a> {
    fn from(vars: &'a WasiConfigVariables) -> Self {
        Self { vars }
    }
}

impl<'a> WasiConfig<'a> {
    /// Create a new view into the `wasi-config` state.
    pub fn new(vars: &'a WasiConfigVariables) -> Self {
        Self { vars }
    }
}

impl generated::Host for WasiConfig<'_> {
    fn get(&mut self, key: String) -> Result<Result<Option<String>, generated::Error>> {
        Ok(Ok(self.vars.0.get(&key).map(|s| s.to_owned())))
    }

    fn get_all(&mut self) -> Result<Result<Vec<(String, String)>, generated::Error>> {
        Ok(Ok(self
            .vars
            .0
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()))
    }
}

/// Add all the `wasi-config` world's interfaces to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl Fn(&mut T) -> WasiConfig<'_> + Send + Sync + Copy + 'static,
) -> Result<()> {
    generated::add_to_linker_get_host(l, f)?;
    Ok(())
}
