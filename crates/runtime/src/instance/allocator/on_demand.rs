use super::{InstanceAllocationRequest, InstanceAllocator};
use crate::instance::RuntimeMemoryCreator;
use crate::memory::{DefaultMemoryCreator, Memory};
use crate::table::Table;
use crate::CompiledModuleId;
use anyhow::Result;
use std::sync::Arc;
use wasmtime_environ::{DefinedMemoryIndex, DefinedTableIndex, PrimaryMap};

/// Represents the on-demand instance allocator.
#[derive(Clone)]
pub struct OnDemandInstanceAllocator {
    mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>,
    #[cfg(feature = "async")]
    stack_size: usize,
}

impl OnDemandInstanceAllocator {
    /// Creates a new on-demand instance allocator.
    pub fn new(mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>, stack_size: usize) -> Self {
        let _ = stack_size; // suppress unused warnings w/o async feature
        Self {
            mem_creator,
            #[cfg(feature = "async")]
            stack_size,
        }
    }
}

impl Default for OnDemandInstanceAllocator {
    fn default() -> Self {
        Self {
            mem_creator: None,
            #[cfg(feature = "async")]
            stack_size: 0,
        }
    }
}

unsafe impl InstanceAllocator for OnDemandInstanceAllocator {
    fn allocate_index(&self, _req: &InstanceAllocationRequest) -> Result<usize> {
        Ok(0)
    }

    fn deallocate_index(&self, index: usize) {
        assert_eq!(index, 0);
    }

    fn allocate_memories(
        &self,
        _index: usize,
        req: &mut InstanceAllocationRequest,
        memories: &mut PrimaryMap<DefinedMemoryIndex, Memory>,
    ) -> Result<()> {
        let module = req.runtime_info.module();
        let creator = self
            .mem_creator
            .as_deref()
            .unwrap_or_else(|| &DefaultMemoryCreator);
        let num_imports = module.num_imported_memories;
        for (memory_idx, plan) in module.memory_plans.iter().skip(num_imports) {
            let defined_memory_idx = module
                .defined_memory_index(memory_idx)
                .expect("Skipped imports, should never be None");
            let image = req.runtime_info.memory_image(defined_memory_idx)?;

            memories.push(Memory::new_dynamic(
                plan,
                creator,
                unsafe {
                    req.store
                        .get()
                        .expect("if module has memory plans, store is not empty")
                },
                image,
            )?);
        }
        Ok(())
    }

    fn deallocate_memories(
        &self,
        _index: usize,
        _mems: &mut PrimaryMap<DefinedMemoryIndex, Memory>,
    ) {
        // normal destructors do cleanup here
    }

    fn allocate_tables(
        &self,
        _index: usize,
        req: &mut InstanceAllocationRequest,
        tables: &mut PrimaryMap<DefinedTableIndex, Table>,
    ) -> Result<()> {
        let module = req.runtime_info.module();
        let num_imports = module.num_imported_tables;
        for (_, table) in module.table_plans.iter().skip(num_imports) {
            tables.push(Table::new_dynamic(table, unsafe {
                req.store
                    .get()
                    .expect("if module has table plans, store is not empty")
            })?);
        }
        Ok(())
    }

    fn deallocate_tables(&self, _index: usize, _tables: &mut PrimaryMap<DefinedTableIndex, Table>) {
        // normal destructors do cleanup here
    }

    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack> {
        if self.stack_size == 0 {
            anyhow::bail!("fiber stacks are not supported by the allocator")
        }

        let stack = wasmtime_fiber::FiberStack::new(self.stack_size)?;
        Ok(stack)
    }

    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, _stack: &wasmtime_fiber::FiberStack) {
        // The on-demand allocator has no further bookkeeping for fiber stacks
    }

    fn purge_module(&self, _: CompiledModuleId) {}
}
