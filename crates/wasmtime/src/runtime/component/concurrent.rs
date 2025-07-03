use crate::component::{Component, Func, HasData, HasSelf, Instance};
use crate::store::{StoreInner, StoreOpaque};
use crate::vm::{VMFuncRef, VMMemoryDefinition, VMStore, component::ComponentInstance};
use crate::{AsContextMut, StoreContextMut, ValRaw};
use anyhow::Result;
use std::any::Any;
use std::boxed::Box;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::pin::{Pin, pin};
use std::task::{Context, Poll, Waker};
use wasmtime_environ::component::{
    RuntimeComponentInstanceIndex, TypeComponentLocalErrorContextTableIndex, TypeFutureTableIndex,
    TypeStreamTableIndex, TypeTupleIndex,
};

pub(crate) use futures_and_streams::ResourcePair;
pub use futures_and_streams::{
    ErrorContext, FutureReader, FutureWriter, HostFuture, HostStream, StreamReader, StreamWriter,
};
pub(crate) use futures_and_streams::{
    lower_error_context_to_index, lower_future_to_index, lower_stream_to_index,
};

mod futures_and_streams;

#[allow(dead_code)]
pub enum Status {
    Starting = 0,
    Started = 1,
    Returned = 2,
    StartCancelled = 3,
    ReturnCancelled = 4,
}

impl Status {
    /// Packs this status and the optional `waitable` provided into a 32-bit
    /// result that the canonical ABI requires.
    ///
    /// The low 4 bits are reserved for the status while the upper 28 bits are
    /// the waitable, if present.
    pub fn pack(self, waitable: Option<u32>) -> u32 {
        _ = waitable;
        todo!()
    }
}

pub(crate) struct ConcurrentState {}

impl ConcurrentState {
    pub(crate) fn new(component: &Component) -> Self {
        _ = component;
        Self {}
    }

    /// Implements the `context.get` intrinsic.
    pub(crate) fn context_get(&mut self, slot: u32) -> Result<u32> {
        _ = slot;
        todo!()
    }

    /// Implements the `context.set` intrinsic.
    pub(crate) fn context_set(&mut self, slot: u32, val: u32) -> Result<()> {
        _ = (slot, val);
        todo!()
    }

    /// Implements the `backpressure.set` intrinsic.
    pub(crate) fn backpressure_set(
        &mut self,
        caller_instance: RuntimeComponentInstanceIndex,
        enabled: u32,
    ) -> Result<()> {
        _ = (caller_instance, enabled);
        todo!()
    }

    /// Implements the `waitable-set.new` intrinsic.
    pub(crate) fn waitable_set_new(
        &mut self,
        caller_instance: RuntimeComponentInstanceIndex,
    ) -> Result<u32> {
        _ = caller_instance;
        todo!()
    }

    /// Implements the `waitable-set.drop` intrinsic.
    pub(crate) fn waitable_set_drop(
        &mut self,
        caller_instance: RuntimeComponentInstanceIndex,
        set: u32,
    ) -> Result<()> {
        _ = (caller_instance, set);
        todo!()
    }

    /// Implements the `waitable.join` intrinsic.
    pub(crate) fn waitable_join(
        &mut self,
        caller_instance: RuntimeComponentInstanceIndex,
        waitable_handle: u32,
        set_handle: u32,
    ) -> Result<()> {
        _ = (caller_instance, waitable_handle, set_handle);
        todo!()
    }

    /// Implements the `subtask.drop` intrinsic.
    pub(crate) fn subtask_drop(
        &mut self,
        caller_instance: RuntimeComponentInstanceIndex,
        task_id: u32,
    ) -> Result<()> {
        _ = (caller_instance, task_id);
        todo!()
    }
}

/// Provides access to either store data (via the `get` method) or the store
/// itself (via [`AsContext`]/[`AsContextMut`]), as well as the component
/// instance to which the current host task belongs.
///
/// See [`Accessor::with`] for details.
pub struct Access<'a, T: 'static, D: HasData = HasSelf<T>> {
    _phantom: PhantomData<(&'a (), T, D)>,
}

/// Provides scoped mutable access to store data in the context of a concurrent
/// host task future.
///
/// This allows multiple host task futures to execute concurrently and access
/// the store between (but not across) `await` points.
pub struct Accessor<T: 'static, D = HasSelf<T>>
where
    D: HasData,
{
    #[expect(dead_code, reason = "to be used in the future")]
    get: fn() -> *mut dyn VMStore,
    #[expect(dead_code, reason = "to be used in the future")]
    get_data: fn(&mut T) -> D::Data<'_>,
    #[expect(dead_code, reason = "to be used in the future")]
    instance: Instance,
}

impl<T, D> Accessor<T, D>
where
    D: HasData,
{
    #[doc(hidden)]
    pub fn with_data<D2: HasData>(
        &mut self,
        get_data: fn(&mut T) -> D2::Data<'_>,
    ) -> Accessor<T, D2> {
        let _ = get_data;
        todo!()
    }
}

impl Instance {
    /// Wrap the specified host function in a future which will call it, passing
    /// it an `&mut Accessor<T>`.
    ///
    /// See the `Accessor` documentation for details.
    pub(crate) fn wrap_call<T: 'static, F, R>(
        self,
        store: StoreContextMut<T>,
        closure: F,
    ) -> impl Future<Output = Result<R>> + 'static
    where
        T: 'static,
        F: FnOnce(&mut Accessor<T>) -> Pin<Box<dyn Future<Output = Result<R>> + Send + '_>>
            + Send
            + Sync
            + 'static,
        R: Send + Sync + 'static,
    {
        _ = (store, closure);
        async { todo!() }
    }

    /// Poll the specified future once on behalf of a guest->host call using an
    /// async-lowered import.
    ///
    /// If it returns `Ready`, return `Ok(None)`.  Otherwise, if it returns
    /// `Pending`, add it to the set of futures to be polled as part of this
    /// instance's event loop until it completes, and then return
    /// `Ok(Some(handle))` where `handle` is the waitable handle to return.
    ///
    /// Whether the future returns `Ready` immediately or later, the `lower`
    /// function will be used to lower the result, if any, into the guest caller's
    /// stack and linear memory unless the task has been cancelled.
    pub(crate) fn first_poll<T: 'static, R: Send + 'static>(
        self,
        mut store: StoreContextMut<T>,
        future: impl Future<Output = Result<R>> + Send + 'static,
        caller_instance: RuntimeComponentInstanceIndex,
        lower: impl FnOnce(StoreContextMut<T>, Instance, R) -> Result<()> + Send + 'static,
    ) -> Result<Option<u32>> {
        _ = (&mut store, future, caller_instance, lower);
        todo!()
    }

    /// Poll the specified future until it completes on behalf of a guest->host
    /// call using a sync-lowered import.
    ///
    /// This is similar to `Self::first_poll` except it's for sync-lowered
    /// imports, meaning we don't need to handle cancellation and we can block
    /// the caller until the task completes, at which point the caller can
    /// handle lowering the result to the guest's stack and linear memory.
    pub(crate) fn poll_and_block<R: Send + Sync + 'static>(
        self,
        store: &mut dyn VMStore,
        future: impl Future<Output = Result<R>> + Send + 'static,
        caller_instance: RuntimeComponentInstanceIndex,
    ) -> Result<R> {
        _ = (store, caller_instance);
        match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(result) => result,
            Poll::Pending => {
                todo!()
            }
        }
    }

    /// TODO: docs
    pub async fn run<F>(&self, mut store: impl AsContextMut, fut: F) -> Result<F::Output>
    where
        F: Future,
    {
        _ = (&mut store, fut);
        todo!()
    }

    /// Implements the `task.return` intrinsic, lifting the result for the
    /// current guest task.
    ///
    /// SAFETY: The `memory` and `storage` pointers must be valid, and `storage`
    /// must contain at least `storage_len` items.
    pub(crate) unsafe fn task_return(
        self,
        store: &mut dyn VMStore,
        ty: TypeTupleIndex,
        memory: *mut VMMemoryDefinition,
        string_encoding: u8,
        storage: *mut ValRaw,
        storage_len: usize,
    ) -> Result<()> {
        _ = (store, ty, memory, string_encoding, storage, storage_len);
        todo!()
    }

    /// Implements the `task.cancel` intrinsic.
    pub(crate) fn task_cancel(
        self,
        store: &mut dyn VMStore,
        _caller_instance: RuntimeComponentInstanceIndex,
    ) -> Result<()> {
        _ = store;
        todo!()
    }

    /// Implements the `waitable-set.wait` intrinsic.
    pub(crate) fn waitable_set_wait(
        self,
        store: &mut dyn VMStore,
        caller_instance: RuntimeComponentInstanceIndex,
        async_: bool,
        memory: *mut VMMemoryDefinition,
        set: u32,
        payload: u32,
    ) -> Result<u32> {
        _ = (store, caller_instance, async_, memory, set, payload);
        todo!()
    }

    /// Implements the `waitable-set.poll` intrinsic.
    pub(crate) fn waitable_set_poll(
        self,
        store: &mut dyn VMStore,
        caller_instance: RuntimeComponentInstanceIndex,
        async_: bool,
        memory: *mut VMMemoryDefinition,
        set: u32,
        payload: u32,
    ) -> Result<u32> {
        _ = (store, caller_instance, async_, memory, set, payload);
        todo!()
    }

    /// Implements the `yield` intrinsic.
    pub(crate) fn yield_(self, store: &mut dyn VMStore, async_: bool) -> Result<bool> {
        _ = (store, async_);
        todo!()
    }

    /// Implements the `subtask.cancel` intrinsic.
    pub(crate) fn subtask_cancel(
        self,
        store: &mut dyn VMStore,
        caller_instance: RuntimeComponentInstanceIndex,
        async_: bool,
        task_id: u32,
    ) -> Result<u32> {
        _ = (store, caller_instance, async_, task_id);
        todo!()
    }

    /// Convenience function to reduce boilerplate.
    pub(crate) fn concurrent_state_mut<'a>(
        &self,
        store: &'a mut StoreOpaque,
    ) -> &'a mut ConcurrentState {
        _ = store;
        todo!()
    }
}

/// Trait representing component model ABI async intrinsics and fused adapter
/// helper functions.
pub unsafe trait VMComponentAsyncStore {
    /// A helper function for fused adapter modules involving calls where the
    /// one of the caller or callee is async.
    ///
    /// This helper is not used when the caller and callee both use the sync
    /// ABI, only when at least one is async is this used.
    unsafe fn prepare_call(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        start: *mut VMFuncRef,
        return_: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        callee_instance: RuntimeComponentInstanceIndex,
        task_return_type: TypeTupleIndex,
        string_encoding: u8,
        result_count: u32,
        storage: *mut ValRaw,
        storage_len: usize,
    ) -> Result<()>;

    /// A helper function for fused adapter modules involving calls where the
    /// caller is sync-lowered but the callee is async-lifted.
    unsafe fn sync_start(
        &mut self,
        instance: Instance,
        callback: *mut VMFuncRef,
        callee: *mut VMFuncRef,
        param_count: u32,
        storage: *mut MaybeUninit<ValRaw>,
        storage_len: usize,
    ) -> Result<()>;

    /// A helper function for fused adapter modules involving calls where the
    /// caller is async-lowered.
    unsafe fn async_start(
        &mut self,
        instance: Instance,
        callback: *mut VMFuncRef,
        post_return: *mut VMFuncRef,
        callee: *mut VMFuncRef,
        param_count: u32,
        result_count: u32,
        flags: u32,
    ) -> Result<u32>;

    /// The `backpressure.set` intrinsic.
    fn backpressure_set(
        &mut self,
        caller_instance: RuntimeComponentInstanceIndex,
        enabled: u32,
    ) -> Result<()>;

    /// The `task.return` intrinsic.
    fn task_return(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeTupleIndex,
        storage: *mut ValRaw,
        storage_len: usize,
    ) -> Result<()>;

    /// The `waitable-set.new` intrinsic.
    fn waitable_set_new(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
    ) -> Result<u32>;

    /// The `waitable-set.wait` intrinsic.
    fn waitable_set_wait(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        set: u32,
        async_: bool,
        memory: *mut VMMemoryDefinition,
        payload: u32,
    ) -> Result<u32>;

    /// The `waitable-set.poll` intrinsic.
    fn waitable_set_poll(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        set: u32,
        async_: bool,
        memory: *mut VMMemoryDefinition,
        payload: u32,
    ) -> Result<u32>;

    /// The `waitable-set.drop` intrinsic.
    fn waitable_set_drop(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        set: u32,
    ) -> Result<()>;

    /// The `waitable.join` intrinsic.
    fn waitable_join(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        set: u32,
        waitable: u32,
    ) -> Result<()>;

    /// The `yield` intrinsic.
    fn yield_(&mut self, instance: &mut ComponentInstance, async_: bool) -> Result<()>;

    /// The `subtask.drop` intrinsic.
    fn subtask_drop(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        task_id: u32,
    ) -> Result<()>;

    /// A helper function for fused adapter modules involving calls where the
    /// caller is sync-lowered but the callee is async-lifted.
    fn sync_enter(
        &mut self,
        start: *mut VMFuncRef,
        return_: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        task_return_type: TypeTupleIndex,
        result_count: u32,
        storage: *mut ValRaw,
        storage_len: usize,
    ) -> Result<()>;

    /// A helper function for fused adapter modules involving calls where the
    /// caller is sync-lowered but the callee is async-lifted.
    fn sync_exit(
        &mut self,
        instance: &mut ComponentInstance,
        callback: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        callee: *mut VMFuncRef,
        callee_instance: RuntimeComponentInstanceIndex,
        param_count: u32,
        storage: *mut MaybeUninit<ValRaw>,
        storage_len: usize,
    ) -> Result<()>;

    /// A helper function for fused adapter modules involving calls where the
    /// caller is async-lowered.
    fn async_enter(
        &mut self,
        start: *mut VMFuncRef,
        return_: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        task_return_type: TypeTupleIndex,
        params: u32,
        results: u32,
    ) -> Result<()>;

    /// A helper function for fused adapter modules involving calls where the
    /// caller is async-lowered.
    fn async_exit(
        &mut self,
        instance: &mut ComponentInstance,
        callback: *mut VMFuncRef,
        post_return: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        callee: *mut VMFuncRef,
        callee_instance: RuntimeComponentInstanceIndex,
        param_count: u32,
        result_count: u32,
        flags: u32,
    ) -> Result<u32>;

    /// The `future.write` intrinsic.
    unsafe fn future_write(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TypeFutureTableIndex,
        future: u32,
        address: u32,
    ) -> Result<u32>;

    /// The `future.read` intrinsic.
    unsafe fn future_read(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TypeFutureTableIndex,
        future: u32,
        address: u32,
    ) -> Result<u32>;

    /// The `stream.write` intrinsic.
    unsafe fn stream_write(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TypeStreamTableIndex,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32>;

    /// The `stream.read` intrinsic.
    unsafe fn stream_read(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TypeStreamTableIndex,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32>;

    /// The "fast-path" implementation of the `stream.write` intrinsic for
    /// "flat" (i.e. memcpy-able) payloads.
    unsafe fn flat_stream_write(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        async_: bool,
        ty: TypeStreamTableIndex,
        payload_size: u32,
        payload_align: u32,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32>;

    /// The "fast-path" implementation of the `stream.read` intrinsic for "flat"
    /// (i.e. memcpy-able) payloads.
    unsafe fn flat_stream_read(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        async_: bool,
        ty: TypeStreamTableIndex,
        payload_size: u32,
        payload_align: u32,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32>;

    /// The `error-context.debug-message` intrinsic.
    unsafe fn error_context_debug_message(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeComponentLocalErrorContextTableIndex,
        err_ctx_handle: u32,
        debug_msg_address: u32,
    ) -> Result<()>;
}

unsafe impl<T> VMComponentAsyncStore for StoreInner<T> {
    unsafe fn prepare_call(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        start: *mut VMFuncRef,
        return_: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        callee_instance: RuntimeComponentInstanceIndex,
        task_return_type: TypeTupleIndex,
        string_encoding: u8,
        result_count: u32,
        storage: *mut ValRaw,
        storage_len: usize,
    ) -> Result<()> {
        _ = (
            instance,
            memory,
            start,
            return_,
            caller_instance,
            callee_instance,
            task_return_type,
            string_encoding,
            result_count,
            storage,
            storage_len,
        );
        todo!()
    }

    unsafe fn sync_start(
        &mut self,
        instance: Instance,
        callback: *mut VMFuncRef,
        callee: *mut VMFuncRef,
        param_count: u32,
        storage: *mut MaybeUninit<ValRaw>,
        storage_len: usize,
    ) -> Result<()> {
        _ = (
            instance,
            callback,
            callee,
            param_count,
            storage,
            storage_len,
        );
        todo!()
    }

    unsafe fn async_start(
        &mut self,
        instance: Instance,
        callback: *mut VMFuncRef,
        post_return: *mut VMFuncRef,
        callee: *mut VMFuncRef,
        param_count: u32,
        result_count: u32,
        flags: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            callback,
            post_return,
            callee,
            param_count,
            result_count,
            flags,
        );
        todo!()
    }

    fn backpressure_set(
        &mut self,
        caller_instance: RuntimeComponentInstanceIndex,
        enabled: u32,
    ) -> Result<()> {
        _ = (caller_instance, enabled);
        todo!()
    }

    fn task_return(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeTupleIndex,
        storage: *mut ValRaw,
        storage_len: usize,
    ) -> Result<()> {
        _ = (instance, ty, storage, storage_len);
        todo!()
    }

    fn waitable_set_new(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
    ) -> Result<u32> {
        _ = (instance, caller_instance);
        todo!();
    }

    fn waitable_set_wait(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        set: u32,
        async_: bool,
        memory: *mut VMMemoryDefinition,
        payload: u32,
    ) -> Result<u32> {
        _ = (instance, caller_instance, set, async_, memory, payload);
        todo!();
    }

    fn waitable_set_poll(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        set: u32,
        async_: bool,
        memory: *mut VMMemoryDefinition,
        payload: u32,
    ) -> Result<u32> {
        _ = (instance, caller_instance, set, async_, memory, payload);
        todo!();
    }

    fn waitable_set_drop(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        set: u32,
    ) -> Result<()> {
        _ = (instance, caller_instance, set);
        todo!();
    }

    fn waitable_join(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        set: u32,
        waitable: u32,
    ) -> Result<()> {
        _ = (instance, caller_instance, set, waitable);
        todo!();
    }

    fn yield_(&mut self, instance: &mut ComponentInstance, async_: bool) -> Result<()> {
        _ = (instance, async_);
        todo!()
    }

    fn subtask_drop(
        &mut self,
        instance: &mut ComponentInstance,
        caller_instance: RuntimeComponentInstanceIndex,
        task_id: u32,
    ) -> Result<()> {
        _ = (instance, caller_instance, task_id);
        todo!()
    }

    fn sync_enter(
        &mut self,
        start: *mut VMFuncRef,
        return_: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        task_return_type: TypeTupleIndex,
        result_count: u32,
        storage: *mut ValRaw,
        storage_len: usize,
    ) -> Result<()> {
        _ = (
            start,
            return_,
            caller_instance,
            task_return_type,
            result_count,
            storage,
            storage_len,
        );
        todo!()
    }

    fn sync_exit(
        &mut self,
        instance: &mut ComponentInstance,
        callback: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        callee: *mut VMFuncRef,
        callee_instance: RuntimeComponentInstanceIndex,
        param_count: u32,
        storage: *mut MaybeUninit<ValRaw>,
        storage_len: usize,
    ) -> Result<()> {
        _ = (
            instance,
            callback,
            caller_instance,
            callee,
            callee_instance,
            param_count,
            storage,
            storage_len,
        );
        todo!()
    }

    fn async_enter(
        &mut self,
        start: *mut VMFuncRef,
        return_: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        task_return_type: TypeTupleIndex,
        params: u32,
        results: u32,
    ) -> Result<()> {
        _ = (
            start,
            return_,
            caller_instance,
            task_return_type,
            params,
            results,
        );
        todo!()
    }

    fn async_exit(
        &mut self,
        instance: &mut ComponentInstance,
        callback: *mut VMFuncRef,
        post_return: *mut VMFuncRef,
        caller_instance: RuntimeComponentInstanceIndex,
        callee: *mut VMFuncRef,
        callee_instance: RuntimeComponentInstanceIndex,
        param_count: u32,
        result_count: u32,
        flags: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            callback,
            post_return,
            caller_instance,
            callee,
            callee_instance,
            param_count,
            result_count,
            flags,
        );
        todo!()
    }

    unsafe fn future_write(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TypeFutureTableIndex,
        future: u32,
        address: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            memory,
            realloc,
            string_encoding,
            async_,
            ty,
            future,
            address,
        );
        todo!()
    }

    unsafe fn future_read(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TypeFutureTableIndex,
        future: u32,
        address: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            memory,
            realloc,
            string_encoding,
            async_,
            ty,
            future,
            address,
        );
        todo!()
    }

    unsafe fn stream_write(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TypeStreamTableIndex,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            memory,
            realloc,
            string_encoding,
            async_,
            ty,
            stream,
            address,
            count,
        );
        todo!()
    }

    unsafe fn stream_read(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TypeStreamTableIndex,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            memory,
            realloc,
            string_encoding,
            async_,
            ty,
            stream,
            address,
            count,
        );
        todo!()
    }

    unsafe fn flat_stream_write(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        async_: bool,
        ty: TypeStreamTableIndex,
        payload_size: u32,
        payload_align: u32,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            memory,
            realloc,
            async_,
            ty,
            payload_size,
            payload_align,
            stream,
            address,
            count,
        );
        todo!()
    }

    unsafe fn flat_stream_read(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        async_: bool,
        ty: TypeStreamTableIndex,
        payload_size: u32,
        payload_align: u32,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            memory,
            realloc,
            async_,
            ty,
            payload_size,
            payload_align,
            stream,
            address,
            count,
        );
        todo!()
    }

    unsafe fn error_context_debug_message(
        &mut self,
        instance: Instance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeComponentLocalErrorContextTableIndex,
        err_ctx_handle: u32,
        debug_msg_address: u32,
    ) -> Result<()> {
        _ = (
            instance,
            memory,
            realloc,
            string_encoding,
            ty,
            err_ctx_handle,
            debug_msg_address,
        );
        todo!()
    }
}

pub(crate) struct PreparedCall<R> {
    _phantom: PhantomData<R>,
}

/// Prepare a call to the specified exported Wasm function, providing functions
/// for lowering the parameters and lifting the result.
///
/// To enqueue the returned `PreparedCall` in the `ComponentInstance`'s event
/// loop, use `queue_call`.
///
/// Note that this function is used in `TypedFunc::call_async`, which accepts
/// parameters of a generic type which might not be `'static`.  However the
/// `GuestTask` created by this function must be `'static`, so it can't safely
/// close over those parameters.  Instead, `PreparedCall` has a `params` field
/// of type `Arc<AtomicPtr<u8>>`, which the caller is responsible for setting to
/// a valid, non-null pointer to the params prior to polling the event loop (at
/// least until the parameters have been lowered), and then resetting back to
/// null afterward.  That ensures that the lowering code never sees a stale
/// pointer, even if the application `drop`s or `mem::forget`s the future
/// returned by `TypedFunc::call_async`.
///
/// In the case where the parameters are passed using a type that _is_
/// `'static`, they can be boxed and stored in `PreparedCall::params`
/// indefinitely; `drop_params` will be called when they are no longer needed.
pub(crate) fn prepare_call<T, R>(
    mut store: StoreContextMut<T>,
    lower_params: impl FnOnce(Func, StoreContextMut<T>, &mut [MaybeUninit<ValRaw>]) -> Result<()>
    + Send
    + Sync
    + 'static,
    lift_result: impl FnOnce(Func, &mut StoreOpaque, &[ValRaw]) -> Result<Box<dyn Any + Send + Sync>>
    + Send
    + Sync
    + 'static,
    handle: Func,
    param_count: usize,
) -> Result<PreparedCall<R>> {
    let _ = (&mut store, lower_params, lift_result, handle, param_count);
    todo!()
}

/// Queue a call previously prepared using `prepare_call` to be run as part of
/// the associated `ComponentInstance`'s event loop.
///
/// The returned future will resolve to the result once it is available, but
/// must only be polled via the instance's event loop.  See `Instance::run` for
/// details.
pub(crate) fn queue_call<T: 'static, R: Send + 'static>(
    mut store: StoreContextMut<T>,
    prepared: PreparedCall<R>,
) -> Result<impl Future<Output = Result<R>> + Send + 'static + use<T, R>> {
    _ = (&mut store, prepared);
    Ok(async { todo!() })
}
