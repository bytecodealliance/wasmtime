//! An `Instance` contains all the runtime state used by execution of a wasm
//! module.

use cranelift_entity::EntityRef;
use cranelift_entity::{BoxedSlice, PrimaryMap};
use cranelift_wasm::{
    DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex,
};
use imports::Imports;
use memory::LinearMemory;
use sig_registry::SignatureRegistry;
use std::string::String;
use table::Table;
use vmcontext::{
    VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMGlobalDefinition, VMMemoryDefinition,
    VMTableDefinition,
};
use wasmtime_environ::{DataInitializer, Module};

/// An Instance of a WebAssemby module.
#[derive(Debug)]
pub struct Instance {
    /// WebAssembly linear memory data.
    memories: BoxedSlice<DefinedMemoryIndex, LinearMemory>,

    /// WebAssembly table data.
    tables: BoxedSlice<DefinedTableIndex, Table>,

    /// Function Signature IDs.
    /// FIXME: This should be shared across instances rather than per-Instance.
    sig_registry: SignatureRegistry,

    /// Resolved imports.
    vmctx_imports: Imports,

    /// Table storage base address vector pointed to by vmctx.
    vmctx_tables: BoxedSlice<DefinedTableIndex, VMTableDefinition>,

    /// Memory base address vector pointed to by vmctx.
    vmctx_memories: BoxedSlice<DefinedMemoryIndex, VMMemoryDefinition>,

    /// WebAssembly global variable data.
    vmctx_globals: BoxedSlice<DefinedGlobalIndex, VMGlobalDefinition>,

    /// Context pointer used by JIT code.
    vmctx: VMContext,
}

impl Instance {
    /// Create a new `Instance`.
    pub fn new(
        module: &Module,
        finished_functions: &BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
        mut vmctx_imports: Imports,
        data_initializers: &[DataInitializer],
    ) -> Result<Self, String> {
        let mut sig_registry = instantiate_signatures(module);
        let mut memories = instantiate_memories(module, data_initializers)?;
        let mut tables = instantiate_tables(
            module,
            finished_functions,
            &vmctx_imports.functions,
            &mut sig_registry,
        );

        let mut vmctx_memories = memories
            .values_mut()
            .map(LinearMemory::vmmemory)
            .collect::<PrimaryMap<DefinedMemoryIndex, _>>()
            .into_boxed_slice();

        let mut vmctx_globals = instantiate_globals(module);

        let mut vmctx_tables = tables
            .values_mut()
            .map(Table::vmtable)
            .collect::<PrimaryMap<DefinedTableIndex, _>>()
            .into_boxed_slice();

        let vmctx_imported_functions_ptr = vmctx_imports
            .functions
            .values_mut()
            .into_slice()
            .as_mut_ptr();
        let vmctx_imported_tables_ptr = vmctx_imports.tables.values_mut().into_slice().as_mut_ptr();
        let vmctx_imported_memories_ptr = vmctx_imports
            .memories
            .values_mut()
            .into_slice()
            .as_mut_ptr();
        let vmctx_imported_globals_ptr =
            vmctx_imports.globals.values_mut().into_slice().as_mut_ptr();
        let vmctx_memories_ptr = vmctx_memories.values_mut().into_slice().as_mut_ptr();
        let vmctx_globals_ptr = vmctx_globals.values_mut().into_slice().as_mut_ptr();
        let vmctx_tables_ptr = vmctx_tables.values_mut().into_slice().as_mut_ptr();
        let vmctx_shared_signatures_ptr = sig_registry.vmshared_signatures();

        Ok(Self {
            memories,
            tables,
            sig_registry,
            vmctx_imports,
            vmctx_memories,
            vmctx_globals,
            vmctx_tables,
            vmctx: VMContext::new(
                vmctx_imported_functions_ptr,
                vmctx_imported_tables_ptr,
                vmctx_imported_memories_ptr,
                vmctx_imported_globals_ptr,
                vmctx_tables_ptr,
                vmctx_memories_ptr,
                vmctx_globals_ptr,
                vmctx_shared_signatures_ptr,
            ),
        })
    }

    /// Return a reference to the vmctx used by JIT code.
    pub fn vmctx(&self) -> &VMContext {
        &self.vmctx
    }

    /// Return a mutable reference to the vmctx used by JIT code.
    pub fn vmctx_mut(&mut self) -> &mut VMContext {
        &mut self.vmctx
    }

    /// Return the offset from the vmctx pointer to its containing Instance.
    pub(crate) fn vmctx_offset() -> isize {
        offset_of!(Self, vmctx) as isize
    }

    /// Grow memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub fn memory_grow(&mut self, memory_index: DefinedMemoryIndex, delta: u32) -> Option<u32> {
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
    pub fn memory_size(&mut self, memory_index: DefinedMemoryIndex) -> u32 {
        self.memories
            .get(memory_index)
            .unwrap_or_else(|| panic!("no memory for index {}", memory_index.index()))
            .size()
    }

    /// Test whether any of the objects inside this instance require signal
    /// handlers to catch out of bounds accesses.
    pub(crate) fn needs_signal_handlers(&self) -> bool {
        self.memories
            .values()
            .any(|memory| memory.needs_signal_handlers)
    }

    /// Return the number of imported memories.
    pub(crate) fn num_imported_memories(&self) -> usize {
        self.vmctx_imports.functions.len()
    }
}

fn instantiate_signatures(module: &Module) -> SignatureRegistry {
    let mut sig_registry = SignatureRegistry::new();
    for (sig_index, sig) in module.signatures.iter() {
        sig_registry.register(sig_index, sig);
    }
    sig_registry
}

/// Allocate memory for just the tables of the current module.
fn instantiate_tables(
    module: &Module,
    finished_functions: &BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
    imported_functions: &BoxedSlice<FuncIndex, *const VMFunctionBody>,
    sig_registry: &mut SignatureRegistry,
) -> BoxedSlice<DefinedTableIndex, Table> {
    let num_imports = module.imported_memories.len();
    let mut tables: PrimaryMap<DefinedTableIndex, _> =
        PrimaryMap::with_capacity(module.table_plans.len() - num_imports);
    for table in &module.table_plans.values().as_slice()[num_imports..] {
        tables.push(Table::new(table));
    }

    for init in &module.table_elements {
        debug_assert!(init.base.is_none(), "globalvar base not supported yet");
        let defined_table_index = module
            .defined_table_index(init.table_index)
            .expect("Initializers for imported tables not supported yet");
        let slice = tables[defined_table_index].as_mut();
        let subslice = &mut slice[init.offset..init.offset + init.elements.len()];
        for (i, func_idx) in init.elements.iter().enumerate() {
            let callee_sig = module.functions[*func_idx];
            let func_ptr = if let Some(index) = module.defined_func_index(*func_idx) {
                finished_functions[index]
            } else {
                imported_functions[*func_idx]
            };
            let type_index = sig_registry.lookup(callee_sig);
            subslice[i] = VMCallerCheckedAnyfunc {
                func_ptr,
                type_index,
            };
        }
    }

    tables.into_boxed_slice()
}

/// Allocate memory for just the memories of the current module.
fn instantiate_memories(
    module: &Module,
    data_initializers: &[DataInitializer],
) -> Result<BoxedSlice<DefinedMemoryIndex, LinearMemory>, String> {
    let num_imports = module.imported_memories.len();
    let mut memories: PrimaryMap<DefinedMemoryIndex, _> =
        PrimaryMap::with_capacity(module.memory_plans.len() - num_imports);
    for plan in &module.memory_plans.values().as_slice()[num_imports..] {
        memories.push(LinearMemory::new(&plan)?);
    }

    for init in data_initializers {
        debug_assert!(init.base.is_none(), "globalvar base not supported yet");
        let defined_memory_index = module
            .defined_memory_index(init.memory_index)
            .expect("Initializers for imported memories not supported yet");
        let mem_mut = memories[defined_memory_index].as_mut();
        let to_init = &mut mem_mut[init.offset..init.offset + init.data.len()];
        to_init.copy_from_slice(init.data);
    }

    Ok(memories.into_boxed_slice())
}

/// Allocate memory for just the globals of the current module,
/// without any initializers applied yet.
fn instantiate_globals(module: &Module) -> BoxedSlice<DefinedGlobalIndex, VMGlobalDefinition> {
    let num_imports = module.imported_globals.len();
    let mut vmctx_globals = PrimaryMap::with_capacity(module.globals.len() - num_imports);

    for global in &module.globals.values().as_slice()[num_imports..] {
        vmctx_globals.push(VMGlobalDefinition::new(global));
    }

    vmctx_globals.into_boxed_slice()
}
