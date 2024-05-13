use crate::memory::{LinearMemory, MemoryCreator};
use crate::module::BareModuleInfo;
use crate::prelude::*;
use crate::runtime::vm::mpk::ProtectionKey;
use crate::runtime::vm::{
    CompiledModuleId, GcHeapAllocationIndex, Imports, InstanceAllocationRequest, InstanceAllocator,
    InstanceAllocatorImpl, Memory, MemoryAllocationIndex, MemoryImage, OnDemandInstanceAllocator,
    RuntimeLinearMemory, RuntimeMemoryCreator, SharedMemory, StorePtr, Table, TableAllocationIndex,
    VMMemoryDefinition,
};
use crate::store::{InstanceId, StoreOpaque};
use crate::MemoryType;
use alloc::sync::Arc;
use anyhow::{anyhow, Result};
use core::ops::Range;
use wasmtime_environ::{
    DefinedMemoryIndex, DefinedTableIndex, EntityIndex, HostPtr, MemoryPlan, MemoryStyle, Module,
    VMOffsets,
};

#[cfg(feature = "component-model")]
use wasmtime_environ::{
    component::{Component, VMComponentOffsets},
    StaticModuleIndex,
};

/// Create a "frankenstein" instance with a single memory.
///
/// This separate instance is necessary because Wasm objects in Wasmtime must be
/// attached to instances (versus the store, e.g.) and some objects exist
/// outside: a host-provided memory import, shared memory.
pub fn create_memory(
    store: &mut StoreOpaque,
    memory_ty: &MemoryType,
    preallocation: Option<&SharedMemory>,
) -> Result<InstanceId> {
    let mut module = Module::new();

    // Create a memory plan for the memory, though it will never be used for
    // constructing a memory with an allocator: instead the memories are either
    // preallocated (i.e., shared memory) or allocated manually below.
    let plan = wasmtime_environ::MemoryPlan::for_memory(
        memory_ty.wasmtime_memory().clone(),
        store.engine().tunables(),
    );
    let memory_id = module.memory_plans.push(plan.clone());

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
    let runtime_info = &BareModuleInfo::maybe_imported_func(Arc::new(module), None).into_traitobj();
    let host_state = Box::new(());
    let imports = Imports::default();
    let request = InstanceAllocationRequest {
        imports,
        host_state,
        store: StorePtr::new(store.traitobj()),
        runtime_info,
        wmemcheck: false,
        pkey: None,
    };

    unsafe {
        let handle = SingleMemoryInstance {
            preallocation,
            ondemand: OnDemandInstanceAllocator::default(),
        }
        .allocate_module(request)?;
        let instance_id = store.add_dummy_instance(handle.clone());
        Ok(instance_id)
    }
}

struct LinearMemoryProxy {
    mem: Box<dyn LinearMemory>,
}

impl RuntimeLinearMemory for LinearMemoryProxy {
    fn byte_size(&self) -> usize {
        self.mem.byte_size()
    }

    fn maximum_byte_size(&self) -> Option<usize> {
        self.mem.maximum_byte_size()
    }

    fn grow_to(&mut self, new_size: usize) -> Result<()> {
        self.mem.grow_to(new_size)
    }

    fn vmmemory(&mut self) -> VMMemoryDefinition {
        VMMemoryDefinition {
            base: self.mem.as_ptr(),
            current_length: self.mem.byte_size().into(),
        }
    }

    fn needs_init(&self) -> bool {
        true
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn wasm_accessible(&self) -> Range<usize> {
        self.mem.wasm_accessible()
    }
}

#[derive(Clone)]
pub(crate) struct MemoryCreatorProxy(pub Arc<dyn MemoryCreator>);

impl RuntimeMemoryCreator for MemoryCreatorProxy {
    fn new_memory(
        &self,
        plan: &MemoryPlan,
        minimum: usize,
        maximum: Option<usize>,
        _: Option<&Arc<MemoryImage>>,
    ) -> Result<Box<dyn RuntimeLinearMemory>> {
        let ty = MemoryType::from_wasmtime_memory(&plan.memory);
        let reserved_size_in_bytes = match plan.style {
            MemoryStyle::Static { byte_reservation } => {
                Some(usize::try_from(byte_reservation).unwrap())
            }
            MemoryStyle::Dynamic { .. } => None,
        };
        self.0
            .new_memory(
                ty,
                minimum,
                maximum,
                reserved_size_in_bytes,
                usize::try_from(plan.offset_guard_size).unwrap(),
            )
            .map(|mem| Box::new(LinearMemoryProxy { mem }) as Box<dyn RuntimeLinearMemory>)
            .map_err(|e| anyhow!(e))
    }
}

struct SingleMemoryInstance<'a> {
    preallocation: Option<&'a SharedMemory>,
    ondemand: OnDemandInstanceAllocator,
}

unsafe impl InstanceAllocatorImpl for SingleMemoryInstance<'_> {
    #[cfg(feature = "component-model")]
    fn validate_component_impl<'a>(
        &self,
        _component: &Component,
        _offsets: &VMComponentOffsets<HostPtr>,
        _get_module: &'a dyn Fn(StaticModuleIndex) -> &'a Module,
    ) -> Result<()> {
        unreachable!("`SingleMemoryInstance` allocator never used with components")
    }

    fn validate_module_impl(&self, module: &Module, offsets: &VMOffsets<HostPtr>) -> Result<()> {
        anyhow::ensure!(
            module.memory_plans.len() == 1,
            "`SingleMemoryInstance` allocator can only be used for modules with a single memory"
        );
        self.ondemand.validate_module_impl(module, offsets)?;
        Ok(())
    }

    fn increment_component_instance_count(&self) -> Result<()> {
        self.ondemand.increment_component_instance_count()
    }

    fn decrement_component_instance_count(&self) {
        self.ondemand.decrement_component_instance_count();
    }

    fn increment_core_instance_count(&self) -> Result<()> {
        self.ondemand.increment_core_instance_count()
    }

    fn decrement_core_instance_count(&self) {
        self.ondemand.decrement_core_instance_count();
    }

    unsafe fn allocate_memory(
        &self,
        request: &mut InstanceAllocationRequest,
        memory_plan: &MemoryPlan,
        memory_index: DefinedMemoryIndex,
    ) -> Result<(MemoryAllocationIndex, Memory)> {
        #[cfg(debug_assertions)]
        {
            let module = request.runtime_info.module();
            let offsets = request.runtime_info.offsets();
            self.validate_module_impl(module, offsets)
                .expect("should have already validated the module before allocating memory");
        }

        match self.preallocation {
            Some(shared_memory) => Ok((
                MemoryAllocationIndex::default(),
                shared_memory.clone().as_memory(),
            )),
            None => self
                .ondemand
                .allocate_memory(request, memory_plan, memory_index),
        }
    }

    unsafe fn deallocate_memory(
        &self,
        memory_index: DefinedMemoryIndex,
        allocation_index: MemoryAllocationIndex,
        memory: Memory,
    ) {
        self.ondemand
            .deallocate_memory(memory_index, allocation_index, memory)
    }

    unsafe fn allocate_table(
        &self,
        req: &mut InstanceAllocationRequest,
        table_plan: &wasmtime_environ::TablePlan,
        table_index: DefinedTableIndex,
    ) -> Result<(TableAllocationIndex, Table)> {
        self.ondemand.allocate_table(req, table_plan, table_index)
    }

    unsafe fn deallocate_table(
        &self,
        table_index: DefinedTableIndex,
        allocation_index: TableAllocationIndex,
        table: Table,
    ) {
        self.ondemand
            .deallocate_table(table_index, allocation_index, table)
    }

    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack> {
        unreachable!()
    }

    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, _stack: &wasmtime_fiber::FiberStack) {
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
        _gc_runtime: &dyn crate::runtime::vm::GcRuntime,
    ) -> Result<(GcHeapAllocationIndex, Box<dyn crate::runtime::vm::GcHeap>)> {
        unreachable!()
    }

    #[cfg(feature = "gc")]
    fn deallocate_gc_heap(
        &self,
        _allocation_index: GcHeapAllocationIndex,
        _gc_heap: Box<dyn crate::runtime::vm::GcHeap>,
    ) {
        unreachable!()
    }
}
