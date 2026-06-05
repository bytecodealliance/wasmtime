use crate::prelude::*;
use crate::runtime::vm::imports::Imports;
use crate::runtime::vm::instance::{Instance, InstanceHandle};
use crate::runtime::vm::memory::Memory;
use crate::runtime::vm::mpk::ProtectionKey;
use crate::runtime::vm::table::Table;
use crate::runtime::vm::{CompiledModuleId, ModuleRuntimeInfo};
use crate::store::{InstanceId, StoreOpaque, StoreResourceLimiter};
use core::future::Future;
use core::mem;
use core::pin::Pin;
use wasmtime_environ::{
    DefinedMemoryIndex, DefinedTableIndex, HostPtr, MemoryKind, Module, VMOffsets,
};

#[cfg(feature = "gc")]
use crate::runtime::vm::{GcHeap, GcRuntime};

#[cfg(feature = "component-model")]
use wasmtime_environ::{
    StaticModuleIndex,
    component::{Component, VMComponentOffsets},
};

mod on_demand;
pub use self::on_demand::OnDemandInstanceAllocator;

#[cfg(feature = "pooling-allocator")]
mod pooling;
#[cfg(feature = "pooling-allocator")]
pub use self::pooling::{
    InstanceLimits, PoolConcurrencyLimitError, PoolingAllocatorMetrics, PoolingInstanceAllocator,
    PoolingInstanceAllocatorConfig,
};

/// Represents a request for a new runtime instance.
pub struct InstanceAllocationRequest<'a, 'b> {
    /// The instance id that this will be assigned within the store once the
    /// allocation has finished.
    pub id: InstanceId,

    /// The info related to the compiled version of this module,
    /// needed for instantiation: function metadata, JIT code
    /// addresses, precomputed images for lazy memory and table
    /// initialization, and the like. This Arc is cloned and held for
    /// the lifetime of the instance.
    pub runtime_info: &'a ModuleRuntimeInfo,

    /// The imports to use for the instantiation.
    pub imports: Imports<'a>,

    /// The store that this instance is being allocated into.
    pub store: &'a StoreOpaque,

    /// The store's resource limiter, if configured by the embedder.
    pub limiter: Option<&'a mut StoreResourceLimiter<'b>>,
}

/// The index of a memory allocation within an `InstanceAllocator`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct MemoryAllocationIndex(u32);

impl Default for MemoryAllocationIndex {
    fn default() -> Self {
        // A default `MemoryAllocationIndex` that can be used with
        // `InstanceAllocator`s that don't actually need indices.
        MemoryAllocationIndex(u32::MAX)
    }
}

impl MemoryAllocationIndex {
    /// Get the underlying index of this `MemoryAllocationIndex`.
    #[cfg(feature = "pooling-allocator")]
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// The index of a table allocation within an `InstanceAllocator`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct TableAllocationIndex(u32);

impl Default for TableAllocationIndex {
    fn default() -> Self {
        // A default `TableAllocationIndex` that can be used with
        // `InstanceAllocator`s that don't actually need indices.
        TableAllocationIndex(u32::MAX)
    }
}

impl TableAllocationIndex {
    /// Get the underlying index of this `TableAllocationIndex`.
    #[cfg(feature = "pooling-allocator")]
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// The index of a table allocation within an `InstanceAllocator`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct GcHeapAllocationIndex(u32);

impl Default for GcHeapAllocationIndex {
    fn default() -> Self {
        // A default `GcHeapAllocationIndex` that can be used with
        // `InstanceAllocator`s that don't actually need indices.
        GcHeapAllocationIndex(u32::MAX)
    }
}

impl GcHeapAllocationIndex {
    /// Get the underlying index of this `GcHeapAllocationIndex`.
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Trait that represents the hooks needed to implement an instance allocator.
///
/// Implement this trait when implementing new instance allocators, but don't
/// use this trait when you need an instance allocator. Instead use the
/// `InstanceAllocator` trait for that, which has additional helper methods and
/// a blanket implementation for all types that implement this trait.
///
/// # Safety
///
/// This trait is unsafe as it requires knowledge of Wasmtime's runtime
/// internals to implement correctly.
pub unsafe trait InstanceAllocator: Send + Sync {
    /// Validate whether a component (including all of its contained core
    /// modules) is allocatable by this instance allocator.
    #[cfg(feature = "component-model")]
    fn validate_component<'a>(
        &self,
        component: &Component,
        offsets: &VMComponentOffsets<HostPtr>,
        get_module: &'a dyn Fn(StaticModuleIndex) -> &'a Module,
    ) -> Result<()>;

    /// Validate whether a module is allocatable by this instance allocator.
    fn validate_module(&self, module: &Module, offsets: &VMOffsets<HostPtr>) -> Result<()>;

    /// Validate whether a memory is allocatable by this instance allocator.
    #[cfg(feature = "gc")]
    fn validate_memory(&self, memory: &wasmtime_environ::Memory) -> Result<()>;

    /// Increment the count of concurrent component instances that are currently
    /// allocated, if applicable.
    ///
    /// Not all instance allocators will have limits for the maximum number of
    /// concurrent component instances that can be live at the same time, and
    /// these allocators may implement this method with a no-op.
    //
    // Note: It would be nice to have an associated type that on construction
    // does the increment and on drop does the decrement but there are two
    // problems with this:
    //
    // 1. This trait's implementations are always used as trait objects, and
    //    associated types are not object safe.
    //
    // 2. We would want a parameterized `Drop` implementation so that we could
    //    pass in the `InstanceAllocator` on drop, but this doesn't exist in
    //    Rust. Therefore, we would be forced to add reference counting and
    //    stuff like that to keep a handle on the instance allocator from this
    //    theoretical type. That's a bummer.
    #[cfg(feature = "component-model")]
    fn increment_component_instance_count(&self) -> Result<()>;

    /// The dual of `increment_component_instance_count`.
    #[cfg(feature = "component-model")]
    fn decrement_component_instance_count(&self);

    /// Increment the count of concurrent core module instances that are
    /// currently allocated, if applicable.
    ///
    /// Not all instance allocators will have limits for the maximum number of
    /// concurrent core module instances that can be live at the same time, and
    /// these allocators may implement this method with a no-op.
    fn increment_core_instance_count(&self) -> Result<()>;

    /// The dual of `increment_core_instance_count`.
    fn decrement_core_instance_count(&self);

    /// Allocate a memory for an instance.
    ///
    /// Returns `Err(OutOfMemory)` if boxing the future fails. The inner
    /// `Result` covers other allocation errors (e.g. resource limits).
    fn allocate_memory<'a, 'b: 'a, 'c: 'a>(
        &'a self,
        request: &'a mut InstanceAllocationRequest<'b, 'c>,
        ty: &'a wasmtime_environ::Memory,
        memory_index: Option<DefinedMemoryIndex>,
        memory_kind: MemoryKind,
    ) -> Pin<Box<dyn Future<Output = Result<(MemoryAllocationIndex, Memory)>> + Send + 'a>>;

    /// Deallocate an instance's previously allocated memory.
    ///
    /// # Unsafety
    ///
    /// The memory must have previously been allocated by
    /// `Self::allocate_memory`, be at the given index, and must currently be
    /// allocated. It must never be used again.
    unsafe fn deallocate_memory(
        &self,
        memory_index: Option<DefinedMemoryIndex>,
        allocation_index: MemoryAllocationIndex,
        memory: Memory,
    );

    /// Allocate a table for an instance.
    ///
    /// Returns `Err(OutOfMemory)` if boxing the future fails. The inner
    /// `Result` covers other allocation errors (e.g. resource limits).
    fn allocate_table<'a, 'b: 'a, 'c: 'a>(
        &'a self,
        req: &'a mut InstanceAllocationRequest<'b, 'c>,
        table: &'a wasmtime_environ::Table,
        table_index: DefinedTableIndex,
    ) -> Pin<Box<dyn Future<Output = Result<(TableAllocationIndex, Table)>> + Send + 'a>>;

    /// Deallocate an instance's previously allocated table.
    ///
    /// # Unsafety
    ///
    /// The table must have previously been allocated by `Self::allocate_table`,
    /// be at the given index, and must currently be allocated. It must never be
    /// used again.
    unsafe fn deallocate_table(
        &self,
        table_index: DefinedTableIndex,
        allocation_index: TableAllocationIndex,
        table: Table,
    );

    /// Allocates a fiber stack for calling async functions on.
    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack>;

    /// Deallocates a fiber stack that was previously allocated with
    /// `allocate_fiber_stack`.
    ///
    /// # Safety
    ///
    /// The provided stack is required to have been allocated with
    /// `allocate_fiber_stack`.
    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, stack: wasmtime_fiber::FiberStack);

    /// Allocate a GC heap for allocating Wasm GC objects within.
    #[cfg(feature = "gc")]
    fn allocate_gc_heap(
        &self,
        engine: &crate::Engine,
        gc_runtime: &dyn GcRuntime,
        memory_alloc_index: MemoryAllocationIndex,
        memory: Memory,
    ) -> Result<(GcHeapAllocationIndex, Box<dyn GcHeap>)>;

    /// Deallocate a GC heap that was previously allocated with
    /// `allocate_gc_heap`.
    #[cfg(feature = "gc")]
    #[must_use = "it is the caller's responsibility to deallocate the GC heap's underlying memory \
                  storage after the GC heap is deallocated"]
    fn deallocate_gc_heap(
        &self,
        allocation_index: GcHeapAllocationIndex,
        gc_heap: Box<dyn GcHeap>,
    ) -> (MemoryAllocationIndex, Memory);

    /// Purges all lingering resources related to `module` from within this
    /// allocator.
    ///
    /// Primarily present for the pooling allocator to remove mappings of
    /// this module from slots in linear memory.
    fn purge_module(&self, module: CompiledModuleId);

    /// Use the next available protection key.
    ///
    /// The pooling allocator can use memory protection keys (MPK) for
    /// compressing the guard regions protecting against OOB. Each
    /// pool-allocated store needs its own key.
    fn next_available_pkey(&self) -> Option<ProtectionKey>;

    /// Restrict access to memory regions protected by `pkey`.
    ///
    /// This is useful for the pooling allocator, which can use memory
    /// protection keys (MPK). Note: this may still allow access to other
    /// protection keys, such as the default kernel key; see implementations of
    /// this.
    fn restrict_to_pkey(&self, pkey: ProtectionKey);

    /// Allow access to memory regions protected by any protection key.
    fn allow_all_pkeys(&self);

    /// Returns `Some(&PoolingInstanceAllocator)` if this is one.
    #[cfg(feature = "pooling-allocator")]
    fn as_pooling(&self) -> Option<&PoolingInstanceAllocator> {
        None
    }
}

impl dyn InstanceAllocator + '_ {
    /// Allocates a fresh `InstanceHandle` for the `req` given.
    ///
    /// This will allocate memories and tables internally from this allocator
    /// and weave that altogether into a final and complete `InstanceHandle`
    /// ready to be registered with a store.
    ///
    /// Note that the returned instance must still have `.initialize(..)` called
    /// on it to complete the instantiation process.
    ///
    /// # Safety
    ///
    /// The `request` provided must be valid, e.g. the imports within are
    /// correctly sized/typed for the instance being created.
    pub(crate) async unsafe fn allocate_module(
        &self,
        mut request: InstanceAllocationRequest<'_, '_>,
    ) -> Result<InstanceHandle> {
        let module = request.runtime_info.env_module();

        if cfg!(debug_assertions) {
            InstanceAllocator::validate_module(self, module, request.runtime_info.offsets())
                .expect("module should have already been validated before allocation");
        }

        let num_defined_memories = module.num_defined_memories();
        let num_defined_tables = module.num_defined_tables();

        let memories = TryPrimaryMap::with_capacity(num_defined_memories)?;
        let tables = TryPrimaryMap::with_capacity(num_defined_tables)?;

        // Note that incrementing the instance count here must be done just
        // before creation of `DeallocateOnDrop`. This is required to ensure
        // that upon successful increment it'll get paired with a decrement
        // below should anything fail.
        self.increment_core_instance_count()?;
        let mut guard = DeallocateOnDrop {
            run_deallocate: true,
            memories,
            tables,
            allocator: self,
            // NB: do not add more initialization here if it can fail, move that
            // above the increment above instead.
        };

        self.allocate_memories(&mut request, &mut guard.memories)
            .await?;
        self.allocate_tables(&mut request, &mut guard.tables)
            .await?;

        // SAFETY: memories/tables were just allocated from the store within
        // `request` and this function's own contract requires that the
        // imports are valid.
        let handle = unsafe { Instance::new(request, &mut guard.memories, &mut guard.tables)? };
        guard.run_deallocate = false;
        return Ok(handle);

        struct DeallocateOnDrop<'a> {
            run_deallocate: bool,
            memories: TryPrimaryMap<DefinedMemoryIndex, (MemoryAllocationIndex, Memory)>,
            tables: TryPrimaryMap<DefinedTableIndex, (TableAllocationIndex, Table)>,
            allocator: &'a (dyn InstanceAllocator + 'a),
        }

        impl Drop for DeallocateOnDrop<'_> {
            fn drop(&mut self) {
                if !self.run_deallocate {
                    debug_assert!(self.memories.is_empty());
                    debug_assert!(self.tables.is_empty());
                    return;
                }
                // SAFETY: these were previously allocated by this allocator
                unsafe {
                    self.allocator.deallocate_memories(&mut self.memories);
                    self.allocator.deallocate_tables(&mut self.tables);
                }
                self.allocator.decrement_core_instance_count();
            }
        }
    }

    /// Deallocates the provided instance.
    ///
    /// This will null-out the pointer within `handle` and otherwise reclaim
    /// resources such as tables, memories, and the instance memory itself.
    ///
    /// # Unsafety
    ///
    /// The instance must have previously been allocated by `Self::allocate`.
    pub(crate) unsafe fn deallocate_module(&self, handle: &mut InstanceHandle) {
        // SAFETY: the contract of `deallocate_*` is itself a contract of this
        // function, that the memories/tables were previously allocated from
        // here.
        unsafe {
            self.deallocate_memories(handle.get_mut().memories_mut());
            self.deallocate_tables(handle.get_mut().tables_mut());
        }

        self.decrement_core_instance_count();
    }

    /// Allocate the memories for the given instance allocation request, pushing
    /// them into `memories`.
    async fn allocate_memories(
        &self,
        request: &mut InstanceAllocationRequest<'_, '_>,
        memories: &mut TryPrimaryMap<DefinedMemoryIndex, (MemoryAllocationIndex, Memory)>,
    ) -> Result<()> {
        let module = request.runtime_info.env_module();

        if cfg!(debug_assertions) {
            InstanceAllocator::validate_module(self, module, request.runtime_info.offsets())
                .expect("module should have already been validated before allocation");
        }

        for (memory_index, ty) in module.memories.iter().skip(module.num_imported_memories) {
            let memory_index = module
                .defined_memory_index(memory_index)
                .expect("should be a defined memory since we skipped imported ones");

            let memory = self
                .allocate_memory(request, ty, Some(memory_index), MemoryKind::LinearMemory)
                .await?;
            memories.push(memory)?;
        }

        Ok(())
    }

    /// Deallocate all the memories in the given primary map.
    ///
    /// # Unsafety
    ///
    /// The memories must have previously been allocated by
    /// `Self::allocate_memories`.
    unsafe fn deallocate_memories(
        &self,
        memories: &mut TryPrimaryMap<DefinedMemoryIndex, (MemoryAllocationIndex, Memory)>,
    ) {
        for (memory_index, (allocation_index, memory)) in mem::take(memories) {
            // Because deallocating memory is infallible, we don't need to worry
            // about leaking subsequent memories if the first memory failed to
            // deallocate. If deallocating memory ever becomes fallible, we will
            // need to be careful here!
            //
            // SAFETY: the unsafe contract here is the same as the unsafe
            // contract of this function, that the memories were previously
            // allocated by this allocator.
            unsafe {
                self.deallocate_memory(Some(memory_index), allocation_index, memory);
            }
        }
    }

    /// Allocate tables for the given instance allocation request, pushing them
    /// into `tables`.
    async fn allocate_tables(
        &self,
        request: &mut InstanceAllocationRequest<'_, '_>,
        tables: &mut TryPrimaryMap<DefinedTableIndex, (TableAllocationIndex, Table)>,
    ) -> Result<()> {
        let module = request.runtime_info.env_module();

        if cfg!(debug_assertions) {
            InstanceAllocator::validate_module(self, module, request.runtime_info.offsets())
                .expect("module should have already been validated before allocation");
        }

        for (index, table) in module.tables.iter().skip(module.num_imported_tables) {
            let def_index = module
                .defined_table_index(index)
                .expect("should be a defined table since we skipped imported ones");

            let table = self.allocate_table(request, table, def_index).await?;
            tables.push(table)?;
        }

        Ok(())
    }

    /// Deallocate all the tables in the given primary map.
    ///
    /// # Unsafety
    ///
    /// The tables must have previously been allocated by
    /// `Self::allocate_tables`.
    unsafe fn deallocate_tables(
        &self,
        tables: &mut TryPrimaryMap<DefinedTableIndex, (TableAllocationIndex, Table)>,
    ) {
        for (table_index, (allocation_index, table)) in mem::take(tables) {
            // SAFETY: the tables here were allocated from this allocator per
            // the contract on this function itself.
            unsafe {
                self.deallocate_table(table_index, allocation_index, table);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_traits_are_object_safe() {
        fn _instance_allocator(_: &dyn InstanceAllocator) {}
    }
}
