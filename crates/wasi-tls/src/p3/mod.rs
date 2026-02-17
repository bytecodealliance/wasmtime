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

use crate::p3::host::{
    CiphertextConsumer, CiphertextProducer, PlaintextConsumer, PlaintextProducer,
};
use bindings::tls::{client, types};
use core::task::Waker;
use std::sync::{Arc, Mutex};
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
    types::add_to_linker::<_, WasiTls>(linker, T::tls)?;
    Ok(())
}

/// TLS client connector state.
#[derive(Default)]
pub struct Connector {
    pub(crate) receive_tx: Option<(
        oneshot::Sender<PlaintextProducer<rustls::ClientConnection>>,
        oneshot::Sender<CiphertextConsumer<rustls::ClientConnection>>,
        oneshot::Sender<rustls::Error>,
    )>,
    pub(crate) send_tx: Option<(
        oneshot::Sender<CiphertextProducer<rustls::ClientConnection>>,
        oneshot::Sender<
            PlaintextConsumer<rustls::ClientConnection, rustls::client::ClientConnectionData>,
        >,
        oneshot::Sender<rustls::Error>,
    )>,
}

type TlsStreamArc<T> = Arc<Mutex<TlsStream<T>>>;

struct TlsStream<T> {
    conn: T,
    plaintext_consumer_dropped: bool,
    ciphertext_consumer_dropped: bool,
    ciphertext_consumer: Option<Waker>,
    ciphertext_producer: Option<Waker>,
    plaintext_consumer: Option<Waker>,
    plaintext_producer: Option<Waker>,
}

impl<T> TlsStream<T> {
    fn new(conn: T) -> Self {
        Self {
            conn,
            plaintext_consumer_dropped: false,
            ciphertext_consumer_dropped: false,
            plaintext_producer: None,
            plaintext_consumer: None,
            ciphertext_producer: None,
            ciphertext_consumer: None,
        }
    }
}
