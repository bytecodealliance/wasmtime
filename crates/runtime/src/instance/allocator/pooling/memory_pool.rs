use super::{
    index_allocator::{MemoryInModule, ModuleAffinityIndexAllocator, SlotId},
    MemoryAllocationIndex,
};
use crate::{
    CompiledModuleId, InstanceAllocationRequest, Memory, MemoryImageSlot, Mmap,
    PoolingInstanceAllocatorConfig,
};
use anyhow::{anyhow, bail, Context, Result};
use libc::c_void;
use std::sync::Mutex;
use wasmtime_environ::{
    DefinedMemoryIndex, MemoryPlan, MemoryStyle, Module, Tunables, WASM_PAGE_SIZE,
};

/// Represents a pool of WebAssembly linear memories.
///
/// A linear memory is divided into accessible pages and guard pages.
///
/// A diagram for this struct's fields is:
///
/// ```ignore
///                       memory_size
///                           /
///         max_accessible   /                    memory_and_guard_size
///                 |       /                               |
///              <--+--->  /                    <-----------+---------->
///              <--------+->
///
/// +-----------+--------+---+-----------+     +--------+---+-----------+
/// | PROT_NONE |            | PROT_NONE | ... |            | PROT_NONE |
/// +-----------+--------+---+-----------+     +--------+---+-----------+
/// |           |<------------------+---------------------------------->
/// \           |                    \
/// mapping     |            `max_total_memories` memories
///            /
///    initial_memory_offset
/// ```
#[derive(Debug)]
pub struct MemoryPool {
    mapping: Mmap,
    index_allocator: ModuleAffinityIndexAllocator,
    // If using a copy-on-write allocation scheme, the slot management. We
    // dynamically transfer ownership of a slot to a Memory when in
    // use.
    image_slots: Vec<Mutex<Option<MemoryImageSlot>>>,
    // The size, in bytes, of each linear memory's reservation, not including
    // any guard region.
    memory_size: usize,
    // The size, in bytes, of each linear memory's reservation plus the trailing
    // guard region allocated for it.
    memory_and_guard_size: usize,
    // The maximum size that can become accessible, in bytes, of each linear
    // memory. Guaranteed to be a whole number of wasm pages.
    max_accessible: usize,
    // The size, in bytes, of the offset to the first linear memory in this
    // pool. This is here to help account for the first region of guard pages,
    // if desired, before the first linear memory.
    initial_memory_offset: usize,
    // The maximum number of memories that can be allocated concurrently, aka
    // our pool's capacity.
    max_total_memories: usize,
    // The maximum number of memories that a single core module instance may
    // use.
    //
    // NB: this is needed for validation but does not affect the pool's size.
    memories_per_instance: usize,
    // How much linear memory, in bytes, to keep resident after resetting for
    // use with the next instance. This much memory will be `memset` to zero
    // when a linear memory is deallocated.
    //
    // Memory exceeding this amount in the wasm linear memory will be released
    // with `madvise` back to the kernel.
    //
    // Only applicable on Linux.
    keep_resident: usize,
}

impl MemoryPool {
    /// Create a new `MemoryPool`.
    pub fn new(config: &PoolingInstanceAllocatorConfig, tunables: &Tunables) -> Result<Self> {
        // The maximum module memory page count cannot exceed 65536 pages
        if config.limits.memory_pages > 0x10000 {
            bail!(
                "module memory page limit of {} exceeds the maximum of 65536",
                config.limits.memory_pages
            );
        }

        // Interpret the larger of the maximal size of memory or the static
        // memory bound as the size of the virtual address space reservation for
        // memory itself. Typically `static_memory_bound` is 4G which helps
        // elide most bounds checks in wasm. If `memory_pages` is larger,
        // though, then this is a non-moving pooling allocator so create larger
        // reservations for account for that.
        let memory_size = config.limits.memory_pages.max(tunables.static_memory_bound)
            * u64::from(WASM_PAGE_SIZE);

        let memory_and_guard_size =
            usize::try_from(memory_size + tunables.static_memory_offset_guard_size)
                .map_err(|_| anyhow!("memory reservation size exceeds addressable memory"))?;

        assert!(
            memory_and_guard_size % crate::page_size() == 0,
            "memory size {} is not a multiple of system page size",
            memory_and_guard_size
        );

        let max_total_memories = config.limits.total_memories as usize;
        let initial_memory_offset = if tunables.guard_before_linear_memory {
            usize::try_from(tunables.static_memory_offset_guard_size).unwrap()
        } else {
            0
        };

        // The entire allocation here is the size of each memory (with guard
        // regions) times the total number of memories in the pool.
        //
        // Note, though, that guard regions are required to be after each linear
        // memory. If the `guard_before_linear_memory` setting is specified,
        // then due to the contiguous layout of linear memories the guard pages
        // after one memory are also guard pages preceding the next linear
        // memory. This means that we only need to handle pre-guard-page sizes
        // specially for the first linear memory, hence the
        // `initial_memory_offset` variable here. If guards aren't specified
        // before linear memories this is set to `0`, otherwise it's set to
        // the same size as guard regions for other memories.
        let allocation_size = memory_and_guard_size
            .checked_mul(max_total_memories)
            .and_then(|c| c.checked_add(initial_memory_offset))
            .ok_or_else(|| {
                anyhow!("total size of memory reservation exceeds addressable memory")
            })?;

        // Create a completely inaccessible region to start
        let mapping = Mmap::accessible_reserved(0, allocation_size)
            .context("failed to create memory pool mapping")?;

        let image_slots: Vec<_> = std::iter::repeat_with(|| Mutex::new(None))
            .take(max_total_memories)
            .collect();

        let pool = Self {
            index_allocator: ModuleAffinityIndexAllocator::new(
                config.limits.total_memories,
                config.max_unused_warm_slots,
            ),
            mapping,
            image_slots,
            memory_size: memory_size.try_into().unwrap(),
            memory_and_guard_size,
            initial_memory_offset,
            max_total_memories,
            memories_per_instance: usize::try_from(config.limits.max_memories_per_module).unwrap(),
            max_accessible: (config.limits.memory_pages as usize) * (WASM_PAGE_SIZE as usize),
            keep_resident: config.linear_memory_keep_resident,
        };

        Ok(pool)
    }

    /// Validate whether this memory pool supports the given module.
    pub fn validate(&self, module: &Module) -> Result<()> {
        let memories = module.memory_plans.len() - module.num_imported_memories;
        if memories > usize::try_from(self.memories_per_instance).unwrap() {
            bail!(
                "defined memories count of {} exceeds the per-instance limit of {}",
                memories,
                self.memories_per_instance,
            );
        }

        for (i, plan) in module
            .memory_plans
            .iter()
            .skip(module.num_imported_memories)
        {
            match plan.style {
                MemoryStyle::Static { bound } => {
                    if u64::try_from(self.memory_size).unwrap() < bound {
                        bail!(
                            "memory size allocated per-memory is too small to \
                             satisfy static bound of {bound:#x} pages"
                        );
                    }
                }
                MemoryStyle::Dynamic { .. } => {}
            }
            let max = self.max_accessible / (WASM_PAGE_SIZE as usize);
            if plan.memory.minimum > u64::try_from(max).unwrap() {
                bail!(
                    "memory index {} has a minimum page size of {} which exceeds the limit of {}",
                    i.as_u32(),
                    plan.memory.minimum,
                    max,
                );
            }
        }
        Ok(())
    }

    /// Are zero slots in use right now?
    pub fn is_empty(&self) -> bool {
        self.index_allocator.is_empty()
    }

    /// Allocate a single memory for the given instance allocation request.
    pub fn allocate(
        &self,
        request: &mut InstanceAllocationRequest,
        memory_plan: &MemoryPlan,
        memory_index: DefinedMemoryIndex,
    ) -> Result<(MemoryAllocationIndex, Memory)> {
        let allocation_index = self
            .index_allocator
            .alloc(
                request
                    .runtime_info
                    .unique_id()
                    .map(|id| MemoryInModule(id, memory_index)),
            )
            .map(|slot| MemoryAllocationIndex(u32::try_from(slot.index()).unwrap()))
            .ok_or_else(|| {
                anyhow!(
                    "maximum concurrent memory limit of {} reached",
                    self.max_total_memories
                )
            })?;

        match (|| {
            // Double-check that the runtime requirements of the memory are
            // satisfied by the configuration of this pooling allocator. This
            // should be returned as an error through `validate_memory_plans`
            // but double-check here to be sure.
            match memory_plan.style {
                MemoryStyle::Static { bound } => {
                    let bound = bound * u64::from(WASM_PAGE_SIZE);
                    assert!(bound <= u64::try_from(self.memory_size).unwrap());
                }
                MemoryStyle::Dynamic { .. } => {}
            }

            let base_ptr = self.get_base(allocation_index);
            let base_capacity = self.max_accessible;

            let mut slot = self.take_memory_image_slot(allocation_index);
            let image = request.runtime_info.memory_image(memory_index)?;
            let initial_size = memory_plan.memory.minimum * WASM_PAGE_SIZE as u64;

            // If instantiation fails, we can propagate the error
            // upward and drop the slot. This will cause the Drop
            // handler to attempt to map the range with PROT_NONE
            // memory, to reserve the space while releasing any
            // stale mappings. The next use of this slot will then
            // create a new slot that will try to map over
            // this, returning errors as well if the mapping
            // errors persist. The unmap-on-drop is best effort;
            // if it fails, then we can still soundly continue
            // using the rest of the pool and allowing the rest of
            // the process to continue, because we never perform a
            // mmap that would leave an open space for someone
            // else to come in and map something.
            slot.instantiate(initial_size as usize, image, memory_plan)?;

            Memory::new_static(
                memory_plan,
                base_ptr,
                base_capacity,
                slot,
                self.memory_and_guard_size,
                unsafe { &mut *request.store.get().unwrap() },
            )
        })() {
            Ok(memory) => Ok((allocation_index, memory)),
            Err(e) => {
                self.index_allocator.free(SlotId(allocation_index.0));
                Err(e)
            }
        }
    }

    /// Deallocate a previously-allocated memory.
    ///
    /// # Safety
    ///
    /// The memory must have been previously allocated from this pool and
    /// assigned the given index, must currently be in an allocated state, and
    /// must never be used again.
    pub unsafe fn deallocate(&self, allocation_index: MemoryAllocationIndex, memory: Memory) {
        let mut image = memory.unwrap_static_image();

        // Reset the image slot. If there is any error clearing the
        // image, just drop it here, and let the drop handler for the
        // slot unmap in a way that retains the address space
        // reservation.
        if image.clear_and_remain_ready(self.keep_resident).is_ok() {
            self.return_memory_image_slot(allocation_index, image);
        }

        self.index_allocator.free(SlotId(allocation_index.0));
    }

    /// Purging everything related to `module`.
    pub fn purge_module(&self, module: CompiledModuleId) {
        // This primarily means clearing out all of its memory images present in
        // the virtual address space. Go through the index allocator for slots
        // affine to `module` and reset them, freeing up the index when we're
        // done.
        //
        // Note that this is only called when the specified `module` won't be
        // allocated further (the module is being dropped) so this shouldn't hit
        // any sort of infinite loop since this should be the final operation
        // working with `module`.
        //
        // TODO: We are given a module id, but key affinity by pair of module id
        // and defined memory index. We are missing any defined memory index or
        // count of how many memories the module defines here. Therefore, we
        // probe up to the maximum number of memories per instance. This is fine
        // because that maximum is generally relatively small. If this method
        // somehow ever gets hot because of unnecessary probing, we should
        // either pass in the actual number of defined memories for the given
        // module to this method, or keep a side table of all slots that are
        // associated with a module (not just module and memory). The latter
        // would require care to make sure that its maintenance wouldn't be too
        // expensive for normal allocation/free operations.
        for i in 0..self.memories_per_instance {
            use wasmtime_environ::EntityRef;
            let memory_index = DefinedMemoryIndex::new(i);
            while let Some(id) = self
                .index_allocator
                .alloc_affine_and_clear_affinity(module, memory_index)
            {
                // Clear the image from the slot and, if successful, return it back
                // to our state. Note that on failure here the whole slot will get
                // paved over with an anonymous mapping.
                let index = MemoryAllocationIndex(id.0);
                let mut slot = self.take_memory_image_slot(index);
                if slot.remove_image().is_ok() {
                    self.return_memory_image_slot(index, slot);
                }

                self.index_allocator.free(id);
            }
        }
    }

    fn get_base(&self, allocation_index: MemoryAllocationIndex) -> *mut u8 {
        assert!(allocation_index.index() < self.max_total_memories);
        let offset =
            self.initial_memory_offset + allocation_index.index() * self.memory_and_guard_size;
        unsafe { self.mapping.as_ptr().offset(offset as isize).cast_mut() }
    }

    /// Take ownership of the given image slot. Must be returned via
    /// `return_memory_image_slot` when the instance is done using it.
    fn take_memory_image_slot(&self, allocation_index: MemoryAllocationIndex) -> MemoryImageSlot {
        let maybe_slot = self.image_slots[allocation_index.index()]
            .lock()
            .unwrap()
            .take();

        maybe_slot.unwrap_or_else(|| {
            MemoryImageSlot::create(
                self.get_base(allocation_index) as *mut c_void,
                0,
                self.max_accessible,
            )
        })
    }

    /// Return ownership of the given image slot.
    fn return_memory_image_slot(
        &self,
        allocation_index: MemoryAllocationIndex,
        slot: MemoryImageSlot,
    ) {
        assert!(!slot.is_dirty());
        *self.image_slots[allocation_index.index()].lock().unwrap() = Some(slot);
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        // Clear the `clear_no_drop` flag (i.e., ask to *not* clear on
        // drop) for all slots, and then drop them here. This is
        // valid because the one `Mmap` that covers the whole region
        // can just do its one munmap.
        for mut slot in std::mem::take(&mut self.image_slots) {
            if let Some(slot) = slot.get_mut().unwrap() {
                slot.no_clear_on_drop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InstanceLimits, PoolingInstanceAllocator};
    use wasmtime_environ::WASM_PAGE_SIZE;

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_memory_pool() -> Result<()> {
        let pool = MemoryPool::new(
            &PoolingInstanceAllocatorConfig {
                limits: InstanceLimits {
                    total_memories: 5,
                    max_tables_per_module: 0,
                    max_memories_per_module: 3,
                    table_elements: 0,
                    memory_pages: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
            &Tunables {
                static_memory_bound: 1,
                static_memory_offset_guard_size: 0,
                ..Tunables::default()
            },
        )?;

        assert_eq!(pool.memory_and_guard_size, WASM_PAGE_SIZE as usize);
        assert_eq!(pool.max_total_memories, 5);
        assert_eq!(pool.max_accessible, WASM_PAGE_SIZE as usize);

        let base = pool.mapping.as_ptr() as usize;

        for i in 0..5 {
            let index = MemoryAllocationIndex(i);
            let ptr = pool.get_base(index);
            assert_eq!(ptr as usize - base, i as usize * pool.memory_and_guard_size);
        }

        Ok(())
    }

    #[test]
    fn test_pooling_allocator_with_reservation_size_exceeded() {
        let config = PoolingInstanceAllocatorConfig {
            limits: InstanceLimits {
                total_memories: 1,
                memory_pages: 2,
                ..Default::default()
            },
            ..PoolingInstanceAllocatorConfig::default()
        };
        let pool = PoolingInstanceAllocator::new(
            &config,
            &Tunables {
                static_memory_bound: 1,
                static_memory_offset_guard_size: 0,
                ..Tunables::default()
            },
        )
        .unwrap();
        assert_eq!(pool.memories.memory_size, 2 * 65536);
    }
}
