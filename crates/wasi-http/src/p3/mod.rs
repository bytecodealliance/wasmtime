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
mod body;
mod conv;
mod helpers;
mod host;
mod proxy;
mod request;
mod response;

pub use request::Request;
pub use response::Response;

/// The default value configured for [`WasiHttpHooks::p3_outgoing_body_chunk_size`].
///
/// [`WasiHttpHooks::p3_outgoing_body_chunk_size`]: crate::WasiHttpHooks::p3_outgoing_body_chunk_size
pub const DEFAULT_OUTGOING_BODY_CHUNK_SIZE: usize = 1024 * 1024;

use crate::{FieldMapError, WasiHttp, WasiHttpNamed, WasiHttpNamedView, WasiHttpView};
use bindings::http::{client, types};
use core::ops::Deref;
use std::sync::Arc;
use wasmtime::component::{Component, Linker};
use wasmtime_wasi::{NamedId, TrappableError, WasiCtxNamedView};

pub(crate) type HttpResult<T> = Result<T, HttpError>;
pub(crate) type HttpError = TrappableError<types::ErrorCode>;

pub(crate) type HeaderResult<T> = Result<T, HeaderError>;
pub(crate) type HeaderError = TrappableError<types::HeaderError>;

impl From<FieldMapError> for HeaderError {
    fn from(e: FieldMapError) -> Self {
        match e {
            FieldMapError::Immutable => types::HeaderError::Immutable.into(),
            FieldMapError::InvalidHeaderName | FieldMapError::InvalidHeaderValue => {
                types::HeaderError::InvalidSyntax.into()
            }
            FieldMapError::TooManyFields | FieldMapError::TotalSizeTooBig => {
                types::HeaderError::SizeExceeded.into()
            }
            FieldMapError::Forbidden => types::HeaderError::Forbidden.into(),
        }
    }
}

pub(crate) type RequestOptionsResult<T> = Result<T, RequestOptionsError>;
pub(crate) type RequestOptionsError = TrappableError<types::RequestOptionsError>;

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
/// use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpCtxView, WasiHttpView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
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
///             hooks: Default::default(),
///         }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiHttpView + 'static,
{
    client::add_to_linker::<_, WasiHttp>(linker, T::http)?;
    types::add_to_linker::<_, WasiHttp>(linker, T::http)?;
    Ok(())
}

/// Interfaces that are added via [`add_named_to_linker`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Interface {
    /// `wasi:http/client`
    HttpClient,
    /// `wasi:http/types`
    HttpTypes,
}

/// Add all interfaces from this module into the `linker` provided for any
/// named imports that a component has.
///
/// This function is similar to [`add_to_linker`] except that it's specifically
/// designed to work with named imports of `wasi:http` interfaces that
/// components may have. This requires a [`Component`] parameter to be passed in
/// when populating the [`Linker`] provided to see what the [`Component`]
/// actually imports.
///
/// If this isn't low level enough you can invoke the bindgen-generated
/// `add_to_linker` functions within the [`named_imports`] module directly
/// instead.
///
/// [`named_imports`]: crate::p3::bindings::named_imports
///
/// The `lookup` function provided here is invoked for every named import found
/// for a particular interface. The [`Interface`] given is what's being bound,
/// and the `&str` argument is the name that the component imports it as. The
/// embedder can then decide how it would like to allocate a [`NamedId`] for
/// this import. If `Ok` is returned then the linker is populated with this
/// name, and imported functions will pass the [`NamedId`] later to the
/// implementation of [`WasiHttpNamedView`] on `T` when invoked. If `Err` is
/// returned then the error will cause this entire function to fail and this
/// function call will return the same error.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use wasmtime::component::{Component, Linker, ResourceTable};
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime_wasi::NamedId;
/// use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpCtxView, WasiHttpNamedView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///     let component = Component::new(&engine, "(component)")?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///
///     // ... add default functionality to `linker` as needed ...
///
///     // and then additionally fill in any specific named imports `component`
///     // might have for `wasi:http` interfaces.
///     let mut name_map = HashMap::new();
///     wasmtime_wasi_http::p3::add_named_to_linker(&mut linker, &component, |_i, name| {
///         let len = name_map.len();
///         Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///     })?;
///
///     // Here a `WasiHttpCtx` is allocated per-named-import and will then be
///     // referred to internally by the [`NamedId`] allocated above. You could
///     // also use `name_map` to configure each context differently.
///     let mut my_state = MyState::default();
///     for _ in 0..name_map.len() {
///         my_state.contexts.push(WasiHttpCtx::default());
///     }
///     let mut store = Store::new(&engine, my_state);
///
///     // ... use `linker` to instantiate within `store` ...
///
///     Ok(())
/// }
///
/// #[derive(Default)]
/// struct MyState {
///     table: ResourceTable,
///     contexts: Vec<WasiHttpCtx>,
/// }
///
/// impl WasiHttpNamedView for MyState {
///     fn http(&mut self, id: NamedId) -> WasiHttpCtxView<'_> {
///         WasiHttpCtxView {
///             ctx: &mut self.contexts[id.0],
///             table: &mut self.table,
///             hooks: Default::default(),
///         }
///     }
/// }
/// ```
pub fn add_named_to_linker<T>(
    linker: &mut Linker<T>,
    component: &Component,
    mut lookup: impl FnMut(Interface, &str) -> wasmtime::Result<NamedId>,
) -> wasmtime::Result<()>
where
    T: WasiHttpNamedView,
{
    use crate::p3::bindings::named_imports::wasi::http::{client, types};
    client::add_to_linker::<_, WasiHttpNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::HttpClient, name),
        |x| WasiCtxNamedView(x),
    )?;
    types::add_to_linker::<_, WasiHttpNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::HttpTypes, name),
        |x| WasiCtxNamedView(x),
    )?;
    Ok(())
}

/// An [Arc], which may be immutable.
///
/// In `wasi:http` resources like `fields` or `request-options` may be
/// mutable or immutable. This construct is used to model them efficiently.
pub enum MaybeMutable<T> {
    /// Clone-on-write, mutable [Arc]
    Mutable(Arc<T>),
    /// Immutable [Arc]
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
        match self {
            Self::Mutable(v) | Self::Immutable(v) => v,
        }
    }
}

impl<T> MaybeMutable<T> {
    /// Construct a mutable [`MaybeMutable`].
    pub fn new_mutable(v: impl Into<Arc<T>>) -> Self {
        Self::Mutable(v.into())
    }

    /// Construct a mutable [`MaybeMutable`] filling it with default `T`.
    pub fn new_mutable_default() -> Self
    where
        T: Default,
    {
        Self::new_mutable(T::default())
    }

    /// Construct an immutable [`MaybeMutable`].
    pub fn new_immutable(v: impl Into<Arc<T>>) -> Self {
        Self::Immutable(v.into())
    }

    /// Unwrap [`MaybeMutable`] into [`Arc`].
    pub fn into_arc(self) -> Arc<T> {
        match self {
            Self::Mutable(v) | Self::Immutable(v) => v,
        }
    }

    /// If this [`MaybeMutable`] is [`Mutable`](MaybeMutable::Mutable),
    /// return a mutable reference to it, otherwise return `None`.
    ///
    /// Internally, this will use [`Arc::make_mut`] and will clone the underlying
    /// value, if multiple strong references to the inner [`Arc`] exist.
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
