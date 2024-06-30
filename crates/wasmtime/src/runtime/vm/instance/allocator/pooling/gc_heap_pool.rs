use super::index_allocator::{SimpleIndexAllocator, SlotId};
use super::GcHeapAllocationIndex;
use crate::prelude::*;
use crate::runtime::vm::{GcHeap, GcRuntime, PoolingInstanceAllocatorConfig, Result};
use std::sync::Mutex;

/// A pool of reusable GC heaps.
pub struct GcHeapPool {
    max_gc_heaps: usize,
    index_allocator: SimpleIndexAllocator,
    heaps: Mutex<Vec<Option<Box<dyn GcHeap>>>>,
}

impl std::fmt::Debug for GcHeapPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcHeapPool")
            .field("max_gc_heaps", &self.max_gc_heaps)
            .field("index_allocator", &self.index_allocator)
            .field("heaps", &"..")
            .finish()
    }
}

impl GcHeapPool {
    /// Create a new `GcHeapPool` with the given configuration.
    pub fn new(config: &PoolingInstanceAllocatorConfig) -> Result<Self> {
        let index_allocator = SimpleIndexAllocator::new(config.limits.total_gc_heaps);
        let max_gc_heaps = usize::try_from(config.limits.total_gc_heaps).unwrap();

        // Each individual GC heap in the pool is lazily allocated. See the
        // `allocate` method.
        let heaps = Mutex::new((0..max_gc_heaps).map(|_| None).collect());

        Ok(Self {
            max_gc_heaps,
            index_allocator,
            heaps,
        })
    }

    /// Are there zero slots in use right now?
    #[allow(unused)] // some cfgs don't use this
    pub fn is_empty(&self) -> bool {
        self.index_allocator.is_empty()
    }

    /// Allocate a single table for the given instance allocation request.
    pub fn allocate(
        &self,
        gc_runtime: &dyn GcRuntime,
    ) -> Result<(GcHeapAllocationIndex, Box<dyn GcHeap>)> {
        let allocation_index = self
            .index_allocator
            .alloc()
            .map(|slot| GcHeapAllocationIndex(slot.0))
            .ok_or_else(|| {
                anyhow!(
                    "maximum concurrent GC heap limit of {} reached",
                    self.max_gc_heaps
                )
            })?;

        let heap = match {
            let mut heaps = self.heaps.lock().unwrap();
            heaps[allocation_index.index()].take()
        } {
            // If we already have a heap at this slot, reuse it.
            Some(heap) => heap,
            // Otherwise, we haven't forced this slot's lazily allocated heap
            // yet. So do that now.
            None => gc_runtime.new_gc_heap()?,
        };

        Ok((allocation_index, heap))
    }

    /// Deallocate a previously-allocated GC heap.
    pub fn deallocate(&self, allocation_index: GcHeapAllocationIndex, mut heap: Box<dyn GcHeap>) {
        heap.reset();

        // NB: Replace the heap before freeing the index. If we did it in the
        // opposite order, a concurrent allocation request could reallocate the
        // index before we have replaced the heap.

        {
            let mut heaps = self.heaps.lock().unwrap();
            let old_entry = std::mem::replace(&mut heaps[allocation_index.index()], Some(heap));
            debug_assert!(old_entry.is_none());
        }

        self.index_allocator.free(SlotId(allocation_index.0));
    }
}
