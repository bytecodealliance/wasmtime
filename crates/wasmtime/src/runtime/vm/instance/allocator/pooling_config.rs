use crate::Enabled;

/// Instance-related limit configuration for pooling.
///
/// More docs on this can be found at `wasmtime::PoolingAllocationConfig`.
#[derive(Debug, Copy, Clone)]
pub struct InstanceLimits {
    /// The maximum number of component instances that may be allocated
    /// concurrently.
    pub total_component_instances: u32,

    /// The maximum size of a component's `VMComponentContext`, including
    /// the aggregate size of all its inner core modules' `VMContext` sizes.
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
    pub total_stacks: u32,

    /// Maximum size of a core instance's `VMContext`.
    pub core_instance_size: usize,

    /// Maximum number of tables per instance.
    pub max_tables_per_module: u32,

    /// Maximum number of word-size elements per table.
    ///
    /// Note that tables for element types such as continuations
    /// that use more than one word of storage may store fewer
    /// elements.
    pub table_elements: usize,

    /// Maximum number of linear memories per instance.
    pub max_memories_per_module: u32,

    /// Maximum byte size of a linear memory, must be smaller than
    /// `memory_reservation` in `Tunables`.
    pub max_memory_size: usize,

    /// The total number of GC heaps in the pool, across all instances.
    pub total_gc_heaps: u32,
}

impl Default for InstanceLimits {
    fn default() -> Self {
        let total = if cfg!(target_pointer_width = "32") {
            100
        } else {
            1000
        };
        // See doc comments for `wasmtime::PoolingAllocationConfig` for these
        // default values
        Self {
            total_component_instances: total,
            component_instance_size: 1 << 20, // 1 MiB
            total_core_instances: total,
            max_core_instances_per_component: u32::MAX,
            max_memories_per_component: u32::MAX,
            max_tables_per_component: u32::MAX,
            total_memories: total,
            total_tables: total,
            total_stacks: total,
            core_instance_size: 1 << 20, // 1 MiB
            max_tables_per_module: 1,
            // NB: in #8504 it was seen that a C# module in debug module can
            // have 10k+ elements.
            table_elements: 20_000,
            max_memories_per_module: 1,
            #[cfg(target_pointer_width = "64")]
            max_memory_size: 1 << 32, // 4G,
            #[cfg(target_pointer_width = "32")]
            max_memory_size: 10 << 20, // 10 MiB
            total_gc_heaps: total,
        }
    }
}

/// Configuration options for the pooling instance allocator supplied at
/// construction.
#[derive(Copy, Clone, Debug)]
pub struct PoolingInstanceAllocatorConfig {
    /// See `PoolingAllocatorConfig::max_unused_warm_slots` in `wasmtime`
    pub max_unused_warm_slots: u32,
    /// The target number of decommits to do per batch. This is not precise, as
    /// we can queue up decommits at times when we aren't prepared to
    /// immediately flush them, and so we may go over this target size
    /// occasionally.
    pub decommit_batch_size: usize,
    /// The size, in bytes, of async stacks to allocate (not including the guard
    /// page).
    #[cfg(feature = "async")]
    pub stack_size: usize,
    /// The limits to apply to instances allocated within this allocator.
    pub limits: InstanceLimits,
    /// Whether or not async stacks are zeroed after use.
    #[cfg(feature = "async")]
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
    pub memory_protection_keys: Enabled,
    /// How many memory protection keys to allocate.
    pub max_memory_protection_keys: usize,
    /// Whether to enable PAGEMAP_SCAN on Linux.
    pub pagemap_scan: Enabled,
}

impl Default for PoolingInstanceAllocatorConfig {
    fn default() -> PoolingInstanceAllocatorConfig {
        PoolingInstanceAllocatorConfig {
            max_unused_warm_slots: 100,
            decommit_batch_size: 1,
            #[cfg(feature = "async")]
            stack_size: 2 << 20,
            limits: InstanceLimits::default(),
            #[cfg(feature = "async")]
            async_stack_zeroing: false,
            async_stack_keep_resident: 0,
            linear_memory_keep_resident: 0,
            table_keep_resident: 0,
            memory_protection_keys: Enabled::No,
            max_memory_protection_keys: 16,
            pagemap_scan: Enabled::No,
        }
    }
}
