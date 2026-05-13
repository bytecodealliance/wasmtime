//! GC-related methods for stores.

use crate::store::{
    Asyncness, AutoAssertNoGc, InstanceId, StoreOpaque, StoreResourceLimiter, yield_now,
};
use crate::type_registry::RegisteredType;
use crate::vm::{self, Backtrace, Frame, GcRootsList, GcStore, SendSyncPtr, VMGcRef};
use crate::{
    ExnRef, GcHeapOutOfMemory, Result, Rooted, Store, StoreContextMut, ThrownException, bail,
    format_err,
};
use core::mem::ManuallyDrop;
use core::num::NonZeroU32;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use wasmtime_environ::DefinedTagIndex;

impl<T> Store<T> {
    /// Perform garbage collection.
    ///
    /// Note that it is not required to actively call this function. GC will
    /// automatically happen according to various internal heuristics. This is
    /// provided if fine-grained control over the GC is desired.
    ///
    /// If you are calling this method after an attempted allocation failed, you
    /// may pass in the [`GcHeapOutOfMemory`][crate::GcHeapOutOfMemory] error.
    /// When you do so, this method will attempt to create enough space in the
    /// GC heap for that allocation, so that it will succeed on the next
    /// attempt.
    ///
    /// # Errors
    ///
    /// This method will fail if an [async limiter is
    /// configured](Store::limiter_async) in which case [`Store::gc_async`] must
    /// be used instead.
    pub fn gc(&mut self, why: Option<&crate::GcHeapOutOfMemory<()>>) -> Result<()> {
        StoreContextMut(&mut self.inner).gc(why)
    }

    /// Returns the current capacity of the GC heap in bytes, or 0 if the GC
    /// heap has not been initialized yet.
    pub fn gc_heap_capacity(&self) -> usize {
        self.inner.gc_heap_capacity()
    }

    /// Set an exception as the currently pending exception, and
    /// return an error that propagates the throw.
    ///
    /// This method takes an exception object and stores it in the
    /// `Store` as the currently pending exception. This is a special
    /// rooted slot that holds the exception as long as it is
    /// propagating. This method then returns a `ThrownException`
    /// error, which is a special type that indicates a pending
    /// exception exists. When this type propagates as an error
    /// returned from a Wasm-to-host call, the pending exception is
    /// thrown within the Wasm context, and either caught or
    /// propagated further to the host-to-Wasm call boundary. If an
    /// exception is thrown out of Wasm (or across Wasm from a
    /// hostcall) back to the host-to-Wasm call boundary, *that*
    /// invocation returns a `ThrownException`, and the pending
    /// exception slot is again set. In other words, the
    /// `ThrownException` error type should propagate upward exactly
    /// and only when a pending exception is set.
    ///
    /// To take the pending exception, use [`Self::take_pending_exception`].
    ///
    /// This method is parameterized over `R` for convenience, but
    /// will always return an `Err`.
    ///
    /// If there is already a pending exception in the store then the previous
    /// one will be overwritten.
    ///
    /// # Errors
    ///
    /// This method will return an error if `exception` is unrooted. Otherwise
    /// this method will always return `ThrownException`.
    pub fn throw<R>(&mut self, exception: Rooted<ExnRef>) -> Result<R> {
        self.inner.throw_impl(exception)
    }

    /// Take the currently pending exception, if any, and return it,
    /// removing it from the "pending exception" slot.
    ///
    /// If there is no pending exception, returns `None`.
    ///
    /// Note: the returned exception is a LIFO root (see
    /// [`crate::Rooted`]), rooted in the current handle scope. Take
    /// care to ensure that it is re-rooted or otherwise does not
    /// escape this scope! It is usually best to allow an exception
    /// object to be rooted in the store's "pending exception" slot
    /// until the final consumer has taken it, rather than root it and
    /// pass it up the callstack in some other way.
    ///
    /// This method is useful to implement ad-hoc exception plumbing
    /// in various ways, but for the most idiomatic handling, see
    /// [`StoreContextMut::throw`].
    pub fn take_pending_exception(&mut self) -> Option<Rooted<ExnRef>> {
        self.inner.take_pending_exception_rooted()
    }
}

impl<'a, T> StoreContextMut<'a, T> {
    /// Perform garbage collection.
    ///
    /// Same as [`Store::gc`].
    pub fn gc(&mut self, why: Option<&GcHeapOutOfMemory<()>>) -> Result<()> {
        let (mut limiter, store) = self.0.validate_sync_resource_limiter_and_store_opaque()?;
        vm::assert_ready(store.gc(
            limiter.as_mut(),
            None,
            why.map(|e| e.bytes_needed()),
            Asyncness::No,
        ));
        Ok(())
    }

    /// Set an exception as the currently pending exception, and
    /// return an error that propagates the throw.
    ///
    /// See [`Store::throw`] for more details.
    #[cfg(feature = "gc")]
    pub fn throw<R>(&mut self, exception: Rooted<ExnRef>) -> Result<R> {
        self.0.inner.throw_impl(exception)
    }

    /// Take the currently pending exception, if any, and return it,
    /// removing it from the "pending exception" slot.
    ///
    /// See [`Store::take_pending_exception`] for more details.
    #[cfg(feature = "gc")]
    pub fn take_pending_exception(&mut self) -> Option<Rooted<ExnRef>> {
        self.0.inner.take_pending_exception_rooted()
    }
}

impl StoreOpaque {
    /// Perform any growth or GC needed to allocate `bytes_needed` bytes.
    ///
    /// Note that even when this function returns it is not guaranteed
    /// that a GC allocation of size `bytes_needed` will succeed. Growing the GC
    /// heap could fail, and then performing a collection could succeed but
    /// might not free up enough space. Therefore, callers should not assume
    /// that a retried allocation will always succeed.
    ///
    /// The `root` argument passed in is considered a root for this GC operation
    /// and its new value is returned as well.
    pub(crate) async fn gc(
        &mut self,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        root: Option<VMGcRef>,
        bytes_needed: Option<u64>,
        asyncness: Asyncness,
    ) -> Option<VMGcRef> {
        let mut scope = crate::OpaqueRootScope::new(self);
        scope.trim_gc_liveness_flags(true);
        let store_id = scope.id();
        let root = root.map(|r| scope.gc_roots_mut().push_lifo_root(store_id, r));

        scope
            .collect_and_maybe_grow_gc_heap(limiter, bytes_needed, asyncness)
            .await;

        root.map(|r| {
            let r = r
                .get_gc_ref(&scope)
                .expect("still in scope")
                .unchecked_copy();
            scope.clone_gc_ref(&r)
        })
    }

    // This lives on the Store because it must simultaneously borrow
    // `gc_store` and `gc_roots`, and is invoked from other modules to
    // which we do not want to expose the raw fields for piecewise
    // borrows.
    pub(crate) fn trim_gc_liveness_flags(&mut self, eager: bool) {
        if let Some(gc_store) = self.gc_store.as_mut() {
            self.gc_roots.trim_liveness_flags(gc_store, eager);
        }
    }

    /// Helper invoked as part of `gc`, whose purpose is to GC and
    /// maybe grow for a pending allocation of a given size.
    async fn collect_and_maybe_grow_gc_heap(
        &mut self,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        bytes_needed: Option<u64>,
        asyncness: Asyncness,
    ) {
        log::trace!("collect_and_maybe_grow_gc_heap(bytes_needed = {bytes_needed:#x?})");
        self.do_gc(asyncness).await;
        if let Some(n) = bytes_needed
            && n > u64::try_from(self.gc_heap_capacity())
                .unwrap()
                .saturating_sub(self.gc_store.as_ref().map_or(0, |gc| {
                    u64::try_from(gc.last_post_gc_allocated_bytes.unwrap_or(0)).unwrap()
                }))
        {
            let _ = self.grow_gc_heap(limiter, n, asyncness).await;
        }
    }

    /// Attempt to grow the GC heap by `bytes_needed` bytes.
    ///
    /// Returns an error if growing the GC heap fails.
    pub(crate) async fn grow_gc_heap(
        &mut self,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        bytes_needed: u64,
        asyncness: Asyncness,
    ) -> Result<()> {
        log::trace!("Attempting to grow the GC heap by at least {bytes_needed:#x} bytes");

        if bytes_needed == 0 {
            return Ok(());
        }

        // If the GC heap needs a collection before growth (e.g. the copying
        // collector's active space is the second half), do a GC first.
        if self
            .gc_store
            .as_ref()
            .map_or(false, |gc| gc.gc_heap.needs_gc_before_next_growth())
        {
            self.do_gc(asyncness).await;
            debug_assert!(
                !self
                    .gc_store
                    .as_ref()
                    .map_or(false, |gc| gc.gc_heap.needs_gc_before_next_growth()),
                "needs_gc_before_next_growth should return false after a GC"
            );
        }

        let page_size = self.engine().tunables().gc_heap_memory_type().page_size();

        // Take the GC heap's underlying memory out of the GC heap, attempt to
        // grow it, then replace it.
        let mut heap = TakenGcHeap::new(self);

        let current_size_in_bytes = u64::try_from(heap.memory.byte_size()).unwrap();
        let current_size_in_pages = current_size_in_bytes / page_size;

        // Aim to double the heap size, amortizing the cost of growth.
        let doubled_size_in_pages = current_size_in_pages.saturating_mul(2);
        assert!(doubled_size_in_pages >= current_size_in_pages);
        let delta_pages_for_doubling = doubled_size_in_pages - current_size_in_pages;

        // When doubling our size, saturate at the maximum memory size in pages.
        //
        // TODO: we should consult the instance allocator for its configured
        // maximum memory size, if any, rather than assuming the index
        // type's maximum size.
        let max_size_in_bytes = 1 << 32;
        let max_size_in_pages = max_size_in_bytes / page_size;
        let delta_to_max_size_in_pages = max_size_in_pages - current_size_in_pages;
        let delta_pages_for_alloc = delta_pages_for_doubling.min(delta_to_max_size_in_pages);

        // But always make sure we are attempting to grow at least as many pages
        // as needed by the requested allocation. This must happen *after* the
        // max-size saturation, so that if we are at the max already, we do not
        // succeed in growing by zero delta pages, and then return successfully
        // to our caller, who would be assuming that there is now capacity for
        // their allocation.
        let pages_needed = bytes_needed.div_ceil(page_size);
        assert!(pages_needed > 0);
        let delta_pages_for_alloc = delta_pages_for_alloc.max(pages_needed);
        assert!(delta_pages_for_alloc > 0);

        // Safety: we pair growing the GC heap with updating its associated
        // `VMMemoryDefinition` in the `VMStoreContext` immediately
        // afterwards.
        unsafe {
            heap.memory
                .grow(delta_pages_for_alloc, limiter)
                .await?
                .ok_or_else(|| format_err!("failed to grow GC heap"))?;
        }
        heap.store.vm_store_context.gc_heap = heap.memory.vmmemory();

        let new_size_in_bytes = u64::try_from(heap.memory.byte_size()).unwrap();
        assert!(new_size_in_bytes > current_size_in_bytes);
        heap.delta_bytes_grown = new_size_in_bytes - current_size_in_bytes;
        let delta_bytes_for_alloc = delta_pages_for_alloc.checked_mul(page_size).unwrap();
        assert!(
            heap.delta_bytes_grown >= delta_bytes_for_alloc,
            "{} should be greater than or equal to {delta_bytes_for_alloc}",
            heap.delta_bytes_grown,
        );
        log::trace!(
            "  -> grew GC heap by {:#x} bytes: new size is {new_size_in_bytes:#x} bytes",
            heap.delta_bytes_grown
        );
        return Ok(());

        struct TakenGcHeap<'a> {
            store: &'a mut StoreOpaque,
            memory: ManuallyDrop<vm::Memory>,
            delta_bytes_grown: u64,
        }

        impl<'a> TakenGcHeap<'a> {
            fn new(store: &'a mut StoreOpaque) -> TakenGcHeap<'a> {
                TakenGcHeap {
                    memory: ManuallyDrop::new(store.unwrap_gc_store_mut().gc_heap.take_memory()),
                    store,
                    delta_bytes_grown: 0,
                }
            }
        }

        impl Drop for TakenGcHeap<'_> {
            fn drop(&mut self) {
                // SAFETY: this `Drop` guard ensures that this has exclusive
                // ownership of fields and is thus safe to take `self.memory`.
                // Additionally for `replace_memory` the memory was previously
                // taken when this was created so it should be safe to place
                // back inside the GC heap.
                unsafe {
                    self.store.unwrap_gc_store_mut().gc_heap.replace_memory(
                        ManuallyDrop::take(&mut self.memory),
                        self.delta_bytes_grown,
                    );
                }
            }
        }
    }

    fn replace_gc_zeal_alloc_counter(
        &mut self,
        new_value: Option<NonZeroU32>,
    ) -> Option<NonZeroU32> {
        if let Some(gc_store) = &mut self.gc_store {
            gc_store.replace_gc_zeal_alloc_counter(new_value)
        } else {
            None
        }
    }

    /// Attempt an allocation, if it fails due to GC OOM, apply the
    /// grow-or-collect heuristic and retry.
    ///
    /// The heuristic is:
    /// - If the last post-collection heap usage is less than half the current
    ///   capacity, collect first, then retry. If that still fails, grow and
    ///   retry one final time.
    /// - Otherwise, grow first and retry.
    pub(crate) async fn retry_after_gc_async<T, U>(
        &mut self,
        mut limiter: Option<&mut StoreResourceLimiter<'_>>,
        value: T,
        asyncness: Asyncness,
        alloc_func: impl Fn(&mut Self, T) -> Result<U>,
    ) -> Result<U>
    where
        T: Send + Sync + 'static,
    {
        self.ensure_gc_store(limiter.as_deref_mut()).await?;

        match alloc_func(self, value) {
            Ok(x) => Ok(x),
            Err(e) => match e.downcast::<crate::GcHeapOutOfMemory<T>>() {
                Ok(oom) => {
                    log::trace!("Got GC heap OOM: {oom}");

                    let (value, oom) = oom.take_inner();
                    let bytes_needed = oom.bytes_needed();

                    let mut store = WithoutGcZealAllocCounter::new(self);

                    let gc_heap_capacity = store
                        .gc_store
                        .as_ref()
                        .map_or(0, |gc_store| gc_store.gc_heap_capacity());
                    let last_gc_heap_usage = store.gc_store.as_ref().map_or(0, |gc_store| {
                        gc_store.last_post_gc_allocated_bytes.unwrap_or(0)
                    });

                    if should_collect_first(bytes_needed, gc_heap_capacity, last_gc_heap_usage) {
                        log::trace!(
                            "Collecting first, then retrying; growing GC heap if collecting didn't \
                             free up enough space, then retrying again"
                        );
                        store
                            .gc(limiter.as_deref_mut(), None, None, asyncness)
                            .await;

                        match alloc_func(&mut store, value) {
                            Ok(x) => Ok(x),
                            Err(e) => match e.downcast::<crate::GcHeapOutOfMemory<T>>() {
                                Ok(oom2) => {
                                    // Collection wasn't enough; grow and try
                                    // one final time.
                                    let (value, _) = oom2.take_inner();
                                    // Ignore error; we'll get one from
                                    // `alloc_func` below if growth failed and
                                    // failure to grow was fatal.
                                    let _ =
                                        store.grow_gc_heap(limiter, bytes_needed, asyncness).await;

                                    alloc_func(&mut store, value)
                                }
                                Err(e) => Err(e),
                            },
                        }
                    } else {
                        log::trace!(
                            "Grow GC heap first, collecting if growth failed, then retrying"
                        );

                        if let Err(e) = store
                            .grow_gc_heap(limiter.as_deref_mut(), bytes_needed.max(1), asyncness)
                            .await
                        {
                            log::trace!("growing GC heap failed: {e}");
                            store.gc(limiter, None, None, asyncness).await;
                        }

                        alloc_func(&mut store, value)
                    }
                }
                Err(e) => Err(e),
            },
        }
    }

    /// Set a pending exception.
    ///
    /// The `exnref` is cloned internally and held on this store to be fetched
    /// later by an unwind. This method does *not* set up an unwind request on
    /// the TLS call state; that must be done separately.
    ///
    /// GC barriers are not required by the caller of this function.
    pub(crate) fn set_pending_exception(&mut self, exnref: &VMGcRef) -> ThrownException {
        debug_assert!(exnref.is_exnref(&*self.unwrap_gc_store_mut().gc_heap));
        let gc_store = self.gc_store.as_mut().unwrap();
        gc_store.write_gc_ref(&mut self.pending_exception, Some(exnref));
        ThrownException
    }

    /// Takes the pending exception from this store, if any, and exposes it to
    /// WebAssembly, returning the raw representation.
    pub(crate) fn expose_pending_exception_to_wasm(&mut self) -> Option<NonZeroU32> {
        let exnref = self.pending_exception.take()?;
        let gc_store = self.unwrap_gc_store_mut();
        debug_assert!(exnref.is_exnref(&*gc_store.gc_heap));
        Some(gc_store.expose_gc_ref_to_wasm(exnref))
    }

    /// Takes the pending exception of the store, yielding ownership of its
    /// reference to the `Rooted` that's returned.
    fn take_pending_exception_rooted(&mut self) -> Option<Rooted<ExnRef>> {
        let vmexnref = self.pending_exception.take()?;
        debug_assert!(vmexnref.is_exnref(&*self.unwrap_gc_store().gc_heap));
        let mut nogc = AutoAssertNoGc::new(self);
        Some(Rooted::new(&mut nogc, vmexnref))
    }

    /// Returns the (instance,tag) pair that the pending exception in this
    /// store, if any, references.
    pub(crate) fn pending_exception_tag_and_instance(
        &mut self,
    ) -> Option<(InstanceId, DefinedTagIndex)> {
        let pending_exnref = self.pending_exception.as_ref()?.unchecked_copy();
        debug_assert!(pending_exnref.is_exnref(&*self.unwrap_gc_store_mut().gc_heap));
        let mut store = AutoAssertNoGc::new(self);
        Some(
            pending_exnref
                .into_exnref_unchecked()
                .tag(&mut store)
                .expect("cannot read tag"),
        )
    }

    /// Get an owned rooted reference to the pending exception,
    /// without taking it off the store.
    #[cfg(feature = "debug")]
    pub(crate) fn pending_exception_owned_rooted(
        &mut self,
    ) -> Result<Option<crate::OwnedRooted<ExnRef>>, crate::OutOfMemory> {
        let pending = match &self.pending_exception {
            Some(r) => r,
            None => return Ok(None),
        };
        let cloned = self.gc_store.as_mut().unwrap().clone_gc_ref(pending);
        let mut nogc = AutoAssertNoGc::new(self);
        Ok(Some(crate::OwnedRooted::new(&mut nogc, cloned)?))
    }

    /// Stores `exception` within the store to later get thrown.
    ///
    /// Delegates to `self.set_pending_exception` after accessing the internal
    /// exception pointer.
    fn throw_impl<R>(&mut self, exception: Rooted<ExnRef>) -> Result<R> {
        let exception = exception.try_gc_ref(self)?.unchecked_copy();
        Err(self.set_pending_exception(&exception).into())
    }

    /// Helper method to require that a `GcStore` was previously allocated for
    /// this store, failing if it has not yet been allocated.
    ///
    /// Note that this should only be used in a context where allocation of a
    /// `GcStore` is sure to have already happened prior, otherwise this may
    /// return a confusing error to embedders which is a bug in Wasmtime.
    ///
    /// Some situations where it's safe to call this method:
    ///
    /// * There's already a non-null and non-i31 `VMGcRef` in scope. By existing
    ///   this shows proof that the `GcStore` was previously allocated.
    /// * During instantiation and instance's `needs_gc_heap` flag will be
    ///   handled and instantiation will automatically create a GC store.
    #[inline]
    pub(crate) fn require_gc_store(&self) -> Result<&GcStore> {
        match &self.gc_store {
            Some(gc_store) => Ok(gc_store),
            None => bail!("GC heap not initialized yet"),
        }
    }

    /// Same as [`Self::require_gc_store`], but mutable.
    #[inline]
    pub(crate) fn require_gc_store_mut(&mut self) -> Result<&mut GcStore> {
        match &mut self.gc_store {
            Some(gc_store) => Ok(gc_store),
            None => bail!("GC heap not initialized yet"),
        }
    }

    /// Returns the current capacity of the GC heap in bytes, or 0 if the GC
    /// heap has not been initialized yet.
    pub(crate) fn gc_heap_capacity(&self) -> usize {
        match self.gc_store.as_ref() {
            Some(gc_store) => gc_store.gc_heap_capacity(),
            None => 0,
        }
    }

    async fn do_gc(&mut self, asyncness: Asyncness) {
        // If the GC heap hasn't been initialized, there is nothing to collect.
        if self.gc_store.is_none() {
            return;
        }

        log::trace!("============ Begin GC ===========");

        // Take the GC roots out of `self` so we can borrow it mutably but still
        // call mutable methods on `self`.
        let mut roots = core::mem::take(&mut self.gc_roots_list);

        self.trace_roots(&mut roots, asyncness).await;
        self.unwrap_gc_store_mut()
            .gc(
                asyncness,
                unsafe { roots.iter() },
                // TODO: Once `Config` has an optional `AsyncFn` field for
                // yielding to the current async runtime
                // (e.g. `tokio::task::yield_now`), use that if set; otherwise
                // fall back to the runtime-agnostic code.
                yield_now,
            )
            .await;

        // Restore the GC roots for the next GC.
        roots.clear();
        self.gc_roots_list = roots;

        log::trace!("============ End GC ===========");
    }

    async fn trace_roots(&mut self, gc_roots_list: &mut GcRootsList, asyncness: Asyncness) {
        log::trace!("Begin trace GC roots");

        // We shouldn't have any leftover, stale GC roots.
        assert!(gc_roots_list.is_empty());

        self.trace_wasm_stack_roots(gc_roots_list);
        if asyncness != Asyncness::No {
            self.yield_now().await;
        }

        #[cfg(feature = "stack-switching")]
        {
            self.trace_wasm_continuation_roots(gc_roots_list);
            if asyncness != Asyncness::No {
                self.yield_now().await;
            }
        }

        self.trace_vmctx_roots(gc_roots_list);
        if asyncness != Asyncness::No {
            self.yield_now().await;
        }

        self.trace_instance_roots(gc_roots_list);
        if asyncness != Asyncness::No {
            self.yield_now().await;
        }

        self.trace_user_roots(gc_roots_list);
        if asyncness != Asyncness::No {
            self.yield_now().await;
        }

        self.trace_pending_exception_roots(gc_roots_list);

        log::trace!("End trace GC roots")
    }

    fn trace_wasm_stack_frame(&self, gc_roots_list: &mut GcRootsList, frame: Frame) {
        let pc = frame.pc();
        debug_assert!(pc != 0, "we should always get a valid PC for Wasm frames");

        let fp = frame.fp() as *mut usize;
        debug_assert!(
            !fp.is_null(),
            "we should always get a valid frame pointer for Wasm frames"
        );

        let (module_with_code, _offset) = self
            .modules()
            .module_and_code_by_pc(pc)
            .expect("should have module info for Wasm frame");

        if let Some(stack_map) = module_with_code.lookup_stack_map(pc) {
            log::trace!(
                "We have a stack map that maps {} bytes in this Wasm frame",
                stack_map.frame_size()
            );

            let sp = unsafe { stack_map.sp(fp) };
            for stack_slot in unsafe { stack_map.live_gc_refs(sp) } {
                unsafe {
                    self.trace_wasm_stack_slot(gc_roots_list, stack_slot);
                }
            }
        }

        #[cfg(feature = "debug")]
        if let Some(frame_table) = module_with_code.module().frame_table() {
            let relpc = module_with_code
                .text_offset(pc)
                .expect("PC should be within module");
            for stack_slot in crate::debug::gc_refs_in_frame(frame_table, relpc, fp) {
                unsafe {
                    self.trace_wasm_stack_slot(gc_roots_list, stack_slot);
                }
            }
        }
    }

    unsafe fn trace_wasm_stack_slot(&self, gc_roots_list: &mut GcRootsList, stack_slot: *mut u32) {
        let raw: u32 = unsafe { core::ptr::read(stack_slot) };
        log::trace!("Stack slot @ {stack_slot:p} = {raw:#x}");

        let gc_ref = vm::VMGcRef::from_raw_u32(raw);
        if gc_ref.is_some() {
            unsafe {
                gc_roots_list
                    .add_wasm_stack_root(SendSyncPtr::new(NonNull::new(stack_slot).unwrap()));
            }
        }
    }

    fn trace_wasm_stack_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        log::trace!("Begin trace GC roots :: Wasm stack");

        Backtrace::trace(self, |frame| {
            self.trace_wasm_stack_frame(gc_roots_list, frame);
            core::ops::ControlFlow::Continue(())
        });

        log::trace!("End trace GC roots :: Wasm stack");
    }

    #[cfg(feature = "stack-switching")]
    fn trace_wasm_continuation_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        use crate::vm::VMStackState;

        log::trace!("Begin trace GC roots :: continuations");

        for continuation in &self.continuations {
            let state = continuation.common_stack_information.state;

            // FIXME(frank-emrich) In general, it is not enough to just trace
            // through the stacks of continuations; we also need to look through
            // their `cont.bind` arguments. However, we don't currently have
            // enough RTTI information to check if any of the values in the
            // buffers used by `cont.bind` are GC values. As a workaround, note
            // that we currently disallow cont.bind-ing GC values altogether.
            // This way, it is okay not to check them here.
            match state {
                VMStackState::Suspended => {
                    Backtrace::trace_suspended_continuation(self, continuation.deref(), |frame| {
                        self.trace_wasm_stack_frame(gc_roots_list, frame);
                        core::ops::ControlFlow::Continue(())
                    });
                }
                VMStackState::Running => {
                    // Handled by `trace_wasm_stack_roots`.
                }
                VMStackState::Parent => {
                    // We don't know whether our child is suspended or running, but in
                    // either case things should be handled correctly when traversing
                    // further along in the chain, nothing required at this point.
                }
                VMStackState::Fresh | VMStackState::Returned => {
                    // Fresh/Returned continuations have no gc values on their stack.
                }
            }
        }

        log::trace!("End trace GC roots :: continuations");
    }

    fn trace_vmctx_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        log::trace!("Begin trace GC roots :: vmctx");
        self.for_each_global(|store, global| global.trace_root(store, gc_roots_list));
        self.for_each_table(|store, table| table.trace_roots(store, gc_roots_list));
        log::trace!("End trace GC roots :: vmctx");
    }

    fn trace_instance_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        log::trace!("Begin trace GC roots :: instance");
        for (_id, instance) in &mut self.instances {
            // SAFETY: the instance's GC roots will remain valid for the
            // duration of this GC cycle.
            unsafe {
                instance
                    .handle
                    .get_mut()
                    .trace_element_segment_roots(gc_roots_list);
            }
        }
        log::trace!("End trace GC roots :: instance");
    }

    fn trace_user_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        log::trace!("Begin trace GC roots :: user");
        self.gc_roots.trace_roots(gc_roots_list);
        log::trace!("End trace GC roots :: user");
    }

    fn trace_pending_exception_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        log::trace!("Begin trace GC roots :: pending exception");
        if let Some(pending_exception) = self.pending_exception.as_mut() {
            unsafe {
                gc_roots_list.add_vmgcref_root(pending_exception.into(), "Pending exception");
            }
        }
        log::trace!("End trace GC roots :: pending exception");
    }

    /// Insert a host-allocated GC type into this store.
    ///
    /// This makes it suitable for the embedder to allocate instances of this
    /// type in this store, and we don't have to worry about the type being
    /// reclaimed (since it is possible that none of the Wasm modules in this
    /// store are holding it alive).
    pub(crate) fn insert_gc_host_alloc_type(&mut self, ty: RegisteredType) {
        // If a GC heap is already allocated, eagerly register trace info
        // now. Otherwise, trace info will be registered when the GC heap
        // is allocated in `StoreOpaque::allocate_gc_store`.
        if let Some(gc_store) = self.optional_gc_store_mut() {
            gc_store.ensure_trace_info(ty.index());
        }
        self.gc_host_alloc_types.insert(ty);
    }
}

/// RAII type to temporarily disable the GC zeal allocation counter.
struct WithoutGcZealAllocCounter<'a> {
    store: &'a mut StoreOpaque,
    counter: Option<NonZeroU32>,
}

impl Deref for WithoutGcZealAllocCounter<'_> {
    type Target = StoreOpaque;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl DerefMut for WithoutGcZealAllocCounter<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

impl Drop for WithoutGcZealAllocCounter<'_> {
    fn drop(&mut self) {
        self.store.replace_gc_zeal_alloc_counter(self.counter);
    }
}

impl<'a> WithoutGcZealAllocCounter<'a> {
    pub fn new(store: &'a mut StoreOpaque) -> Self {
        let counter = store.replace_gc_zeal_alloc_counter(None);
        WithoutGcZealAllocCounter { store, counter }
    }
}

/// Given that we've hit a `GcHeapOutOfMemory` error, should we try freeing up
/// space by collecting first or by growing the GC heap first?
///
/// * `bytes_needed`: the number of bytes the mutator wants to allocate
///
/// * `gc_heap_capacity`: The current size of the GC heap.
///
/// * `last_gc_heap_usage`: The precise GC heap usage after the last collection.
#[track_caller]
fn should_collect_first(
    bytes_needed: u64,
    gc_heap_capacity: usize,
    last_gc_heap_usage: usize,
) -> bool {
    debug_assert!(last_gc_heap_usage <= gc_heap_capacity);

    // If we haven't allocated the GC heap yet, there's nothing to collect.
    //
    // Make sure to grow in this scenario even when the GC zeal infrastructure
    // passes `bytes_needed = 0`. This way our retry-after-gc logic doesn't
    // auto-fail on its second attempt, which would be bad because it doesn't
    // necessarily retry more than once.
    if gc_heap_capacity == 0 {
        return false;
    }

    // The GC zeal infrastructure will use `bytes_needed = 0` to trigger extra
    // collections.
    if bytes_needed == 0 {
        return true;
    }

    let Ok(bytes_needed) = usize::try_from(bytes_needed) else {
        // No point wasting time on collection if we will never be able to
        // satisfy the allocation.
        return false;
    };

    if bytes_needed > isize::MAX.cast_unsigned() {
        // Similarly, no allocation can be larger than `isize::MAX` in Rust (or
        // LLVM), so don't bother wasting time on collection if we will never be
        // able to satisfy the allocation.
        return false;
    }

    let Some(predicted_usage) = last_gc_heap_usage.checked_add(bytes_needed) else {
        // If we can't represent our predicted usage as a `usize`, we won't be
        // able to grow the GC heap to that size, so try collecting first to
        // free up space.
        return true;
    };

    // Common case: to balance collection frequency (and its time overhead) with
    // GC heap growth (and its space overhead), only prefer growing first if the
    // predicted GC heap utilization is greater than half the GC heap's
    // capacity.
    predicted_usage < gc_heap_capacity / 2
}

#[cfg(test)]
mod tests {
    use super::should_collect_first;

    #[test]
    fn test_should_collect_first() {
        // No GC heap yet special case.
        for bytes_needed in 0..256 {
            assert_eq!(should_collect_first(bytes_needed, 0, 0), false);
        }

        // GC zeal special case.
        for cap in 1..256 {
            for usage in 0..=cap {
                assert_eq!(should_collect_first(0, cap, usage), true);
            }
        }

        let max_alloc_usize = isize::MAX.cast_unsigned();
        let max_alloc_u64 = u64::try_from(max_alloc_usize).unwrap();

        // Allocation size larger than `isize::MAX` --> will never succeed, do
        // not bother collecting.
        assert_eq!(
            should_collect_first(max_alloc_u64 + 1, max_alloc_usize, 0),
            false,
        );

        // Predicted usage overflow --> growth will likely fail, collect first.
        assert_eq!(should_collect_first(1, usize::MAX, usize::MAX), true);

        // Common case: predicted usage is low --> we likely have more than
        // enough space already, so collect first.
        assert_eq!(should_collect_first(16, 1024, 64), true);

        // Common case: predicted usage is high --> plausible we may not have
        // enough space, and we want to amortize the cost of collections, so
        // grow first.
        assert_eq!(should_collect_first(16, 1024, 512), false);
    }
}
