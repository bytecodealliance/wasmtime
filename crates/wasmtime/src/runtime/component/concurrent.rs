use {
    crate::{
        AsContextMut, ValRaw,
        store::StoreInner,
        vm::{VMFuncRef, VMMemoryDefinition, component::ComponentInstance},
    },
    anyhow::Result,
    futures::{FutureExt, stream::FuturesUnordered},
    std::{boxed::Box, future::Future, mem::MaybeUninit, pin::Pin},
    wasmtime_environ::component::{
        RuntimeComponentInstanceIndex, TypeComponentLocalErrorContextTableIndex,
        TypeFutureTableIndex, TypeStreamTableIndex, TypeTupleIndex,
    },
};

pub use futures_and_streams::{ErrorContext, FutureReader, StreamReader};

mod futures_and_streams;

/// Represents the result of a concurrent operation.
///
/// This is similar to a [`std::future::Future`] except that it represents an
/// operation which requires exclusive access to a store in order to make
/// progress -- without monopolizing that store for the lifetime of the
/// operation.
pub struct Promise<T>(Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>);

impl<T: 'static> Promise<T> {
    /// Map the result of this `Promise` from one value to another.
    pub fn map<U>(self, fun: impl FnOnce(T) -> U + Send + Sync + 'static) -> Promise<U> {
        Promise(Box::pin(self.0.map(fun)))
    }

    /// Convert this `Promise` to a future which may be `await`ed for its
    /// result.
    ///
    /// The returned future will require exclusive use of the store until it
    /// completes.  If you need to await more than one `Promise` concurrently,
    /// use [`PromisesUnordered`].
    pub async fn get<U: Send>(self, store: impl AsContextMut<Data = U>) -> Result<T> {
        _ = store;
        todo!()
    }

    /// Convert this `Promise` to a future which may be `await`ed for its
    /// result.
    ///
    /// Unlike [`Self::get`], this does _not_ take a store parameter, meaning
    /// the returned future will not make progress until and unless the event
    /// loop for the store it came from is polled.  Thus, this method should
    /// only be used from within host functions and not from top-level embedder
    /// code.
    pub fn into_future(self) -> Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>> {
        self.0
    }
}

/// Represents a collection of zero or more concurrent operations.
///
/// Similar to [`futures::stream::FuturesUnordered`], this type supports
/// `await`ing more than one [`Promise`]s concurrently.
pub struct PromisesUnordered<T>(
    FuturesUnordered<Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>>,
);

impl<T: 'static> PromisesUnordered<T> {
    /// Create a new `PromisesUnordered` with no entries.
    pub fn new() -> Self {
        Self(FuturesUnordered::new())
    }

    /// Add the specified [`Promise`] to this collection.
    pub fn push(&mut self, promise: Promise<T>) {
        self.0.push(promise.0)
    }

    /// Get the next result from this collection, if any.
    pub async fn next<U: Send>(&mut self, store: impl AsContextMut<Data = U>) -> Result<Option<T>> {
        _ = store;
        todo!()
    }
}

/// Trait representing component model ABI async intrinsics and fused adapter
/// helper functions.
pub unsafe trait VMComponentAsyncStore {
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

    /// The `future.new` intrinsic.
    fn future_new(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
    ) -> Result<u32>;

    /// The `future.write` intrinsic.
    fn future_write(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeFutureTableIndex,
        future: u32,
        address: u32,
    ) -> Result<u32>;

    /// The `future.read` intrinsic.
    fn future_read(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeFutureTableIndex,
        future: u32,
        address: u32,
    ) -> Result<u32>;

    /// The `future.cancel-write` intrinsic.
    fn future_cancel_write(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32>;

    /// The `future.cancel-read` intrinsic.
    fn future_cancel_read(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32>;

    /// The `future.close-writable` intrinsic.
    fn future_close_writable(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
        writer: u32,
    ) -> Result<()>;

    /// The `future.close-readable` intrinsic.
    fn future_close_readable(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
        reader: u32,
    ) -> Result<()>;

    /// The `stream.new` intrinsic.
    fn stream_new(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
    ) -> Result<u32>;

    /// The `stream.write` intrinsic.
    fn stream_write(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeStreamTableIndex,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32>;

    /// The `stream.read` intrinsic.
    fn stream_read(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeStreamTableIndex,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32>;

    /// The `stream.cancel-write` intrinsic.
    fn stream_cancel_write(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32>;

    /// The `stream.cancel-read` intrinsic.
    fn stream_cancel_read(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32>;

    /// The `stream.close-writable` intrinsic.
    fn stream_close_writable(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
        writer: u32,
    ) -> Result<()>;

    /// The `stream.close-readable` intrinsic.
    fn stream_close_readable(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
        reader: u32,
    ) -> Result<()>;

    /// The "fast-path" implementation of the `stream.write` intrinsic for
    /// "flat" (i.e. memcpy-able) payloads.
    fn flat_stream_write(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        ty: TypeStreamTableIndex,
        payload_size: u32,
        payload_align: u32,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32>;

    /// The "fast-path" implementation of the `stream.read` intrinsic for "flat"
    /// (i.e. memcpy-able) payloads.
    fn flat_stream_read(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        ty: TypeStreamTableIndex,
        payload_size: u32,
        payload_align: u32,
        stream: u32,
        address: u32,
        count: u32,
    ) -> Result<u32>;

    /// The `error-context.new` intrinsic.
    fn error_context_new(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeComponentLocalErrorContextTableIndex,
        debug_msg_address: u32,
        debug_msg_len: u32,
    ) -> Result<u32>;

    /// The `error-context.debug-message` intrinsic.
    fn error_context_debug_message(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeComponentLocalErrorContextTableIndex,
        err_ctx_handle: u32,
        debug_msg_address: u32,
    ) -> Result<()>;

    /// The `error-context.drop` intrinsic.
    fn error_context_drop(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeComponentLocalErrorContextTableIndex,
        err_ctx_handle: u32,
    ) -> Result<()>;
}

unsafe impl<T> VMComponentAsyncStore for StoreInner<T> {
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

    fn future_new(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
    ) -> Result<u32> {
        _ = (instance, ty);
        todo!()
    }

    fn future_write(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeFutureTableIndex,
        future: u32,
        address: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            memory,
            realloc,
            string_encoding,
            ty,
            future,
            address,
        );
        todo!()
    }

    fn future_read(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeFutureTableIndex,
        future: u32,
        address: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            memory,
            realloc,
            string_encoding,
            ty,
            future,
            address,
        );
        todo!()
    }

    fn future_cancel_write(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        _ = (instance, ty, async_, writer);
        todo!()
    }

    fn future_cancel_read(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        _ = (instance, ty, async_, reader);
        todo!()
    }

    fn future_close_writable(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
        writer: u32,
    ) -> Result<()> {
        _ = (instance, ty, writer);
        todo!()
    }

    fn future_close_readable(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeFutureTableIndex,
        reader: u32,
    ) -> Result<()> {
        _ = (instance, ty, reader);
        todo!()
    }

    fn stream_new(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
    ) -> Result<u32> {
        _ = (instance, ty);
        todo!()
    }

    fn stream_write(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
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
            ty,
            stream,
            address,
            count,
        );
        todo!()
    }

    fn stream_read(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
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
            ty,
            stream,
            address,
            count,
        );
        todo!()
    }

    fn stream_cancel_write(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        _ = (instance, ty, async_, writer);
        todo!()
    }

    fn stream_cancel_read(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        _ = (instance, ty, async_, reader);
        todo!()
    }

    fn stream_close_writable(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
        writer: u32,
    ) -> Result<()> {
        _ = (instance, ty, writer);
        todo!()
    }

    fn stream_close_readable(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeStreamTableIndex,
        reader: u32,
    ) -> Result<()> {
        _ = (instance, ty, reader);
        todo!()
    }

    fn flat_stream_write(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
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
            ty,
            payload_size,
            payload_align,
            stream,
            address,
            count,
        );
        todo!()
    }

    fn flat_stream_read(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
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
            ty,
            payload_size,
            payload_align,
            stream,
            address,
            count,
        );
        todo!()
    }

    fn error_context_new(
        &mut self,
        instance: &mut ComponentInstance,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeComponentLocalErrorContextTableIndex,
        debug_msg_address: u32,
        debug_msg_len: u32,
    ) -> Result<u32> {
        _ = (
            instance,
            memory,
            realloc,
            string_encoding,
            ty,
            debug_msg_address,
            debug_msg_len,
        );
        todo!()
    }

    fn error_context_debug_message(
        &mut self,
        instance: &mut ComponentInstance,
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

    fn error_context_drop(
        &mut self,
        instance: &mut ComponentInstance,
        ty: TypeComponentLocalErrorContextTableIndex,
        err_ctx_handle: u32,
    ) -> Result<()> {
        _ = (instance, ty, err_ctx_handle);
        todo!()
    }
}
