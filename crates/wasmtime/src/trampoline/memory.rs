use crate::memory::{LinearMemory, MemoryCreator};
use crate::store::{InstanceId, StoreOpaque};
use crate::trampoline::create_handle;
use crate::MemoryType;
use anyhow::{anyhow, Result};
use std::convert::TryFrom;
use std::sync::Arc;
use wasmtime_environ::{EntityIndex, MemoryPlan, MemoryStyle, Module, WASM_PAGE_SIZE};
use wasmtime_runtime::{RuntimeLinearMemory, RuntimeMemoryCreator, VMMemoryDefinition};

pub fn create_memory(store: &mut StoreOpaque<'_>, memory: &MemoryType) -> Result<InstanceId> {
    let mut module = Module::new();

    let memory_plan = wasmtime_environ::MemoryPlan::for_memory(
        memory.wasmtime_memory().clone(),
        &store.engine().config().tunables,
    );
    let memory_id = module.memory_plans.push(memory_plan);
    module
        .exports
        .insert(String::new(), EntityIndex::Memory(memory_id));

    create_handle(module, store, Box::new(()), &[], None)
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

    fn grow_to(&mut self, new_size: usize) -> Option<()> {
        self.mem.grow_to(new_size)
    }

    fn vmmemory(&self) -> VMMemoryDefinition {
        VMMemoryDefinition {
            base: self.mem.as_ptr(),
            current_length: self.mem.byte_size(),
        }
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
    ) -> Result<Box<dyn RuntimeLinearMemory>> {
        let ty = MemoryType::from_wasmtime_memory(&plan.memory);
        let reserved_size_in_bytes = match plan.style {
            MemoryStyle::Static { bound } => {
                Some(usize::try_from(bound * (WASM_PAGE_SIZE as u64)).unwrap())
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
