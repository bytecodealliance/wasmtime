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
use core::pin::Pin;
use core::task::{Context, Poll};
use tokio::sync::oneshot;
use wasmtime::StoreContextMut;
use wasmtime::component::{Destination, Linker, StreamProducer, StreamResult, VecBuffer};

// Default buffer capacity to use for reads of byte-sized values.
const DEFAULT_BUFFER_CAPACITY: usize = 8192;

/// Helper structure to convert an iterator of `Result<T, E>` into a `stream<T>`
/// plus a `future<result<_, T>>` in WIT.
///
/// This will drain the iterator on calls to `poll_produce` and place as many
/// items as the input buffer has capacity for into the result. This will avoid
/// doing anything if the async read is cancelled.
///
/// Note that this does not actually do anything async, it's assuming that the
/// internal `iter` is either fast or intended to block.
struct FallibleIteratorProducer<I, E> {
    iter: I,
    result: Option<oneshot::Sender<Result<(), E>>>,
}

impl<I, T, E, D> StreamProducer<D> for FallibleIteratorProducer<I, E>
where
    I: Iterator<Item = Result<T, E>> + Send + Unpin + 'static,
    T: Send + Sync + 'static,
    E: Send + 'static,
{
    type Item = T;
    type Buffer = VecBuffer<T>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        // Explicitly ignore `_finish` because this implementation never
        // returns `Poll::Pending` anyway meaning that it never "blocks" in the
        // async sense.
        _finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        // Take up to `count` items as requested by the guest, or pick some
        // reasonable-ish number for the host.
        let count = dst.remaining(&mut store).unwrap_or(32);

        // Handle 0-length reads which test for readiness as saying "we're
        // always ready" since, in theory, this is.
        if count == 0 {
            return Poll::Ready(Ok(StreamResult::Completed));
        }

        // Drain `self.iter`. Successful results go into `buf`. Any errors make
        // their way to the `oneshot` result inside this structure. Otherwise
        // this only gets dropped if `None` is seen or an error. Also this'll
        // terminate once `buf` grows too large.
        let mut buf = Vec::new();
        let result = loop {
            match self.iter.next() {
                Some(Ok(item)) => buf.push(item),
                Some(Err(e)) => {
                    self.close(Err(e));
                    break StreamResult::Dropped;
                }

                None => {
                    self.close(Ok(()));
                    break StreamResult::Dropped;
                }
            }
            if buf.len() >= count {
                break StreamResult::Completed;
            }
        };

        dst.set_buffer(buf.into());
        return Poll::Ready(Ok(result));
    }
}

impl<I, E> FallibleIteratorProducer<I, E> {
    fn new(iter: I, result: oneshot::Sender<Result<(), E>>) -> Self {
        Self {
            iter,
            result: Some(result),
        }
    }

    fn close(&mut self, result: Result<(), E>) {
        // Ignore send failures because it means the other end wasn't interested
        // in the final error, if any.
        let _ = self.result.take().unwrap().send(result);
    }
}

impl<I, E> Drop for FallibleIteratorProducer<I, E> {
    fn drop(&mut self) {
        if self.result.is_some() {
            self.close(Ok(()));
        }
    }
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
