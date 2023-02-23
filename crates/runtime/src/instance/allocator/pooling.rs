//! Implements the pooling instance allocator.
//!
//! The pooling instance allocator maps memory in advance
//! and allocates instances, memories, tables, and stacks from
//! a pool of available resources.
//!
//! Using the pooling instance allocator can speed up module instantiation
//! when modules can be constrained based on configurable limits.

use super::{InstanceAllocationRequest, InstanceAllocator};
use crate::{instance::Instance, Memory, Mmap, Table};
use crate::{CompiledModuleId, MemoryImageSlot};
use anyhow::{anyhow, bail, Context, Result};
use libc::c_void;
use std::convert::TryFrom;
use std::mem;
use std::sync::Mutex;
use wasmtime_environ::{
    DefinedMemoryIndex, DefinedTableIndex, HostPtr, MemoryStyle, Module, PrimaryMap, Tunables,
    VMOffsets, WASM_PAGE_SIZE,
};

mod index_allocator;
use index_allocator::{IndexAllocator, SlotId};

cfg_if::cfg_if! {
    if #[cfg(windows)] {
        mod windows;
        use windows as imp;
    } else {
        mod unix;
        use unix as imp;
    }
}

use imp::{commit_table_pages, decommit_table_pages};

#[cfg(all(feature = "async", unix))]
use imp::{commit_stack_pages, reset_stack_pages_to_zero};

fn round_up_to_pow2(n: usize, to: usize) -> usize {
    debug_assert!(to > 0);
    debug_assert!(to.is_power_of_two());
    (n + to - 1) & !(to - 1)
}

/// Instance-related limit configuration for pooling.
///
/// More docs on this can be found at `wasmtime::PoolingAllocationConfig`.
#[derive(Debug, Copy, Clone)]
pub struct InstanceLimits {
    /// Maximum instances to support
    pub count: u32,

    /// Maximum size of instance VMContext
    pub size: usize,

    /// Maximum number of tables per instance
    pub tables: u32,

    /// Maximum number of table elements per table
    pub table_elements: u32,

    /// Maximum number of linear memories per instance
    pub memories: u32,

    /// Maximum number of wasm pages for each linear memory.
    pub memory_pages: u64,
}

impl Default for InstanceLimits {
    fn default() -> Self {
        // See doc comments for `wasmtime::PoolingAllocationConfig` for these
        // default values
        Self {
            count: 1000,
            size: 1 << 20, // 1 MB
            tables: 1,
            table_elements: 10_000,
            memories: 1,
            memory_pages: 160,
        }
    }
}

/// Represents a pool of WebAssembly linear memories.
///
/// A linear memory is divided into accessible pages and guard pages.
///
/// Each instance index into the pool returns an iterator over the base
/// addresses of the instance's linear memories.
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
/// mapping     |     `max_instances * max_memories` memories
///            /
///    initial_memory_offset
/// ```
#[derive(Debug)]
struct MemoryPool {
    mapping: Mmap,
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
    max_memories: usize,
    max_instances: usize,
}

impl MemoryPool {
    fn new(instance_limits: &InstanceLimits, tunables: &Tunables) -> Result<Self> {
        // The maximum module memory page count cannot exceed 65536 pages
        if instance_limits.memory_pages > 0x10000 {
            bail!(
                "module memory page limit of {} exceeds the maximum of 65536",
                instance_limits.memory_pages
            );
        }

        // Interpret the larger of the maximal size of memory or the static
        // memory bound as the size of the virtual address space reservation for
        // memory itself. Typically `static_memory_bound` is 4G which helps
        // elide most bounds checks in wasm. If `memory_pages` is larger,
        // though, then this is a non-moving pooling allocator so create larger
        // reservations for account for that.
        let memory_size = instance_limits
            .memory_pages
            .max(tunables.static_memory_bound)
            * u64::from(WASM_PAGE_SIZE);

        let memory_and_guard_size =
            usize::try_from(memory_size + tunables.static_memory_offset_guard_size)
                .map_err(|_| anyhow!("memory reservation size exceeds addressable memory"))?;

        assert!(
            memory_and_guard_size % crate::page_size() == 0,
            "memory size {} is not a multiple of system page size",
            memory_and_guard_size
        );

        let max_instances = instance_limits.count as usize;
        let max_memories = instance_limits.memories as usize;
        let initial_memory_offset = if tunables.guard_before_linear_memory {
            usize::try_from(tunables.static_memory_offset_guard_size).unwrap()
        } else {
            0
        };

        // The entire allocation here is the size of each memory times the
        // max memories per instance times the number of instances allowed in
        // this pool, plus guard regions.
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
            .checked_mul(max_memories)
            .and_then(|c| c.checked_mul(max_instances))
            .and_then(|c| c.checked_add(initial_memory_offset))
            .ok_or_else(|| {
                anyhow!("total size of memory reservation exceeds addressable memory")
            })?;

        // Create a completely inaccessible region to start
        let mapping = Mmap::accessible_reserved(0, allocation_size)
            .context("failed to create memory pool mapping")?;

        let num_image_slots = max_instances * max_memories;
        let image_slots: Vec<_> = std::iter::repeat_with(|| Mutex::new(None))
            .take(num_image_slots)
            .collect();

        let pool = Self {
            mapping,
            image_slots,
            memory_size: memory_size.try_into().unwrap(),
            memory_and_guard_size,
            initial_memory_offset,
            max_memories,
            max_instances,
            max_accessible: (instance_limits.memory_pages as usize) * (WASM_PAGE_SIZE as usize),
        };

        Ok(pool)
    }

    fn get_base(&self, instance_index: usize, memory_index: DefinedMemoryIndex) -> *mut u8 {
        assert!(instance_index < self.max_instances);
        let memory_index = memory_index.as_u32() as usize;
        assert!(memory_index < self.max_memories);
        let idx = instance_index * self.max_memories + memory_index;
        let offset = self.initial_memory_offset + idx * self.memory_and_guard_size;
        unsafe { self.mapping.as_mut_ptr().offset(offset as isize) }
    }

    #[cfg(test)]
    fn get<'a>(&'a self, instance_index: usize) -> impl Iterator<Item = *mut u8> + 'a {
        (0..self.max_memories)
            .map(move |i| self.get_base(instance_index, DefinedMemoryIndex::from_u32(i as u32)))
    }

    /// Take ownership of the given image slot. Must be returned via
    /// `return_memory_image_slot` when the instance is done using it.
    fn take_memory_image_slot(
        &self,
        instance_index: usize,
        memory_index: DefinedMemoryIndex,
    ) -> MemoryImageSlot {
        let idx = instance_index * self.max_memories + (memory_index.as_u32() as usize);
        let maybe_slot = self.image_slots[idx].lock().unwrap().take();

        maybe_slot.unwrap_or_else(|| {
            MemoryImageSlot::create(
                self.get_base(instance_index, memory_index) as *mut c_void,
                0,
                self.max_accessible,
            )
        })
    }

    /// Return ownership of the given image slot.
    fn return_memory_image_slot(
        &self,
        instance_index: usize,
        memory_index: DefinedMemoryIndex,
        slot: MemoryImageSlot,
    ) {
        assert!(!slot.is_dirty());
        let idx = instance_index * self.max_memories + (memory_index.as_u32() as usize);
        *self.image_slots[idx].lock().unwrap() = Some(slot);
    }

    /// Resets all the images for the instance index slot specified to clear out
    /// any prior mappings.
    ///
    /// This is used when a `Module` is dropped at the `wasmtime` layer to clear
    /// out any remaining mappings and ensure that its memfd backing, if any, is
    /// removed from the address space to avoid lingering references to it.
    fn clear_images(&self, instance_index: usize) {
        for i in 0..self.max_memories {
            let index = DefinedMemoryIndex::from_u32(i as u32);

            // Clear the image from the slot and, if successful, return it back
            // to our state. Note that on failure here the whole slot will get
            // paved over with an anonymous mapping.
            let mut slot = self.take_memory_image_slot(instance_index, index);
            if slot.remove_image().is_ok() {
                self.return_memory_image_slot(instance_index, index, slot);
            }
        }
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

/// Represents a pool of WebAssembly tables.
///
/// Each instance index into the pool returns an iterator over the base addresses
/// of the instance's tables.
#[derive(Debug)]
struct TablePool {
    mapping: Mmap,
    table_size: usize,
    max_tables: usize,
    max_instances: usize,
    page_size: usize,
    max_elements: u32,
}

impl TablePool {
    fn new(instance_limits: &InstanceLimits) -> Result<Self> {
        let page_size = crate::page_size();

        let table_size = round_up_to_pow2(
            mem::size_of::<*mut u8>()
                .checked_mul(instance_limits.table_elements as usize)
                .ok_or_else(|| anyhow!("table size exceeds addressable memory"))?,
            page_size,
        );

        let max_instances = instance_limits.count as usize;
        let max_tables = instance_limits.tables as usize;

        let allocation_size = table_size
            .checked_mul(max_tables)
            .and_then(|c| c.checked_mul(max_instances))
            .ok_or_else(|| anyhow!("total size of instance tables exceeds addressable memory"))?;

        let mapping = Mmap::accessible_reserved(allocation_size, allocation_size)
            .context("failed to create table pool mapping")?;

        Ok(Self {
            mapping,
            table_size,
            max_tables,
            max_instances,
            page_size,
            max_elements: instance_limits.table_elements,
        })
    }

    fn get(&self, instance_index: usize) -> impl Iterator<Item = *mut u8> {
        assert!(instance_index < self.max_instances);

        let base: *mut u8 = unsafe {
            self.mapping
                .as_mut_ptr()
                .add(instance_index * self.table_size * self.max_tables) as _
        };

        let size = self.table_size;
        (0..self.max_tables).map(move |i| unsafe { base.add(i * size) })
    }
}

/// Represents a pool of execution stacks (used for the async fiber implementation).
///
/// Each index into the pool represents a single execution stack. The maximum number of
/// stacks is the same as the maximum number of instances.
///
/// As stacks grow downwards, each stack starts (lowest address) with a guard page
/// that can be used to detect stack overflow.
///
/// The top of the stack (starting stack pointer) is returned when a stack is allocated
/// from the pool.
#[cfg(all(feature = "async", unix))]
#[derive(Debug)]
struct StackPool {
    mapping: Mmap,
    stack_size: usize,
    max_instances: usize,
    page_size: usize,
    index_allocator: IndexAllocator,
    async_stack_zeroing: bool,
    async_stack_keep_resident: usize,
}

#[cfg(all(feature = "async", unix))]
impl StackPool {
    fn new(config: &PoolingInstanceAllocatorConfig) -> Result<Self> {
        use rustix::mm::{mprotect, MprotectFlags};

        let page_size = crate::page_size();

        // Add a page to the stack size for the guard page when using fiber stacks
        let stack_size = if config.stack_size == 0 {
            0
        } else {
            round_up_to_pow2(config.stack_size, page_size)
                .checked_add(page_size)
                .ok_or_else(|| anyhow!("stack size exceeds addressable memory"))?
        };

        let max_instances = config.limits.count as usize;

        let allocation_size = stack_size
            .checked_mul(max_instances)
            .ok_or_else(|| anyhow!("total size of execution stacks exceeds addressable memory"))?;

        let mapping = Mmap::accessible_reserved(allocation_size, allocation_size)
            .context("failed to create stack pool mapping")?;

        // Set up the stack guard pages
        if allocation_size > 0 {
            unsafe {
                for i in 0..max_instances {
                    // Make the stack guard page inaccessible
                    let bottom_of_stack = mapping.as_mut_ptr().add(i * stack_size);
                    mprotect(bottom_of_stack.cast(), page_size, MprotectFlags::empty())
                        .context("failed to protect stack guard page")?;
                }
            }
        }

        Ok(Self {
            mapping,
            stack_size,
            max_instances,
            page_size,
            async_stack_zeroing: config.async_stack_zeroing,
            async_stack_keep_resident: config.async_stack_keep_resident,
            // Note that `max_unused_warm_slots` is set to zero since stacks
            // have no affinity so there's no need to keep intentionally unused
            // warm slots around.
            index_allocator: IndexAllocator::new(config.limits.count, 0),
        })
    }

    fn allocate(&self) -> Result<wasmtime_fiber::FiberStack> {
        if self.stack_size == 0 {
            bail!("pooling allocator not configured to enable fiber stack allocation");
        }

        let index = self
            .index_allocator
            .alloc(None)
            .ok_or_else(|| {
                anyhow!(
                    "maximum concurrent fiber limit of {} reached",
                    self.max_instances
                )
            })?
            .index();

        assert!(index < self.max_instances);

        unsafe {
            // Remove the guard page from the size
            let size_without_guard = self.stack_size - self.page_size;

            let bottom_of_stack = self
                .mapping
                .as_mut_ptr()
                .add((index * self.stack_size) + self.page_size);

            commit_stack_pages(bottom_of_stack, size_without_guard)?;

            let stack =
                wasmtime_fiber::FiberStack::from_top_ptr(bottom_of_stack.add(size_without_guard))?;
            Ok(stack)
        }
    }

    fn deallocate(&self, stack: &wasmtime_fiber::FiberStack) {
        let top = stack
            .top()
            .expect("fiber stack not allocated from the pool") as usize;

        let base = self.mapping.as_ptr() as usize;
        let len = self.mapping.len();
        assert!(
            top > base && top <= (base + len),
            "fiber stack top pointer not in range"
        );

        // Remove the guard page from the size
        let stack_size = self.stack_size - self.page_size;
        let bottom_of_stack = top - stack_size;
        let start_of_stack = bottom_of_stack - self.page_size;
        assert!(start_of_stack >= base && start_of_stack < (base + len));
        assert!((start_of_stack - base) % self.stack_size == 0);

        let index = (start_of_stack - base) / self.stack_size;
        assert!(index < self.max_instances);

        if self.async_stack_zeroing {
            self.zero_stack(bottom_of_stack, stack_size);
        }

        self.index_allocator.free(SlotId(index as u32));
    }

    fn zero_stack(&self, bottom: usize, size: usize) {
        // Manually zero the top of the stack to keep the pages resident in
        // memory and avoid future page faults. Use the system to deallocate
        // pages past this. This hopefully strikes a reasonable balance between:
        //
        // * memset for the whole range is probably expensive
        // * madvise for the whole range incurs expensive future page faults
        // * most threads probably don't use most of the stack anyway
        let size_to_memset = size.min(self.async_stack_keep_resident);
        unsafe {
            std::ptr::write_bytes(
                (bottom + size - size_to_memset) as *mut u8,
                0,
                size_to_memset,
            );
        }

        // Use the system to reset remaining stack pages to zero.
        reset_stack_pages_to_zero(bottom as _, size - size_to_memset).unwrap();
    }
}

/// Configuration options for the pooling instance allocator supplied at
/// construction.
#[derive(Copy, Clone, Debug)]
pub struct PoolingInstanceAllocatorConfig {
    /// See `PoolingAllocatorConfig::max_unused_warm_slots` in `wasmtime`
    pub max_unused_warm_slots: u32,
    /// The size, in bytes, of async stacks to allocate (not including the guard
    /// page).
    pub stack_size: usize,
    /// The limits to apply to instances allocated within this allocator.
    pub limits: InstanceLimits,
    /// Whether or not async stacks are zeroed after use.
    pub async_stack_zeroing: bool,
    /// If async stack zeroing is enabled and the host platform is Linux this is
    /// how much memory to zero out with `memset`.
    ///
    /// The rest of memory will be zeroed out with `madvise`.
    pub async_stack_keep_resident: usize,
    /// How much linear memory, in bytes, to keep resident after resetting for
    /// use with the next instance. This much memory will be `memset` to zero
    /// when a linear memory is deallocated.
    ///
    /// Memory exceeding this amount in the wasm linear memory will be released
    /// with `madvise` back to the kernel.
    ///
    /// Only applicable on Linux.
    pub linear_memory_keep_resident: usize,
    /// Same as `linear_memory_keep_resident` but for tables.
    pub table_keep_resident: usize,
}

impl Default for PoolingInstanceAllocatorConfig {
    fn default() -> PoolingInstanceAllocatorConfig {
        PoolingInstanceAllocatorConfig {
            max_unused_warm_slots: 100,
            stack_size: 2 << 20,
            limits: InstanceLimits::default(),
            async_stack_zeroing: false,
            async_stack_keep_resident: 0,
            linear_memory_keep_resident: 0,
            table_keep_resident: 0,
        }
    }
}

/// Implements the pooling instance allocator.
///
/// This allocator internally maintains pools of instances, memories, tables, and stacks.
///
/// Note: the resource pools are manually dropped so that the fault handler terminates correctly.
#[derive(Debug)]
pub struct PoolingInstanceAllocator {
    instance_size: usize,
    max_instances: usize,
    index_allocator: IndexAllocator,
    memories: MemoryPool,
    tables: TablePool,
    linear_memory_keep_resident: usize,
    table_keep_resident: usize,

    #[cfg(all(feature = "async", unix))]
    stacks: StackPool,
    #[cfg(all(feature = "async", windows))]
    stack_size: usize,
}

impl PoolingInstanceAllocator {
    /// Creates a new pooling instance allocator with the given strategy and limits.
    pub fn new(config: &PoolingInstanceAllocatorConfig, tunables: &Tunables) -> Result<Self> {
        if config.limits.count == 0 {
            bail!("the instance count limit cannot be zero");
        }

        let max_instances = config.limits.count as usize;

        Ok(Self {
            instance_size: round_up_to_pow2(config.limits.size, mem::align_of::<Instance>()),
            max_instances,
            index_allocator: IndexAllocator::new(config.limits.count, config.max_unused_warm_slots),
            memories: MemoryPool::new(&config.limits, tunables)?,
            tables: TablePool::new(&config.limits)?,
            linear_memory_keep_resident: config.linear_memory_keep_resident,
            table_keep_resident: config.table_keep_resident,
            #[cfg(all(feature = "async", unix))]
            stacks: StackPool::new(config)?,
            #[cfg(all(feature = "async", windows))]
            stack_size: config.stack_size,
        })
    }

    fn reset_table_pages_to_zero(&self, base: *mut u8, size: usize) -> Result<()> {
        let size_to_memset = size.min(self.table_keep_resident);
        unsafe {
            std::ptr::write_bytes(base, 0, size_to_memset);
            decommit_table_pages(base.add(size_to_memset), size - size_to_memset)?;
        }
        Ok(())
    }

    fn validate_table_plans(&self, module: &Module) -> Result<()> {
        let tables = module.table_plans.len() - module.num_imported_tables;
        if tables > self.tables.max_tables {
            bail!(
                "defined tables count of {} exceeds the limit of {}",
                tables,
                self.tables.max_tables,
            );
        }

        for (i, plan) in module.table_plans.iter().skip(module.num_imported_tables) {
            if plan.table.minimum > self.tables.max_elements {
                bail!(
                    "table index {} has a minimum element size of {} which exceeds the limit of {}",
                    i.as_u32(),
                    plan.table.minimum,
                    self.tables.max_elements,
                );
            }
        }
        Ok(())
    }

    fn validate_memory_plans(&self, module: &Module) -> Result<()> {
        let memories = module.memory_plans.len() - module.num_imported_memories;
        if memories > self.memories.max_memories {
            bail!(
                "defined memories count of {} exceeds the limit of {}",
                memories,
                self.memories.max_memories,
            );
        }

        for (i, plan) in module
            .memory_plans
            .iter()
            .skip(module.num_imported_memories)
        {
            match plan.style {
                MemoryStyle::Static { bound } => {
                    if (self.memories.memory_size as u64) < bound {
                        bail!(
                            "memory size allocated per-memory is too small to \
                             satisfy static bound of {bound:#x} pages"
                        );
                    }
                }
                MemoryStyle::Dynamic { .. } => {}
            }
            let max = self.memories.max_accessible / (WASM_PAGE_SIZE as usize);
            if plan.memory.minimum > (max as u64) {
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

    fn validate_instance_size(&self, offsets: &VMOffsets<HostPtr>) -> Result<()> {
        let layout = Instance::alloc_layout(offsets);
        if layout.size() <= self.instance_size {
            return Ok(());
        }

        // If this `module` exceeds the allocation size allotted to it then an
        // error will be reported here. The error of "required N bytes but
        // cannot allocate that" is pretty opaque, however, because it's not
        // clear what the breakdown of the N bytes are and what to optimize
        // next. To help provide a better error message here some fancy-ish
        // logic is done here to report the breakdown of the byte request into
        // the largest portions and where it's coming from.
        let mut message = format!(
            "instance allocation for this module \
             requires {} bytes which exceeds the configured maximum \
             of {} bytes; breakdown of allocation requirement:\n\n",
            layout.size(),
            self.instance_size,
        );

        let mut remaining = layout.size();
        let mut push = |name: &str, bytes: usize| {
            assert!(remaining >= bytes);
            remaining -= bytes;

            // If the `name` region is more than 5% of the allocation request
            // then report it here, otherwise ignore it. We have less than 20
            // fields so we're guaranteed that something should be reported, and
            // otherwise it's not particularly interesting to learn about 5
            // different fields that are all 8 or 0 bytes. Only try to report
            // the "major" sources of bytes here.
            if bytes > layout.size() / 20 {
                message.push_str(&format!(
                    " * {:.02}% - {} bytes - {}\n",
                    ((bytes as f32) / (layout.size() as f32)) * 100.0,
                    bytes,
                    name,
                ));
            }
        };

        // The `Instance` itself requires some size allocated to it.
        push("instance state management", mem::size_of::<Instance>());

        // Afterwards the `VMContext`'s regions are why we're requesting bytes,
        // so ask it for descriptions on each region's byte size.
        for (desc, size) in offsets.region_sizes() {
            push(desc, size as usize);
        }

        // double-check we accounted for all the bytes
        assert_eq!(remaining, 0);

        bail!("{}", message)
    }
}

unsafe impl InstanceAllocator for PoolingInstanceAllocator {
    fn validate(&self, module: &Module, offsets: &VMOffsets<HostPtr>) -> Result<()> {
        self.validate_memory_plans(module)?;
        self.validate_table_plans(module)?;
        self.validate_instance_size(offsets)?;

        Ok(())
    }

    fn allocate_index(&self, req: &InstanceAllocationRequest) -> Result<usize> {
        self.index_allocator
            .alloc(req.runtime_info.unique_id())
            .map(|id| id.index())
            .ok_or_else(|| {
                anyhow!(
                    "maximum concurrent instance limit of {} reached",
                    self.max_instances
                )
            })
    }

    fn deallocate_index(&self, index: usize) {
        self.index_allocator.free(SlotId(index as u32));
    }

    fn allocate_memories(
        &self,
        index: usize,
        req: &mut InstanceAllocationRequest,
        memories: &mut PrimaryMap<DefinedMemoryIndex, Memory>,
    ) -> Result<()> {
        let module = req.runtime_info.module();

        self.validate_memory_plans(module)?;

        for (memory_index, plan) in module
            .memory_plans
            .iter()
            .skip(module.num_imported_memories)
        {
            let defined_index = module
                .defined_memory_index(memory_index)
                .expect("should be a defined memory since we skipped imported ones");

            // Double-check that the runtime requirements of the memory are
            // satisfied by the configuration of this pooling allocator. This
            // should be returned as an error through `validate_memory_plans`
            // but double-check here to be sure.
            match plan.style {
                MemoryStyle::Static { bound } => {
                    let bound = bound * u64::from(WASM_PAGE_SIZE);
                    assert!(bound <= (self.memories.memory_size as u64));
                }
                MemoryStyle::Dynamic { .. } => {}
            }

            let memory = unsafe {
                std::slice::from_raw_parts_mut(
                    self.memories.get_base(index, defined_index),
                    self.memories.max_accessible,
                )
            };

            let mut slot = self.memories.take_memory_image_slot(index, defined_index);
            let image = req.runtime_info.memory_image(defined_index)?;
            let initial_size = plan.memory.minimum * WASM_PAGE_SIZE as u64;

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
            slot.instantiate(initial_size as usize, image, &plan.style)?;

            memories.push(Memory::new_static(plan, memory, slot, unsafe {
                &mut *req.store.get().unwrap()
            })?);
        }

        Ok(())
    }

    fn deallocate_memories(&self, index: usize, mems: &mut PrimaryMap<DefinedMemoryIndex, Memory>) {
        // Decommit any linear memories that were used.
        for (def_mem_idx, memory) in mem::take(mems) {
            let mut image = memory.unwrap_static_image();
            // Reset the image slot. If there is any error clearing the
            // image, just drop it here, and let the drop handler for the
            // slot unmap in a way that retains the address space
            // reservation.
            if image
                .clear_and_remain_ready(self.linear_memory_keep_resident)
                .is_ok()
            {
                self.memories
                    .return_memory_image_slot(index, def_mem_idx, image);
            }
        }
    }

    fn allocate_tables(
        &self,
        index: usize,
        req: &mut InstanceAllocationRequest,
        tables: &mut PrimaryMap<DefinedTableIndex, Table>,
    ) -> Result<()> {
        let module = req.runtime_info.module();

        self.validate_table_plans(module)?;

        let mut bases = self.tables.get(index);
        for (_, plan) in module.table_plans.iter().skip(module.num_imported_tables) {
            let base = bases.next().unwrap() as _;

            commit_table_pages(
                base as *mut u8,
                self.tables.max_elements as usize * mem::size_of::<*mut u8>(),
            )?;

            tables.push(Table::new_static(
                plan,
                unsafe { std::slice::from_raw_parts_mut(base, self.tables.max_elements as usize) },
                unsafe { &mut *req.store.get().unwrap() },
            )?);
        }

        Ok(())
    }

    fn deallocate_tables(&self, index: usize, tables: &mut PrimaryMap<DefinedTableIndex, Table>) {
        // Decommit any tables that were used
        for (table, base) in tables.values_mut().zip(self.tables.get(index)) {
            let table = mem::take(table);
            assert!(table.is_static());

            let size = round_up_to_pow2(
                table.size() as usize * mem::size_of::<*mut u8>(),
                self.tables.page_size,
            );

            drop(table);
            self.reset_table_pages_to_zero(base, size)
                .expect("failed to decommit table pages");
        }
    }

    #[cfg(all(feature = "async", unix))]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack> {
        self.stacks.allocate()
    }

    #[cfg(all(feature = "async", unix))]
    unsafe fn deallocate_fiber_stack(&self, stack: &wasmtime_fiber::FiberStack) {
        self.stacks.deallocate(stack);
    }

    #[cfg(all(feature = "async", windows))]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack> {
        if self.stack_size == 0 {
            bail!("fiber stack allocation not supported")
        }

        // On windows, we don't use a stack pool as we use the native fiber implementation
        let stack = wasmtime_fiber::FiberStack::new(self.stack_size)?;
        Ok(stack)
    }

    #[cfg(all(feature = "async", windows))]
    unsafe fn deallocate_fiber_stack(&self, _stack: &wasmtime_fiber::FiberStack) {
        // A no-op as we don't own the fiber stack on Windows
    }

    fn purge_module(&self, module: CompiledModuleId) {
        // Purging everything related to `module` primarily means clearing out
        // all of its memory images present in the virtual address space. Go
        // through the index allocator for slots affine to `module` and reset
        // them, freeing up the index when we're done.
        //
        // Note that this is only called when the specified `module` won't be
        // allocated further (the module is being dropped) so this shouldn't hit
        // any sort of infinite loop since this should be the final operation
        // working with `module`.
        while let Some(index) = self.index_allocator.alloc_affine_and_clear_affinity(module) {
            self.memories.clear_images(index.index());
            self.index_allocator.free(index);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        CompiledModuleId, Imports, MemoryImage, ModuleRuntimeInfo, StorePtr, VMFunctionBody,
        VMSharedSignatureIndex,
    };
    use std::sync::Arc;
    use wasmtime_environ::{DefinedFuncIndex, DefinedMemoryIndex};

    pub(crate) fn empty_runtime_info(
        module: Arc<wasmtime_environ::Module>,
    ) -> Arc<dyn ModuleRuntimeInfo> {
        struct RuntimeInfo(Arc<wasmtime_environ::Module>, VMOffsets<HostPtr>);

        impl ModuleRuntimeInfo for RuntimeInfo {
            fn module(&self) -> &Arc<wasmtime_environ::Module> {
                &self.0
            }
            fn function(&self, _: DefinedFuncIndex) -> *mut VMFunctionBody {
                unimplemented!()
            }
            fn memory_image(
                &self,
                _: DefinedMemoryIndex,
            ) -> anyhow::Result<Option<&Arc<MemoryImage>>> {
                Ok(None)
            }

            fn unique_id(&self) -> Option<CompiledModuleId> {
                None
            }
            fn wasm_data(&self) -> &[u8] {
                &[]
            }
            fn signature_ids(&self) -> &[VMSharedSignatureIndex] {
                &[]
            }
            fn offsets(&self) -> &VMOffsets<HostPtr> {
                &self.1
            }
        }

        let offsets = VMOffsets::new(HostPtr, &module);
        Arc::new(RuntimeInfo(module, offsets))
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_instance_pool() -> Result<()> {
        let mut config = PoolingInstanceAllocatorConfig::default();
        config.max_unused_warm_slots = 0;
        config.limits = InstanceLimits {
            count: 3,
            tables: 1,
            memories: 1,
            table_elements: 10,
            size: 1000,
            memory_pages: 1,
            ..Default::default()
        };

        let instances = PoolingInstanceAllocator::new(
            &config,
            &Tunables {
                static_memory_bound: 1,
                ..Tunables::default()
            },
        )?;

        assert_eq!(instances.instance_size, 1008); // round 1000 up to alignment
        assert_eq!(instances.max_instances, 3);

        assert_eq!(instances.index_allocator.testing_freelist(), []);

        let mut handles = Vec::new();
        let module = Arc::new(Module::default());

        for _ in (0..3).rev() {
            handles.push(
                instances
                    .allocate(InstanceAllocationRequest {
                        runtime_info: &empty_runtime_info(module.clone()),
                        imports: Imports {
                            functions: &[],
                            tables: &[],
                            memories: &[],
                            globals: &[],
                        },
                        host_state: Box::new(()),
                        store: StorePtr::empty(),
                    })
                    .expect("allocation should succeed"),
            );
        }

        assert_eq!(instances.index_allocator.testing_freelist(), []);

        match instances.allocate(InstanceAllocationRequest {
            runtime_info: &empty_runtime_info(module),
            imports: Imports {
                functions: &[],
                tables: &[],
                memories: &[],
                globals: &[],
            },
            host_state: Box::new(()),
            store: StorePtr::empty(),
        }) {
            Err(_) => {}
            _ => panic!("unexpected error"),
        };

        for mut handle in handles.drain(..) {
            instances.deallocate(&mut handle);
        }

        assert_eq!(
            instances.index_allocator.testing_freelist(),
            [SlotId(0), SlotId(1), SlotId(2)]
        );

        Ok(())
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_memory_pool() -> Result<()> {
        let pool = MemoryPool::new(
            &InstanceLimits {
                count: 5,
                tables: 0,
                memories: 3,
                table_elements: 0,
                memory_pages: 1,
                ..Default::default()
            },
            &Tunables {
                static_memory_bound: 1,
                static_memory_offset_guard_size: 0,
                ..Tunables::default()
            },
        )?;

        assert_eq!(pool.memory_and_guard_size, WASM_PAGE_SIZE as usize);
        assert_eq!(pool.max_memories, 3);
        assert_eq!(pool.max_instances, 5);
        assert_eq!(pool.max_accessible, WASM_PAGE_SIZE as usize);

        let base = pool.mapping.as_ptr() as usize;

        for i in 0..5 {
            let mut iter = pool.get(i);

            for j in 0..3 {
                assert_eq!(
                    iter.next().unwrap() as usize - base,
                    ((i * 3) + j) * pool.memory_and_guard_size
                );
            }

            assert_eq!(iter.next(), None);
        }

        Ok(())
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_table_pool() -> Result<()> {
        let pool = TablePool::new(&InstanceLimits {
            count: 7,
            table_elements: 100,
            memory_pages: 0,
            tables: 4,
            memories: 0,
            ..Default::default()
        })?;

        let host_page_size = crate::page_size();

        assert_eq!(pool.table_size, host_page_size);
        assert_eq!(pool.max_tables, 4);
        assert_eq!(pool.max_instances, 7);
        assert_eq!(pool.page_size, host_page_size);
        assert_eq!(pool.max_elements, 100);

        let base = pool.mapping.as_ptr() as usize;

        for i in 0..7 {
            let mut iter = pool.get(i);

            for j in 0..4 {
                assert_eq!(
                    iter.next().unwrap() as usize - base,
                    ((i * 4) + j) * pool.table_size
                );
            }

            assert_eq!(iter.next(), None);
        }

        Ok(())
    }

    #[cfg(all(unix, target_pointer_width = "64", feature = "async"))]
    #[test]
    fn test_stack_pool() -> Result<()> {
        let config = PoolingInstanceAllocatorConfig {
            limits: InstanceLimits {
                count: 10,
                ..Default::default()
            },
            stack_size: 1,
            async_stack_zeroing: true,
            ..PoolingInstanceAllocatorConfig::default()
        };
        let pool = StackPool::new(&config)?;

        let native_page_size = crate::page_size();
        assert_eq!(pool.stack_size, 2 * native_page_size);
        assert_eq!(pool.max_instances, 10);
        assert_eq!(pool.page_size, native_page_size);

        assert_eq!(pool.index_allocator.testing_freelist(), []);

        let base = pool.mapping.as_ptr() as usize;

        let mut stacks = Vec::new();
        for i in 0..10 {
            let stack = pool.allocate().expect("allocation should succeed");
            assert_eq!(
                ((stack.top().unwrap() as usize - base) / pool.stack_size) - 1,
                i
            );
            stacks.push(stack);
        }

        assert_eq!(pool.index_allocator.testing_freelist(), []);

        pool.allocate().unwrap_err();

        for stack in stacks {
            pool.deallocate(&stack);
        }

        assert_eq!(
            pool.index_allocator.testing_freelist(),
            [
                SlotId(0),
                SlotId(1),
                SlotId(2),
                SlotId(3),
                SlotId(4),
                SlotId(5),
                SlotId(6),
                SlotId(7),
                SlotId(8),
                SlotId(9)
            ],
        );

        Ok(())
    }

    #[test]
    fn test_pooling_allocator_with_zero_instance_count() {
        let config = PoolingInstanceAllocatorConfig {
            limits: InstanceLimits {
                count: 0,
                ..Default::default()
            },
            ..PoolingInstanceAllocatorConfig::default()
        };
        assert_eq!(
            PoolingInstanceAllocator::new(&config, &Tunables::default(),)
                .map_err(|e| e.to_string())
                .expect_err("expected a failure constructing instance allocator"),
            "the instance count limit cannot be zero"
        );
    }

    #[test]
    fn test_pooling_allocator_with_memory_pages_exceeded() {
        let config = PoolingInstanceAllocatorConfig {
            limits: InstanceLimits {
                count: 1,
                memory_pages: 0x10001,
                ..Default::default()
            },
            ..PoolingInstanceAllocatorConfig::default()
        };
        assert_eq!(
            PoolingInstanceAllocator::new(
                &config,
                &Tunables {
                    static_memory_bound: 1,
                    ..Tunables::default()
                },
            )
            .map_err(|e| e.to_string())
            .expect_err("expected a failure constructing instance allocator"),
            "module memory page limit of 65537 exceeds the maximum of 65536"
        );
    }

    #[test]
    fn test_pooling_allocator_with_reservation_size_exceeded() {
        let config = PoolingInstanceAllocatorConfig {
            limits: InstanceLimits {
                count: 1,
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

    #[cfg(all(unix, target_pointer_width = "64", feature = "async"))]
    #[test]
    fn test_stack_zeroed() -> Result<()> {
        let config = PoolingInstanceAllocatorConfig {
            max_unused_warm_slots: 0,
            limits: InstanceLimits {
                count: 1,
                table_elements: 0,
                memory_pages: 0,
                tables: 0,
                memories: 0,
                ..Default::default()
            },
            stack_size: 128,
            async_stack_zeroing: true,
            ..PoolingInstanceAllocatorConfig::default()
        };
        let allocator = PoolingInstanceAllocator::new(&config, &Tunables::default())?;

        unsafe {
            for _ in 0..255 {
                let stack = allocator.allocate_fiber_stack()?;

                // The stack pointer is at the top, so decrement it first
                let addr = stack.top().unwrap().sub(1);

                assert_eq!(*addr, 0);
                *addr = 1;

                allocator.deallocate_fiber_stack(&stack);
            }
        }

        Ok(())
    }

    #[cfg(all(unix, target_pointer_width = "64", feature = "async"))]
    #[test]
    fn test_stack_unzeroed() -> Result<()> {
        let config = PoolingInstanceAllocatorConfig {
            max_unused_warm_slots: 0,
            limits: InstanceLimits {
                count: 1,
                table_elements: 0,
                memory_pages: 0,
                tables: 0,
                memories: 0,
                ..Default::default()
            },
            stack_size: 128,
            async_stack_zeroing: false,
            ..PoolingInstanceAllocatorConfig::default()
        };
        let allocator = PoolingInstanceAllocator::new(&config, &Tunables::default())?;

        unsafe {
            for i in 0..255 {
                let stack = allocator.allocate_fiber_stack()?;

                // The stack pointer is at the top, so decrement it first
                let addr = stack.top().unwrap().sub(1);

                assert_eq!(*addr, i);
                *addr = i + 1;

                allocator.deallocate_fiber_stack(&stack);
            }
        }

        Ok(())
    }
}
