//! Experimental, unstable and incomplete implementation of wasip3 version of `wasi:http`.
//!
//! This module is under heavy development.
//! It is not compliant with semver and is not ready
//! for production use.
//!
//! Bug and security fixes limited to wasip3 will not be given patch releases.
//!
//! Documentation of this module may be incorrect or out-of-sync with the implementation.

pub mod bindings;
mod conv;
#[expect(unused, reason = "work in progress")] // TODO: implement
mod host;
mod request;
mod response;

pub use request::{Request, RequestOptions};
pub use response::Response;

use bindings::http::{handler, types};
use core::ops::Deref;
use std::sync::Arc;
use wasmtime::component::{HasData, Linker, ResourceTable};

pub(crate) struct WasiHttp;

impl HasData for WasiHttp {
    type Data<'a> = WasiHttpCtxView<'a>;
}

#[derive(Clone, Default)]
pub struct WasiHttpCtx {}

pub struct WasiHttpCtxView<'a> {
    pub ctx: &'a mut WasiHttpCtx,
    pub table: &'a mut ResourceTable,
}

pub trait WasiHttpView: Send {
    fn http(&mut self) -> WasiHttpCtxView<'_>;
}

/// Add all interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:http/imports` world supported by
/// this module.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime_wasi_http::p3::{WasiHttpCtx, WasiHttpCtxView, WasiHttpView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_http::p3::add_to_linker(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut store = Store::new(
///         &engine,
///         MyState::default(),
///     );
///
///     // ... use `linker` to instantiate within `store` ...
///
///     Ok(())
/// }
///
/// #[derive(Default)]
/// struct MyState {
///     http: WasiHttpCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiHttpView for MyState {
///     fn http(&mut self) -> WasiHttpCtxView<'_> {
///         WasiHttpCtxView {
///             ctx: &mut self.http,
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiHttpView + 'static,
{
    handler::add_to_linker::<_, WasiHttp>(linker, T::http)?;
    types::add_to_linker::<_, WasiHttp>(linker, T::http)?;
    Ok(())
}

pub enum MaybeMutable<T> {
    Mutable(Arc<T>),
    Immutable(Arc<T>),
}

impl<T> From<MaybeMutable<T>> for Arc<T> {
    fn from(v: MaybeMutable<T>) -> Self {
        v.into_arc()
    }
}

impl<T> Deref for MaybeMutable<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        self.as_arc()
    }
}

impl<T> MaybeMutable<T> {
    pub fn new_mutable(v: impl Into<Arc<T>>) -> Self {
        Self::Mutable(v.into())
    }

    pub fn new_immutable(v: impl Into<Arc<T>>) -> Self {
        Self::Immutable(v.into())
    }

    fn as_arc(&self) -> &Arc<T> {
        match self {
            Self::Mutable(v) | Self::Immutable(v) => v,
        }
    }

    fn into_arc(self) -> Arc<T> {
        match self {
            Self::Mutable(v) | Self::Immutable(v) => v,
        }
    }

    pub fn get(&self) -> &T {
        self
    }

    pub fn get_mut(&mut self) -> Option<&mut T>
    where
        T: Clone,
    {
        match self {
            Self::Mutable(v) => Some(Arc::make_mut(v)),
            Self::Immutable(..) => None,
        }
    }
}
