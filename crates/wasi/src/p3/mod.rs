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

use crate::p3::bindings::LinkOptions;
use crate::{NamedId, WasiNamedView, WasiView};
use core::pin::Pin;
use core::task::{Context, Poll};
use tokio::sync::oneshot;
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Component, Destination, Linker, StreamProducer, StreamResult, VecBuffer,
};

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
    _options: &LinkOptions,
) -> wasmtime::Result<()>
where
    T: WasiView + 'static,
{
    cli::add_to_linker(linker)?;
    clocks::add_to_linker(linker)?;
    filesystem::add_to_linker(linker)?;
    random::add_to_linker(linker)?;
    sockets::add_to_linker(linker)?;
    Ok(())
}

/// Interfaces that are added via [`add_named_to_linker`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Interface {
    /// `wasi:clocks/monotonic-clock`
    ClocksMonotonicClock,
    /// `wasi:clocks/system-clock`
    ClocksSystemClock,
    /// `wasi:random/random`
    RandomRandom,
    /// `wasi:random/insecure`
    RandomInsecure,
    /// `wasi:random/insecure-seed`
    RandomInsecureSeed,
    /// `wasi:cli/exit`
    CliExit,
    /// `wasi:cli/environment`
    CliEnvironment,
    /// `wasi:cli/stdin`
    CliStdin,
    /// `wasi:cli/stdout`
    CliStdout,
    /// `wasi:cli/stderr`
    CliStderr,
    /// `wasi:cli/terminal-input`
    CliTerminalInput,
    /// `wasi:cli/terminal-output`
    CliTerminalOutput,
    /// `wasi:cli/terminal-stdin`
    CliTerminalStdin,
    /// `wasi:cli/terminal-stdout`
    CliTerminalStdout,
    /// `wasi:cli/terminal-stderr`
    CliTerminalStderr,
    /// `wasi:filesystem/types`
    FilesystemTypes,
    /// `wasi:filesystem/preopens`
    FilesystemPreopens,
    /// `wasi:sockets/types`
    SocketsTypes,
    /// `wasi:sockets/ip-name-lookup`
    SocketsIpNameLookup,
}

/// Add all WASI interfaces from this module into the `linker` provided for any
/// named imports that a component has.
///
/// This function is similar to [`add_to_linker`] except that it's specifically
/// designed to work with named imports of WASI interfaces that components may
/// have. This requires a [`Component`] parameter to be passed in when
/// populating the [`Linker`] provided to see what the [`Component`] actually
/// imports.
///
/// All interfaces implemented by this module are added here, so the
/// per-package functions such as [`cli::add_named_to_linker`] do not
/// additionally need to be invoked. If this isn't low level enough you can
/// invoke those functions, or the bindgen-generated `add_to_linker` functions
/// within the [`named_imports`] module, directly instead.
///
/// [`named_imports`]: crate::p3::bindings::named_imports
///
/// The `lookup` function provided here is invoked for every named import found
/// for a particular interface. The [`Interface`] given is what's being bound,
/// and the `&str` argument is the name that the component imports it as. The
/// embedder can then decide how it would like to allocate a [`NamedId`] for
/// this import. If `Ok` is returned then the linker is populated with this
/// name, and imported functions will pass the [`NamedId`] later to the
/// implementation of [`WasiNamedView`] on `T` when invoked. If `Err` is
/// returned then the error will cause this entire function to fail and this
/// function call will return the same error.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use wasmtime::component::{Component, Linker, ResourceTable};
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime_wasi::{NamedId, WasiCtx, WasiCtxView, WasiNamedView};
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
///     // might have for WASI interfaces.
///     let mut name_map = HashMap::new();
///     wasmtime_wasi::p3::add_named_to_linker(&mut linker, &component, |_i, name| {
///         let len = name_map.len();
///         Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///     })?;
///
///     // Here a `WasiCtx` is allocated per-named-import and will then be
///     // referred to internally by the [`NamedId`] allocated above. You could
///     // also use `name_map` to configure each context differently.
///     let mut my_state = MyState::default();
///     for _ in 0..name_map.len() {
///         my_state.contexts.push(WasiCtx::default());
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
///     contexts: Vec<WasiCtx>,
/// }
///
/// impl WasiNamedView for MyState {
///     fn ctx(&mut self, id: NamedId) -> WasiCtxView<'_> {
///         WasiCtxView {
///             ctx: &mut self.contexts[id.0],
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_named_to_linker<T>(
    linker: &mut Linker<T>,
    component: &Component,
    lookup: impl FnMut(Interface, &str) -> wasmtime::Result<NamedId>,
) -> wasmtime::Result<()>
where
    T: WasiNamedView + 'static,
{
    let options = LinkOptions::default();
    add_named_to_linker_with_options(linker, &options, component, lookup)
}

/// Same as [`add_named_to_linker`] except [`LinkOptions`] can be specified to
/// configure interfaces that are added.
pub fn add_named_to_linker_with_options<T>(
    linker: &mut Linker<T>,
    _options: &LinkOptions,
    component: &Component,
    mut lookup: impl FnMut(Interface, &str) -> wasmtime::Result<NamedId>,
) -> wasmtime::Result<()>
where
    T: WasiNamedView + 'static,
{
    cli::add_named_to_linker(linker, component, &mut lookup)?;
    clocks::add_named_to_linker(linker, component, &mut lookup)?;
    filesystem::add_named_to_linker(linker, component, &mut lookup)?;
    random::add_named_to_linker(linker, component, &mut lookup)?;
    sockets::add_named_to_linker(linker, component, &mut lookup)?;
    Ok(())
}
