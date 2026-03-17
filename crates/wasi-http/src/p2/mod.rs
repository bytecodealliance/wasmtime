//! # Wasmtime's WASI HTTPp2 Implementation
//!
//! This module is Wasmtime's host implementation of the `wasi:http` package as
//! part of WASIp2. This crate's implementation is primarily built on top of
//! [`hyper`] and [`tokio`].
//!
//! # WASI HTTP Interfaces
//!
//! This crate contains implementations of the following interfaces:
//!
//! * [`wasi:http/incoming-handler`]
//! * [`wasi:http/outgoing-handler`]
//! * [`wasi:http/types`]
//!
//! The crate also contains an implementation of the [`wasi:http/proxy`] world.
//!
//! [`wasi:http/proxy`]: crate::p2::bindings::Proxy
//! [`wasi:http/outgoing-handler`]: crate::p2::bindings::http::outgoing_handler::Host
//! [`wasi:http/types`]: crate::p2::bindings::http::types::Host
//! [`wasi:http/incoming-handler`]: crate::p2::bindings::exports::wasi::http::incoming_handler::Guest
//!
//! This crate is very similar to [`wasmtime_wasi`] in the it uses the
//! `bindgen!` macro in Wasmtime to generate bindings to interfaces. Bindings
//! are located in the [`bindings`] module.
//!
//! # The `WasiHttp{View,Hooks}` traits
//!
//! All `bindgen!`-generated `Host` traits are implemented for the
//! [`WasiHttpCtxView`] type. This type is created from a store's data `T`
//! through the [`WasiHttpView`] trait. The [`add_to_linker_async`] function,
//! for example, uses [`WasiHttpView`] to acquire the context view.
//!
//! The [`WasiHttpCtxView`] structure requires that a [`ResourceTable`] and
//! [`WasiHttpCtx`] live within the store. This is store-specific state that is
//! used to implement various APIs and store host state.
//!
//! The final `hooks` field within [`WasiHttpCtxView`] is a trait object of
//! [`WasiHttpHooks`]. This provides a few more hooks, dynamically, to configure
//! how `wasi:http` behaves. For example [`WasiHttpHooks::send_request`] can
//! customize how outgoing HTTP requests are handled. The `hooks` field can be
//! initialized with the [`default_hooks`] function for the default behavior.
//!
//! # Async and Sync
//!
//! There are both asynchronous and synchronous bindings in this crate. For
//! example [`add_to_linker_async`] is for asynchronous embedders and
//! [`add_to_linker_sync`] is for synchronous embedders. Note that under the
//! hood both versions are implemented with `async` on top of [`tokio`].
//!
//! # Examples
//!
//! Usage of this crate is done through a few steps to get everything hooked up:
//!
//! 1. First implement [`WasiHttpView`] for your type which is the `T` in
//!    [`wasmtime::Store<T>`].
//! 2. Add WASI HTTP interfaces to a [`wasmtime::component::Linker<T>`]. There
//!    are a few options of how to do this:
//!    * Use [`add_to_linker_async`] to bundle all interfaces in
//!      `wasi:http/proxy` together
//!    * Use [`add_only_http_to_linker_async`] to add only HTTP interfaces but
//!      no others. This is useful when working with
//!      [`wasmtime_wasi::p2::add_to_linker_async`] for example.
//!    * Add individual interfaces such as with the
//!      [`bindings::http::outgoing_handler::add_to_linker`] function.
//! 3. Use [`ProxyPre`](bindings::ProxyPre) to pre-instantiate a component
//!    before serving requests.
//! 4. When serving requests use
//!    [`ProxyPre::instantiate_async`](bindings::ProxyPre::instantiate_async)
//!    to create instances and handle HTTP requests.
//!
//! A standalone example of doing all this looks like:
//!
//! ```no_run
//! use wasmtime::bail;
//! use hyper::server::conn::http1;
//! use std::sync::Arc;
//! use tokio::net::TcpListener;
//! use wasmtime::component::{Component, Linker, ResourceTable};
//! use wasmtime::{Engine, Result, Store};
//! use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
//! use wasmtime_wasi_http::p2::bindings::ProxyPre;
//! use wasmtime_wasi_http::p2::bindings::http::types::Scheme;
//! use wasmtime_wasi_http::p2::body::HyperOutgoingBody;
//! use wasmtime_wasi_http::io::TokioIo;
//! use wasmtime_wasi_http::{WasiHttpCtx, p2::{WasiHttpView, WasiHttpCtxView}};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let component = std::env::args().nth(1).unwrap();
//!
//!     // Prepare the `Engine` for Wasmtime
//!     let engine = Engine::default();
//!
//!     // Compile the component on the command line to machine code
//!     let component = Component::from_file(&engine, &component)?;
//!
//!     // Prepare the `ProxyPre` which is a pre-instantiated version of the
//!     // component that we have. This will make per-request instantiation
//!     // much quicker.
//!     let mut linker = Linker::new(&engine);
//!     wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
//!     wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker)?;
//!     let pre = ProxyPre::new(linker.instantiate_pre(&component)?)?;
//!
//!     // Prepare our server state and start listening for connections.
//!     let server = Arc::new(MyServer { pre });
//!     let listener = TcpListener::bind("127.0.0.1:8000").await?;
//!     println!("Listening on {}", listener.local_addr()?);
//!
//!     loop {
//!         // Accept a TCP connection and serve all of its requests in a separate
//!         // tokio task. Note that for now this only works with HTTP/1.1.
//!         let (client, addr) = listener.accept().await?;
//!         println!("serving new client from {addr}");
//!
//!         let server = server.clone();
//!         tokio::task::spawn(async move {
//!             if let Err(e) = http1::Builder::new()
//!                 .keep_alive(true)
//!                 .serve_connection(
//!                     TokioIo::new(client),
//!                     hyper::service::service_fn(move |req| {
//!                         let server = server.clone();
//!                         async move { server.handle_request(req).await }
//!                     }),
//!                 )
//!                 .await
//!             {
//!                 eprintln!("error serving client[{addr}]: {e:?}");
//!             }
//!         });
//!     }
//! }
//!
//! struct MyServer {
//!     pre: ProxyPre<MyClientState>,
//! }
//!
//! impl MyServer {
//!     async fn handle_request(
//!         &self,
//!         req: hyper::Request<hyper::body::Incoming>,
//!     ) -> Result<hyper::Response<HyperOutgoingBody>> {
//!         // Create per-http-request state within a `Store` and prepare the
//!         // initial resources  passed to the `handle` function.
//!         let mut store = Store::new(
//!             self.pre.engine(),
//!             MyClientState {
//!                 table: ResourceTable::new(),
//!                 wasi: WasiCtx::builder().inherit_stdio().build(),
//!                 http: WasiHttpCtx::new(),
//!             },
//!         );
//!         let (sender, receiver) = tokio::sync::oneshot::channel();
//!         let req = store.data_mut().http().new_incoming_request(Scheme::Http, req)?;
//!         let out = store.data_mut().http().new_response_outparam(sender)?;
//!         let pre = self.pre.clone();
//!
//!         // Run the http request itself in a separate task so the task can
//!         // optionally continue to execute beyond after the initial
//!         // headers/response code are sent.
//!         let task = tokio::task::spawn(async move {
//!             let proxy = pre.instantiate_async(&mut store).await?;
//!
//!             if let Err(e) = proxy
//!                 .wasi_http_incoming_handler()
//!                 .call_handle(store, req, out)
//!                 .await
//!             {
//!                 return Err(e);
//!             }
//!
//!             Ok(())
//!         });
//!
//!         match receiver.await {
//!             // If the client calls `response-outparam::set` then one of these
//!             // methods will be called.
//!             Ok(Ok(resp)) => Ok(resp),
//!             Ok(Err(e)) => Err(e.into()),
//!
//!             // Otherwise the `sender` will get dropped along with the `Store`
//!             // meaning that the oneshot will get disconnected and here we can
//!             // inspect the `task` result to see what happened
//!             Err(_) => {
//!                 let e = match task.await {
//!                     Ok(Ok(())) => {
//!                         bail!("guest never invoked `response-outparam::set` method")
//!                     }
//!                     Ok(Err(e)) => e,
//!                     Err(e) => e.into(),
//!                 };
//!                 return Err(e.context("guest never invoked `response-outparam::set` method"));
//!             }
//!         }
//!     }
//! }
//!
//! struct MyClientState {
//!     wasi: WasiCtx,
//!     http: WasiHttpCtx,
//!     table: ResourceTable,
//! }
//!
//! impl WasiView for MyClientState {
//!     fn ctx(&mut self) -> WasiCtxView<'_> {
//!         WasiCtxView { ctx: &mut self.wasi, table: &mut self.table }
//!     }
//! }
//!
//! impl WasiHttpView for MyClientState {
//!     fn http(&mut self) -> WasiHttpCtxView<'_> {
//!         WasiHttpCtxView {
//!             ctx: &mut self.http,
//!             table: &mut self.table,
//!             hooks: Default::default(),
//!         }
//!     }
//! }
//! ```

#[cfg(feature = "default-send-request")]
use self::bindings::http::types::ErrorCode;
use crate::{DEFAULT_FORBIDDEN_HEADERS, WasiHttpCtx};
use http::HeaderName;
use wasmtime::component::{HasData, Linker, ResourceTable};

mod error;
mod http_impl;
mod types_impl;

pub mod bindings;
pub mod body;
pub mod types;

pub use self::error::*;

/// A trait which provides hooks into internal WASI HTTP operations.
///
/// # Example
///
/// ```
/// use wasmtime::component::ResourceTable;
/// use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
/// use wasmtime_wasi_http::WasiHttpCtx;
/// use wasmtime_wasi_http::p2::{WasiHttpView, WasiHttpCtxView};
///
/// struct MyState {
///     ctx: WasiCtx,
///     http_ctx: WasiHttpCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiHttpView for MyState {
///     fn http(&mut self) -> WasiHttpCtxView<'_> {
///         WasiHttpCtxView {
///             ctx: &mut self.http_ctx,
///             table: &mut self.table,
///             hooks: Default::default(),
///         }
///     }
/// }
///
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> WasiCtxView<'_> {
///         WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
///     }
/// }
///
/// impl MyState {
///     fn new() -> MyState {
///         let mut wasi = WasiCtx::builder();
///         wasi.arg("./foo.wasm");
///         wasi.arg("--help");
///         wasi.env("FOO", "bar");
///
///         MyState {
///             ctx: wasi.build(),
///             table: ResourceTable::new(),
///             http_ctx: WasiHttpCtx::new(),
///         }
///     }
/// }
/// ```
pub trait WasiHttpHooks: Send {
    /// Send an outgoing request.
    #[cfg(feature = "default-send-request")]
    fn send_request(
        &mut self,
        request: hyper::Request<body::HyperOutgoingBody>,
        config: types::OutgoingRequestConfig,
    ) -> HttpResult<types::HostFutureIncomingResponse> {
        Ok(default_send_request(request, config))
    }

    /// Send an outgoing request.
    #[cfg(not(feature = "default-send-request"))]
    fn send_request(
        &mut self,
        request: hyper::Request<body::HyperOutgoingBody>,
        config: types::OutgoingRequestConfig,
    ) -> HttpResult<types::HostFutureIncomingResponse>;

    /// Whether a given header should be considered forbidden and not allowed.
    fn is_forbidden_header(&mut self, name: &HeaderName) -> bool {
        DEFAULT_FORBIDDEN_HEADERS.contains(name)
    }

    /// Number of distinct write calls to the outgoing body's output-stream
    /// that the implementation will buffer.
    /// Default: 1.
    fn outgoing_body_buffer_chunks(&mut self) -> usize {
        DEFAULT_OUTGOING_BODY_BUFFER_CHUNKS
    }

    /// Maximum size allowed in a write call to the outgoing body's output-stream.
    /// Default: 1024 * 1024.
    fn outgoing_body_chunk_size(&mut self) -> usize {
        DEFAULT_OUTGOING_BODY_CHUNK_SIZE
    }
}

#[cfg(feature = "default-send-request")]
impl<'a> Default for &'a mut dyn WasiHttpHooks {
    fn default() -> Self {
        let x: &mut [(); 0] = &mut [];
        x
    }
}

#[doc(hidden)]
#[cfg(feature = "default-send-request")]
impl WasiHttpHooks for [(); 0] {}

/// Returns a value suitable for the `WasiHttpCtxView::hooks` field which has
/// the default behavior for `wasi:http`.
#[cfg(feature = "default-send-request")]
pub fn default_hooks() -> &'static mut dyn WasiHttpHooks {
    Default::default()
}

/// The default value configured for [`WasiHttpHooks::outgoing_body_buffer_chunks`] in [`WasiHttpView`].
pub const DEFAULT_OUTGOING_BODY_BUFFER_CHUNKS: usize = 1;
/// The default value configured for [`WasiHttpHooks::outgoing_body_chunk_size`] in [`WasiHttpView`].
pub const DEFAULT_OUTGOING_BODY_CHUNK_SIZE: usize = 1024 * 1024;

/// Structure which `wasi:http` `Host`-style traits are implemented for.
///
/// This structure is used by embedders with the [`WasiHttpView`] trait's return
/// value and is used to provide access to this crate all internals necessary to
/// implement `wasi:http`. This is similar to [`wasmtime_wasi::WasiCtxView`]
/// for example.
pub struct WasiHttpCtxView<'a> {
    /// A reference to a per-store [`WasiHttpCtx`].
    pub ctx: &'a mut WasiHttpCtx,
    /// A reference to a per-store table of resources to store host structures
    /// within.
    pub table: &'a mut ResourceTable,
    /// A reference to a per-store set of hooks that can be used to customize
    /// `wasi:http` behavior.
    pub hooks: &'a mut dyn WasiHttpHooks,
}

/// The type for which this crate implements the `wasi:http` interfaces.
pub struct WasiHttp;

impl HasData for WasiHttp {
    type Data<'a> = WasiHttpCtxView<'a>;
}

/// A trait used to project state that this crate needs to implement `wasi:http`
/// from the `self` type.
///
/// This trait is used in [`add_to_linker_sync`] and [`add_to_linker_async`] for
/// example as a bound on `T` in `Store<T>`. This is used to access data from
/// `T`, the data within a `Store`, an instance of [`WasiHttpCtxView`]. The
/// [`WasiHttpCtxView`] contains contextual information such as the
/// [`ResourceTable`] for the store, HTTP context info in [`WasiHttpCtx`], and
/// any hooks via [`WasiHttpHooks`] if the embedder desires.
///
/// # Example
///
/// ```
/// use wasmtime::component::ResourceTable;
/// use wasmtime_wasi_http::WasiHttpCtx;
/// use wasmtime_wasi_http::p2::{WasiHttpView, WasiHttpCtxView};
///
/// struct MyState {
///     http_ctx: WasiHttpCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiHttpView for MyState {
///     fn http(&mut self) -> WasiHttpCtxView<'_> {
///         WasiHttpCtxView {
///             ctx: &mut self.http_ctx,
///             table: &mut self.table,
///             hooks: Default::default(),
///         }
///     }
/// }
/// ```
pub trait WasiHttpView {
    /// Returns an instance of [`WasiHttpCtxView`] projected out of `self`.
    fn http(&mut self) -> WasiHttpCtxView<'_>;
}

/// Add all of the `wasi:http/proxy` world's interfaces to a [`wasmtime::component::Linker`].
///
/// This function will add the `async` variant of all interfaces into the
/// `Linker` provided. For embeddings with async support disabled see
/// [`add_to_linker_sync`] instead.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
/// use wasmtime_wasi_http::{WasiHttpCtx, p2::{WasiHttpView, WasiHttpCtxView}};
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_http::p2::add_to_linker_async(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     Ok(())
/// }
///
/// struct MyState {
///     ctx: WasiCtx,
///     http_ctx: WasiHttpCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiHttpView for MyState {
///     fn http(&mut self) -> WasiHttpCtxView<'_> {
///         WasiHttpCtxView {
///             ctx: &mut self.http_ctx,
///             table: &mut self.table,
///             hooks: Default::default(),
///         }
///     }
/// }
///
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> WasiCtxView<'_> {
///         WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
///     }
/// }
/// ```
pub fn add_to_linker_async<T>(l: &mut wasmtime::component::Linker<T>) -> wasmtime::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView + 'static,
{
    wasmtime_wasi::p2::add_to_linker_proxy_interfaces_async(l)?;
    add_only_http_to_linker_async(l)
}

/// A slimmed down version of [`add_to_linker_async`] which only adds
/// `wasi:http` interfaces to the linker.
///
/// This is useful when using [`wasmtime_wasi::p2::add_to_linker_async`] for
/// example to avoid re-adding the same interfaces twice.
pub fn add_only_http_to_linker_async<T>(
    l: &mut wasmtime::component::Linker<T>,
) -> wasmtime::Result<()>
where
    T: WasiHttpView + 'static,
{
    let options = bindings::LinkOptions::default(); // FIXME: Thread through to the CLI options.
    bindings::http::outgoing_handler::add_to_linker::<_, WasiHttp>(l, T::http)?;
    bindings::http::types::add_to_linker::<_, WasiHttp>(l, &options.into(), T::http)?;

    Ok(())
}

/// Add all of the `wasi:http/proxy` world's interfaces to a [`wasmtime::component::Linker`].
///
/// This function will add the `sync` variant of all interfaces into the
/// `Linker` provided. For embeddings with async support see
/// [`add_to_linker_async`] instead.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
/// use wasmtime_wasi_http::WasiHttpCtx;
/// use wasmtime_wasi_http::p2::{WasiHttpView, WasiHttpCtxView};
///
/// fn main() -> Result<()> {
///     let config = Config::default();
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_http::p2::add_to_linker_sync(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     Ok(())
/// }
///
/// struct MyState {
///     ctx: WasiCtx,
///     http_ctx: WasiHttpCtx,
///     table: ResourceTable,
/// }
/// impl WasiHttpView for MyState {
///     fn http(&mut self) -> WasiHttpCtxView<'_> {
///         WasiHttpCtxView {
///             ctx: &mut self.http_ctx,
///             table: &mut self.table,
///             hooks: Default::default(),
///         }
///     }
/// }
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> WasiCtxView<'_> {
///         WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
///     }
/// }
/// ```
pub fn add_to_linker_sync<T>(l: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView + 'static,
{
    wasmtime_wasi::p2::add_to_linker_proxy_interfaces_sync(l)?;
    add_only_http_to_linker_sync(l)
}

/// A slimmed down version of [`add_to_linker_sync`] which only adds
/// `wasi:http` interfaces to the linker.
///
/// This is useful when using [`wasmtime_wasi::p2::add_to_linker_sync`] for
/// example to avoid re-adding the same interfaces twice.
pub fn add_only_http_to_linker_sync<T>(l: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiHttpView + 'static,
{
    let options = bindings::LinkOptions::default(); // FIXME: Thread through to the CLI options.
    bindings::sync::http::outgoing_handler::add_to_linker::<_, WasiHttp>(l, T::http)?;
    bindings::sync::http::types::add_to_linker::<_, WasiHttp>(l, &options.into(), T::http)?;

    Ok(())
}

/// The default implementation of how an outgoing request is sent.
///
/// This implementation is used by the `wasi:http/outgoing-handler` interface
/// default implementation.
#[cfg(feature = "default-send-request")]
pub fn default_send_request(
    request: hyper::Request<body::HyperOutgoingBody>,
    config: types::OutgoingRequestConfig,
) -> types::HostFutureIncomingResponse {
    let handle = wasmtime_wasi::runtime::spawn(async move {
        Ok(default_send_request_handler(request, config).await)
    });
    types::HostFutureIncomingResponse::pending(handle)
}

/// The underlying implementation of how an outgoing request is sent. This should likely be spawned
/// in a task.
///
/// This is called from [default_send_request] to actually send the request.
#[cfg(feature = "default-send-request")]
pub async fn default_send_request_handler(
    mut request: hyper::Request<body::HyperOutgoingBody>,
    types::OutgoingRequestConfig {
        use_tls,
        connect_timeout,
        first_byte_timeout,
        between_bytes_timeout,
    }: types::OutgoingRequestConfig,
) -> Result<types::IncomingResponse, ErrorCode> {
    use crate::io::TokioIo;
    use crate::p2::{error::dns_error, hyper_request_error};
    use http_body_util::BodyExt;
    use tokio::net::TcpStream;
    use tokio::time::timeout;

    let authority = if let Some(authority) = request.uri().authority() {
        if authority.port().is_some() {
            authority.to_string()
        } else {
            let port = if use_tls { 443 } else { 80 };
            format!("{}:{port}", authority.to_string())
        }
    } else {
        return Err(ErrorCode::HttpRequestUriInvalid);
    };
    let tcp_stream = timeout(connect_timeout, TcpStream::connect(&authority))
        .await
        .map_err(|_| ErrorCode::ConnectionTimeout)?
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AddrNotAvailable => {
                dns_error("address not available".to_string(), 0)
            }

            _ => {
                if e.to_string()
                    .starts_with("failed to lookup address information")
                {
                    dns_error("address not available".to_string(), 0)
                } else {
                    ErrorCode::ConnectionRefused
                }
            }
        })?;

    let (mut sender, worker) = if use_tls {
        use rustls::pki_types::ServerName;

        // derived from https://github.com/rustls/rustls/blob/main/examples/src/bin/simpleclient.rs
        let root_cert_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.into(),
        };
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        let mut parts = authority.split(":");
        let host = parts.next().unwrap_or(&authority);
        let domain = ServerName::try_from(host)
            .map_err(|e| {
                tracing::warn!("dns lookup error: {e:?}");
                dns_error("invalid dns name".to_string(), 0)
            })?
            .to_owned();
        let stream = connector.connect(domain, tcp_stream).await.map_err(|e| {
            tracing::warn!("tls protocol error: {e:?}");
            ErrorCode::TlsProtocolError
        })?;
        let stream = TokioIo::new(stream);

        let (sender, conn) = timeout(
            connect_timeout,
            hyper::client::conn::http1::handshake(stream),
        )
        .await
        .map_err(|_| ErrorCode::ConnectionTimeout)?
        .map_err(hyper_request_error)?;

        let worker = wasmtime_wasi::runtime::spawn(async move {
            match conn.await {
                Ok(()) => {}
                // TODO: shouldn't throw away this error and ideally should
                // surface somewhere.
                Err(e) => tracing::warn!("dropping error {e}"),
            }
        });

        (sender, worker)
    } else {
        let tcp_stream = TokioIo::new(tcp_stream);
        let (sender, conn) = timeout(
            connect_timeout,
            // TODO: we should plumb the builder through the http context, and use it here
            hyper::client::conn::http1::handshake(tcp_stream),
        )
        .await
        .map_err(|_| ErrorCode::ConnectionTimeout)?
        .map_err(hyper_request_error)?;

        let worker = wasmtime_wasi::runtime::spawn(async move {
            match conn.await {
                Ok(()) => {}
                // TODO: same as above, shouldn't throw this error away.
                Err(e) => tracing::warn!("dropping error {e}"),
            }
        });

        (sender, worker)
    };

    // at this point, the request contains the scheme and the authority, but
    // the http packet should only include those if addressing a proxy, so
    // remove them here, since SendRequest::send_request does not do it for us
    *request.uri_mut() = http::Uri::builder()
        .path_and_query(
            request
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or("/"),
        )
        .build()
        .expect("comes from valid request");

    let resp = timeout(first_byte_timeout, sender.send_request(request))
        .await
        .map_err(|_| ErrorCode::ConnectionReadTimeout)?
        .map_err(hyper_request_error)?
        .map(|body| body.map_err(hyper_request_error).boxed_unsync());

    Ok(types::IncomingResponse {
        resp,
        worker: Some(worker),
        between_bytes_timeout,
    })
}
