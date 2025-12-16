//! Experimental, unstable and incomplete implementation of wasip3 version of `wasi:tls`.
//!
//! This module is under heavy development.
//! It is not compliant with semver and is not ready
//! for production use.
//!
//! Bug and security fixes limited to wasip3 will not be given patch releases.
//!
//! Documentation of this module may be incorrect or out-of-sync with the implementation.

pub mod bindings;
mod host;

use core::task::Waker;
use std::sync::{Arc, Mutex};

use bindings::tls::{client, server, types};
use rustls::pki_types::ServerName;
use tokio::sync::oneshot;
use wasmtime::component::{HasData, Linker, ResourceTable};

/// The type for which this crate implements the `wasi:tls` interfaces.
pub struct WasiTls;

impl HasData for WasiTls {
    type Data<'a> = WasiTlsCtxView<'a>;
}

/// A trait which provides internal WASI TLS state.
pub trait WasiTlsCtx: Send {}

/// Default implementation of [WasiTlsCtx].
#[derive(Clone, Default)]
pub struct DefaultWasiTlsCtx;

impl WasiTlsCtx for DefaultWasiTlsCtx {}

/// View into [WasiTlsCtx] implementation and [ResourceTable].
pub struct WasiTlsCtxView<'a> {
    /// Mutable reference to the WASI TLS context.
    pub ctx: &'a mut dyn WasiTlsCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

/// A trait which provides internal WASI TLS state.
pub trait WasiTlsView: Send {
    /// Return a [WasiTlsCtxView] from mutable reference to self.
    fn tls(&mut self) -> WasiTlsCtxView<'_>;
}

/// Add all interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:tls/imports` world supported by
/// this module.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime_wasi_tls::p3::{DefaultWasiTlsCtx, WasiTlsCtxView, WasiTlsView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_tls::p3::add_to_linker(&mut linker)?;
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
///     tls: DefaultWasiTlsCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiTlsView for MyState {
///     fn tls(&mut self) -> WasiTlsCtxView<'_> {
///         WasiTlsCtxView {
///             ctx: &mut self.tls,
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiTlsView + 'static,
{
    client::add_to_linker::<_, WasiTls>(linker, T::tls)?;
    server::add_to_linker::<_, WasiTls>(linker, T::tls)?;
    types::add_to_linker::<_, WasiTls>(linker, T::tls)?;
    Ok(())
}

/// Client hello
#[derive(Clone, Default, Eq, PartialEq, Hash)]
pub struct ClientHello {
    /// Server name indicator.
    pub server_name: Option<ServerName<'static>>,
    /// ALPN IDs
    pub alpn_ids: Option<Vec<Vec<u8>>>,
    /// Cipher suites
    pub cipher_suites: Vec<u16>,
}

/// Server hello
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ServerHello {
    /// Cipher suite
    pub cipher_suite: u16,
}

impl ServerHello {
    /// Constructs a new server hello message
    pub fn new(cipher_suite: u16) -> Self {
        Self { cipher_suite }
    }
}

type TlsStreamArc<T> = Arc<Mutex<TlsStream<T>>>;
type TlsStreamClientArc = TlsStreamArc<rustls::ClientConnection>;
type TlsStreamServerArc = TlsStreamArc<rustls::ServerConnection>;

/// Client handshake
pub struct ClientHandshake {
    stream: TlsStreamClientArc,
    error_rx: oneshot::Receiver<rustls::Error>,
}

/// Server handshake
pub struct ServerHandshake {
    accepted: rustls::server::Accepted,
    consumer_tx: oneshot::Sender<TlsStreamServerArc>,
    producer_tx: oneshot::Sender<TlsStreamServerArc>,
}

/// Certificate
pub struct Certificate;

struct TlsStream<T> {
    conn: T,
    error_tx: Option<oneshot::Sender<rustls::Error>>,
    close_notify: bool,
    read_tls: Option<Waker>,
    ciphertext_consumer: Option<Waker>,
    ciphertext_producer: Option<Waker>,
    plaintext_consumer: Option<Waker>,
    plaintext_producer: Option<Waker>,
}

impl<T> TlsStream<T> {
    fn new(conn: T, error_tx: oneshot::Sender<rustls::Error>) -> Self {
        Self {
            conn,
            error_tx: Some(error_tx),
            close_notify: false,
            read_tls: None,
            plaintext_producer: None,
            plaintext_consumer: None,
            ciphertext_producer: None,
            ciphertext_consumer: None,
        }
    }
}
