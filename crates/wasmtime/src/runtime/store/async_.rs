#[cfg(feature = "call-hook")]
use crate::CallHook;
use crate::fiber::{self};
use crate::prelude::*;
#[cfg(feature = "gc")]
use crate::runtime::vm::VMStore;
use crate::store::{AsContextMut, Asyncness, ResourceLimiterInner, StoreInner, StoreOpaque};
use crate::{Store, StoreContextMut, UpdateDeadline};

/// An object that can take callbacks when the runtime enters or exits hostcalls.
#[cfg(feature = "call-hook")]
#[async_trait::async_trait]
pub trait CallHookHandler<T>: Send {
    /// A callback to run when wasmtime is about to enter a host call, or when about to
    /// exit the hostcall.
    async fn handle_call_event(&self, t: crate::StoreHookState<'_, T>, ch: CallHook) -> Result<()>;
}

impl<T> Store<T> {
    /// Configures the [`ResourceLimiterAsync`](crate::ResourceLimiterAsync)
    /// used to limit resource creation within this [`Store`].
    ///
    /// This method is an asynchronous variant of the [`Store::limiter`] method
    /// where the embedder can block the wasm request for more resources with
    /// host `async` execution of futures.
    ///
    /// By using a [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`)
    /// with a [`Store`], you can no longer use
    /// [`Memory::new`](`crate::Memory::new`),
    /// [`Memory::grow`](`crate::Memory::grow`),
    /// [`Table::new`](`crate::Table::new`), and
    /// [`Table::grow`](`crate::Table::grow`). Instead, you must use their
    /// `async` variants: [`Memory::new_async`](`crate::Memory::new_async`),
    /// [`Memory::grow_async`](`crate::Memory::grow_async`),
    /// [`Table::new_async`](`crate::Table::new_async`), and
    /// [`Table::grow_async`](`crate::Table::grow_async`).
    ///
    /// Note that this limiter is only used to limit the creation/growth of
    /// resources in the future, this does not retroactively attempt to apply
    /// limits to the [`Store`].
    ///
    /// After configuring this method it's required that synchronous APIs in
    /// Wasmtime are no longer used, such as [`Func::call`](crate::Func::call).
    /// Instead APIs such as [`Func::call_async`](crate::Func::call_async) must
    /// be used instead.
    pub fn limiter_async(
        &mut self,
        mut limiter: impl (FnMut(&mut T) -> &mut dyn crate::ResourceLimiterAsync)
        + Send
        + Sync
        + 'static,
    ) {
        // Apply the limits on instances, tables, and memory given by the limiter:
        let inner = &mut self.inner;
        let (instance_limit, table_limit, memory_limit) = {
            let l = limiter(inner.data_mut());
            (l.instances(), l.tables(), l.memories())
        };
        let innermost = &mut inner.inner;
        innermost.instance_limit = instance_limit;
        innermost.table_limit = table_limit;
        innermost.memory_limit = memory_limit;

        // Save the limiter accessor function:
        inner.limiter = Some(ResourceLimiterInner::Async(Box::new(limiter)));
        inner.set_async_required(Asyncness::Yes);
    }

    /// Configures an async function that runs on calls and returns between
    /// WebAssembly and host code. For the non-async equivalent of this method,
    /// see [`Store::call_hook`].
    ///
    /// The function is passed a [`CallHook`] argument, which indicates which
    /// state transition the VM is making.
    ///
    /// This function's future may return a [`Trap`]. If a trap is returned
    /// when an import was called, it is immediately raised as-if the host
    /// import had returned the trap. If a trap is returned after wasm returns
    /// to the host then the wasm function's result is ignored and this trap is
    /// returned instead.
    ///
    /// After this function returns a trap, it may be called for subsequent
    /// returns to host or wasm code as the trap propagates to the root call.
    ///
    /// [`Trap`]: crate::Trap
    #[cfg(feature = "call-hook")]
    pub fn call_hook_async(&mut self, hook: impl CallHookHandler<T> + Send + Sync + 'static) {
        self.inner.call_hook = Some(crate::store::CallHookInner::Async(Box::new(hook)));
        self.inner.set_async_required(Asyncness::Yes);
    }

    /// Perform garbage collection asynchronously.
    ///
    /// Note that it is not required to actively call this function. GC will
    /// automatically happen according to various internal heuristics. This is
    /// provided if fine-grained control over the GC is desired.
    ///
    /// This method is only available when the `gc` Cargo feature is enabled.
    #[cfg(feature = "gc")]
    pub async fn gc_async(&mut self, why: Option<&crate::GcHeapOutOfMemory<()>>) -> Result<()>
    where
        T: Send,
    {
        StoreContextMut(&mut self.inner).gc_async(why).await
    }

    /// Manually grow the GC heap by at least `bytes` bytes.
    ///
    /// For more information, see the documentation of [`Store::gc_heap_grow`].
    #[cfg(feature = "gc")]
    pub async fn gc_heap_grow_async(&mut self, bytes: u64) -> Result<()> {
        StoreContextMut(&mut self.inner)
            .gc_heap_grow_async(bytes)
            .await
    }

    /// Configures epoch-deadline expiration to yield to the async
    /// caller and the update the deadline.
    ///
    /// When epoch-interruption-instrumented code is executed on this
    /// store and the epoch deadline is reached before completion,
    /// with the store configured in this way, execution will yield
    /// (the future will return `Pending` but re-awake itself for
    /// later execution) and, upon resuming, the store will be
    /// configured with an epoch deadline equal to the current epoch
    /// plus `delta` ticks.
    ///
    /// This setting is intended to allow for cooperative timeslicing
    /// of multiple CPU-bound Wasm guests in different stores, all
    /// executing under the control of an async executor. To drive
    /// this, stores should be configured to "yield and update"
    /// automatically with this function, and some external driver (a
    /// thread that wakes up periodically, or a timer
    /// signal/interrupt) should call
    /// [`Engine::increment_epoch()`](crate::Engine::increment_epoch).
    ///
    /// See documentation on
    /// [`Config::epoch_interruption()`](crate::Config::epoch_interruption)
    /// for an introduction to epoch-based interruption.
    #[cfg(target_has_atomic = "64")]
    pub fn epoch_deadline_async_yield_and_update(&mut self, delta: u64) {
        self.inner.epoch_deadline_async_yield_and_update(delta);
    }

    /// Sets the maximum number of simultaneously-live fibers permitted in this
    /// store.
    ///
    /// Async entrypoints in Wasmtime, such as [`Func::call_async`] will
    /// allocate fibers to execute WebAssembly code on. Fibers are additionally
    /// used for implementing component model async-related primitives. Fibers
    /// represent an allocated chunk of the host process's virtual address space
    /// for use as a temporary native stack. Due to the cost of a fiber
    /// allocation they're limited by default per-store with the ability for
    /// embedders to tune limits as desired.
    ///
    /// Attempts to exceed this limit will be met with an immediate error. For
    /// example [`Func::call_async`] will return an error when this limit is
    /// reached.
    ///
    /// The limit counts live fibers, so a fiber returns budget once it
    /// completes. Lowering it below the current live count does not affect
    /// existing fibers, only blocks new ones until the count falls back under
    /// it. A limit of 0 makes every async entry point fail.
    ///
    /// The default value for this is 64 fibers.
    ///
    /// [`Engine`]: crate::Engine
    /// [`Func::call_async`]: crate::Func::call_async
    pub fn set_max_fibers(&mut self, limit: u32) {
        self.as_context_mut().set_max_fibers(limit)
    }
}

impl<'a, T> StoreContextMut<'a, T> {
    /// See [`Store::set_max_fibers`].
    pub fn set_max_fibers(&mut self, limit: u32) {
        self.0.async_state.max_fibers = limit;
    }

    /// Perform garbage collection of `ExternRef`s.
    ///
    /// Same as [`Store::gc`].
    ///
    /// This method is only available when the `gc` Cargo feature is enabled.
    #[cfg(feature = "gc")]
    pub async fn gc_async(&mut self, why: Option<&crate::GcHeapOutOfMemory<()>>) -> Result<()>
    where
        T: Send + 'static,
    {
        let (mut limiter, store) = self.0.resource_limiter_and_store_opaque();
        store
            .gc(
                limiter.as_mut(),
                None,
                why.map(|e| e.bytes_needed()),
                crate::store::Asyncness::Yes,
            )
            .await?;
        Ok(())
    }

    /// Manually grow the GC heap by at least `bytes` bytes.
    ///
    /// For more information, see the documentation of [`Store::gc_heap_grow`].
    #[cfg(feature = "gc")]
    pub async fn gc_heap_grow_async(&mut self, bytes: u64) -> Result<()> {
        let (mut limiter, store) = self.0.resource_limiter_and_store_opaque();
        store
            .grow_gc_heap(limiter.as_mut(), bytes, crate::store::Asyncness::Yes)
            .await
    }

    /// Configures epoch-deadline expiration to yield to the async
    /// caller and the update the deadline.
    ///
    /// For more information see
    /// [`Store::epoch_deadline_async_yield_and_update`].
    #[cfg(target_has_atomic = "64")]
    pub fn epoch_deadline_async_yield_and_update(&mut self, delta: u64) {
        self.0.epoch_deadline_async_yield_and_update(delta);
    }
}

impl<T> StoreInner<T> {
    #[cfg(target_has_atomic = "64")]
    fn epoch_deadline_async_yield_and_update(&mut self, delta: u64) {
        // All future entrypoints must be async to handle the case that an epoch
        // changes and a yield is required.
        self.set_async_required(Asyncness::Yes);

        self.epoch_deadline_behavior =
            Some(Box::new(move |_store| Ok(UpdateDeadline::Yield(delta))));
    }
}

#[doc(hidden)]
impl StoreOpaque {
    pub(crate) fn allocate_fiber_stack(&mut self) -> Result<wasmtime_fiber::FiberStack> {
        if self.async_state.live_fibers >= self.async_state.max_fibers {
            bail!("store limit of fibers has been reached");
        }
        let stack = match self.async_state.last_fiber_stack().take() {
            Some(stack) => stack,
            None => self.engine().allocator().allocate_fiber_stack()?,
        };
        self.async_state.live_fibers += 1;
        Ok(stack)
    }

    pub(crate) fn deallocate_fiber_stack(&mut self, stack: wasmtime_fiber::FiberStack) {
        self.async_state.live_fibers -= 1;
        self.flush_fiber_stack();
        *self.async_state.last_fiber_stack() = Some(stack);
    }

    /// Releases the last fiber stack to the underlying instance allocator, if
    /// present.
    pub fn flush_fiber_stack(&mut self) {
        if let Some(stack) = self.async_state.last_fiber_stack().take() {
            unsafe {
                self.engine.allocator().deallocate_fiber_stack(stack);
            }
        }
    }
}

impl<T> StoreContextMut<'_, T> {
    /// Executes a synchronous computation `func` asynchronously on a new fiber.
    pub(crate) async fn on_fiber<R: Send + Sync>(
        &mut self,
        func: impl FnOnce(&mut StoreContextMut<'_, T>) -> R + Send + Sync,
    ) -> Result<R> {
        fiber::on_fiber(self.0, |me| func(&mut StoreContextMut(me))).await
    }
}
