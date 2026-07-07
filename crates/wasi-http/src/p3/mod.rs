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

use crate::{FieldMapError, WasiHttp, WasiHttpView};
use bindings::http::{client, types};
use core::ops::Deref;
use std::sync::Arc;
use wasmtime::component::Linker;
use wasmtime_wasi::TrappableError;

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
