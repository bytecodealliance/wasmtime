//! This module contains types exposed via `Config` relating to the pooling allocator feature.

/// Represents the limits placed on a module for compiling with the pooling instance allocation strategy.
#[derive(Debug, Copy, Clone)]
pub struct ModuleLimits {
    /// The maximum number of imported functions for a module (default is 1000).
    ///
    /// This value controls the capacity of the `VMFunctionImport` table and the
    /// `VMCallerCheckedAnyfunc` table in each instance's `VMContext` structure.
    ///
    /// The allocated size of the `VMFunctionImport` table will be `imported_functions * sizeof(VMFunctionImport)`
    /// for each instance regardless of how many functions an instance imports.
    ///
    /// The allocated size of the `VMCallerCheckedAnyfunc` table will be
    /// `imported_functions * functions * sizeof(VMCallerCheckedAnyfunc)` for each instance regardless of
    /// how many functions are imported and defined by an instance.
    pub imported_functions: u32,

    /// The maximum number of imported tables for a module (default is 0).
    ///
    /// This value controls the capacity of the `VMTableImport` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `imported_tables * sizeof(VMTableImport)` for each
    /// instance regardless of how many tables an instance imports.
    pub imported_tables: u32,

    /// The maximum number of imported linear memories for a module (default is 0).
    ///
    /// This value controls the capacity of the `VMMemoryImport` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `imported_memories * sizeof(VMMemoryImport)` for each
    /// instance regardless of how many memories an instance imports.
    pub imported_memories: u32,

    /// The maximum number of imported globals for a module (default is 0).
    ///
    /// This value controls the capacity of the `VMGlobalImport` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `imported_globals * sizeof(VMGlobalImport)` for each
    /// instance regardless of how many globals an instance imports.
    pub imported_globals: u32,

    /// The maximum number of defined types for a module (default is 100).
    ///
    /// This value controls the capacity of the `VMSharedSignatureIndex` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `types * sizeof(VMSharedSignatureIndex)` for each
    /// instance regardless of how many types are defined by an instance's module.
    pub types: u32,

    /// The maximum number of defined functions for a module (default is 10000).
    ///
    /// This value controls the capacity of the `VMCallerCheckedAnyfunc` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the `VMCallerCheckedAnyfunc` table will be
    /// `imported_functions * functions * sizeof(VMCallerCheckedAnyfunc)` for each instance
    /// regardless of how many functions are imported and defined by an instance.
    pub functions: u32,

    /// The maximum number of defined tables for a module (default is 1).
    ///
    /// This value controls the capacity of the `VMTableDefinition` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `tables * sizeof(VMTableDefinition)` for each
    /// instance regardless of how many tables are defined by an instance's module.
    pub tables: u32,

    /// The maximum number of defined linear memories for a module (default is 1).
    ///
    /// This value controls the capacity of the `VMMemoryDefinition` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `memories * sizeof(VMMemoryDefinition)` for each
    /// instance regardless of how many memories are defined by an instance's module.
    pub memories: u32,

    /// The maximum number of defined globals for a module (default is 10).
    ///
    /// This value controls the capacity of the `VMGlobalDefinition` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `globals * sizeof(VMGlobalDefinition)` for each
    /// instance regardless of how many globals are defined by an instance's module.
    pub globals: u32,

    /// The maximum table elements for any table defined in a module (default is 10000).
    ///
    /// If a table's minimum element limit is greater than this value, the module will
    /// fail to compile.
    ///
    /// If a table's maximum element limit is unbounded or greater than this value,
    /// the maximum will be `table_elements` for the purpose of any `table.grow` instruction.
    ///
    /// This value is used to reserve the maximum space for each supported table; table elements
    /// are pointer-sized in the Wasmtime runtime.  Therefore, the space reserved for each instance
    /// is `tables * table_elements * sizeof::<*const ()>`.
    pub table_elements: u32,

    /// The maximum number of pages for any linear memory defined in a module (default is 160).
    ///
    /// The default of 160 means at most 10 MiB of host memory may be committed for each instance.
    ///
    /// If a memory's minimum page limit is greater than this value, the module will
    /// fail to compile.
    ///
    /// If a memory's maximum page limit is unbounded or greater than this value,
    /// the maximum will be `memory_pages` for the purpose of any `memory.grow` instruction.
    ///
    /// This value is used to control the maximum accessible space for each linear memory of an instance.
    ///
    /// The reservation size of each linear memory is controlled by the
    /// [`static_memory_maximum_size`](super::Config::static_memory_maximum_size) setting and this value cannot
    /// exceed the configured static memory maximum size.
    pub memory_pages: u64,
}

impl Default for ModuleLimits {
    fn default() -> Self {
        // Use the defaults from the runtime
        let wasmtime_runtime::ModuleLimits {
            imported_functions,
            imported_tables,
            imported_memories,
            imported_globals,
            types,
            functions,
            tables,
            memories,
            globals,
            table_elements,
            memory_pages,
        } = wasmtime_runtime::ModuleLimits::default();

        Self {
            imported_functions,
            imported_tables,
            imported_memories,
            imported_globals,
            types,
            functions,
            tables,
            memories,
            globals,
            table_elements,
            memory_pages,
        }
    }
}

// This exists so we can convert between the public Wasmtime API and the runtime representation
// without having to export runtime types from the Wasmtime API.
#[doc(hidden)]
impl Into<wasmtime_runtime::ModuleLimits> for ModuleLimits {
    fn into(self) -> wasmtime_runtime::ModuleLimits {
        let Self {
            imported_functions,
            imported_tables,
            imported_memories,
            imported_globals,
            types,
            functions,
            tables,
            memories,
            globals,
            table_elements,
            memory_pages,
        } = self;

        wasmtime_runtime::ModuleLimits {
            imported_functions,
            imported_tables,
            imported_memories,
            imported_globals,
            types,
            functions,
            tables,
            memories,
            globals,
            table_elements,
            memory_pages,
        }
    }
}

/// Represents the limits placed on instances by the pooling instance allocation strategy.
#[derive(Debug, Copy, Clone)]
pub struct InstanceLimits {
    /// The maximum number of concurrent instances supported (default is 1000).
    ///
    /// This value has a direct impact on the amount of memory allocated by the pooling
    /// instance allocator.
    ///
    /// The pooling instance allocator allocates three memory pools with sizes depending on this value:
    ///
    /// * An instance pool, where each entry in the pool can store the runtime representation
    ///   of an instance, including a maximal `VMContext` structure (see [`ModuleLimits`](ModuleLimits)
    ///   for the various settings that control the size of each instance's `VMContext` structure).
    ///
    /// * A memory pool, where each entry in the pool contains the reserved address space for each
    ///   linear memory supported by an instance.
    ///
    /// * A table pool, where each entry in the pool contains the space needed for each WebAssembly table
    ///   supported by an instance (see `[ModuleLimits::table_elements`] to control the size of each table).
    ///
    /// Additionally, this value will also control the maximum number of execution stacks allowed for
    /// asynchronous execution (one per instance), when enabled.
    ///
    /// The memory pool will reserve a large quantity of host process address space to elide the bounds
    /// checks required for correct WebAssembly memory semantics. Even for 64-bit address spaces, the
    /// address space is limited when dealing with a large number of supported instances.
    ///
    /// For example, on Linux x86_64, the userland address space limit is 128 TiB. That might seem like a lot,
    /// but each linear memory will *reserve* 6 GiB of space by default. Multiply that by the number of linear
    /// memories each instance supports and then by the number of supported instances and it becomes apparent
    /// that address space can be exhausted depending on the number of supported instances.
    pub count: u32,
}

impl Default for InstanceLimits {
    fn default() -> Self {
        let wasmtime_runtime::InstanceLimits { count } =
            wasmtime_runtime::InstanceLimits::default();

        Self { count }
    }
}

// This exists so we can convert between the public Wasmtime API and the runtime representation
// without having to export runtime types from the Wasmtime API.
#[doc(hidden)]
impl Into<wasmtime_runtime::InstanceLimits> for InstanceLimits {
    fn into(self) -> wasmtime_runtime::InstanceLimits {
        let Self { count } = self;

        wasmtime_runtime::InstanceLimits { count }
    }
}

/// The allocation strategy to use for the pooling instance allocation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolingAllocationStrategy {
    /// Allocate from the next available instance.
    NextAvailable,
    /// Allocate from a random available instance.
    Random,
    /// Try to allocate an instance slot that was previously used for
    /// the same module, potentially enabling faster instantiation by
    /// reusing e.g. memory mappings.
    ReuseAffinity,
}

impl Default for PoolingAllocationStrategy {
    fn default() -> Self {
        match wasmtime_runtime::PoolingAllocationStrategy::default() {
            wasmtime_runtime::PoolingAllocationStrategy::NextAvailable => Self::NextAvailable,
            wasmtime_runtime::PoolingAllocationStrategy::Random => Self::Random,
            wasmtime_runtime::PoolingAllocationStrategy::ReuseAffinity => Self::ReuseAffinity,
        }
    }
}

// This exists so we can convert between the public Wasmtime API and the runtime representation
// without having to export runtime types from the Wasmtime API.
#[doc(hidden)]
impl Into<wasmtime_runtime::PoolingAllocationStrategy> for PoolingAllocationStrategy {
    fn into(self) -> wasmtime_runtime::PoolingAllocationStrategy {
        match self {
            Self::NextAvailable => wasmtime_runtime::PoolingAllocationStrategy::NextAvailable,
            Self::Random => wasmtime_runtime::PoolingAllocationStrategy::Random,
            Self::ReuseAffinity => wasmtime_runtime::PoolingAllocationStrategy::ReuseAffinity,
        }
    }
}
