//! # Wasmtime's WASI HTTP Implementation
//!
//! This crate is Wasmtime's host implementation of the `wasi:http` package as
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
//! [`wasi:http/proxy`]: crate::bindings::Proxy
//! [`wasi:http/outgoing-handler`]: crate::bindings::http::outgoing_handler::Host
//! [`wasi:http/types`]: crate::bindings::http::types::Host
//! [`wasi:http/incoming-handler`]: crate::bindings::exports::wasi::http::incoming_handler::Guest
//!
//! This crate is very similar to [`wasmtime-wasi`] in the it uses the
//! `bindgen!` macro in Wasmtime to generate bindings to interfaces. Bindings
//! are located in the [`bindings`] module.
//!
//! # The `WasiHttpView` trait
//!
//! All `bindgen!`-generated `Host` traits are implemented in terms of a
//! [`WasiHttpView`] trait which provides basic access to [`WasiHttpCtx`],
//! configuration for WASI HTTP, and a [`wasmtime_wasi::ResourceTable`], the
//! state for all host-defined component model resources.
//!
//! The [`WasiHttpView`] trait additionally offers a few other configuration
//! methods such as [`WasiHttpView::send_request`] to customize how outgoing
//! HTTP requests are handled.
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
//!      [`wasmtime_wasi::add_to_linker_async`] for example.
//!    * Add individual interfaces such as with the
//!      [`bindings::http::outgoing_handler::add_to_linker_get_host`] function.
//! 3. Use [`ProxyPre`](bindings::ProxyPre) to pre-instantiate a component
//!    before serving requests.
//! 4. When serving requests use
//!    [`ProxyPre::instantiate_async`](bindings::ProxyPre::instantiate_async)
//!    to create instances and handle HTTP requests.
//!
//! A standalone example of doing all this looks like:
//!
//! ```no_run
//! use anyhow::bail;
//! use hyper::server::conn::http1;
//! use std::sync::Arc;
//! use tokio::net::TcpListener;
//! use wasmtime::component::{Component, Linker, ResourceTable};
//! use wasmtime::{Config, Engine, Result, Store};
//! use wasmtime_wasi::{IoView, WasiCtx, WasiCtxBuilder, WasiView};
//! use wasmtime_wasi_http::bindings::ProxyPre;
//! use wasmtime_wasi_http::bindings::http::types::Scheme;
//! use wasmtime_wasi_http::body::HyperOutgoingBody;
//! use wasmtime_wasi_http::io::TokioIo;
//! use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let component = std::env::args().nth(1).unwrap();
//!
//!     // Prepare the `Engine` for Wasmtime
//!     let mut config = Config::new();
//!     config.async_support(true);
//!     let engine = Engine::new(&config)?;
//!
//!     // Compile the component on the command line to machine code
//!     let component = Component::from_file(&engine, &component)?;
//!
//!     // Prepare the `ProxyPre` which is a pre-instantiated version of the
//!     // component that we have. This will make per-request instantiation
//!     // much quicker.
//!     let mut linker = Linker::new(&engine);
//!     wasmtime_wasi_http::add_to_linker_async(&mut linker)?;
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
//!                 wasi: WasiCtxBuilder::new().inherit_stdio().build(),
//!                 http: WasiHttpCtx::new(),
//!             },
//!         );
//!         let (sender, receiver) = tokio::sync::oneshot::channel();
//!         let req = store.data_mut().new_incoming_request(Scheme::Http, req)?;
//!         let out = store.data_mut().new_response_outparam(sender)?;
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
//!                     Ok(r) => r.unwrap_err(),
//!                     Err(e) => e.into(),
//!                 };
//!                 bail!("guest never invoked `response-outparam::set` method: {e:?}")
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
//! impl IoView for MyClientState {
//!     fn table(&mut self) -> &mut ResourceTable {
//!         &mut self.table
//!     }
//! }
//! impl WasiView for MyClientState {
//!     fn ctx(&mut self) -> &mut WasiCtx {
//!         &mut self.wasi
//!     }
//! }
//!
//! impl WasiHttpView for MyClientState {
//!     fn ctx(&mut self) -> &mut WasiHttpCtx {
//!         &mut self.http
//!     }
//! }
//! ```

#![deny(missing_docs)]
#![doc(test(attr(deny(warnings))))]
#![doc(test(attr(allow(dead_code, unused_variables, unused_mut))))]
#![expect(clippy::allow_attributes_without_reason, reason = "crate not migrated")]

mod error;
mod http_impl;
mod types_impl;

pub mod body;
pub mod io;
pub mod types;

pub mod bindings;

pub use crate::error::{
    http_request_error, hyper_request_error, hyper_response_error, HttpError, HttpResult,
};
#[doc(inline)]
pub use crate::types::{
    WasiHttpCtx, WasiHttpImpl, WasiHttpView, DEFAULT_OUTGOING_BODY_BUFFER_CHUNKS,
    DEFAULT_OUTGOING_BODY_CHUNK_SIZE,
};
use wasmtime_wasi::IoImpl;
/// Add all of the `wasi:http/proxy` world's interfaces to a [`wasmtime::component::Linker`].
///
/// This function will add the `async` variant of all interfaces into the
/// `Linker` provided. By `async` this means that this function is only
/// compatible with [`Config::async_support(true)`][async]. For embeddings with
/// async support disabled see [`add_to_linker_sync`] instead.
///
/// [async]: wasmtime::Config::async_support
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{IoView, WasiCtx, WasiView};
/// use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_http::add_to_linker_async(&mut linker)?;
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
/// impl IoView for MyState {
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// impl WasiHttpView for MyState {
///     fn ctx(&mut self) -> &mut WasiHttpCtx { &mut self.http_ctx }
/// }
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
/// }
/// ```
pub fn add_to_linker_async<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView,
{
    let io_closure = type_annotate_io::<T, _>(|t| wasmtime_wasi::IoImpl(t));
    wasmtime_wasi::bindings::io::poll::add_to_linker_get_host(l, io_closure)?;
    wasmtime_wasi::bindings::io::error::add_to_linker_get_host(l, io_closure)?;
    wasmtime_wasi::bindings::io::streams::add_to_linker_get_host(l, io_closure)?;

    let closure = type_annotate_wasi::<T, _>(|t| wasmtime_wasi::WasiImpl(wasmtime_wasi::IoImpl(t)));
    wasmtime_wasi::bindings::clocks::wall_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::clocks::monotonic_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stdin::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stdout::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stderr::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::random::random::add_to_linker_get_host(l, closure)?;

    add_only_http_to_linker_async(l)
}

// NB: workaround some rustc inference - a future refactoring may make this
// obsolete.
fn type_annotate_http<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> WasiHttpImpl<&mut T>,
{
    val
}
fn type_annotate_wasi<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> wasmtime_wasi::WasiImpl<&mut T>,
{
    val
}
fn type_annotate_io<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> wasmtime_wasi::IoImpl<&mut T>,
{
    val
}

/// A slimmed down version of [`add_to_linker_async`] which only adds
/// `wasi:http` interfaces to the linker.
///
/// This is useful when using [`wasmtime_wasi::add_to_linker_async`] for
/// example to avoid re-adding the same interfaces twice.
pub fn add_only_http_to_linker_async<T>(
    l: &mut wasmtime::component::Linker<T>,
) -> anyhow::Result<()>
where
    T: WasiHttpView,
{
    let closure = type_annotate_http::<T, _>(|t| WasiHttpImpl(IoImpl(t)));
    crate::bindings::http::outgoing_handler::add_to_linker_get_host(l, closure)?;
    crate::bindings::http::types::add_to_linker_get_host(l, closure)?;

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
/// use wasmtime_wasi::{IoView, WasiCtx, WasiView};
/// use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
///
/// fn main() -> Result<()> {
///     let config = Config::default();
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_http::add_to_linker_sync(&mut linker)?;
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
/// impl IoView for MyState {
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// impl WasiHttpView for MyState {
///     fn ctx(&mut self) -> &mut WasiHttpCtx { &mut self.http_ctx }
/// }
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
/// }
/// ```
pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView,
{
    let io_closure = type_annotate_io::<T, _>(|t| wasmtime_wasi::IoImpl(t));
    // For the sync linker, use the definitions of poll and streams from the
    // wasmtime_wasi::bindings::sync space because those are defined using in_tokio.
    wasmtime_wasi::bindings::sync::io::poll::add_to_linker_get_host(l, io_closure)?;
    wasmtime_wasi::bindings::sync::io::streams::add_to_linker_get_host(l, io_closure)?;
    // The error interface in the wasmtime_wasi is synchronous
    wasmtime_wasi::bindings::io::error::add_to_linker_get_host(l, io_closure)?;

    let closure = type_annotate_wasi::<T, _>(|t| wasmtime_wasi::WasiImpl(wasmtime_wasi::IoImpl(t)));

    wasmtime_wasi::bindings::clocks::wall_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::clocks::monotonic_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stdin::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stdout::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stderr::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::random::random::add_to_linker_get_host(l, closure)?;

    add_only_http_to_linker_sync(l)?;

    Ok(())
}

/// A slimmed down version of [`add_to_linker_sync`] which only adds
/// `wasi:http` interfaces to the linker.
///
/// This is useful when using [`wasmtime_wasi::add_to_linker_sync`] for
/// example to avoid re-adding the same interfaces twice.
pub fn add_only_http_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView,
{
    let closure = type_annotate_http::<T, _>(|t| WasiHttpImpl(IoImpl(t)));

    crate::bindings::http::outgoing_handler::add_to_linker_get_host(l, closure)?;
    crate::bindings::http::types::add_to_linker_get_host(l, closure)?;

    Ok(())
}
