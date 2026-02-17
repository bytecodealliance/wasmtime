//! GC-related methods for stores.

use super::*;
use crate::runtime::vm::VMGcRef;

impl StoreOpaque {
    /// Attempt to grow the GC heap by `bytes_needed` or, if that fails, perform
    /// a garbage collection.
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
            .grow_or_collect_gc_heap(limiter, bytes_needed, asyncness)
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

    async fn grow_or_collect_gc_heap(
        &mut self,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        bytes_needed: Option<u64>,
        asyncness: Asyncness,
    ) {
        if let Some(n) = bytes_needed {
            if self.grow_gc_heap(limiter, n).await.is_ok() {
                return;
            }
        }
        self.do_gc(asyncness).await;
    }

    /// Attempt to grow the GC heap by `bytes_needed` bytes.
    ///
    /// Returns an error if growing the GC heap fails.
    async fn grow_gc_heap(
        &mut self,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        bytes_needed: u64,
    ) -> Result<()> {
        log::trace!("Attempting to grow the GC heap by {bytes_needed} bytes");
        assert!(bytes_needed > 0);

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

    /// Attempt an allocation, if it fails due to GC OOM, then do a GC and
    /// retry.
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
                    let (value, oom) = oom.take_inner();
                    self.gc(limiter, None, Some(oom.bytes_needed()), asyncness)
                        .await;
                    alloc_func(self, value)
                }
                Err(e) => Err(e),
            },
        }
    }
}
