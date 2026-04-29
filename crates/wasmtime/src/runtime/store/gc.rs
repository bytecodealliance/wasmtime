//! GC-related methods for stores.

use super::*;
use crate::runtime::vm::VMGcRef;

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
        log::trace!("Attempting to grow the GC heap by {bytes_needed} bytes");

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

    fn reset_gc_zeal_alloc_counter(&mut self) {
        if let Some(gc_store) = &mut self.gc_store {
            gc_store.reset_gc_zeal_alloc_counter();
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

                    let gc_heap_capacity = self
                        .gc_store
                        .as_ref()
                        .map_or(0, |gc_store| gc_store.gc_heap_capacity());
                    let last_gc_heap_usage = self.gc_store.as_ref().map_or(0, |gc_store| {
                        gc_store.last_post_gc_allocated_bytes.unwrap_or(0)
                    });

                    if should_collect_first(bytes_needed, gc_heap_capacity, last_gc_heap_usage) {
                        log::trace!(
                            "Collecting first, then retrying; growing GC heap if collecting didn't \
                             free up enough space, then retrying again"
                        );
                        self.gc(limiter.as_deref_mut(), None, None, asyncness).await;

                        self.reset_gc_zeal_alloc_counter();
                        match alloc_func(self, value) {
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
                                        self.grow_gc_heap(limiter, bytes_needed, asyncness).await;

                                    self.reset_gc_zeal_alloc_counter();
                                    alloc_func(self, value)
                                }
                                Err(e) => Err(e),
                            },
                        }
                    } else {
                        log::trace!(
                            "Grow GC heap first, collecting if growth failed, then retrying"
                        );

                        if let Err(e) = self
                            .grow_gc_heap(limiter.as_deref_mut(), bytes_needed.max(1), asyncness)
                            .await
                        {
                            log::trace!("growing GC heap failed: {e}");
                            self.gc(limiter, None, None, asyncness).await;
                        }

                        self.reset_gc_zeal_alloc_counter();
                        alloc_func(self, value)
                    }
                }
                Err(e) => Err(e),
            },
        }
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
