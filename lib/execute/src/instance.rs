//! An `Instance` contains all the runtime state used by execution of a wasm
//! module.

use cranelift_entity::EntityRef;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{GlobalIndex, MemoryIndex, TableIndex};
use memory::LinearMemory;
use sig_registry::SignatureRegistry;
use std::string::String;
use table::Table;
use vmcontext::{VMCallerCheckedAnyfunc, VMContext, VMGlobal, VMMemory, VMTable};
use wasmtime_environ::{Compilation, DataInitializer, Module};

/// An Instance of a WebAssemby module.
#[derive(Debug)]
pub struct Instance {
    /// WebAssembly linear memory data.
    memories: PrimaryMap<MemoryIndex, LinearMemory>,

    /// WebAssembly table data.
    tables: PrimaryMap<TableIndex, Table>,

    /// Function Signature IDs.
    /// FIXME: This should be shared across instances rather than per-Instance.
    sig_registry: SignatureRegistry,

    /// Memory base address vector pointed to by vmctx.
    vmctx_memories: PrimaryMap<MemoryIndex, VMMemory>,

    /// WebAssembly global variable data.
    vmctx_globals: PrimaryMap<GlobalIndex, VMGlobal>,

    /// Table storage base address vector pointed to by vmctx.
    vmctx_tables: PrimaryMap<TableIndex, VMTable>,

    /// Context pointer used by JIT code.
    vmctx: VMContext,
}

impl Instance {
    /// Create a new `Instance`.
    pub fn new(
        module: &Module,
        compilation: &Compilation,
        data_initializers: &[DataInitializer],
    ) -> Result<Self, String> {
        let mut sig_registry = SignatureRegistry::new();
        let mut memories = instantiate_memories(module, data_initializers)?;
        let mut tables = instantiate_tables(module, compilation, &mut sig_registry);

        let mut vmctx_memories = memories
            .values_mut()
            .map(LinearMemory::vmmemory)
            .collect::<PrimaryMap<MemoryIndex, _>>();

        let mut vmctx_globals = instantiate_globals(module);

        let mut vmctx_tables = tables
            .values_mut()
            .map(Table::vmtable)
            .collect::<PrimaryMap<TableIndex, _>>();

        let vmctx_memories_ptr = vmctx_memories.values_mut().into_slice().as_mut_ptr();
        let vmctx_globals_ptr = vmctx_globals.values_mut().into_slice().as_mut_ptr();
        let vmctx_tables_ptr = vmctx_tables.values_mut().into_slice().as_mut_ptr();
        let signature_ids_ptr = sig_registry.vmsignature_ids();

        Ok(Self {
            memories,
            tables,
            sig_registry,
            vmctx_memories,
            vmctx_globals,
            vmctx_tables,
            vmctx: VMContext::new(
                vmctx_memories_ptr,
                vmctx_globals_ptr,
                vmctx_tables_ptr,
                signature_ids_ptr,
            ),
        })
    }

    /// Return the vmctx pointer to be passed into JIT code.
    pub fn vmctx(&mut self) -> *mut VMContext {
        &mut self.vmctx as *mut VMContext
    }

    /// Return the offset from the vmctx pointer to its containing Instance.
    pub fn vmctx_offset() -> isize {
        offset_of!(Instance, vmctx) as isize
    }

    /// Grow memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub fn memory_grow(&mut self, memory_index: MemoryIndex, delta: u32) -> Option<u32> {
        let result = self
            .memories
            .get_mut(memory_index)
            .unwrap_or_else(|| panic!("no memory for index {}", memory_index.index()))
            .grow(delta);

        // Keep current the VMContext pointers used by JIT code.
        self.vmctx_memories[memory_index] = self.memories[memory_index].vmmemory();

        result
    }

    /// Returns the number of allocated wasm pages.
    pub fn memory_size(&mut self, memory_index: MemoryIndex) -> u32 {
        self.memories
            .get(memory_index)
            .unwrap_or_else(|| panic!("no memory for index {}", memory_index.index()))
            .size()
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
    pub fn inspect_global(&self, global_index: GlobalIndex) -> &VMGlobal {
        &self.vmctx_globals[global_index]
    }
}

/// Allocate memory for just the memories of the current module.
fn instantiate_memories(
    module: &Module,
    data_initializers: &[DataInitializer],
) -> Result<PrimaryMap<MemoryIndex, LinearMemory>, String> {
    let mut memories = PrimaryMap::with_capacity(module.memory_plans.len());
    for plan in module.memory_plans.values() {
        memories.push(LinearMemory::new(&plan)?);
    }

    for init in data_initializers {
        debug_assert!(init.base.is_none(), "globalvar base not supported yet");
        let mem_mut = memories[init.memory_index].as_mut();
        let to_init = &mut mem_mut[init.offset..init.offset + init.data.len()];
        to_init.copy_from_slice(init.data);
    }

    Ok(memories)
}

/// Allocate memory for just the tables of the current module.
fn instantiate_tables(
    module: &Module,
    compilation: &Compilation,
    sig_registry: &mut SignatureRegistry,
) -> PrimaryMap<TableIndex, Table> {
    let mut tables = PrimaryMap::with_capacity(module.table_plans.len());
    for table in module.table_plans.values() {
        tables.push(Table::new(table));
    }

    for init in &module.table_elements {
        debug_assert!(init.base.is_none(), "globalvar base not supported yet");
        let slice = &mut tables[init.table_index].as_mut();
        let subslice = &mut slice[init.offset..init.offset + init.elements.len()];
        for (i, func_idx) in init.elements.iter().enumerate() {
            let callee_sig = module.functions[*func_idx];
            let code_buf = &compilation.functions[module
                .defined_func_index(*func_idx)
                .expect("table element initializer with imported function not supported yet")];
            let type_id = sig_registry.register(callee_sig, &module.signatures[callee_sig]);
            subslice[i] = VMCallerCheckedAnyfunc {
                func_ptr: code_buf.as_ptr(),
                type_id,
            };
        }
    }

    tables
}

/// Allocate memory for just the globals of the current module,
/// without any initializers applied yet.
fn instantiate_globals(module: &Module) -> PrimaryMap<GlobalIndex, VMGlobal> {
    let mut vmctx_globals = PrimaryMap::with_capacity(module.globals.len());

    for _ in 0..module.globals.len() {
        vmctx_globals.push(VMGlobal::default());
    }

    vmctx_globals
}
