//! An `Instance` contains all the runtime state used by execution of a wasm
//! module.

use cranelift_codegen::ir;
use cranelift_entity::EntityRef;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{GlobalIndex, MemoryIndex, TableIndex};
use memory::LinearMemory;
use std::string::String;
use std::vec::Vec;
use wasmtime_environ::{Compilation, DataInitializer, Module, TableElements};

/// An Instance of a WebAssemby module.
#[derive(Debug)]
pub struct Instance {
    /// WebAssembly table data.
    pub tables: PrimaryMap<TableIndex, Vec<usize>>,

    /// WebAssembly linear memory data.
    pub memories: PrimaryMap<MemoryIndex, LinearMemory>,

    /// WebAssembly global variable data.
    pub globals: Vec<u8>,

    /// Memory base address vector pointed to by vmctx.
    pub mem_base_addrs: Vec<*mut u8>,
}

impl Instance {
    /// Create a new `Instance`.
    pub fn new(
        module: &Module,
        compilation: &Compilation,
        data_initializers: &[DataInitializer],
    ) -> Result<Self, String> {
        let mut result = Self {
            tables: PrimaryMap::new(),
            memories: PrimaryMap::new(),
            globals: Vec::new(),
            mem_base_addrs: Vec::new(),
        };
        result.instantiate_tables(module, compilation, &module.table_elements);
        result.instantiate_memories(module, data_initializers)?;
        result.instantiate_globals(module);
        Ok(result)
    }

    /// Allocate memory in `self` for just the tables of the current module.
    fn instantiate_tables(
        &mut self,
        module: &Module,
        compilation: &Compilation,
        table_initializers: &[TableElements],
    ) {
        debug_assert!(self.tables.is_empty());
        self.tables.reserve_exact(module.tables.len());
        for table in module.tables.values() {
            let len = table.minimum as usize;
            let mut v = Vec::with_capacity(len);
            v.resize(len, 0);
            self.tables.push(v);
        }
        for init in table_initializers {
            debug_assert!(init.base.is_none(), "globalvar base not supported yet");
            let to_init =
                &mut self.tables[init.table_index][init.offset..init.offset + init.elements.len()];
            for (i, func_idx) in init.elements.iter().enumerate() {
                let code_buf = &compilation.functions[module.defined_func_index(*func_idx).expect(
                    "table element initializer with imported function not supported yet",
                )];
                to_init[i] = code_buf.as_ptr() as usize;
            }
        }
    }

    /// Allocate memory in `instance` for just the memories of the current module.
    fn instantiate_memories(
        &mut self,
        module: &Module,
        data_initializers: &[DataInitializer],
    ) -> Result<(), String> {
        debug_assert!(self.memories.is_empty());
        // Allocate the underlying memory and initialize it to all zeros.
        self.memories.reserve_exact(module.memory_plans.len());
        for plan in module.memory_plans.values() {
            let v = LinearMemory::new(&plan)?;
            self.memories.push(v);
        }
        for init in data_initializers {
            debug_assert!(init.base.is_none(), "globalvar base not supported yet");
            let mem_mut = self.memories[init.memory_index].as_mut();
            let to_init = &mut mem_mut[init.offset..init.offset + init.data.len()];
            to_init.copy_from_slice(init.data);
        }
        Ok(())
    }

    /// Allocate memory in `instance` for just the globals of the current module,
    /// without any initializers applied yet.
    fn instantiate_globals(&mut self, module: &Module) {
        debug_assert!(self.globals.is_empty());
        // Allocate the underlying memory and initialize it to all zeros.
        let globals_data_size = module.globals.len() * 8;
        self.globals.resize(globals_data_size, 0);
    }

    /// Returns a mutable reference to a linear memory under the specified index.
    pub fn memory_mut(&mut self, memory_index: MemoryIndex) -> &mut LinearMemory {
        self.memories
            .get_mut(memory_index)
            .unwrap_or_else(|| panic!("no memory for index {}", memory_index.index()))
    }

    /// Returns a slice of the contents of allocated linear memory.
    pub fn inspect_memory(&self, memory_index: MemoryIndex, address: usize, len: usize) -> &[u8] {
        &self
            .memories
            .get(memory_index)
            .unwrap_or_else(|| panic!("no memory for index {}", memory_index.index()))
            .as_ref()[address..address + len]
    }

    /// Shows the value of a global variable.
    pub fn inspect_global(&self, global_index: GlobalIndex, ty: ir::Type) -> &[u8] {
        let offset = global_index.index() * 8;
        let len = ty.bytes() as usize;
        &self.globals[offset..offset + len]
    }
}
