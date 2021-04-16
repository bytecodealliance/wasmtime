use crate::externref::{ModuleInfoLookup, VMExternRefActivationsTable, EMPTY_MODULE_LOOKUP};
use crate::imports::Imports;
use crate::instance::{Instance, InstanceHandle, RuntimeMemoryCreator};
use crate::memory::{DefaultMemoryCreator, Memory};
use crate::table::{Table, TableElement};
use crate::traphandlers::Trap;
use crate::vmcontext::{
    VMBuiltinFunctionsArray, VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMFunctionImport,
    VMGlobalDefinition, VMGlobalImport, VMInterrupts, VMMemoryImport, VMSharedSignatureIndex,
    VMTableImport,
};
use anyhow::Result;
use std::alloc;
use std::any::Any;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::ptr::{self, NonNull};
use std::slice;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_environ::entity::{packed_option::ReservedValue, EntityRef, EntitySet, PrimaryMap};
use wasmtime_environ::wasm::{
    DefinedFuncIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex, GlobalInit, SignatureIndex,
    TableElementType, WasmType,
};
use wasmtime_environ::{
    ir, MemoryInitialization, MemoryInitializer, Module, ModuleType, TableInitializer, VMOffsets,
    WASM_PAGE_SIZE,
};

mod pooling;

pub use self::pooling::{
    InstanceLimits, ModuleLimits, PoolingAllocationStrategy, PoolingInstanceAllocator,
};

/// Represents a request for a new runtime instance.
pub struct InstanceAllocationRequest<'a> {
    /// The module being instantiated.
    pub module: Arc<Module>,

    /// The finished (JIT) functions for the module.
    pub finished_functions: &'a PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,

    /// The imports to use for the instantiation.
    pub imports: Imports<'a>,

    /// Translation from `SignatureIndex` to `VMSharedSignatureIndex`
    pub shared_signatures: SharedSignatures<'a>,

    /// The host state to associate with the instance.
    pub host_state: Box<dyn Any>,

    /// The pointer to the VM interrupts structure to use for the instance.
    pub interrupts: *const VMInterrupts,

    /// The pointer to the reference activations table to use for the instance.
    pub externref_activations_table: *mut VMExternRefActivationsTable,

    /// The pointer to the module info lookup to use for the instance.
    pub module_info_lookup: Option<*const dyn ModuleInfoLookup>,
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
    Resource(anyhow::Error),

    /// A wasm link error occured.
    #[error("Failed to link module")]
    Link(#[from] LinkError),

    /// A trap ocurred during instantiation, after linking.
    #[error("Trap occurred during instantiation")]
    Trap(Trap),

    /// A limit on how many instances are supported has been reached.
    #[error("Limit of {0} concurrent instances has been reached")]
    Limit(u32),
}

/// An error while creating a fiber stack.
#[cfg(feature = "async")]
#[derive(Error, Debug)]
pub enum FiberStackError {
    /// Insufficient resources available for the request.
    #[error("Insufficient resources: {0}")]
    Resource(anyhow::Error),
    /// An error for when the allocator doesn't support fiber stacks.
    #[error("fiber stacks are not supported by the allocator")]
    NotSupported,
    /// A limit on how many fibers are supported has been reached.
    #[error("Limit of {0} concurrent fibers has been reached")]
    Limit(u32),
}

/// Represents a runtime instance allocator.
///
/// # Safety
///
/// This trait is unsafe as it requires knowledge of Wasmtime's runtime internals to implement correctly.
pub unsafe trait InstanceAllocator: Send + Sync {
    /// Validates that a module is supported by the allocator.
    fn validate(&self, module: &Module) -> Result<()> {
        drop(module);
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

    /// Allocates a fiber stack for calling async functions on.
    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack, FiberStackError>;

    /// Deallocates a fiber stack that was previously allocated with `allocate_fiber_stack`.
    ///
    /// # Safety
    ///
    /// The provided stack is required to have been allocated with `allocate_fiber_stack`.
    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, stack: &wasmtime_fiber::FiberStack);
}

pub enum SharedSignatures<'a> {
    /// Used for instantiating user-defined modules
    Table(&'a PrimaryMap<SignatureIndex, VMSharedSignatureIndex>),
    /// Used for instance creation that has only a single function
    Always(VMSharedSignatureIndex),
    /// Used for instance creation that has no functions
    None,
}

impl SharedSignatures<'_> {
    fn lookup(&self, index: SignatureIndex) -> VMSharedSignatureIndex {
        match self {
            SharedSignatures::Table(table) => table[index],
            SharedSignatures::Always(index) => *index,
            SharedSignatures::None => unreachable!(),
        }
    }
}

impl<'a> From<VMSharedSignatureIndex> for SharedSignatures<'a> {
    fn from(val: VMSharedSignatureIndex) -> SharedSignatures<'a> {
        SharedSignatures::Always(val)
    }
}

impl<'a> From<Option<VMSharedSignatureIndex>> for SharedSignatures<'a> {
    fn from(val: Option<VMSharedSignatureIndex>) -> SharedSignatures<'a> {
        match val {
            Some(idx) => SharedSignatures::Always(idx),
            None => SharedSignatures::None,
        }
    }
}

impl<'a> From<&'a PrimaryMap<SignatureIndex, VMSharedSignatureIndex>> for SharedSignatures<'a> {
    fn from(val: &'a PrimaryMap<SignatureIndex, VMSharedSignatureIndex>) -> SharedSignatures<'a> {
        SharedSignatures::Table(val)
    }
}

fn get_table_init_start(
    init: &TableInitializer,
    instance: &Instance,
) -> Result<usize, InstantiationError> {
    match init.base {
        Some(base) => {
            let val = unsafe {
                if let Some(def_index) = instance.module.defined_global_index(base) {
                    *instance.global(def_index).as_u32()
                } else {
                    *(*instance.imported_global(base).from).as_u32()
                }
            };

            init.offset.checked_add(val as usize).ok_or_else(|| {
                InstantiationError::Link(LinkError(
                    "element segment global base overflows".to_owned(),
                ))
            })
        }
        None => Ok(init.offset),
    }
}

fn check_table_init_bounds(instance: &Instance) -> Result<(), InstantiationError> {
    for init in &instance.module.table_initializers {
        let table = instance.get_table(init.table_index);
        let start = get_table_init_start(init, instance)?;
        let end = start.checked_add(init.elements.len());

        match end {
            Some(end) if end <= table.size() as usize => {
                // Initializer is in bounds
            }
            _ => {
                return Err(InstantiationError::Link(LinkError(
                    "table out of bounds: elements segment does not fit".to_owned(),
                )))
            }
        }
    }

    Ok(())
}

fn initialize_tables(instance: &Instance) -> Result<(), InstantiationError> {
    for init in &instance.module.table_initializers {
        let table = instance.get_table(init.table_index);
        let start = get_table_init_start(init, instance)?;
        let end = start.checked_add(init.elements.len());

        match end {
            Some(end) if end <= table.size() as usize => {
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
            _ => {
                return Err(InstantiationError::Trap(Trap::wasm(
                    ir::TrapCode::TableOutOfBounds,
                )))
            }
        }
    }

    Ok(())
}

fn get_memory_init_start(
    init: &MemoryInitializer,
    instance: &Instance,
) -> Result<usize, InstantiationError> {
    match init.base {
        Some(base) => {
            let val = unsafe {
                if let Some(def_index) = instance.module.defined_global_index(base) {
                    *instance.global(def_index).as_u32()
                } else {
                    *(*instance.imported_global(base).from).as_u32()
                }
            };

            init.offset.checked_add(val as usize).ok_or_else(|| {
                InstantiationError::Link(LinkError("data segment global base overflows".to_owned()))
            })
        }
        None => Ok(init.offset),
    }
}

unsafe fn get_memory_slice<'instance>(
    init: &MemoryInitializer,
    instance: &'instance Instance,
) -> &'instance mut [u8] {
    let memory = if let Some(defined_memory_index) =
        instance.module.defined_memory_index(init.memory_index)
    {
        instance.memory(defined_memory_index)
    } else {
        let import = instance.imported_memory(init.memory_index);
        let foreign_instance = (&mut *(import).vmctx).instance();
        let foreign_memory = &mut *(import).from;
        let foreign_index = foreign_instance.memory_index(foreign_memory);
        foreign_instance.memory(foreign_index)
    };
    &mut *ptr::slice_from_raw_parts_mut(memory.base, memory.current_length)
}

fn check_memory_init_bounds(
    instance: &Instance,
    initializers: &[MemoryInitializer],
) -> Result<(), InstantiationError> {
    for init in initializers {
        let memory = instance.get_memory(init.memory_index);
        let start = get_memory_init_start(init, instance)?;
        let end = start.checked_add(init.data.len());

        match end {
            Some(end) if end <= memory.current_length => {
                // Initializer is in bounds
            }
            _ => {
                return Err(InstantiationError::Link(LinkError(
                    "memory out of bounds: data segment does not fit".into(),
                )))
            }
        }
    }

    Ok(())
}

fn initialize_memories(
    instance: &Instance,
    initializers: &[MemoryInitializer],
) -> Result<(), InstantiationError> {
    for init in initializers {
        let memory = instance.get_memory(init.memory_index);
        let start = get_memory_init_start(init, instance)?;
        let end = start.checked_add(init.data.len());

        match end {
            Some(end) if end <= memory.current_length => {
                let mem_slice = unsafe { get_memory_slice(init, instance) };
                mem_slice[start..end].copy_from_slice(&init.data);
            }
            _ => {
                return Err(InstantiationError::Trap(Trap::wasm(
                    ir::TrapCode::HeapOutOfBounds,
                )))
            }
        }
    }

    Ok(())
}

fn check_init_bounds(instance: &Instance) -> Result<(), InstantiationError> {
    check_table_init_bounds(instance)?;

    match &instance.module.memory_initialization {
        MemoryInitialization::Paged { out_of_bounds, .. } => {
            if *out_of_bounds {
                return Err(InstantiationError::Link(LinkError(
                    "memory out of bounds: data segment does not fit".into(),
                )));
            }
        }
        MemoryInitialization::Segmented(initializers) => {
            check_memory_init_bounds(instance, initializers)?;
        }
    }

    Ok(())
}

fn initialize_instance(
    instance: &Instance,
    is_bulk_memory: bool,
) -> Result<(), InstantiationError> {
    // If bulk memory is not enabled, bounds check the data and element segments before
    // making any changes. With bulk memory enabled, initializers are processed
    // in-order and side effects are observed up to the point of an out-of-bounds
    // initializer, so the early checking is not desired.
    if !is_bulk_memory {
        check_init_bounds(instance)?;
    }

    // Initialize the tables
    initialize_tables(instance)?;

    // Initialize the memories
    match &instance.module.memory_initialization {
        MemoryInitialization::Paged { map, out_of_bounds } => {
            for (index, pages) in map {
                let memory = instance.memory(index);
                let slice =
                    unsafe { slice::from_raw_parts_mut(memory.base, memory.current_length) };

                for (page_index, page) in pages.iter().enumerate() {
                    if let Some(data) = page {
                        debug_assert_eq!(data.len(), WASM_PAGE_SIZE as usize);
                        let start = page_index * WASM_PAGE_SIZE as usize;
                        let end = start + WASM_PAGE_SIZE as usize;
                        slice[start..end].copy_from_slice(data);
                    }
                }
            }

            // Check for out of bound access after initializing the pages to maintain
            // the expected behavior of the bulk memory spec.
            if *out_of_bounds {
                return Err(InstantiationError::Trap(Trap::wasm(
                    ir::TrapCode::HeapOutOfBounds,
                )));
            }
        }
        MemoryInitialization::Segmented(initializers) => {
            initialize_memories(instance, initializers)?;
        }
    }

    Ok(())
}

unsafe fn initialize_vmcontext(instance: &Instance, req: InstanceAllocationRequest) {
    let module = &instance.module;

    *instance.interrupts() = req.interrupts;
    *instance.externref_activations_table() = req.externref_activations_table;
    *instance.module_info_lookup() = req.module_info_lookup.unwrap_or(&EMPTY_MODULE_LOOKUP);

    // Initialize shared signatures
    let mut ptr = instance.signature_ids_ptr();
    for sig in module.types.values() {
        *ptr = match sig {
            ModuleType::Function(sig) => req.shared_signatures.lookup(*sig),
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
    debug_assert_eq!(req.imports.functions.len(), module.num_imported_funcs);
    ptr::copy(
        req.imports.functions.as_ptr(),
        instance.imported_functions_ptr() as *mut VMFunctionImport,
        req.imports.functions.len(),
    );
    debug_assert_eq!(req.imports.tables.len(), module.num_imported_tables);
    ptr::copy(
        req.imports.tables.as_ptr(),
        instance.imported_tables_ptr() as *mut VMTableImport,
        req.imports.tables.len(),
    );
    debug_assert_eq!(req.imports.memories.len(), module.num_imported_memories);
    ptr::copy(
        req.imports.memories.as_ptr(),
        instance.imported_memories_ptr() as *mut VMMemoryImport,
        req.imports.memories.len(),
    );
    debug_assert_eq!(req.imports.globals.len(), module.num_imported_globals);
    ptr::copy(
        req.imports.globals.as_ptr(),
        instance.imported_globals_ptr() as *mut VMGlobalImport,
        req.imports.globals.len(),
    );

    // Initialize the functions
    for (index, sig) in instance.module.functions.iter() {
        let type_index = req.shared_signatures.lookup(*sig);

        let (func_ptr, vmctx) = if let Some(def_index) = instance.module.defined_func_index(index) {
            (
                NonNull::new(req.finished_functions[def_index] as *mut _).unwrap(),
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
        ptr::write(ptr, instance.tables[DefinedTableIndex::new(i)].vmtable());
        ptr = ptr.add(1);
    }

    // Initialize the defined memories
    let mut ptr = instance.memories_ptr();
    for i in 0..module.memory_plans.len() - module.num_imported_memories {
        ptr::write(
            ptr,
            instance.memories[DefinedMemoryIndex::new(i)].vmmemory(),
        );
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
    stack_size: usize,
}

impl OnDemandInstanceAllocator {
    /// Creates a new on-demand instance allocator.
    pub fn new(mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>, stack_size: usize) -> Self {
        Self {
            mem_creator,
            stack_size,
        }
    }

    fn create_tables(module: &Module) -> PrimaryMap<DefinedTableIndex, Table> {
        let num_imports = module.num_imported_tables;
        let mut tables: PrimaryMap<DefinedTableIndex, _> =
            PrimaryMap::with_capacity(module.table_plans.len() - num_imports);
        for table in &module.table_plans.values().as_slice()[num_imports..] {
            tables.push(Table::new_dynamic(table));
        }
        tables
    }

    fn create_memories(
        &self,
        module: &Module,
    ) -> Result<PrimaryMap<DefinedMemoryIndex, Memory>, InstantiationError> {
        let creator = self
            .mem_creator
            .as_deref()
            .unwrap_or_else(|| &DefaultMemoryCreator);
        let num_imports = module.num_imported_memories;
        let mut memories: PrimaryMap<DefinedMemoryIndex, _> =
            PrimaryMap::with_capacity(module.memory_plans.len() - num_imports);
        for plan in &module.memory_plans.values().as_slice()[num_imports..] {
            memories
                .push(Memory::new_dynamic(plan, creator).map_err(InstantiationError::Resource)?);
        }
        Ok(memories)
    }
}

impl Default for OnDemandInstanceAllocator {
    fn default() -> Self {
        Self {
            mem_creator: None,
            stack_size: 0,
        }
    }
}

unsafe impl InstanceAllocator for OnDemandInstanceAllocator {
    unsafe fn allocate(
        &self,
        mut req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError> {
        let memories = self.create_memories(&req.module)?;
        let tables = Self::create_tables(&req.module);

        let host_state = std::mem::replace(&mut req.host_state, Box::new(()));

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
                host_state,
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

        initialize_vmcontext(handle.instance(), req);

        Ok(handle)
    }

    unsafe fn initialize(
        &self,
        handle: &InstanceHandle,
        is_bulk_memory: bool,
    ) -> Result<(), InstantiationError> {
        initialize_instance(handle.instance(), is_bulk_memory)
    }

    unsafe fn deallocate(&self, handle: &InstanceHandle) {
        let layout = handle.instance().alloc_layout();
        ptr::drop_in_place(handle.instance);
        alloc::dealloc(handle.instance.cast(), layout);
    }

    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack, FiberStackError> {
        if self.stack_size == 0 {
            return Err(FiberStackError::NotSupported);
        }

        wasmtime_fiber::FiberStack::new(self.stack_size)
            .map_err(|e| FiberStackError::Resource(e.into()))
    }

    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, _stack: &wasmtime_fiber::FiberStack) {
        // The on-demand allocator has no further bookkeeping for fiber stacks
    }
}
