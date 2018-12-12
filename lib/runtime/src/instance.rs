//! An `Instance` contains all the runtime state used by execution of a wasm
//! module.

use cranelift_entity::EntityRef;
use cranelift_entity::{BoxedSlice, PrimaryMap};
use cranelift_wasm::{
    DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, GlobalInit,
};
use export::Export;
use imports::Imports;
use memory::LinearMemory;
use sig_registry::SignatureRegistry;
use signalhandlers::{wasmtime_init_eager, wasmtime_init_finish};
use std::rc::Rc;
use std::slice;
use std::string::String;
use table::Table;
use traphandlers::wasmtime_call;
use vmcontext::{
    VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMGlobalDefinition, VMMemoryDefinition,
    VMTableDefinition,
};
use wasmtime_environ::{DataInitializer, Module};

/// An Instance of a WebAssemby module.
#[derive(Debug)]
pub struct Instance {
    /// The `Module` this `Instance` was instantiated from.
    module: Rc<Module>,

    /// WebAssembly linear memory data.
    memories: BoxedSlice<DefinedMemoryIndex, LinearMemory>,

    /// WebAssembly table data.
    tables: BoxedSlice<DefinedTableIndex, Table>,

    /// Function Signature IDs.
    /// FIXME: This should be shared across instances rather than per-Instance.
    sig_registry: SignatureRegistry,

    /// Resolved imports.
    vmctx_imports: Imports,

    /// Pointers to functions in executable memory.
    finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,

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
        module: Rc<Module>,
        finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
        mut vmctx_imports: Imports,
        data_initializers: Vec<DataInitializer>,
    ) -> Result<Box<Self>, InstantiationError> {
        let mut sig_registry = create_and_initialize_signatures(&module);
        let mut tables = create_tables(&module);
        let mut memories = create_memories(&module)?;

        let mut vmctx_tables = tables
            .values_mut()
            .map(Table::vmtable)
            .collect::<PrimaryMap<DefinedTableIndex, _>>()
            .into_boxed_slice();

        let mut vmctx_memories = memories
            .values_mut()
            .map(LinearMemory::vmmemory)
            .collect::<PrimaryMap<DefinedMemoryIndex, _>>()
            .into_boxed_slice();

        let mut vmctx_globals = create_globals(&module);

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
        let vmctx_tables_ptr = vmctx_tables.values_mut().into_slice().as_mut_ptr();
        let vmctx_memories_ptr = vmctx_memories.values_mut().into_slice().as_mut_ptr();
        let vmctx_globals_ptr = vmctx_globals.values_mut().into_slice().as_mut_ptr();
        let vmctx_shared_signatures_ptr = sig_registry.vmshared_signatures();

        let mut result = Box::new(Self {
            module,
            memories,
            tables,
            sig_registry,
            vmctx_imports,
            finished_functions,
            vmctx_tables,
            vmctx_memories,
            vmctx_globals,
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
        });

        // Check initializer bounds before initializing anything.
        check_table_init_bounds(&mut *result)?;
        check_memory_init_bounds(&mut *result, &data_initializers)?;

        // Apply the initializers.
        initialize_tables(&mut *result)?;
        initialize_memories(&mut *result, data_initializers)?;
        initialize_globals(&mut *result);

        // Rather than writing inline assembly to jump to the code region, we use the fact that
        // the Rust ABI for calling a function with no arguments and no return values matches the one
        // of the generated code. Thanks to this, we can transmute the code region into a first-class
        // Rust function and call it.
        // Ensure that our signal handlers are ready for action.
        // TODO: Move these calls out of `Instance`.
        wasmtime_init_eager();
        wasmtime_init_finish(result.vmctx_mut());

        // The WebAssembly spec specifies that the start function is
        // invoked automatically at instantiation time.
        result.invoke_start_function()?;

        Ok(result)
    }

    /// Return a reference to the vmctx used by JIT code.
    pub fn vmctx(&self) -> &VMContext {
        &self.vmctx
    }

    /// Return a raw pointer to the vmctx used by JIT code.
    pub fn vmctx_ptr(&self) -> *const VMContext {
        self.vmctx()
    }

    /// Return a mutable reference to the vmctx used by JIT code.
    pub fn vmctx_mut(&mut self) -> &mut VMContext {
        &mut self.vmctx
    }

    /// Return a mutable raw pointer to the vmctx used by JIT code.
    pub fn vmctx_mut_ptr(&mut self) -> *mut VMContext {
        self.vmctx_mut()
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
        self.vmctx_imports.memories.len()
    }

    /// Invoke the WebAssembly start function of the instance, if one is present.
    fn invoke_start_function(&mut self) -> Result<(), InstantiationError> {
        if let Some(start_index) = self.module.start_func {
            let (callee_address, callee_vmctx) = match self.module.defined_func_index(start_index) {
                Some(defined_start_index) => {
                    let body = *self
                        .finished_functions
                        .get(defined_start_index)
                        .expect("start function index is out of bounds");
                    (body, self.vmctx_mut() as *mut VMContext)
                }
                None => {
                    assert!(start_index.index() < self.module.imported_funcs.len());
                    let import = unsafe { self.vmctx.imported_function(start_index) };
                    (import.body, import.vmctx)
                }
            };

            // Make the call.
            unsafe { wasmtime_call(callee_address, callee_vmctx) }
                .map_err(InstantiationError::StartTrap)?;
        }

        Ok(())
    }

    /// Lookup an export with the given name.
    pub fn lookup(&mut self, field: &str) -> Option<Export> {
        if let Some(export) = self.module.exports.get(field) {
            Some(match export {
                wasmtime_environ::Export::Function(index) => {
                    let signature = self.module.signatures[self.module.functions[*index]].clone();
                    let (address, vmctx) =
                        if let Some(def_index) = self.module.defined_func_index(*index) {
                            (
                                self.finished_functions[def_index],
                                &mut self.vmctx as *mut VMContext,
                            )
                        } else {
                            let import = unsafe { self.vmctx.imported_function(*index) };
                            (import.body, import.vmctx)
                        };
                    Export::Function {
                        address,
                        signature,
                        vmctx,
                    }
                }
                wasmtime_environ::Export::Table(index) => {
                    let (address, vmctx) = if let Some(def_index) =
                        self.module.defined_table_index(*index)
                    {
                        (
                            unsafe { self.vmctx.table_mut(def_index) } as *mut VMTableDefinition,
                            &mut self.vmctx as *mut VMContext,
                        )
                    } else {
                        let import = unsafe { self.vmctx.imported_table(*index) };
                        (import.from, import.vmctx)
                    };
                    Export::Table {
                        address,
                        vmctx,
                        table: self.module.table_plans[*index].clone(),
                    }
                }
                wasmtime_environ::Export::Memory(index) => {
                    let (address, vmctx) = if let Some(def_index) =
                        self.module.defined_memory_index(*index)
                    {
                        (
                            unsafe { self.vmctx.memory_mut(def_index) } as *mut VMMemoryDefinition,
                            &mut self.vmctx as *mut VMContext,
                        )
                    } else {
                        let import = unsafe { self.vmctx.imported_memory(*index) };
                        (import.from, import.vmctx)
                    };
                    Export::Memory {
                        address,
                        vmctx,
                        memory: self.module.memory_plans[*index].clone(),
                    }
                }
                wasmtime_environ::Export::Global(index) => Export::Global {
                    address: if let Some(def_index) = self.module.defined_global_index(*index) {
                        unsafe { self.vmctx.global_mut(def_index) }
                    } else {
                        unsafe { self.vmctx.imported_global(*index).from }
                    },
                    global: self.module.globals[*index],
                },
            })
        } else {
            None
        }
    }

    /// Lookup an export with the given name. This takes an immutable reference,
    /// and the result is an `Export` that can only be used to read, not write.
    /// This requirement is not enforced in the type system, so this function is
    /// unsafe.
    pub unsafe fn lookup_immutable(&self, field: &str) -> Option<Export> {
        let temporary_mut = &mut *(self as *const Instance as *mut Instance);
        temporary_mut.lookup(field)
    }
}

fn check_table_init_bounds(instance: &mut Instance) -> Result<(), InstantiationError> {
    for init in &instance.module.table_elements {
        // TODO: Refactor this.
        let mut start = init.offset;
        if let Some(base) = init.base {
            let global = if let Some(def_index) = instance.module.defined_global_index(base) {
                unsafe { instance.vmctx.global_mut(def_index) }
            } else {
                unsafe { instance.vmctx.imported_global(base).from }
            };
            start += unsafe { *(&*global).as_i32() } as u32 as usize;
        }

        // TODO: Refactor this.
        let slice = if let Some(defined_table_index) =
            instance.module.defined_table_index(init.table_index)
        {
            instance.tables[defined_table_index].as_mut()
        } else {
            let import = &instance.vmctx_imports.tables[init.table_index];
            let foreign_instance = unsafe { (&mut *(import).vmctx).instance() };
            let foreign_table = unsafe { &mut *(import).from };
            let foreign_index = foreign_instance.vmctx().table_index(foreign_table);
            foreign_instance.tables[foreign_index].as_mut()
        };

        if slice.get_mut(start..start + init.elements.len()).is_none() {
            return Err(InstantiationError::Link(
                "elements segment does not fit".to_owned(),
            ));
        }
    }

    Ok(())
}

fn check_memory_init_bounds(
    instance: &mut Instance,
    data_initializers: &[DataInitializer],
) -> Result<(), InstantiationError> {
    for init in data_initializers {
        // TODO: Refactor this.
        let mut start = init.offset;
        if let Some(base) = init.base {
            let global = if let Some(def_index) = instance.module.defined_global_index(base) {
                unsafe { instance.vmctx.global_mut(def_index) }
            } else {
                unsafe { instance.vmctx.imported_global(base).from }
            };
            start += unsafe { *(&*global).as_i32() } as u32 as usize;
        }

        // TODO: Refactor this.
        let memory = if let Some(defined_memory_index) =
            instance.module.defined_memory_index(init.memory_index)
        {
            unsafe { instance.vmctx.memory(defined_memory_index) }
        } else {
            let import = &instance.vmctx_imports.memories[init.memory_index];
            let foreign_instance = unsafe { (&mut *(import).vmctx).instance() };
            let foreign_memory = unsafe { &mut *(import).from };
            let foreign_index = foreign_instance.vmctx().memory_index(foreign_memory);
            unsafe { foreign_instance.vmctx.memory(foreign_index) }
        };
        let mem_slice = unsafe { slice::from_raw_parts_mut(memory.base, memory.current_length) };

        if mem_slice.get_mut(start..start + init.data.len()).is_none() {
            return Err(InstantiationError::Link(
                "data segment does not fit".to_owned(),
            ));
        }
    }

    Ok(())
}

fn create_and_initialize_signatures(module: &Module) -> SignatureRegistry {
    let mut sig_registry = SignatureRegistry::new();
    for (sig_index, sig) in module.signatures.iter() {
        sig_registry.register(sig_index, sig);
    }
    sig_registry
}

/// Allocate memory for just the tables of the current module.
fn create_tables(module: &Module) -> BoxedSlice<DefinedTableIndex, Table> {
    let num_imports = module.imported_tables.len();
    let mut tables: PrimaryMap<DefinedTableIndex, _> =
        PrimaryMap::with_capacity(module.table_plans.len() - num_imports);
    for table in &module.table_plans.values().as_slice()[num_imports..] {
        tables.push(Table::new(table));
    }
    tables.into_boxed_slice()
}

/// Initialize the table memory from the provided initializers.
fn initialize_tables(instance: &mut Instance) -> Result<(), InstantiationError> {
    let vmctx: *mut VMContext = instance.vmctx_mut();
    for init in &instance.module.table_elements {
        let mut start = init.offset;
        if let Some(base) = init.base {
            let global = if let Some(def_index) = instance.module.defined_global_index(base) {
                unsafe { instance.vmctx.global_mut(def_index) }
            } else {
                unsafe { instance.vmctx.imported_global(base).from }
            };
            start += unsafe { *(&*global).as_i32() } as u32 as usize;
        }

        let slice = if let Some(defined_table_index) =
            instance.module.defined_table_index(init.table_index)
        {
            instance.tables[defined_table_index].as_mut()
        } else {
            let import = &instance.vmctx_imports.tables[init.table_index];
            let foreign_instance = unsafe { (&mut *(import).vmctx).instance() };
            let foreign_table = unsafe { &mut *(import).from };
            let foreign_index = foreign_instance.vmctx().table_index(foreign_table);
            foreign_instance.tables[foreign_index].as_mut()
        };
        if let Some(subslice) = slice.get_mut(start..start + init.elements.len()) {
            for (i, func_idx) in init.elements.iter().enumerate() {
                let callee_sig = instance.module.functions[*func_idx];
                let (callee_ptr, callee_vmctx) =
                    if let Some(index) = instance.module.defined_func_index(*func_idx) {
                        (instance.finished_functions[index], vmctx)
                    } else {
                        let imported_func = &instance.vmctx_imports.functions[*func_idx];
                        (imported_func.body, imported_func.vmctx)
                    };
                let type_index = instance.sig_registry.lookup(callee_sig);
                subslice[i] = VMCallerCheckedAnyfunc {
                    func_ptr: callee_ptr,
                    type_index,
                    vmctx: callee_vmctx,
                };
            }
        } else {
            return Err(InstantiationError::Link(
                "elements segment does not fit".to_owned(),
            ));
        }
    }

    Ok(())
}

/// Allocate memory for just the memories of the current module.
fn create_memories(
    module: &Module,
) -> Result<BoxedSlice<DefinedMemoryIndex, LinearMemory>, InstantiationError> {
    let num_imports = module.imported_memories.len();
    let mut memories: PrimaryMap<DefinedMemoryIndex, _> =
        PrimaryMap::with_capacity(module.memory_plans.len() - num_imports);
    for plan in &module.memory_plans.values().as_slice()[num_imports..] {
        memories.push(LinearMemory::new(&plan).map_err(InstantiationError::Resource)?);
    }
    Ok(memories.into_boxed_slice())
}

/// Initialize the table memory from the provided initializers.
fn initialize_memories(
    instance: &mut Instance,
    data_initializers: Vec<DataInitializer>,
) -> Result<(), InstantiationError> {
    for init in data_initializers {
        let mut start = init.offset;
        if let Some(base) = init.base {
            let global = if let Some(def_index) = instance.module.defined_global_index(base) {
                unsafe { instance.vmctx.global_mut(def_index) }
            } else {
                unsafe { instance.vmctx.imported_global(base).from }
            };
            start += unsafe { *(&*global).as_i32() } as u32 as usize;
        }

        let memory = if let Some(defined_memory_index) =
            instance.module.defined_memory_index(init.memory_index)
        {
            unsafe { instance.vmctx.memory(defined_memory_index) }
        } else {
            let import = &instance.vmctx_imports.memories[init.memory_index];
            let foreign_instance = unsafe { (&mut *(import).vmctx).instance() };
            let foreign_memory = unsafe { &mut *(import).from };
            let foreign_index = foreign_instance.vmctx().memory_index(foreign_memory);
            unsafe { foreign_instance.vmctx.memory(foreign_index) }
        };
        let mem_slice = unsafe { slice::from_raw_parts_mut(memory.base, memory.current_length) };
        if let Some(to_init) = mem_slice.get_mut(start..start + init.data.len()) {
            to_init.copy_from_slice(init.data);
        } else {
            return Err(InstantiationError::Link(
                "data segment does not fit".to_owned(),
            ));
        }
    }

    Ok(())
}

/// Allocate memory for just the globals of the current module,
/// with initializers applied.
fn create_globals(module: &Module) -> BoxedSlice<DefinedGlobalIndex, VMGlobalDefinition> {
    let num_imports = module.imported_globals.len();
    let mut vmctx_globals = PrimaryMap::with_capacity(module.globals.len() - num_imports);

    for _ in &module.globals.values().as_slice()[num_imports..] {
        vmctx_globals.push(VMGlobalDefinition::new());
    }

    vmctx_globals.into_boxed_slice()
}

fn initialize_globals(instance: &mut Instance) {
    let num_imports = instance.module.imported_globals.len();
    for (index, global) in instance.module.globals.iter().skip(num_imports) {
        let def_index = instance.module.defined_global_index(index).unwrap();
        let to: *mut VMGlobalDefinition = unsafe { instance.vmctx.global_mut(def_index) };
        match global.initializer {
            GlobalInit::I32Const(x) => *unsafe { (*to).as_i32_mut() } = x,
            GlobalInit::I64Const(x) => *unsafe { (*to).as_i64_mut() } = x,
            GlobalInit::F32Const(x) => *unsafe { (*to).as_f32_bits_mut() } = x,
            GlobalInit::F64Const(x) => *unsafe { (*to).as_f64_bits_mut() } = x,
            GlobalInit::GetGlobal(x) => {
                let from = if let Some(def_x) = instance.module.defined_global_index(x) {
                    unsafe { instance.vmctx.global_mut(def_x) }
                } else {
                    unsafe { instance.vmctx.imported_global(x).from }
                };
                unsafe { *to = *from };
            }
            GlobalInit::Import => panic!("locally-defined global initialized as import"),
        }
    }
}

/// An error while instantiating a module.
#[derive(Fail, Debug)]
pub enum InstantiationError {
    /// Insufficient resources available for execution.
    #[fail(display = "Insufficient resources: {}", _0)]
    Resource(String),

    /// A wasm translation error occured.
    #[fail(display = "Link error: {}", _0)]
    Link(String),

    /// A compilation error occured.
    #[fail(display = "Trap occurred while invoking start function: {}", _0)]
    StartTrap(String),
}
