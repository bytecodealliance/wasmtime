use crate::externref::{StackMapRegistry, VMExternRefActivationsTable};
use crate::imports::Imports;
use crate::instance::{Instance, InstanceHandle, RuntimeMemoryCreator};
use crate::memory::{DefaultMemoryCreator, RuntimeLinearMemory};
use crate::table::{Table, TableElement};
use crate::traphandlers::Trap;
use crate::vmcontext::{
    VMBuiltinFunctionsArray, VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMFunctionImport,
    VMGlobalDefinition, VMGlobalImport, VMInterrupts, VMMemoryDefinition, VMMemoryImport,
    VMSharedSignatureIndex, VMTableDefinition, VMTableImport,
};
use std::alloc;
use std::any::Any;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::ptr::{self, NonNull};
use std::slice;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_environ::entity::{
    packed_option::ReservedValue, BoxedSlice, EntityRef, EntitySet, PrimaryMap,
};
use wasmtime_environ::wasm::{
    DefinedFuncIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex, GlobalInit, SignatureIndex,
    TableElementType, WasmType,
};
use wasmtime_environ::{
    ir, Module, ModuleTranslation, ModuleType, OwnedDataInitializer, TableElements, VMOffsets,
};

/// Represents a request for a new runtime instance.
pub struct InstanceAllocationRequest<'a> {
    /// The module being instantiated.
    pub module: Arc<Module>,

    /// The finished (JIT) functions for the module.
    pub finished_functions: &'a PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,

    /// The imports to use for the instantiation.
    pub imports: Imports<'a>,

    /// A callback for looking up shared signature indexes.
    pub lookup_shared_signature: &'a dyn Fn(SignatureIndex) -> VMSharedSignatureIndex,

    /// The host state to associate with the instance.
    pub host_state: Box<dyn Any>,

    /// The pointer to the VM interrupts structure to use for the instance.
    pub interrupts: *const VMInterrupts,

    /// The pointer to the reference activations table to use for the instance.
    pub externref_activations_table: *mut VMExternRefActivationsTable,

    /// The pointer to the stack map registry to use for the instance.
    pub stack_map_registry: *mut StackMapRegistry,
}

/// An link error while instantiating a module.
#[derive(Error, Debug)]
#[error("Link error: {0}")]
pub struct LinkError(pub String);

/// An error while instantiating a module.
#[derive(Error, Debug)]
pub enum InstantiationError {
    /// Insufficient resources available for execution.
    #[error("Insufficient resources: {0}")]
    Resource(String),

    /// A wasm link error occured.
    #[error("Failed to link module")]
    Link(#[from] LinkError),

    /// A trap ocurred during instantiation, after linking.
    #[error("Trap occurred during instantiation")]
    Trap(Trap),
}

/// Represents a runtime instance allocator.
///
/// # Safety
///
/// This trait is unsafe as it requires knowledge of Wasmtime's runtime internals to implement correctly.
pub unsafe trait InstanceAllocator: Send + Sync {
    /// Validates a module translation.
    ///
    /// This is used to ensure a module being compiled is supported by the instance allocator.
    fn validate_module(&self, translation: &ModuleTranslation) -> Result<(), String> {
        drop(translation);
        Ok(())
    }

    /// Adjusts the tunables prior to creation of any JIT compiler.
    ///
    /// This method allows the instance allocator control over tunables passed to a `wasmtime_jit::Compiler`.
    fn adjust_tunables(&self, tunables: &mut wasmtime_environ::Tunables) {
        drop(tunables);
    }

    /// Allocates an instance for the given allocation request.
    ///
    /// # Safety
    ///
    /// This method is not inherently unsafe, but care must be made to ensure
    /// pointers passed in the allocation request outlive the returned instance.
    unsafe fn allocate(
        &self,
        req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError>;

    /// Finishes the instantiation process started by an instance allocator.
    ///
    /// # Safety
    ///
    /// This method is only safe to call immediately after an instance has been allocated.
    unsafe fn initialize(
        &self,
        handle: &InstanceHandle,
        is_bulk_memory: bool,
        data_initializers: &Arc<[OwnedDataInitializer]>,
    ) -> Result<(), InstantiationError>;

    /// Deallocates a previously allocated instance.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees that the given handle
    /// is the only owner of the underlying instance to deallocate.
    ///
    /// Use extreme care when deallocating an instance so that there are no dangling instance pointers.
    unsafe fn deallocate(&self, handle: &InstanceHandle);
}

unsafe fn initialize_vmcontext(
    instance: &Instance,
    functions: &[VMFunctionImport],
    tables: &[VMTableImport],
    memories: &[VMMemoryImport],
    globals: &[VMGlobalImport],
    finished_functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    lookup_shared_signature: &dyn Fn(SignatureIndex) -> VMSharedSignatureIndex,
    interrupts: *const VMInterrupts,
    externref_activations_table: *mut VMExternRefActivationsTable,
    stack_map_registry: *mut StackMapRegistry,
    get_mem_def: impl Fn(DefinedMemoryIndex) -> VMMemoryDefinition,
    get_table_def: impl Fn(DefinedTableIndex) -> VMTableDefinition,
) {
    let module = &instance.module;

    *instance.interrupts() = interrupts;
    *instance.externref_activations_table() = externref_activations_table;
    *instance.stack_map_registry() = stack_map_registry;

    // Initialize shared signatures
    let mut ptr = instance.signature_ids_ptr();
    for sig in module.types.values() {
        *ptr = match sig {
            ModuleType::Function(sig) => lookup_shared_signature(*sig),
            _ => VMSharedSignatureIndex::new(u32::max_value()),
        };
        ptr = ptr.add(1);
    }

    // Initialize the built-in functions
    ptr::write(
        instance.builtin_functions_ptr() as *mut VMBuiltinFunctionsArray,
        VMBuiltinFunctionsArray::initialized(),
    );

    // Initialize the imports
    debug_assert_eq!(functions.len(), module.num_imported_funcs);
    ptr::copy(
        functions.as_ptr(),
        instance.imported_functions_ptr() as *mut VMFunctionImport,
        functions.len(),
    );
    debug_assert_eq!(tables.len(), module.num_imported_tables);
    ptr::copy(
        tables.as_ptr(),
        instance.imported_tables_ptr() as *mut VMTableImport,
        tables.len(),
    );
    debug_assert_eq!(memories.len(), module.num_imported_memories);
    ptr::copy(
        memories.as_ptr(),
        instance.imported_memories_ptr() as *mut VMMemoryImport,
        memories.len(),
    );
    debug_assert_eq!(globals.len(), module.num_imported_globals);
    ptr::copy(
        globals.as_ptr(),
        instance.imported_globals_ptr() as *mut VMGlobalImport,
        globals.len(),
    );

    // Initialize the defined functions
    for (index, sig) in instance.module.functions.iter() {
        let type_index = lookup_shared_signature(*sig);

        let (func_ptr, vmctx) = if let Some(def_index) = instance.module.defined_func_index(index) {
            (
                NonNull::new(finished_functions[def_index] as *mut _).unwrap(),
                instance.vmctx_ptr(),
            )
        } else {
            let import = instance.imported_function(index);
            (import.body, import.vmctx)
        };

        ptr::write(
            instance.anyfunc_ptr(index),
            VMCallerCheckedAnyfunc {
                func_ptr,
                type_index,
                vmctx,
            },
        );
    }

    // Initialize the defined tables
    let mut ptr = instance.tables_ptr();
    for i in 0..module.table_plans.len() - module.num_imported_tables {
        ptr::write(ptr, get_table_def(DefinedTableIndex::new(i)));
        ptr = ptr.add(1);
    }

    // Initialize the defined memories
    let mut ptr = instance.memories_ptr();
    for i in 0..module.memory_plans.len() - module.num_imported_memories {
        ptr::write(ptr, get_mem_def(DefinedMemoryIndex::new(i)));
        ptr = ptr.add(1);
    }

    // Initialize the defined globals
    initialize_vmcontext_globals(instance);
}

unsafe fn initialize_vmcontext_globals(instance: &Instance) {
    let module = &instance.module;
    let num_imports = module.num_imported_globals;
    for (index, global) in module.globals.iter().skip(num_imports) {
        let def_index = module.defined_global_index(index).unwrap();
        let to = instance.global_ptr(def_index);

        // Initialize the global before writing to it
        ptr::write(to, VMGlobalDefinition::new());

        match global.initializer {
            GlobalInit::I32Const(x) => *(*to).as_i32_mut() = x,
            GlobalInit::I64Const(x) => *(*to).as_i64_mut() = x,
            GlobalInit::F32Const(x) => *(*to).as_f32_bits_mut() = x,
            GlobalInit::F64Const(x) => *(*to).as_f64_bits_mut() = x,
            GlobalInit::V128Const(x) => *(*to).as_u128_bits_mut() = x.0,
            GlobalInit::GetGlobal(x) => {
                let from = if let Some(def_x) = module.defined_global_index(x) {
                    instance.global(def_x)
                } else {
                    *instance.imported_global(x).from
                };
                *to = from;
            }
            GlobalInit::RefFunc(f) => {
                *(*to).as_anyfunc_mut() = instance.get_caller_checked_anyfunc(f).unwrap()
                    as *const VMCallerCheckedAnyfunc;
            }
            GlobalInit::RefNullConst => match global.wasm_ty {
                WasmType::FuncRef => *(*to).as_anyfunc_mut() = ptr::null(),
                WasmType::ExternRef => *(*to).as_externref_mut() = None,
                ty => panic!("unsupported reference type for global: {:?}", ty),
            },
            GlobalInit::Import => panic!("locally-defined global initialized as import"),
        }
    }
}

/// Represents the on-demand instance allocator.
#[derive(Clone)]
pub struct OnDemandInstanceAllocator {
    mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>,
}

impl OnDemandInstanceAllocator {
    /// Creates a new on-demand instance allocator.
    pub fn new(mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>) -> Self {
        Self { mem_creator }
    }

    fn create_tables(module: &Module) -> BoxedSlice<DefinedTableIndex, Table> {
        let num_imports = module.num_imported_tables;
        let mut tables: PrimaryMap<DefinedTableIndex, _> =
            PrimaryMap::with_capacity(module.table_plans.len() - num_imports);
        for table in &module.table_plans.values().as_slice()[num_imports..] {
            tables.push(Table::new(table));
        }
        tables.into_boxed_slice()
    }

    fn create_memories(
        &self,
        module: &Module,
    ) -> Result<BoxedSlice<DefinedMemoryIndex, Box<dyn RuntimeLinearMemory>>, InstantiationError>
    {
        let creator = self
            .mem_creator
            .as_deref()
            .unwrap_or_else(|| &DefaultMemoryCreator);
        let num_imports = module.num_imported_memories;
        let mut memories: PrimaryMap<DefinedMemoryIndex, _> =
            PrimaryMap::with_capacity(module.memory_plans.len() - num_imports);
        for plan in &module.memory_plans.values().as_slice()[num_imports..] {
            memories.push(
                creator
                    .new_memory(plan)
                    .map_err(InstantiationError::Resource)?,
            );
        }
        Ok(memories.into_boxed_slice())
    }

    fn check_table_init_bounds(instance: &Instance) -> Result<(), InstantiationError> {
        for init in &instance.module.table_elements {
            let start = Self::get_table_init_start(init, instance);
            let table = instance.get_table(init.table_index);

            let size = usize::try_from(table.size()).unwrap();
            if size < start + init.elements.len() {
                return Err(InstantiationError::Link(LinkError(
                    "table out of bounds: elements segment does not fit".to_owned(),
                )));
            }
        }

        Ok(())
    }

    fn get_memory_init_start(init: &OwnedDataInitializer, instance: &Instance) -> usize {
        let mut start = init.location.offset;

        if let Some(base) = init.location.base {
            let val = unsafe {
                if let Some(def_index) = instance.module.defined_global_index(base) {
                    *instance.global(def_index).as_u32()
                } else {
                    *(*instance.imported_global(base).from).as_u32()
                }
            };
            start += usize::try_from(val).unwrap();
        }

        start
    }

    unsafe fn get_memory_slice<'instance>(
        init: &OwnedDataInitializer,
        instance: &'instance Instance,
    ) -> &'instance mut [u8] {
        let memory = if let Some(defined_memory_index) = instance
            .module
            .defined_memory_index(init.location.memory_index)
        {
            instance.memory(defined_memory_index)
        } else {
            let import = instance.imported_memory(init.location.memory_index);
            let foreign_instance = (&mut *(import).vmctx).instance();
            let foreign_memory = &mut *(import).from;
            let foreign_index = foreign_instance.memory_index(foreign_memory);
            foreign_instance.memory(foreign_index)
        };
        slice::from_raw_parts_mut(memory.base, memory.current_length)
    }

    fn check_memory_init_bounds(
        instance: &Instance,
        data_initializers: &[OwnedDataInitializer],
    ) -> Result<(), InstantiationError> {
        for init in data_initializers {
            let start = Self::get_memory_init_start(init, instance);
            unsafe {
                let mem_slice = Self::get_memory_slice(init, instance);
                if mem_slice.get_mut(start..start + init.data.len()).is_none() {
                    return Err(InstantiationError::Link(LinkError(
                        "memory out of bounds: data segment does not fit".into(),
                    )));
                }
            }
        }

        Ok(())
    }

    fn get_table_init_start(init: &TableElements, instance: &Instance) -> usize {
        let mut start = init.offset;

        if let Some(base) = init.base {
            let val = unsafe {
                if let Some(def_index) = instance.module.defined_global_index(base) {
                    *instance.global(def_index).as_u32()
                } else {
                    *(*instance.imported_global(base).from).as_u32()
                }
            };
            start += usize::try_from(val).unwrap();
        }

        start
    }

    fn initialize_tables(instance: &Instance) -> Result<(), InstantiationError> {
        for init in &instance.module.table_elements {
            let start = Self::get_table_init_start(init, instance);
            let table = instance.get_table(init.table_index);

            if start
                .checked_add(init.elements.len())
                .map_or(true, |end| end > table.size() as usize)
            {
                return Err(InstantiationError::Trap(Trap::wasm(
                    ir::TrapCode::TableOutOfBounds,
                )));
            }

            for (i, func_idx) in init.elements.iter().enumerate() {
                let item = match table.element_type() {
                    TableElementType::Func => instance
                        .get_caller_checked_anyfunc(*func_idx)
                        .map_or(ptr::null_mut(), |f: &VMCallerCheckedAnyfunc| {
                            f as *const VMCallerCheckedAnyfunc as *mut VMCallerCheckedAnyfunc
                        })
                        .into(),
                    TableElementType::Val(_) => {
                        assert!(*func_idx == FuncIndex::reserved_value());
                        TableElement::ExternRef(None)
                    }
                };
                table.set(u32::try_from(start + i).unwrap(), item).unwrap();
            }
        }

        Ok(())
    }

    /// Initialize the table memory from the provided initializers.
    fn initialize_memories(
        instance: &Instance,
        data_initializers: &[OwnedDataInitializer],
    ) -> Result<(), InstantiationError> {
        for init in data_initializers {
            let memory = instance.get_memory(init.location.memory_index);

            let start = Self::get_memory_init_start(init, instance);
            if start
                .checked_add(init.data.len())
                .map_or(true, |end| end > memory.current_length)
            {
                return Err(InstantiationError::Trap(Trap::wasm(
                    ir::TrapCode::HeapOutOfBounds,
                )));
            }

            unsafe {
                let mem_slice = Self::get_memory_slice(init, instance);
                let end = start + init.data.len();
                let to_init = &mut mem_slice[start..end];
                to_init.copy_from_slice(&init.data);
            }
        }

        Ok(())
    }
}

unsafe impl InstanceAllocator for OnDemandInstanceAllocator {
    unsafe fn allocate(
        &self,
        req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError> {
        debug_assert!(!req.externref_activations_table.is_null());
        debug_assert!(!req.stack_map_registry.is_null());

        let memories = self.create_memories(&req.module)?;
        let tables = Self::create_tables(&req.module);

        let handle = {
            let instance = Instance {
                module: req.module.clone(),
                offsets: VMOffsets::new(std::mem::size_of::<*const u8>() as u8, &req.module),
                memories,
                tables,
                dropped_elements: RefCell::new(EntitySet::with_capacity(
                    req.module.passive_elements.len(),
                )),
                dropped_data: RefCell::new(EntitySet::with_capacity(req.module.passive_data.len())),
                host_state: req.host_state,
                vmctx: VMContext {},
            };
            let layout = instance.alloc_layout();
            let instance_ptr = alloc::alloc(layout) as *mut Instance;
            if instance_ptr.is_null() {
                alloc::handle_alloc_error(layout);
            }
            ptr::write(instance_ptr, instance);
            InstanceHandle::new(instance_ptr)
        };

        let instance = handle.instance();
        initialize_vmcontext(
            instance,
            req.imports.functions,
            req.imports.tables,
            req.imports.memories,
            req.imports.globals,
            req.finished_functions,
            req.lookup_shared_signature,
            req.interrupts,
            req.externref_activations_table,
            req.stack_map_registry,
            &|index| instance.memories[index].vmmemory(),
            &|index| instance.tables[index].vmtable(),
        );

        Ok(handle)
    }

    unsafe fn initialize(
        &self,
        handle: &InstanceHandle,
        is_bulk_memory: bool,
        data_initializers: &Arc<[OwnedDataInitializer]>,
    ) -> Result<(), InstantiationError> {
        // Check initializer bounds before initializing anything. Only do this
        // when bulk memory is disabled, since the bulk memory proposal changes
        // instantiation such that the intermediate results of failed
        // initializations are visible.
        if !is_bulk_memory {
            Self::check_table_init_bounds(handle.instance())?;
            Self::check_memory_init_bounds(handle.instance(), data_initializers.as_ref())?;
        }

        // Apply fallible initializers. Note that this can "leak" state even if
        // it fails.
        Self::initialize_tables(handle.instance())?;
        Self::initialize_memories(handle.instance(), data_initializers.as_ref())?;

        Ok(())
    }

    unsafe fn deallocate(&self, handle: &InstanceHandle) {
        let instance = handle.instance();
        let layout = instance.alloc_layout();
        ptr::drop_in_place(instance as *const Instance as *mut Instance);
        alloc::dealloc(instance as *const Instance as *mut _, layout);
    }
}
