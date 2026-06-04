use crate::component::concurrent::TaskId;
use crate::component::concurrent::{self, GuestTaskId, PreparedCall};
use crate::component::func::LowerContext;
use crate::component::{AsAccessor, ComponentNamedList, Func, Lift, Lower, TypedFunc, Val};
use crate::prelude::*;
use crate::runtime::vm::SendSyncPtr;
use crate::{AsContextMut, StoreContextMut, ValRaw};
use core::marker;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use wasmtime_environ::component::{InterfaceType, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS};

/// Returned from [`Func::start_call_concurrent`] to represent a
/// pending-but-not-yet-resolved call into wasm.
pub struct FuncCallConcurrent<'a, T> {
    call: concurrent::QueuedCall<Vec<Val>>,
    results: &'a mut [Val],
    _marker: marker::PhantomData<fn(T)>,
}

impl Func {
    /// Start a concurrent call to this function.
    ///
    /// Concurrency is achieved by relying on the [`Accessor`] argument, which
    /// can be obtained by calling [`StoreContextMut::run_concurrent`].
    ///
    /// Unlike [`Self::call`] and [`Self::call_async`] (both of which require
    /// exclusive access to the store until the completion of the call), calls
    /// made using this method may run concurrently with other calls to the same
    /// instance.  In addition, the runtime will call the `post-return` function
    /// (if any) automatically when the guest task completes.
    ///
    /// # Progress
    ///
    /// For the wasm task being created in `call_concurrent` to make progress it
    /// must be run within the scope of [`run_concurrent`]. If there are no
    /// active calls to [`run_concurrent`] then the wasm task will appear as
    /// stalled. This is typically not a concern as an [`Accessor`] is bound
    /// by default to a scope of [`run_concurrent`].
    ///
    /// One situation in which this can arise, for example, is that if a
    /// [`run_concurrent`] computation finishes its async closure before all
    /// wasm tasks have completed, then there will be no scope of
    /// [`run_concurrent`] anywhere. In this situation the wasm tasks that have
    /// not yet completed will not make progress until [`run_concurrent`] is
    /// called again.
    ///
    /// Embedders will need to ensure that this future is `await`'d within the
    /// scope of [`run_concurrent`] to ensure that the value can be produced
    /// during the `await` call.
    ///
    /// # Cancellation
    ///
    /// Cancelling an async task created via `call_concurrent`, at this time, is
    /// only possible by dropping the store that the computation runs within.
    /// With [#11833] implemented then it will be possible to request
    /// cancellation of a task, but that is not yet implemented. Hard-cancelling
    /// a task will only ever be possible by dropping the entire store and it is
    /// not possible to remove just one task from a store.
    ///
    /// This async function behaves more like a "spawn" than a normal Rust async
    /// function. When this function is invoked then metadata for the function
    /// call is recorded in the store connected to the `accessor` argument and
    /// the wasm invocation is from then on connected to the store. If the
    /// future created by this function is dropped it does not cancel the
    /// in-progress execution of the wasm task. Dropping the future
    /// relinquishes the host's ability to learn about the result of the task
    /// but the task will still progress and invoke callbacks and such until
    /// completion.
    ///
    /// This function will return an error if [`Config::concurrency_support`] is
    /// disabled.
    ///
    /// [`Config::concurrency_support`]: crate::Config::concurrency_support
    /// [`run_concurrent`]: crate::Store::run_concurrent
    /// [#11833]: https://github.com/bytecodealliance/wasmtime/issues/11833
    /// [`Accessor`]: crate::component::Accessor
    ///
    /// # Panics
    ///
    /// Panics if the store that the [`Accessor`] is derived from does not own
    /// this function.
    ///
    /// # Example
    ///
    /// Using [`StoreContextMut::run_concurrent`] to get an [`Accessor`]:
    ///
    /// ```
    /// # use {
    /// #   wasmtime::{
    /// #     error::{Result},
    /// #     component::{Component, Linker, ResourceTable},
    /// #     Config, Engine, Store
    /// #   },
    /// # };
    /// #
    /// # struct Ctx { table: ResourceTable }
    /// #
    /// # async fn foo() -> Result<()> {
    /// # let mut config = Config::new();
    /// # let engine = Engine::new(&config)?;
    /// # let mut store = Store::new(&engine, Ctx { table: ResourceTable::new() });
    /// # let mut linker = Linker::new(&engine);
    /// # let component = Component::new(&engine, "")?;
    /// # let instance = linker.instantiate_async(&mut store, &component).await?;
    /// let my_func = instance.get_func(&mut store, "my_func").unwrap();
    /// store.run_concurrent(async |accessor| -> wasmtime::Result<_> {
    ///    my_func.call_concurrent(accessor, &[], &mut Vec::new()).await?;
    ///    Ok(())
    /// }).await??;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call_concurrent(
        self,
        accessor: impl AsAccessor<Data: Send>,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()> {
        let accessor = accessor.as_accessor();
        let call = accessor.with(|store| self.start_call_concurrent(store, params, results))?;
        self.finish_call_concurrent(accessor, call).await
    }

    /// Performs preparatory work for invoking this function with `params`,
    /// returning a [`FuncCallConcurrent`]
    /// which can be passed to [`Func::finish_call_concurrent`] to resolve
    /// the call.
    ///
    /// For more information see [`Func::call_concurrent`].
    pub fn start_call_concurrent<'a, T: Send + 'static>(
        self,
        mut store: impl AsContextMut<Data = T>,
        params: &'a [Val],
        results: &'a mut [Val],
    ) -> Result<FuncCallConcurrent<'a, T>> {
        self.check_params_results(store.as_context_mut(), params, results)?;
        let prepared = self.prepare_call_dynamic(store.as_context_mut(), params.to_vec())?;
        let call = concurrent::QueuedCall::new(store.as_context_mut(), prepared)?;
        Ok(FuncCallConcurrent {
            call,
            results,
            _marker: marker::PhantomData,
        })
    }

    /// Completes a call that was initiated via
    /// [`Func::start_call_concurrent`].
    pub async fn finish_call_concurrent<T: Send>(
        self,
        accessor: impl AsAccessor<Data = T>,
        call: FuncCallConcurrent<'_, T>,
    ) -> Result<()> {
        // Intentionally not used today, but left here for future API
        // compatibility with using this.
        let _ = accessor;
        let FuncCallConcurrent { call, results, .. } = call;
        let run_results = call.await?;
        assert_eq!(run_results.len(), results.len());
        for (result, slot) in run_results.into_iter().zip(results) {
            *slot = result;
        }
        Ok(())
    }

    /// Calls `concurrent::prepare_call` with monomorphized functions for
    /// lowering the parameters and lifting the result.
    fn prepare_call_dynamic<'a, T: Send + 'static>(
        self,
        mut store: StoreContextMut<'a, T>,
        params: Vec<Val>,
    ) -> Result<PreparedCall<Vec<Val>>> {
        let store = store.as_context_mut();

        concurrent::prepare_call(
            store,
            self,
            MAX_FLAT_PARAMS,
            false,
            move |func, store, params_out| {
                func.with_lower_context(store, |cx, ty| {
                    Self::lower_args(cx, &params, ty, params_out)
                })
            },
            move |func, store, results| {
                let max_flat = if func.abi_async(store) {
                    MAX_FLAT_PARAMS
                } else {
                    MAX_FLAT_RESULTS
                };
                let results = func.with_lift_context(store, |cx, ty| {
                    Self::lift_results(cx, ty, results, max_flat)?.collect::<Result<Vec<_>>>()
                })?;
                Ok(Box::new(results))
            },
        )
    }
}

impl<T> FuncCallConcurrent<'_, T> {
    /// Returns the task that this invocation corresponds to.
    ///
    /// This can be later correlated with [`StoreContextMut::async_call_stack`]
    /// for example.
    pub fn task(&self) -> GuestTaskId {
        self.call.task()
    }
}

/// Returned from [`TypedFunc::start_call_concurrent`] to represent a
/// pending-but-not-yet-resolved call into wasm.
pub struct TypedFuncCallConcurrent<T, P, R> {
    call: concurrent::QueuedCall<R>,
    _marker: marker::PhantomData<fn(T, P)>,
}

impl<Params, Return> TypedFunc<Params, Return>
where
    Params: ComponentNamedList + Lower,
    Return: ComponentNamedList + Lift,
{
    pub(crate) async fn call_async_concurrent(
        &self,
        mut store: impl AsContextMut<Data: Send>,
        params: Params,
    ) -> Result<Return>
    where
        Return: 'static,
    {
        let mut store = store.as_context_mut();
        let ptr = SendSyncPtr::from(NonNull::from(&params).cast::<u8>());
        let prepared = self.prepare_call(store.as_context_mut(), true, move |cx, ty, dst| {
            // SAFETY: The goal here is to get `Params`, a non-`'static`
            // value, to live long enough to the lowering of the
            // parameters. We're guaranteed that `Params` lives in the
            // future of the outer function (we're in an `async fn`) so it'll
            // stay alive as long as the future itself. That is distinct,
            // for example, from the signature of `call_concurrent` below.
            //
            // Here a pointer to `Params` is smuggled to this location
            // through a `SendSyncPtr<u8>` to thwart the `'static` check
            // of rustc and the signature of `prepare_call`.
            //
            // Note the use of `SignalOnDrop` in the code that follows
            // this closure, which ensures that the task will be removed
            // from the concurrent state to which it belongs when the
            // containing `Future` is dropped, so long as the parameters
            // have not yet been lowered. Since this closure is removed from
            // the task after the parameters are lowered, it will never be called
            // after the containing `Future` is dropped.
            let params = unsafe { ptr.cast::<Params>().as_ref() };
            Self::lower_args(cx, ty, dst, params)
        })?;

        struct SignalOnDrop<'a, T: 'static> {
            store: StoreContextMut<'a, T>,
            task: TaskId,
        }

        impl<'a, T> Drop for SignalOnDrop<'a, T> {
            fn drop(&mut self) {
                self.task.host_future_dropped(self.store.0).unwrap();
            }
        }

        let mut wrapper = SignalOnDrop {
            store,
            task: prepared.task_id(),
        };

        let result = concurrent::QueuedCall::new(wrapper.store.as_context_mut(), prepared)?;
        wrapper
            .store
            .as_context_mut()
            .run_concurrent_trap_on_idle(async |_| Ok(result.await?))
            .await?
    }

    /// Start a concurrent call to this function.
    ///
    /// Concurrency is achieved by relying on the [`Accessor`] argument, which
    /// can be obtained by calling [`StoreContextMut::run_concurrent`].
    ///
    /// Unlike [`Self::call`] and [`Self::call_async`] (both of which require
    /// exclusive access to the store until the completion of the call), calls
    /// made using this method may run concurrently with other calls to the same
    /// instance.  In addition, the runtime will call the `post-return` function
    /// (if any) automatically when the guest task completes.
    ///
    /// This function will return an error if [`Config::concurrency_support`] is
    /// disabled.
    ///
    /// [`Config::concurrency_support`]: crate::Config::concurrency_support
    ///
    /// # Progress and Cancellation
    ///
    /// For more information about how to make progress on the wasm task or how
    /// to cancel the wasm task see the documentation for
    /// [`Func::call_concurrent`].
    ///
    /// [`Func::call_concurrent`]: crate::component::Func::call_concurrent
    ///
    /// # Panics
    ///
    /// Panics if the store that the [`Accessor`] is derived from does not own
    /// this function.
    ///
    /// [`Accessor`]: crate::component::Accessor
    ///
    /// # Example
    ///
    /// Using [`StoreContextMut::run_concurrent`] to get an [`Accessor`]:
    ///
    /// ```
    /// # use {
    /// #   wasmtime::{
    /// #     error::{Result},
    /// #     component::{Component, Linker, ResourceTable},
    /// #     Config, Engine, Store
    /// #   },
    /// # };
    /// #
    /// # struct Ctx { table: ResourceTable }
    /// #
    /// # async fn foo() -> Result<()> {
    /// # let mut config = Config::new();
    /// # let engine = Engine::new(&config)?;
    /// # let mut store = Store::new(&engine, Ctx { table: ResourceTable::new() });
    /// # let mut linker = Linker::new(&engine);
    /// # let component = Component::new(&engine, "")?;
    /// # let instance = linker.instantiate_async(&mut store, &component).await?;
    /// let my_typed_func = instance.get_typed_func::<(), ()>(&mut store, "my_typed_func")?;
    /// store.run_concurrent(async |accessor| -> wasmtime::Result<_> {
    ///    my_typed_func.call_concurrent(accessor, ()).await?;
    ///    Ok(())
    /// }).await??;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call_concurrent(
        self,
        accessor: impl AsAccessor<Data: Send>,
        params: Params,
    ) -> Result<Return>
    where
        Params: 'static,
        Return: 'static,
    {
        let call = accessor
            .as_accessor()
            .with(|store| self.start_call_concurrent(store, params))?;
        self.finish_call_concurrent(accessor, call).await
    }

    /// Performs preparatory work for invoking this function with `params`,
    /// returning a [`TypedFuncCallConcurrent`]
    /// which can be passed to [`TypedFunc::finish_call_concurrent`] to resolve
    /// the call.
    ///
    /// For more information see [`TypedFunc::call_concurrent`].
    pub fn start_call_concurrent<T>(
        self,
        mut store: impl AsContextMut<Data = T>,
        params: Params,
    ) -> Result<TypedFuncCallConcurrent<T, Params, Return>>
    where
        T: Send + 'static,
        Params: 'static,
        Return: 'static,
    {
        let mut store = store.as_context_mut();
        let mut store = store.as_context_mut();
        ensure!(
            store.0.concurrency_support(),
            "cannot use `call_concurrent` Config::concurrency_support disabled",
        );

        let prepared = self.prepare_call(store.as_context_mut(), false, move |cx, ty, dst| {
            Self::lower_args(cx, ty, dst, &params)
        })?;
        let call = concurrent::QueuedCall::new(store, prepared)?;
        Ok(TypedFuncCallConcurrent {
            call,
            _marker: marker::PhantomData,
        })
    }

    /// Completes a call that was initiated via
    /// [`TypedFunc::start_call_concurrent`].
    pub async fn finish_call_concurrent<T>(
        self,
        accessor: impl AsAccessor<Data = T>,
        call: TypedFuncCallConcurrent<T, Params, Return>,
    ) -> Result<Return>
    where
        T: Send + 'static,
        Params: 'static,
        Return: 'static,
    {
        // This is intentionally part of the public API but not used yet.
        // This'll likely want to be used in future refactorings.
        let _ = accessor;
        call.call.await
    }

    /// Calls `concurrent::prepare_call` with monomorphized functions for
    /// lowering the parameters and lifting the result according to the number
    /// of core Wasm parameters and results in the signature of the function to
    /// be called.
    fn prepare_call<T>(
        self,
        store: StoreContextMut<'_, T>,
        host_future_present: bool,
        lower: impl FnOnce(
            &mut LowerContext<T>,
            InterfaceType,
            &mut [MaybeUninit<ValRaw>],
        ) -> Result<()>
        + Send
        + Sync
        + 'static,
    ) -> Result<PreparedCall<Return>>
    where
        Return: 'static,
    {
        use crate::component::storage::slice_to_storage;
        debug_assert!(store.0.concurrency_support());

        let param_count = if Params::flatten_count() <= MAX_FLAT_PARAMS {
            Params::flatten_count()
        } else {
            1
        };
        let max_results = if self.func().abi_async(store.0) {
            MAX_FLAT_PARAMS
        } else {
            MAX_FLAT_RESULTS
        };
        concurrent::prepare_call(
            store,
            *self.func(),
            param_count,
            host_future_present,
            move |func, store, params_out| {
                func.with_lower_context(store, |cx, ty| lower(cx, ty, params_out))
            },
            move |func, store, results| {
                let result = if Return::flatten_count() <= max_results {
                    func.with_lift_context(store, |cx, ty| {
                        // SAFETY: Per the safety requirements documented for the
                        // `ComponentType` trait, `Return::Lower` must be
                        // compatible at the binary level with a `[ValRaw; N]`,
                        // where `N` is `mem::size_of::<Return::Lower>() /
                        // mem::size_of::<ValRaw>()`.  And since this function
                        // is only used when `Return::flatten_count() <=
                        // MAX_FLAT_RESULTS` and `MAX_FLAT_RESULTS == 1`, `N`
                        // can only either be 0 or 1.
                        //
                        // See `ComponentInstance::exit_call` for where we use
                        // the result count passed from
                        // `wasmtime_environ::fact::trampoline`-generated code
                        // to ensure the slice has the correct length, and also
                        // `concurrent::start_call` for where we conservatively
                        // use a slice length of 1 unconditionally.  Also note
                        // that, as of this writing `slice_to_storage`
                        // double-checks the slice length is sufficient.
                        let results: &Return::Lower = unsafe { slice_to_storage(results) };
                        Self::lift_stack_result(cx, ty, results)
                    })?
                } else {
                    func.with_lift_context(store, |cx, ty| {
                        Self::lift_heap_result(cx, ty, &results[0])
                    })?
                };
                Ok(Box::new(result))
            },
        )
    }
}

impl<T, P, R> TypedFuncCallConcurrent<T, P, R> {
    /// Returns the task that this invocation corresponds to.
    ///
    /// This can be later correlated with [`StoreContextMut::async_call_stack`]
    /// for example.
    pub fn task(&self) -> GuestTaskId {
        self.call.task()
    }
}
