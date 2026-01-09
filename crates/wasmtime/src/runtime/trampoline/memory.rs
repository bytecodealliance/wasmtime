use crate::MemoryType;
use crate::memory::{LinearMemory, MemoryCreator};
use crate::prelude::*;
use crate::runtime::vm::mpk::ProtectionKey;
use crate::runtime::vm::{
    CompiledModuleId, InstanceAllocationRequest, InstanceAllocator, Memory, MemoryAllocationIndex,
    MemoryBase, ModuleRuntimeInfo, OnDemandInstanceAllocator, RuntimeLinearMemory,
    RuntimeMemoryCreator, SharedMemory, Table, TableAllocationIndex,
};
use crate::store::{AllocateInstanceKind, InstanceId, StoreOpaque, StoreResourceLimiter};
use alloc::sync::Arc;
use wasmtime_environ::{
    DefinedMemoryIndex, DefinedTableIndex, EntityIndex, HostPtr, Module, StaticModuleIndex,
    Tunables, VMOffsets,
};

#[cfg(feature = "component-model")]
use wasmtime_environ::component::{Component, VMComponentOffsets};

/// Create a "frankenstein" instance with a single memory.
///
/// This separate instance is necessary because Wasm objects in Wasmtime must be
/// attached to instances (versus the store, e.g.) and some objects exist
/// outside: a host-provided memory import, shared memory.
pub async fn create_memory(
    store: &mut StoreOpaque,
    limiter: Option<&mut StoreResourceLimiter<'_>>,
    memory_ty: &MemoryType,
    preallocation: Option<&SharedMemory>,
) -> Result<InstanceId> {
    let mut module = Module::new(StaticModuleIndex::from_u32(0));

    // Create a memory, though it will never be used for constructing a memory
    // with an allocator: instead the memories are either preallocated (i.e.,
    // shared memory) or allocated manually below.
    let memory_id = module.memories.push(*memory_ty.wasmtime_memory());

    // Since we have only associated a single memory with the "frankenstein"
    // instance, it will be exported at index 0.
    debug_assert_eq!(memory_id.as_u32(), 0);
    module
        .exports
        .insert(String::new(), EntityIndex::Memory(memory_id));

    // We create an instance in the on-demand allocator when creating handles
    // associated with external objects. The configured instance allocator
    // should only be used when creating module instances as we don't want host
    // objects to count towards instance limits.
    let allocator = SingleMemoryInstance {
        preallocation,
        ondemand: OnDemandInstanceAllocator::default(),
    };
    unsafe {
        store
            .allocate_instance(
                limiter,
                AllocateInstanceKind::Dummy {
                    allocator: &allocator,
                },
                &ModuleRuntimeInfo::bare(Arc::new(module)),
                Default::default(),
            )
            .await
    }
}

struct LinearMemoryProxy {
    mem: Box<dyn LinearMemory>,
}

impl RuntimeLinearMemory for LinearMemoryProxy {
    fn byte_size(&self) -> usize {
        self.mem.byte_size()
    }

    fn byte_capacity(&self) -> usize {
        self.mem.byte_capacity()
    }

    fn grow_to(&mut self, new_size: usize) -> Result<()> {
        self.mem.grow_to(new_size)
    }

    fn base(&self) -> MemoryBase {
        MemoryBase::new_raw(self.mem.as_ptr())
    }

    fn vmmemory(&self) -> crate::vm::VMMemoryDefinition {
        let base = core::ptr::NonNull::new(self.mem.as_ptr()).unwrap();
        crate::vm::VMMemoryDefinition {
            base: base.into(),
            current_length: self.mem.byte_size().into(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct MemoryCreatorProxy(pub Arc<dyn MemoryCreator>);

impl RuntimeMemoryCreator for MemoryCreatorProxy {
    fn new_memory(
        &self,
        ty: &wasmtime_environ::Memory,
        tunables: &Tunables,
        minimum: usize,
        maximum: Option<usize>,
    ) -> Result<Box<dyn RuntimeLinearMemory>> {
        let reserved_size_in_bytes = Some(tunables.memory_reservation.try_into().unwrap());
        self.0
            .new_memory(
                MemoryType::from_wasmtime_memory(ty),
                minimum,
                maximum,
                reserved_size_in_bytes,
                usize::try_from(tunables.memory_guard_size).unwrap(),
            )
            .map(|mem| Box::new(LinearMemoryProxy { mem }) as Box<dyn RuntimeLinearMemory>)
            .map_err(|e| format_err!(e))
    }
}

struct SingleMemoryInstance<'a> {
    preallocation: Option<&'a SharedMemory>,
    ondemand: OnDemandInstanceAllocator,
}

#[async_trait::async_trait]
unsafe impl InstanceAllocator for SingleMemoryInstance<'_> {
    #[cfg(feature = "component-model")]
    fn validate_component<'a>(
        &self,
        _component: &Component,
        _offsets: &VMComponentOffsets<HostPtr>,
        _get_module: &'a dyn Fn(StaticModuleIndex) -> &'a Module,
    ) -> Result<()> {
        unreachable!("`SingleMemoryInstance` allocator never used with components")
    }

    fn validate_module(&self, module: &Module, offsets: &VMOffsets<HostPtr>) -> Result<()> {
        crate::ensure!(
            module.memories.len() == 1,
            "`SingleMemoryInstance` allocator can only be used for modules with a single memory"
        );
        self.ondemand.validate_module(module, offsets)?;
        Ok(())
    }

    #[cfg(feature = "gc")]
    fn validate_memory(&self, memory: &wasmtime_environ::Memory) -> Result<()> {
        self.ondemand.validate_memory(memory)
    }

    #[cfg(feature = "component-model")]
    fn increment_component_instance_count(&self) -> Result<()> {
        self.ondemand.increment_component_instance_count()
    }

    #[cfg(feature = "component-model")]
    fn decrement_component_instance_count(&self) {
        self.ondemand.decrement_component_instance_count();
    }

    fn increment_core_instance_count(&self) -> Result<()> {
        self.ondemand.increment_core_instance_count()
    }

    fn decrement_core_instance_count(&self) {
        self.ondemand.decrement_core_instance_count();
    }

    async fn allocate_memory(
        &self,
        request: &mut InstanceAllocationRequest<'_, '_>,
        ty: &wasmtime_environ::Memory,
        memory_index: Option<DefinedMemoryIndex>,
    ) -> Result<(MemoryAllocationIndex, Memory)> {
        if cfg!(debug_assertions) {
            let module = request.runtime_info.env_module();
            let offsets = request.runtime_info.offsets();
            self.validate_module(module, offsets)
                .expect("should have already validated the module before allocating memory");
        }

        match self.preallocation {
            Some(shared_memory) => Ok((
                MemoryAllocationIndex::default(),
                shared_memory.clone().as_memory(),
            )),
            None => {
                self.ondemand
                    .allocate_memory(request, ty, memory_index)
                    .await
            }
        }
    }

    unsafe fn deallocate_memory(
        &self,
        memory_index: Option<DefinedMemoryIndex>,
        allocation_index: MemoryAllocationIndex,
        memory: Memory,
    ) {
        unsafe {
            self.ondemand
                .deallocate_memory(memory_index, allocation_index, memory)
        }
    }

    async fn allocate_table(
        &self,
        req: &mut InstanceAllocationRequest<'_, '_>,
        ty: &wasmtime_environ::Table,
        table_index: DefinedTableIndex,
    ) -> Result<(TableAllocationIndex, Table)> {
        self.ondemand.allocate_table(req, ty, table_index).await
    }

    unsafe fn deallocate_table(
        &self,
        table_index: DefinedTableIndex,
        allocation_index: TableAllocationIndex,
        table: Table,
    ) {
        unsafe {
            self.ondemand
                .deallocate_table(table_index, allocation_index, table)
        }
    }

    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack> {
        unreachable!()
    }

    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, _stack: wasmtime_fiber::FiberStack) {
        unreachable!()
    }

    fn purge_module(&self, _: CompiledModuleId) {
        unreachable!()
    }

    fn next_available_pkey(&self) -> Option<ProtectionKey> {
        unreachable!()
    }

    fn restrict_to_pkey(&self, _: ProtectionKey) {
        unreachable!()
    }

    fn allow_all_pkeys(&self) {
        unreachable!()
    }

    #[cfg(feature = "gc")]
    fn allocate_gc_heap(
        &self,
        _engine: &crate::Engine,
        _gc_runtime: &dyn crate::vm::GcRuntime,
        _memory_alloc_index: crate::vm::MemoryAllocationIndex,
        _memory: Memory,
    ) -> Result<(crate::vm::GcHeapAllocationIndex, Box<dyn crate::vm::GcHeap>)> {
        unreachable!()
    }

    #[cfg(feature = "gc")]
    fn deallocate_gc_heap(
        &self,
        _allocation_index: crate::vm::GcHeapAllocationIndex,
        _gc_heap: Box<dyn crate::vm::GcHeap>,
    ) -> (crate::vm::MemoryAllocationIndex, crate::vm::Memory) {
        unreachable!()
    }
}
