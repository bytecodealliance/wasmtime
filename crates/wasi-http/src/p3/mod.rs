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
mod host;
mod proxy;
mod request;
mod response;

#[cfg(feature = "default-send-request")]
pub use request::default_send_request;
pub use request::{Request, RequestOptions};
pub use response::Response;

use crate::p3::bindings::http::types::ErrorCode;
use crate::types::DEFAULT_FORBIDDEN_HEADERS;
use bindings::http::{client, types};
use bytes::Bytes;
use core::ops::Deref;
use http::HeaderName;
use http::uri::Scheme;
use http_body_util::combinators::UnsyncBoxBody;
use std::sync::Arc;
use wasmtime::component::{HasData, Linker, ResourceTable};
use wasmtime_wasi::TrappableError;

pub(crate) type HttpResult<T> = Result<T, HttpError>;
pub(crate) type HttpError = TrappableError<types::ErrorCode>;

pub(crate) type HeaderResult<T> = Result<T, HeaderError>;
pub(crate) type HeaderError = TrappableError<types::HeaderError>;

pub(crate) type RequestOptionsResult<T> = Result<T, RequestOptionsError>;
pub(crate) type RequestOptionsError = TrappableError<types::RequestOptionsError>;

/// The type for which this crate implements the `wasi:http` interfaces.
pub struct WasiHttp;

impl HasData for WasiHttp {
    type Data<'a> = WasiHttpCtxView<'a>;
}

/// A trait which provides internal WASI HTTP state.
pub trait WasiHttpCtx: Send {
    /// Whether a given header should be considered forbidden and not allowed.
    fn is_forbidden_header(&mut self, name: &HeaderName) -> bool {
        DEFAULT_FORBIDDEN_HEADERS.contains(name)
    }

    /// Whether a given scheme should be considered supported.
    ///
    /// `handle` will return [ErrorCode::HttpProtocolError] for unsupported schemes.
    fn is_supported_scheme(&mut self, scheme: &Scheme) -> bool {
        *scheme == Scheme::HTTP || *scheme == Scheme::HTTPS
    }

    /// Whether to set `host` header in the request passed to `send_request`.
    fn set_host_header(&mut self) -> bool {
        true
    }

    /// Scheme to default to, when not set by the guest.
    ///
    /// If [None], `handle` will return [ErrorCode::HttpProtocolError]
    /// for requests missing a scheme.
    fn default_scheme(&mut self) -> Option<Scheme> {
        Some(Scheme::HTTPS)
    }

    /// Send an outgoing request.
    ///
    /// This function will be used by the `wasi:http/handler#handle` implementation.
    ///
    /// The specified [Future] `fut` will be used to communicate
    /// a response processing error, if any.
    /// For example, if the response body is consumed via `wasi:http/types.response#consume-body`,
    /// a result will be sent on `fut`.
    ///
    /// The returned [Future] can be used to communicate
    /// a request processing error, if any, to the constructor of the request.
    /// For example, if the request was constructed via `wasi:http/types.request#new`,
    /// a result resolved from it will be forwarded to the guest on the future handle returned.
    ///
    /// `Content-Length` of the request passed to this function will be validated, however no
    /// `Content-Length` validation will be performed for the received response.
    #[cfg(feature = "default-send-request")]
    fn send_request(
        &mut self,
        request: http::Request<UnsyncBoxBody<Bytes, ErrorCode>>,
        options: Option<RequestOptions>,
        fut: Box<dyn Future<Output = Result<(), ErrorCode>> + Send>,
    ) -> Box<
        dyn Future<
                Output = HttpResult<(
                    http::Response<UnsyncBoxBody<Bytes, ErrorCode>>,
                    Box<dyn Future<Output = Result<(), ErrorCode>> + Send>,
                )>,
            > + Send,
    > {
        _ = fut;
        Box::new(async move {
            use http_body_util::BodyExt;

            let (res, io) = default_send_request(request, options).await?;
            Ok((
                res.map(BodyExt::boxed_unsync),
                Box::new(io) as Box<dyn Future<Output = _> + Send>,
            ))
        })
    }

    /// Send an outgoing request.
    ///
    /// This function will be used by the `wasi:http/handler#handle` implementation.
    ///
    /// The specified [Future] `fut` will be used to communicate
    /// a response processing error, if any.
    /// For example, if the response body is consumed via `wasi:http/types.response#consume-body`,
    /// a result will be sent on `fut`.
    ///
    /// The returned [Future] can be used to communicate
    /// a request processing error, if any, to the constructor of the request.
    /// For example, if the request was constructed via `wasi:http/types.request#new`,
    /// a result resolved from it will be forwarded to the guest on the future handle returned.
    ///
    /// `Content-Length` of the request passed to this function will be validated, however no
    /// `Content-Length` validation will be performed for the received response.
    #[cfg(not(feature = "default-send-request"))]
    fn send_request(
        &mut self,
        request: http::Request<UnsyncBoxBody<Bytes, ErrorCode>>,
        options: Option<RequestOptions>,
        fut: Box<dyn Future<Output = Result<(), ErrorCode>> + Send>,
    ) -> Box<
        dyn Future<
                Output = HttpResult<(
                    http::Response<UnsyncBoxBody<Bytes, ErrorCode>>,
                    Box<dyn Future<Output = Result<(), ErrorCode>> + Send>,
                )>,
            > + Send,
    >;
}

/// Default implementation of [WasiHttpCtx].
#[cfg(feature = "default-send-request")]
#[derive(Clone, Default)]
pub struct DefaultWasiHttpCtx;

#[cfg(feature = "default-send-request")]
impl WasiHttpCtx for DefaultWasiHttpCtx {}

/// View into [WasiHttpCtx] implementation and [ResourceTable].
pub struct WasiHttpCtxView<'a> {
    /// Mutable reference to the WASI HTTP context.
    pub ctx: &'a mut dyn WasiHttpCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

/// A trait which provides internal WASI HTTP state.
pub trait WasiHttpView: Send {
    /// Return a [WasiHttpCtxView] from mutable reference to self.
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
/// use wasmtime_wasi_http::p3::{DefaultWasiHttpCtx, WasiHttpCtxView, WasiHttpView};
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
///     http: DefaultWasiHttpCtx,
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
