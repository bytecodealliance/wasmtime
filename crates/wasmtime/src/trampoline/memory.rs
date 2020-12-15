use super::create_handle::create_handle;
use crate::externals::{LinearMemory, MemoryCreator};
use crate::trampoline::StoreInstanceHandle;
use crate::Store;
use crate::{Limits, MemoryType};
use anyhow::Result;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::{wasm, MemoryPlan, MemoryStyle, Module, WASM_PAGE_SIZE};
use wasmtime_runtime::{RuntimeLinearMemory, RuntimeMemoryCreator, VMMemoryDefinition};

use std::sync::Arc;

pub fn create_handle_with_memory(
    store: &Store,
    memory: &MemoryType,
) -> Result<StoreInstanceHandle> {
    let mut module = Module::new();

    let memory = wasm::Memory {
        minimum: memory.limits().min(),
        maximum: memory.limits().max(),
        shared: false, // TODO
    };

    let memory_plan =
        wasmtime_environ::MemoryPlan::for_memory(memory, &store.engine().config().tunables);
    let memory_id = module.memory_plans.push(memory_plan);
    module
        .exports
        .insert(String::new(), wasm::EntityIndex::Memory(memory_id));

    create_handle(module, store, PrimaryMap::new(), Box::new(()), &[], None)
}

struct LinearMemoryProxy {
    mem: Box<dyn LinearMemory>,
}

impl RuntimeLinearMemory for LinearMemoryProxy {
    fn size(&self) -> u32 {
        self.mem.size()
    }

    fn grow(&self, delta: u32) -> Option<u32> {
        self.mem.grow(delta)
    }

    fn vmmemory(&self) -> VMMemoryDefinition {
        VMMemoryDefinition {
            base: self.mem.as_ptr(),
            current_length: self.mem.size() as usize * WASM_PAGE_SIZE as usize,
        }
    }
}

#[derive(Clone)]
pub(crate) struct MemoryCreatorProxy {
    pub(crate) mem_creator: Arc<dyn MemoryCreator>,
}

impl RuntimeMemoryCreator for MemoryCreatorProxy {
    fn new_memory(&self, plan: &MemoryPlan) -> Result<Box<dyn RuntimeLinearMemory>, String> {
        let ty = MemoryType::new(Limits::new(plan.memory.minimum, plan.memory.maximum));
        let reserved_size_in_bytes = match plan.style {
            MemoryStyle::Static { bound } => Some(bound as u64 * WASM_PAGE_SIZE as u64),
            MemoryStyle::Dynamic => None,
        };
        self.mem_creator
            .new_memory(ty, reserved_size_in_bytes, plan.offset_guard_size)
            .map(|mem| Box::new(LinearMemoryProxy { mem }) as Box<dyn RuntimeLinearMemory>)
    }
}
