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
//!     Engine, Result, Store,
//! };
//! use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
//! use wasmtime_wasi_config::{WasiConfig, WasiConfigVariables};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let engine = Engine::default();
//!
//!     let mut store = Store::new(&engine, Ctx {
//!         table: ResourceTable::new(),
//!         wasi_ctx: WasiCtx::builder().build(),
//!         wasi_config_vars: WasiConfigVariables::from_iter(vec![
//!             ("config_key1", "value1"),
//!             ("config_key2", "value2"),
//!         ]),
//!     });
//!
//!     let mut linker = Linker::<Ctx>::new(&engine);
//!     wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
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
//! impl WasiView for Ctx {
//!     fn ctx(&mut self) -> WasiCtxView<'_> {
//!         WasiCtxView { ctx: &mut self.wasi_ctx, table: &mut self.table }
//!     }
//! }
//! ```
//!
//! [wasi-config]: https://github.com/WebAssembly/wasi-config
//! [wasi:cli]: https://docs.rs/wasmtime-wasi/latest
//! [wasi:http]: https://docs.rs/wasmtime-wasi-http/latest

#![deny(missing_docs)]

use std::collections::HashMap;
use std::marker;
use wasmtime::Result;
use wasmtime::component::{Component, HasData, Linker};
use wasmtime_wasi::{NamedId, WasiCtxNamedView};

mod gen_ {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "wasi:config/imports",
        imports: { default: trappable },
        named_imports: {
            "wasi:config/store": wasmtime_wasi::NamedId,
        },
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
pub fn add_to_linker<T: 'static>(
    l: &mut wasmtime::component::Linker<T>,
    f: fn(&mut T) -> WasiConfig<'_>,
) -> Result<()> {
    generated::add_to_linker::<T, HasWasiConfig>(l, f)?;
    Ok(())
}

struct HasWasiConfig;

impl HasData for HasWasiConfig {
    type Data<'a> = WasiConfig<'a>;
}

/// A trait used to look up a specific `wasi-config` context for a named
/// import.
///
/// This is used by [`add_named_to_linker`] to resolve a [`NamedId`], allocated
/// by the embedder, to the [`WasiConfig`] that a particular named import
/// reads from.
///
/// # Examples
///
/// ```no_run
/// use std::collections::HashMap;
/// use wasmtime::component::{Component, Linker};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::NamedId;
/// use wasmtime_wasi_config::{WasiConfig, WasiConfigNamedView, WasiConfigVariables};
///
/// struct MyState {
///     configs: Vec<WasiConfigVariables>,
/// }
///
/// impl WasiConfigNamedView for MyState {
///     fn config(&mut self, id: NamedId) -> WasiConfig<'_> {
///         WasiConfig::new(&self.configs[id.0])
///     }
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let component = Component::from_file(&engine, "my-component.wasm")?;
///     let mut linker = Linker::<MyState>::new(&engine);
///
///     let mut names = HashMap::new();
///     wasmtime_wasi_config::add_named_to_linker(&mut linker, &component, |_, name| {
///         let len = names.len();
///         Ok(NamedId(*names.entry(name.to_string()).or_insert(len)))
///     })?;
///
///     // ... create `WasiConfigVariables` per entry in `names` and instantiate ...
///
///     Ok(())
/// }
/// ```
pub trait WasiConfigNamedView: 'static {
    /// Returns the [`WasiConfig`] to use for the named import identified by
    /// `id`.
    fn config(&mut self, id: NamedId) -> WasiConfig<'_>;
}

struct WasiConfigNamed<T>(marker::PhantomData<fn() -> T>);

impl<T: WasiConfigNamedView> HasData for WasiConfigNamed<T> {
    type Data<'a> = WasiCtxNamedView<'a, T>;
}

/// Interfaces that are added via [`add_named_to_linker`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Interface {
    /// `wasi:config/store`
    ConfigStore,
}

/// Add the `wasi-config` world's interfaces to `linker` for any named imports
/// of them that `component` has.
///
/// This is the named-import counterpart of [`add_to_linker`]: `component`'s
/// imports are inspected and, for each import that implements a
/// `wasi:config` interface under a different name, `lookup` is invoked with
/// the [`Interface`] being bound and the name of the import. The [`NamedId`]
/// returned is later passed to [`WasiConfigNamedView::config`] when that
/// import is used. If `lookup` returns an error then this function returns
/// that error.
pub fn add_named_to_linker<T>(
    linker: &mut Linker<T>,
    component: &Component,
    mut lookup: impl FnMut(Interface, &str) -> Result<NamedId>,
) -> Result<()>
where
    T: WasiConfigNamedView,
{
    gen_::named_imports::wasi::config::store::add_to_linker::<_, WasiConfigNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::ConfigStore, name),
        |x| WasiCtxNamedView(x),
    )
}

impl<T: WasiConfigNamedView> gen_::named_imports::wasi::config::store::Host
    for WasiCtxNamedView<'_, T>
{
    fn get(
        &mut self,
        id: NamedId,
        key: String,
    ) -> Result<Result<Option<String>, generated::Error>> {
        generated::Host::get(&mut self.0.config(id), key)
    }

    fn get_all(&mut self, id: NamedId) -> Result<Result<Vec<(String, String)>, generated::Error>> {
        generated::Host::get_all(&mut self.0.config(id))
    }
}
