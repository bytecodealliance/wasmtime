//! Implements the pooling instance allocator.
//!
//! The pooling instance allocator maps memory in advance and allocates
//! instances, memories, tables, and stacks from a pool of available resources.
//! Using the pooling instance allocator can speed up module instantiation when
//! modules can be constrained based on configurable limits
//! ([`InstanceLimits`]). Each new instance is stored in a "slot"; as instances
//! are allocated and freed, these slots are either filled or emptied:
//!
//! ```text
//! ┌──────┬──────┬──────┬──────┬──────┐
//! │Slot 0│Slot 1│Slot 2│Slot 3│......│
//! └──────┴──────┴──────┴──────┴──────┘
//! ```
//!
//! Each slot has a "slot ID"--an index into the pool. Slot IDs are handed out
//! by the [`index_allocator`] module. Note that each kind of pool-allocated
//! item is stored in its own separate pool: [`memory_pool`], [`table_pool`],
//! [`stack_pool`]. See those modules for more details.

mod index_allocator;
mod memory_pool;
mod table_pool;

#[cfg(feature = "gc")]
mod gc_heap_pool;

#[cfg(all(feature = "async"))]
mod generic_stack_pool;
#[cfg(all(feature = "async", unix, not(miri)))]
mod unix_stack_pool;

#[cfg(all(feature = "async"))]
cfg_if::cfg_if! {
    if #[cfg(all(unix, not(miri), not(asan)))] {
        use unix_stack_pool as stack_pool;
    } else {
        use generic_stack_pool as stack_pool;
    }
}

use super::{
    InstanceAllocationRequest, InstanceAllocatorImpl, MemoryAllocationIndex, TableAllocationIndex,
};
use crate::{
    instance::Instance,
    mpk::{self, MpkEnabled, ProtectionKey, ProtectionMask},
    CompiledModuleId, Memory, Table,
};
use anyhow::{bail, Result};
use memory_pool::MemoryPool;
use std::{
    mem,
    sync::atomic::{AtomicU64, Ordering},
};
use table_pool::TablePool;
use wasmtime_environ::{
    DefinedMemoryIndex, DefinedTableIndex, HostPtr, MemoryPlan, Module, TablePlan, Tunables,
    VMOffsets,
};

#[cfg(feature = "gc")]
use super::GcHeapAllocationIndex;
#[cfg(feature = "gc")]
use crate::{GcHeap, GcRuntime};
#[cfg(feature = "gc")]
use gc_heap_pool::GcHeapPool;

#[cfg(feature = "async")]
use stack_pool::StackPool;

#[cfg(feature = "component-model")]
use wasmtime_environ::{
    component::{Component, VMComponentOffsets},
    StaticModuleIndex,
};

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
    /// The maximum number of component instances that may be allocated
    /// concurrently.
    pub total_component_instances: u32,

    /// The maximum size of a component's `VMComponentContext`, not including
    /// any of its inner core modules' `VMContext` sizes.
    pub component_instance_size: usize,

    /// The maximum number of core module instances that may be allocated
    /// concurrently.
    pub total_core_instances: u32,

    /// The maximum number of core module instances that a single component may
    /// transitively contain.
    pub max_core_instances_per_component: u32,

    /// The maximum number of Wasm linear memories that a component may
    /// transitively contain.
    pub max_memories_per_component: u32,

    /// The maximum number of tables that a component may transitively contain.
    pub max_tables_per_component: u32,

    /// The total number of linear memories in the pool, across all instances.
    pub total_memories: u32,

    /// The total number of tables in the pool, across all instances.
    pub total_tables: u32,

    /// The total number of async stacks in the pool, across all instances.
    #[cfg(feature = "async")]
    pub total_stacks: u32,

    /// Maximum size of a core instance's `VMContext`.
    pub core_instance_size: usize,

    /// Maximum number of tables per instance.
    pub max_tables_per_module: u32,

    /// Maximum number of table elements per table.
    pub table_elements: u32,

    /// Maximum number of linear memories per instance.
    pub max_memories_per_module: u32,

    /// Maximum number of Wasm pages for each linear memory.
    pub memory_pages: u64,

    /// The total number of GC heaps in the pool, across all instances.
    #[cfg(feature = "gc")]
    pub total_gc_heaps: u32,
}

impl Default for InstanceLimits {
    fn default() -> Self {
        // See doc comments for `wasmtime::PoolingAllocationConfig` for these
        // default values
        Self {
            total_component_instances: 1000,
            component_instance_size: 1 << 20, // 1 MiB
            total_core_instances: 1000,
            max_core_instances_per_component: 20,
            max_memories_per_component: 20,
            max_tables_per_component: 20,
            total_memories: 1000,
            total_tables: 1000,
            #[cfg(feature = "async")]
            total_stacks: 1000,
            core_instance_size: 1 << 20, // 1 MiB
            max_tables_per_module: 1,
            table_elements: 10_000,
            max_memories_per_module: 1,
            memory_pages: 160,
            #[cfg(feature = "gc")]
            total_gc_heaps: 1000,
        }
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
    /// Whether to enable memory protection keys.
    pub memory_protection_keys: MpkEnabled,
    /// How many memory protection keys to allocate.
    pub max_memory_protection_keys: usize,
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
            memory_protection_keys: MpkEnabled::Disable,
            max_memory_protection_keys: 16,
        }
    }
}

/// Implements the pooling instance allocator.
///
/// This allocator internally maintains pools of instances, memories, tables,
/// and stacks.
///
/// Note: the resource pools are manually dropped so that the fault handler
/// terminates correctly.
#[derive(Debug)]
pub struct PoolingInstanceAllocator {
    limits: InstanceLimits,

    // The number of live core module and component instances at any given
    // time. Note that this can temporarily go over the configured limit. This
    // doesn't mean we have actually overshot, but that we attempted to allocate
    // a new instance and incremented the counter, we've seen (or are about to
    // see) that the counter is beyond the configured threshold, and are going
    // to decrement the counter and return an error but haven't done so yet. See
    // the increment trait methods for more details.
    live_core_instances: AtomicU64,
    live_component_instances: AtomicU64,

    memories: MemoryPool,
    tables: TablePool,

    #[cfg(feature = "gc")]
    gc_heaps: GcHeapPool,

    #[cfg(feature = "async")]
    stacks: StackPool,
}

impl Drop for PoolingInstanceAllocator {
    fn drop(&mut self) {
        debug_assert_eq!(self.live_component_instances.load(Ordering::Acquire), 0);
        debug_assert_eq!(self.live_core_instances.load(Ordering::Acquire), 0);

        debug_assert!(self.memories.is_empty());
        debug_assert!(self.tables.is_empty());

        #[cfg(feature = "gc")]
        debug_assert!(self.gc_heaps.is_empty());

        #[cfg(feature = "async")]
        debug_assert!(self.stacks.is_empty());
    }
}

impl PoolingInstanceAllocator {
    /// Creates a new pooling instance allocator with the given strategy and limits.
    pub fn new(config: &PoolingInstanceAllocatorConfig, tunables: &Tunables) -> Result<Self> {
        Ok(Self {
            limits: config.limits,
            live_component_instances: AtomicU64::new(0),
            live_core_instances: AtomicU64::new(0),
            memories: MemoryPool::new(config, tunables)?,
            tables: TablePool::new(config)?,
            #[cfg(feature = "gc")]
            gc_heaps: GcHeapPool::new(config)?,
            #[cfg(feature = "async")]
            stacks: StackPool::new(config)?,
        })
    }

    fn core_instance_size(&self) -> usize {
        round_up_to_pow2(self.limits.core_instance_size, mem::align_of::<Instance>())
    }

    fn validate_table_plans(&self, module: &Module) -> Result<()> {
        self.tables.validate(module)
    }

    fn validate_memory_plans(&self, module: &Module) -> Result<()> {
        self.memories.validate(module)
    }

    fn validate_core_instance_size(&self, offsets: &VMOffsets<HostPtr>) -> Result<()> {
        let layout = Instance::alloc_layout(offsets);
        if layout.size() <= self.core_instance_size() {
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
            self.core_instance_size(),
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

    #[cfg(feature = "component-model")]
    fn validate_component_instance_size(
        &self,
        offsets: &VMComponentOffsets<HostPtr>,
    ) -> Result<()> {
        if usize::try_from(offsets.size_of_vmctx()).unwrap() <= self.limits.component_instance_size
        {
            return Ok(());
        }

        // TODO: Add context with detailed accounting of what makes up all the
        // `VMComponentContext`'s space like we do for module instances.
        bail!(
            "instance allocation for this component requires {} bytes of `VMComponentContext` \
             space which exceeds the configured maximum of {} bytes",
            offsets.size_of_vmctx(),
            self.limits.component_instance_size
        )
    }
}

unsafe impl InstanceAllocatorImpl for PoolingInstanceAllocator {
    #[cfg(feature = "component-model")]
    fn validate_component_impl<'a>(
        &self,
        component: &Component,
        offsets: &VMComponentOffsets<HostPtr>,
        get_module: &'a dyn Fn(StaticModuleIndex) -> &'a Module,
    ) -> Result<()> {
        self.validate_component_instance_size(offsets)?;

        let mut num_core_instances = 0;
        let mut num_memories = 0;
        let mut num_tables = 0;
        for init in &component.initializers {
            use wasmtime_environ::component::GlobalInitializer::*;
            use wasmtime_environ::component::InstantiateModule;
            match init {
                InstantiateModule(InstantiateModule::Import(_, _)) => {
                    num_core_instances += 1;
                    // Can't statically account for the total vmctx size, number
                    // of memories, and number of tables in this component.
                }
                InstantiateModule(InstantiateModule::Static(static_module_index, _)) => {
                    let module = get_module(*static_module_index);
                    let offsets = VMOffsets::new(HostPtr, &module);
                    self.validate_module_impl(module, &offsets)?;
                    num_core_instances += 1;
                    num_memories += module.memory_plans.len() - module.num_imported_memories;
                    num_tables += module.table_plans.len() - module.num_imported_tables;
                }
                LowerImport { .. }
                | ExtractMemory(_)
                | ExtractRealloc(_)
                | ExtractPostReturn(_)
                | Resource(_) => {}
            }
        }

        if num_core_instances
            > usize::try_from(self.limits.max_core_instances_per_component).unwrap()
        {
            bail!(
                "The component transitively contains {num_core_instances} core module instances, \
                 which exceeds the configured maximum of {}",
                self.limits.max_core_instances_per_component
            );
        }

        if num_memories > usize::try_from(self.limits.max_memories_per_component).unwrap() {
            bail!(
                "The component transitively contains {num_memories} Wasm linear memories, which \
                 exceeds the configured maximum of {}",
                self.limits.max_memories_per_component
            );
        }

        if num_tables > usize::try_from(self.limits.max_tables_per_component).unwrap() {
            bail!(
                "The component transitively contains {num_tables} tables, which exceeds the \
                 configured maximum of {}",
                self.limits.max_tables_per_component
            );
        }

        Ok(())
    }

    fn validate_module_impl(&self, module: &Module, offsets: &VMOffsets<HostPtr>) -> Result<()> {
        self.validate_memory_plans(module)?;
        self.validate_table_plans(module)?;
        self.validate_core_instance_size(offsets)?;
        Ok(())
    }

    fn increment_component_instance_count(&self) -> Result<()> {
        let old_count = self.live_component_instances.fetch_add(1, Ordering::AcqRel);
        if old_count >= u64::from(self.limits.total_component_instances) {
            self.decrement_component_instance_count();
            bail!(
                "maximum concurrent component instance limit of {} reached",
                self.limits.total_component_instances
            );
        }
        Ok(())
    }

    fn decrement_component_instance_count(&self) {
        self.live_component_instances.fetch_sub(1, Ordering::AcqRel);
    }

    fn increment_core_instance_count(&self) -> Result<()> {
        let old_count = self.live_core_instances.fetch_add(1, Ordering::AcqRel);
        if old_count >= u64::from(self.limits.total_core_instances) {
            self.decrement_core_instance_count();
            bail!(
                "maximum concurrent core instance limit of {} reached",
                self.limits.total_core_instances
            );
        }
        Ok(())
    }

    fn decrement_core_instance_count(&self) {
        self.live_core_instances.fetch_sub(1, Ordering::AcqRel);
    }

    unsafe fn allocate_memory(
        &self,
        request: &mut InstanceAllocationRequest,
        memory_plan: &MemoryPlan,
        memory_index: DefinedMemoryIndex,
    ) -> Result<(MemoryAllocationIndex, Memory)> {
        self.memories.allocate(request, memory_plan, memory_index)
    }

    unsafe fn deallocate_memory(
        &self,
        _memory_index: DefinedMemoryIndex,
        allocation_index: MemoryAllocationIndex,
        memory: Memory,
    ) {
        self.memories.deallocate(allocation_index, memory);
    }

    unsafe fn allocate_table(
        &self,
        request: &mut InstanceAllocationRequest,
        table_plan: &TablePlan,
        _table_index: DefinedTableIndex,
    ) -> Result<(super::TableAllocationIndex, Table)> {
        self.tables.allocate(request, table_plan)
    }

    unsafe fn deallocate_table(
        &self,
        _table_index: DefinedTableIndex,
        allocation_index: TableAllocationIndex,
        table: Table,
    ) {
        self.tables.deallocate(allocation_index, table);
    }

    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack> {
        self.stacks.allocate()
    }

    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, stack: &wasmtime_fiber::FiberStack) {
        self.stacks.deallocate(stack);
    }

    fn purge_module(&self, module: CompiledModuleId) {
        self.memories.purge_module(module);
    }

    fn next_available_pkey(&self) -> Option<ProtectionKey> {
        self.memories.next_available_pkey()
    }

    fn restrict_to_pkey(&self, pkey: ProtectionKey) {
        mpk::allow(ProtectionMask::zero().or(pkey));
    }

    fn allow_all_pkeys(&self) {
        mpk::allow(ProtectionMask::all());
    }

    #[cfg(feature = "gc")]
    fn allocate_gc_heap(
        &self,
        gc_runtime: &dyn GcRuntime,
    ) -> Result<(GcHeapAllocationIndex, Box<dyn GcHeap>)> {
        self.gc_heaps.allocate(gc_runtime)
    }

    #[cfg(feature = "gc")]
    fn deallocate_gc_heap(
        &self,
        allocation_index: GcHeapAllocationIndex,
        gc_heap: Box<dyn GcHeap>,
    ) {
        self.gc_heaps.deallocate(allocation_index, gc_heap);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pooling_allocator_with_memory_pages_exceeded() {
        let config = PoolingInstanceAllocatorConfig {
            limits: InstanceLimits {
                total_memories: 1,
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
                    ..Tunables::default_host()
                },
            )
            .map_err(|e| e.to_string())
            .expect_err("expected a failure constructing instance allocator"),
            "module memory page limit of 65537 exceeds the maximum of 65536"
        );
    }

    #[cfg(all(unix, target_pointer_width = "64", feature = "async", not(miri)))]
    #[test]
    fn test_stack_zeroed() -> Result<()> {
        let config = PoolingInstanceAllocatorConfig {
            max_unused_warm_slots: 0,
            limits: InstanceLimits {
                total_stacks: 1,
                total_memories: 0,
                total_tables: 0,
                ..Default::default()
            },
            stack_size: 128,
            async_stack_zeroing: true,
            ..PoolingInstanceAllocatorConfig::default()
        };
        let allocator = PoolingInstanceAllocator::new(&config, &Tunables::default_host())?;

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

    #[cfg(all(unix, target_pointer_width = "64", feature = "async", not(miri)))]
    #[test]
    fn test_stack_unzeroed() -> Result<()> {
        let config = PoolingInstanceAllocatorConfig {
            max_unused_warm_slots: 0,
            limits: InstanceLimits {
                total_stacks: 1,
                total_memories: 0,
                total_tables: 0,
                ..Default::default()
            },
            stack_size: 128,
            async_stack_zeroing: false,
            ..PoolingInstanceAllocatorConfig::default()
        };
        let allocator = PoolingInstanceAllocator::new(&config, &Tunables::default_host())?;

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
