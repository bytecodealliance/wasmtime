//! Experimental, unstable and incomplete implementation of wasip3 version of WASI.
//!
//! This module is under heavy development.
//! It is not compliant with semver and is not ready
//! for production use.
//!
//! Bug and security fixes limited to wasip3 will not be given patch releases.
//!
//! Documentation of this module may be incorrect or out-of-sync with the implementation.

pub mod bindings;
pub mod cli;
pub mod clocks;
pub mod filesystem;
pub mod random;
pub mod sockets;

use crate::WasiView;
use crate::p3::bindings::LinkOptions;
use anyhow::Context as _;
use bytes::BytesMut;
use std::io::Cursor;
use tokio::sync::oneshot;
use wasmtime::AsContextMut as _;
use wasmtime::component::{
    Accessor, Destination, FutureProducer, Linker, StreamProducer, StreamState,
};

// Default buffer capacity to use for reads of byte-sized values.
const DEFAULT_BUFFER_CAPACITY: usize = 8192;

// Maximum buffer capacity to use for reads of byte-sized values.
const MAX_BUFFER_CAPACITY: usize = 4 * DEFAULT_BUFFER_CAPACITY;

struct StreamEmptyProducer;

impl<T, D> StreamProducer<D, T> for StreamEmptyProducer {
    async fn produce(
        &mut self,
        _: &Accessor<D>,
        _: &mut Destination<T>,
    ) -> wasmtime::Result<StreamState> {
        Ok(StreamState::Closed)
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        Ok(StreamState::Closed)
    }
}

struct FutureReadyProducer<T>(T);

impl<T, D> FutureProducer<D, T> for FutureReadyProducer<T>
where
    T: Send + 'static,
{
    async fn produce(self, _: &Accessor<D>) -> wasmtime::Result<T> {
        Ok(self.0)
    }
}

struct FutureOneshotProducer<T>(oneshot::Receiver<T>);

impl<T, D> FutureProducer<D, T> for FutureOneshotProducer<T>
where
    T: Send + 'static,
{
    async fn produce(self, _: &Accessor<D>) -> wasmtime::Result<T> {
        self.0.await.context("oneshot sender dropped")
    }
}

async fn write_buffered_bytes<T>(
    store: &Accessor<T>,
    src: &mut Cursor<BytesMut>,
    dst: &mut Destination<u8>,
) -> wasmtime::Result<()> {
    if !store.with(|mut store| {
        dst.as_guest_destination(store.as_context_mut())
            .map(|mut dst| {
                let start = src.position() as _;
                let buffered = src.get_ref().len().saturating_sub(start);
                let n = dst.remaining().len().min(buffered);
                debug_assert!(n > 0);
                let end = start.saturating_add(n);
                dst.remaining()[..n].copy_from_slice(&src.get_ref()[start..end]);
                dst.mark_written(n);
                src.set_position(end as _);
            })
            .is_some()
    }) {
        // FIXME: `mem::take` rather than `clone` when we can ensure cancellation-safety
        //let buf = mem::take(src);
        let buf = src.clone();
        *src = dst.write(store, buf).await?;
    }
    if src.position() as usize == src.get_ref().len() {
        src.get_mut().clear();
        src.set_position(0);
    }
    Ok(())
}

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:cli/imports` world supported by
/// this module.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::add_to_linker(&mut linker)?;
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
///     ctx: WasiCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> WasiCtxView<'_> {
///         WasiCtxView{
///             ctx: &mut self.ctx,
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiView + 'static,
{
    let options = LinkOptions::default();
    add_to_linker_with_options(linker, &options)
}

/// Similar to [`add_to_linker`], but with the ability to enable unstable features.
pub fn add_to_linker_with_options<T>(
    linker: &mut Linker<T>,
    options: &LinkOptions,
) -> wasmtime::Result<()>
where
    T: WasiView + 'static,
{
    cli::add_to_linker_with_options(linker, &options.into())?;
    clocks::add_to_linker(linker)?;
    filesystem::add_to_linker(linker)?;
    random::add_to_linker(linker)?;
    sockets::add_to_linker(linker)?;
    Ok(())
}
