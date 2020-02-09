//! An `Instance` contains all the runtime state used by execution of a
//! wasm module (except its callstack and register state). An
//! `InstanceHandle` is a reference-counting handle for an `Instance`.

use crate::export::Export;
use crate::imports::Imports;
use crate::jit_int::GdbJitImageRegistration;
use crate::memory::LinearMemory;
use crate::mmap::Mmap;
use crate::signalhandlers;
use crate::table::Table;
use crate::traphandlers::{wasmtime_call, Trap};
use crate::vmcontext::{
    VMBuiltinFunctionsArray, VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMFunctionImport,
    VMGlobalDefinition, VMGlobalImport, VMMemoryDefinition, VMMemoryImport, VMSharedSignatureIndex,
    VMTableDefinition, VMTableImport,
};
use crate::TrapRegistration;
use memoffset::offset_of;
use more_asserts::assert_lt;
use std::any::Any;
use std::cell::Cell;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::rc::Rc;
use std::sync::Arc;
use std::{mem, ptr, slice};
use thiserror::Error;
use wasmtime_environ::entity::{BoxedSlice, EntityRef, PrimaryMap};
use wasmtime_environ::wasm::{
    DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex,
    GlobalIndex, GlobalInit, MemoryIndex, SignatureIndex, TableIndex,
};
use wasmtime_environ::{DataInitializer, Module, TableElements, VMOffsets};

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        pub type SignalHandler = dyn Fn(libc::c_int, *const libc::siginfo_t, *const libc::c_void) -> bool;

        impl InstanceHandle {
            /// Set a custom signal handler
            pub fn set_signal_handler<H>(&mut self, handler: H)
            where
                H: 'static + Fn(libc::c_int, *const libc::siginfo_t, *const libc::c_void) -> bool,
            {
                self.instance().signal_handler.set(Some(Box::new(handler)));
            }
        }
    } else if #[cfg(target_os = "windows")] {
        pub type SignalHandler = dyn Fn(winapi::um::winnt::PEXCEPTION_POINTERS) -> bool;

        impl InstanceHandle {
            /// Set a custom signal handler
            pub fn set_signal_handler<H>(&mut self, handler: H)
            where
                H: 'static + Fn(winapi::um::winnt::PEXCEPTION_POINTERS) -> bool,
            {
                self.instance().signal_handler.set(Some(Box::new(handler)));
            }
        }
    }
}

/// A WebAssembly instance.
///
/// This is repr(C) to ensure that the vmctx field is last.
#[repr(C)]
pub(crate) struct Instance {
    /// The number of references to this `Instance`.
    refcount: Cell<usize>,

    /// `Instance`s from which this `Instance` imports. These won't
    /// create reference cycles because wasm instances can't cyclically
    /// import from each other.
    dependencies: HashSet<InstanceHandle>,

    /// The underlying mmap that holds this `Instance`.
    mmap: Cell<Mmap>,

    /// The `Module` this `Instance` was instantiated from.
    module: Arc<Module>,

    /// Offsets in the `vmctx` region.
    offsets: VMOffsets,

    /// WebAssembly linear memory data.
    memories: BoxedSlice<DefinedMemoryIndex, LinearMemory>,

    /// WebAssembly table data.
    tables: BoxedSlice<DefinedTableIndex, Table>,

    /// Pointers to functions in executable memory.
    finished_functions: BoxedSlice<DefinedFuncIndex, *mut [VMFunctionBody]>,

    /// Hosts can store arbitrary per-instance information here.
    host_state: Box<dyn Any>,

    /// Optional image of JIT'ed code for debugger registration.
    dbg_jit_registration: Option<Rc<GdbJitImageRegistration>>,

    /// Handler run when `SIGBUS`, `SIGFPE`, `SIGILL`, or `SIGSEGV` are caught by the instance thread.
    pub(crate) signal_handler: Cell<Option<Box<SignalHandler>>>,

    /// Handle to our registration of traps so signals know what trap to return
    /// when a segfault/sigill happens.
    pub(crate) trap_registration: TrapRegistration,

    /// Additional context used by compiled wasm code. This field is last, and
    /// represents a dynamically-sized array that extends beyond the nominal
    /// end of the struct (similar to a flexible array member).
    vmctx: VMContext,
}

#[allow(clippy::cast_ptr_alignment)]
impl Instance {
    /// Helper function to access various locations offset from our `*mut
    /// VMContext` object.
    unsafe fn vmctx_plus_offset<T>(&self, offset: u32) -> *mut T {
        (self.vmctx_ptr() as *mut u8)
            .add(usize::try_from(offset).unwrap())
            .cast()
    }

    /// Return the indexed `VMSharedSignatureIndex`.
    fn signature_id(&self, index: SignatureIndex) -> VMSharedSignatureIndex {
        let index = usize::try_from(index.as_u32()).unwrap();
        unsafe { *self.signature_ids_ptr().add(index) }
    }

    /// Return a pointer to the `VMSharedSignatureIndex`s.
    fn signature_ids_ptr(&self) -> *mut VMSharedSignatureIndex {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_signature_ids_begin()) }
    }

    /// Return the indexed `VMFunctionImport`.
    fn imported_function(&self, index: FuncIndex) -> &VMFunctionImport {
        let index = usize::try_from(index.as_u32()).unwrap();
        unsafe { &*self.imported_functions_ptr().add(index) }
    }

    /// Return a pointer to the `VMFunctionImport`s.
    fn imported_functions_ptr(&self) -> *mut VMFunctionImport {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_imported_functions_begin()) }
    }

    /// Return the index `VMTableImport`.
    fn imported_table(&self, index: TableIndex) -> &VMTableImport {
        let index = usize::try_from(index.as_u32()).unwrap();
        unsafe { &*self.imported_tables_ptr().add(index) }
    }

    /// Return a pointer to the `VMTableImports`s.
    fn imported_tables_ptr(&self) -> *mut VMTableImport {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_imported_tables_begin()) }
    }

    /// Return the indexed `VMMemoryImport`.
    fn imported_memory(&self, index: MemoryIndex) -> &VMMemoryImport {
        let index = usize::try_from(index.as_u32()).unwrap();
        unsafe { &*self.imported_memories_ptr().add(index) }
    }

    /// Return a pointer to the `VMMemoryImport`s.
    fn imported_memories_ptr(&self) -> *mut VMMemoryImport {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_imported_memories_begin()) }
    }

    /// Return the indexed `VMGlobalImport`.
    fn imported_global(&self, index: GlobalIndex) -> &VMGlobalImport {
        let index = usize::try_from(index.as_u32()).unwrap();
        unsafe { &*self.imported_globals_ptr().add(index) }
    }

    /// Return a pointer to the `VMGlobalImport`s.
    fn imported_globals_ptr(&self) -> *mut VMGlobalImport {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_imported_globals_begin()) }
    }

    /// Return the indexed `VMTableDefinition`.
    #[allow(dead_code)]
    fn table(&self, index: DefinedTableIndex) -> VMTableDefinition {
        unsafe { *self.table_ptr(index) }
    }

    /// Updates the value for a defined table to `VMTableDefinition`.
    fn set_table(&self, index: DefinedTableIndex, table: VMTableDefinition) {
        unsafe {
            *self.table_ptr(index) = table;
        }
    }

    /// Return the indexed `VMTableDefinition`.
    fn table_ptr(&self, index: DefinedTableIndex) -> *mut VMTableDefinition {
        let index = usize::try_from(index.as_u32()).unwrap();
        unsafe { self.tables_ptr().add(index) }
    }

    /// Return a pointer to the `VMTableDefinition`s.
    fn tables_ptr(&self) -> *mut VMTableDefinition {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_tables_begin()) }
    }

    /// Return the indexed `VMMemoryDefinition`.
    fn memory(&self, index: DefinedMemoryIndex) -> VMMemoryDefinition {
        unsafe { *self.memory_ptr(index) }
    }

    /// Set the indexed memory to `VMMemoryDefinition`.
    fn set_memory(&self, index: DefinedMemoryIndex, mem: VMMemoryDefinition) {
        unsafe {
            *self.memory_ptr(index) = mem;
        }
    }

    /// Return the indexed `VMMemoryDefinition`.
    fn memory_ptr(&self, index: DefinedMemoryIndex) -> *mut VMMemoryDefinition {
        let index = usize::try_from(index.as_u32()).unwrap();
        unsafe { self.memories_ptr().add(index) }
    }

    /// Return a pointer to the `VMMemoryDefinition`s.
    fn memories_ptr(&self) -> *mut VMMemoryDefinition {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_memories_begin()) }
    }

    /// Return the indexed `VMGlobalDefinition`.
    fn global(&self, index: DefinedGlobalIndex) -> VMGlobalDefinition {
        unsafe { *self.global_ptr(index) }
    }

    /// Set the indexed global to `VMGlobalDefinition`.
    #[allow(dead_code)]
    fn set_global(&self, index: DefinedGlobalIndex, global: VMGlobalDefinition) {
        unsafe {
            *self.global_ptr(index) = global;
        }
    }

    /// Return the indexed `VMGlobalDefinition`.
    fn global_ptr(&self, index: DefinedGlobalIndex) -> *mut VMGlobalDefinition {
        let index = usize::try_from(index.as_u32()).unwrap();
        unsafe { self.globals_ptr().add(index) }
    }

    /// Return a pointer to the `VMGlobalDefinition`s.
    fn globals_ptr(&self) -> *mut VMGlobalDefinition {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_globals_begin()) }
    }

    /// Return a pointer to the `VMBuiltinFunctionsArray`.
    fn builtin_functions_ptr(&self) -> *mut VMBuiltinFunctionsArray {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_builtin_functions_begin()) }
    }

    /// Return a reference to the vmctx used by compiled wasm code.
    pub fn vmctx(&self) -> &VMContext {
        &self.vmctx
    }

    /// Return a raw pointer to the vmctx used by compiled wasm code.
    pub fn vmctx_ptr(&self) -> *mut VMContext {
        self.vmctx() as *const VMContext as *mut VMContext
    }

    /// Lookup an export with the given name.
    pub fn lookup(&self, field: &str) -> Option<Export> {
        let export = if let Some(export) = self.module.exports.get(field) {
            export.clone()
        } else {
            return None;
        };
        Some(self.lookup_by_declaration(&export))
    }

    /// Lookup an export with the given export declaration.
    pub fn lookup_by_declaration(&self, export: &wasmtime_environ::Export) -> Export {
        match export {
            wasmtime_environ::Export::Function(index) => {
                let signature = self.module.signatures[self.module.functions[*index]].clone();
                let (address, vmctx) =
                    if let Some(def_index) = self.module.defined_func_index(*index) {
                        (
                            self.finished_functions[def_index] as *const _,
                            self.vmctx_ptr(),
                        )
                    } else {
                        let import = self.imported_function(*index);
                        (import.body, import.vmctx)
                    };
                Export::Function {
                    address,
                    signature,
                    vmctx,
                }
            }
            wasmtime_environ::Export::Table(index) => {
                let (definition, vmctx) =
                    if let Some(def_index) = self.module.defined_table_index(*index) {
                        (self.table_ptr(def_index), self.vmctx_ptr())
                    } else {
                        let import = self.imported_table(*index);
                        (import.from, import.vmctx)
                    };
                Export::Table {
                    definition,
                    vmctx,
                    table: self.module.table_plans[*index].clone(),
                }
            }
            wasmtime_environ::Export::Memory(index) => {
                let (definition, vmctx) =
                    if let Some(def_index) = self.module.defined_memory_index(*index) {
                        (self.memory_ptr(def_index), self.vmctx_ptr())
                    } else {
                        let import = self.imported_memory(*index);
                        (import.from, import.vmctx)
                    };
                Export::Memory {
                    definition,
                    vmctx,
                    memory: self.module.memory_plans[*index].clone(),
                }
            }
            wasmtime_environ::Export::Global(index) => Export::Global {
                definition: if let Some(def_index) = self.module.defined_global_index(*index) {
                    self.global_ptr(def_index)
                } else {
                    self.imported_global(*index).from
                },
                vmctx: self.vmctx_ptr(),
                global: self.module.globals[*index],
            },
        }
    }

    /// Return an iterator over the exports of this instance.
    ///
    /// Specifically, it provides access to the key-value pairs, where they keys
    /// are export names, and the values are export declarations which can be
    /// resolved `lookup_by_declaration`.
    pub fn exports(&self) -> indexmap::map::Iter<String, wasmtime_environ::Export> {
        self.module.exports.iter()
    }

    /// Return a reference to the custom state attached to this instance.
    pub fn host_state(&self) -> &dyn Any {
        &*self.host_state
    }

    fn invoke_function(&self, index: FuncIndex) -> Result<(), InstantiationError> {
        // TODO: Check that the callee's calling convention matches what we expect.

        let (callee_address, callee_vmctx) = match self.module.defined_func_index(index) {
            Some(defined_index) => {
                let body = *self
                    .finished_functions
                    .get(defined_index)
                    .expect("function index is out of bounds");
                (body as *const _, self.vmctx_ptr())
            }
            None => {
                assert_lt!(index.index(), self.module.imported_funcs.len());
                let import = self.imported_function(index);
                (import.body, import.vmctx)
            }
        };

        // Make the call.
        unsafe { wasmtime_call(callee_vmctx, self.vmctx_ptr(), callee_address) }
            .map_err(InstantiationError::StartTrap)
    }

    /// Invoke the WebAssembly start function of the instance, if one is present.
    fn invoke_start_function(&self) -> Result<(), InstantiationError> {
        if let Some(start_index) = self.module.start_func {
            self.invoke_function(start_index)
        } else {
            Ok(())
        }
    }

    /// Return the offset from the vmctx pointer to its containing Instance.
    pub(crate) fn vmctx_offset() -> isize {
        offset_of!(Self, vmctx) as isize
    }

    /// Return the table index for the given `VMTableDefinition`.
    pub(crate) fn table_index(&self, table: &VMTableDefinition) -> DefinedTableIndex {
        let offsets = &self.offsets;
        let begin = unsafe {
            (&self.vmctx as *const VMContext as *const u8)
                .add(usize::try_from(offsets.vmctx_tables_begin()).unwrap())
        } as *const VMTableDefinition;
        let end: *const VMTableDefinition = table;
        // TODO: Use `offset_from` once it stablizes.
        let index = DefinedTableIndex::new(
            (end as usize - begin as usize) / mem::size_of::<VMTableDefinition>(),
        );
        assert_lt!(index.index(), self.tables.len());
        index
    }

    /// Return the memory index for the given `VMMemoryDefinition`.
    pub(crate) fn memory_index(&self, memory: &VMMemoryDefinition) -> DefinedMemoryIndex {
        let offsets = &self.offsets;
        let begin = unsafe {
            (&self.vmctx as *const VMContext as *const u8)
                .add(usize::try_from(offsets.vmctx_memories_begin()).unwrap())
        } as *const VMMemoryDefinition;
        let end: *const VMMemoryDefinition = memory;
        // TODO: Use `offset_from` once it stablizes.
        let index = DefinedMemoryIndex::new(
            (end as usize - begin as usize) / mem::size_of::<VMMemoryDefinition>(),
        );
        assert_lt!(index.index(), self.memories.len());
        index
    }

    /// Grow memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub(crate) fn memory_grow(&self, memory_index: DefinedMemoryIndex, delta: u32) -> Option<u32> {
        let result = self
            .memories
            .get(memory_index)
            .unwrap_or_else(|| panic!("no memory for index {}", memory_index.index()))
            .grow(delta);

        // Keep current the VMContext pointers used by compiled wasm code.
        self.set_memory(memory_index, self.memories[memory_index].vmmemory());

        result
    }

    /// Grow imported memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    ///
    /// # Safety
    /// This and `imported_memory_size` are currently unsafe because they
    /// dereference the memory import's pointers.
    pub(crate) unsafe fn imported_memory_grow(
        &self,
        memory_index: MemoryIndex,
        delta: u32,
    ) -> Option<u32> {
        let import = self.imported_memory(memory_index);
        let foreign_instance = (&*import.vmctx).instance();
        let foreign_memory = &*import.from;
        let foreign_index = foreign_instance.memory_index(foreign_memory);

        foreign_instance.memory_grow(foreign_index, delta)
    }

    /// Returns the number of allocated wasm pages.
    pub(crate) fn memory_size(&self, memory_index: DefinedMemoryIndex) -> u32 {
        self.memories
            .get(memory_index)
            .unwrap_or_else(|| panic!("no memory for index {}", memory_index.index()))
            .size()
    }

    /// Returns the number of allocated wasm pages in an imported memory.
    ///
    /// # Safety
    /// This and `imported_memory_grow` are currently unsafe because they
    /// dereference the memory import's pointers.
    pub(crate) unsafe fn imported_memory_size(&self, memory_index: MemoryIndex) -> u32 {
        let import = self.imported_memory(memory_index);
        let foreign_instance = (&mut *import.vmctx).instance();
        let foreign_memory = &mut *import.from;
        let foreign_index = foreign_instance.memory_index(foreign_memory);

        foreign_instance.memory_size(foreign_index)
    }

    /// Grow table by the specified amount of elements.
    ///
    /// Returns `None` if table can't be grown by the specified amount
    /// of elements.
    pub(crate) fn table_grow(&self, table_index: DefinedTableIndex, delta: u32) -> Option<u32> {
        let result = self
            .tables
            .get(table_index)
            .unwrap_or_else(|| panic!("no table for index {}", table_index.index()))
            .grow(delta);

        // Keep current the VMContext pointers used by compiled wasm code.
        self.set_table(table_index, self.tables[table_index].vmtable());

        result
    }

    // Get table element by index.
    fn table_get(
        &self,
        table_index: DefinedTableIndex,
        index: u32,
    ) -> Option<VMCallerCheckedAnyfunc> {
        self.tables
            .get(table_index)
            .unwrap_or_else(|| panic!("no table for index {}", table_index.index()))
            .get(index)
    }

    fn table_set(
        &self,
        table_index: DefinedTableIndex,
        index: u32,
        val: VMCallerCheckedAnyfunc,
    ) -> Result<(), ()> {
        self.tables
            .get(table_index)
            .unwrap_or_else(|| panic!("no table for index {}", table_index.index()))
            .set(index, val)
    }
}

/// A handle holding an `Instance` of a WebAssembly module.
#[derive(Hash, PartialEq, Eq)]
pub struct InstanceHandle {
    instance: *mut Instance,
}

impl InstanceHandle {
    /// Create a new `InstanceHandle` pointing at a new `Instance`.
    ///
    /// # Unsafety
    ///
    /// This method is not necessarily inherently unsafe to call, but in general
    /// the APIs of an `Instance` are quite unsafe and have not been really
    /// audited for safety that much. As a result the unsafety here on this
    /// method is a low-overhead way of saying "this is an extremely unsafe type
    /// to work with".
    ///
    /// Extreme care must be taken when working with `InstanceHandle` and it's
    /// recommended to have relatively intimate knowledge of how it works
    /// internally if you'd like to do so. If possible it's recommended to use
    /// the `wasmtime` crate API rather than this type since that is vetted for
    /// safety.
    pub unsafe fn new(
        module: Arc<Module>,
        trap_registration: TrapRegistration,
        finished_functions: BoxedSlice<DefinedFuncIndex, *mut [VMFunctionBody]>,
        imports: Imports,
        data_initializers: &[DataInitializer<'_>],
        vmshared_signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
        dbg_jit_registration: Option<Rc<GdbJitImageRegistration>>,
        host_state: Box<dyn Any>,
    ) -> Result<Self, InstantiationError> {
        let tables = create_tables(&module);
        let memories = create_memories(&module)?;

        let vmctx_tables = tables
            .values()
            .map(Table::vmtable)
            .collect::<PrimaryMap<DefinedTableIndex, _>>()
            .into_boxed_slice();

        let vmctx_memories = memories
            .values()
            .map(LinearMemory::vmmemory)
            .collect::<PrimaryMap<DefinedMemoryIndex, _>>()
            .into_boxed_slice();

        let vmctx_globals = create_globals(&module);

        let offsets = VMOffsets::new(mem::size_of::<*const u8>() as u8, &module);

        let mut instance_mmap = Mmap::with_at_least(
            mem::size_of::<Instance>()
                .checked_add(usize::try_from(offsets.size_of_vmctx()).unwrap())
                .unwrap(),
        )
        .map_err(InstantiationError::Resource)?;

        let handle = {
            #[allow(clippy::cast_ptr_alignment)]
            let instance_ptr = instance_mmap.as_mut_ptr() as *mut Instance;
            let instance = Instance {
                refcount: Cell::new(1),
                dependencies: imports.dependencies,
                mmap: Cell::new(instance_mmap),
                module,
                offsets,
                memories,
                tables,
                finished_functions,
                dbg_jit_registration,
                host_state,
                signal_handler: Cell::new(None),
                trap_registration,
                vmctx: VMContext {},
            };
            ptr::write(instance_ptr, instance);
            InstanceHandle {
                instance: instance_ptr,
            }
        };
        let instance = handle.instance();

        ptr::copy(
            vmshared_signatures.values().as_slice().as_ptr(),
            instance.signature_ids_ptr() as *mut VMSharedSignatureIndex,
            vmshared_signatures.len(),
        );
        ptr::copy(
            imports.functions.values().as_slice().as_ptr(),
            instance.imported_functions_ptr() as *mut VMFunctionImport,
            imports.functions.len(),
        );
        ptr::copy(
            imports.tables.values().as_slice().as_ptr(),
            instance.imported_tables_ptr() as *mut VMTableImport,
            imports.tables.len(),
        );
        ptr::copy(
            imports.memories.values().as_slice().as_ptr(),
            instance.imported_memories_ptr() as *mut VMMemoryImport,
            imports.memories.len(),
        );
        ptr::copy(
            imports.globals.values().as_slice().as_ptr(),
            instance.imported_globals_ptr() as *mut VMGlobalImport,
            imports.globals.len(),
        );
        ptr::copy(
            vmctx_tables.values().as_slice().as_ptr(),
            instance.tables_ptr() as *mut VMTableDefinition,
            vmctx_tables.len(),
        );
        ptr::copy(
            vmctx_memories.values().as_slice().as_ptr(),
            instance.memories_ptr() as *mut VMMemoryDefinition,
            vmctx_memories.len(),
        );
        ptr::copy(
            vmctx_globals.values().as_slice().as_ptr(),
            instance.globals_ptr() as *mut VMGlobalDefinition,
            vmctx_globals.len(),
        );
        ptr::write(
            instance.builtin_functions_ptr() as *mut VMBuiltinFunctionsArray,
            VMBuiltinFunctionsArray::initialized(),
        );

        // Check initializer bounds before initializing anything.
        check_table_init_bounds(instance)?;
        check_memory_init_bounds(instance, data_initializers)?;

        // Apply the initializers.
        initialize_tables(instance)?;
        initialize_memories(instance, data_initializers)?;
        initialize_globals(instance);

        // Ensure that our signal handlers are ready for action.
        // TODO: Move these calls out of `InstanceHandle`.
        signalhandlers::init();

        // The WebAssembly spec specifies that the start function is
        // invoked automatically at instantiation time.
        instance.invoke_start_function()?;

        Ok(handle)
    }

    /// Create a new `InstanceHandle` pointing at the instance
    /// pointed to by the given `VMContext` pointer.
    ///
    /// # Safety
    /// This is unsafe because it doesn't work on just any `VMContext`, it must
    /// be a `VMContext` allocated as part of an `Instance`.
    pub unsafe fn from_vmctx(vmctx: *mut VMContext) -> Self {
        let instance = (&mut *vmctx).instance();
        instance.refcount.set(instance.refcount.get() + 1);
        Self {
            instance: instance as *const Instance as *mut Instance,
        }
    }

    /// Return a reference to the vmctx used by compiled wasm code.
    pub fn vmctx(&self) -> &VMContext {
        self.instance().vmctx()
    }

    /// Return a raw pointer to the vmctx used by compiled wasm code.
    pub fn vmctx_ptr(&self) -> *mut VMContext {
        self.instance().vmctx_ptr()
    }

    /// Return a reference-counting pointer to a module.
    pub fn module(&self) -> &Arc<Module> {
        &self.instance().module
    }

    /// Return a reference to a module.
    pub fn module_ref(&self) -> &Module {
        &self.instance().module
    }

    /// Lookup an export with the given name.
    pub fn lookup(&self, field: &str) -> Option<Export> {
        self.instance().lookup(field)
    }

    /// Lookup an export with the given export declaration.
    pub fn lookup_by_declaration(&self, export: &wasmtime_environ::Export) -> Export {
        self.instance().lookup_by_declaration(export)
    }

    /// Return an iterator over the exports of this instance.
    ///
    /// Specifically, it provides access to the key-value pairs, where the keys
    /// are export names, and the values are export declarations which can be
    /// resolved `lookup_by_declaration`.
    pub fn exports(&self) -> indexmap::map::Iter<String, wasmtime_environ::Export> {
        self.instance().exports()
    }

    /// Return a reference to the custom state attached to this instance.
    pub fn host_state(&self) -> &dyn Any {
        self.instance().host_state()
    }

    /// Return the memory index for the given `VMMemoryDefinition` in this instance.
    pub fn memory_index(&self, memory: &VMMemoryDefinition) -> DefinedMemoryIndex {
        self.instance().memory_index(memory)
    }

    /// Grow memory in this instance by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub fn memory_grow(&self, memory_index: DefinedMemoryIndex, delta: u32) -> Option<u32> {
        self.instance().memory_grow(memory_index, delta)
    }

    /// Return the table index for the given `VMTableDefinition` in this instance.
    pub fn table_index(&self, table: &VMTableDefinition) -> DefinedTableIndex {
        self.instance().table_index(table)
    }

    /// Grow table in this instance by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub fn table_grow(&self, table_index: DefinedTableIndex, delta: u32) -> Option<u32> {
        self.instance().table_grow(table_index, delta)
    }

    /// Get table element reference.
    ///
    /// Returns `None` if index is out of bounds.
    pub fn table_get(
        &self,
        table_index: DefinedTableIndex,
        index: u32,
    ) -> Option<VMCallerCheckedAnyfunc> {
        self.instance().table_get(table_index, index)
    }

    /// Set table element reference.
    ///
    /// Returns an error if the index is out of bounds
    pub fn table_set(
        &self,
        table_index: DefinedTableIndex,
        index: u32,
        val: VMCallerCheckedAnyfunc,
    ) -> Result<(), ()> {
        self.instance().table_set(table_index, index, val)
    }

    /// Return a reference to the contained `Instance`.
    pub(crate) fn instance(&self) -> &Instance {
        unsafe { &*(self.instance as *const Instance) }
    }
}

impl Clone for InstanceHandle {
    fn clone(&self) -> Self {
        let instance = self.instance();
        instance.refcount.set(instance.refcount.get() + 1);
        Self {
            instance: self.instance,
        }
    }
}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        let instance = self.instance();
        let count = instance.refcount.get();
        instance.refcount.set(count - 1);
        if count == 1 {
            let mmap = instance.mmap.replace(Mmap::new());
            unsafe { ptr::drop_in_place(self.instance) };
            mem::drop(mmap);
        }
    }
}

fn check_table_init_bounds(instance: &Instance) -> Result<(), InstantiationError> {
    let module = Arc::clone(&instance.module);
    for init in &module.table_elements {
        let start = get_table_init_start(init, instance);
        let table = get_table(init, instance);

        let size = usize::try_from(table.size()).unwrap();
        if size < start + init.elements.len() {
            return Err(InstantiationError::Link(LinkError(
                "elements segment does not fit".to_owned(),
            )));
        }
    }

    Ok(())
}

/// Compute the offset for a memory data initializer.
fn get_memory_init_start(init: &DataInitializer<'_>, instance: &Instance) -> usize {
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

/// Return a byte-slice view of a memory's data.
unsafe fn get_memory_slice<'instance>(
    init: &DataInitializer<'_>,
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
    data_initializers: &[DataInitializer<'_>],
) -> Result<(), InstantiationError> {
    for init in data_initializers {
        let start = get_memory_init_start(init, instance);
        unsafe {
            let mem_slice = get_memory_slice(init, instance);
            if mem_slice.get_mut(start..start + init.data.len()).is_none() {
                return Err(InstantiationError::Link(LinkError(
                    "data segment does not fit".to_owned(),
                )));
            }
        }
    }

    Ok(())
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

/// Compute the offset for a table element initializer.
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

/// Return a byte-slice view of a table's data.
fn get_table<'instance>(init: &TableElements, instance: &'instance Instance) -> &'instance Table {
    if let Some(defined_table_index) = instance.module.defined_table_index(init.table_index) {
        &instance.tables[defined_table_index]
    } else {
        let import = instance.imported_table(init.table_index);
        let foreign_instance = unsafe { (&mut *(import).vmctx).instance() };
        let foreign_table = unsafe { &mut *(import).from };
        let foreign_index = foreign_instance.table_index(foreign_table);
        &foreign_instance.tables[foreign_index]
    }
}

/// Initialize the table memory from the provided initializers.
fn initialize_tables(instance: &Instance) -> Result<(), InstantiationError> {
    let vmctx = instance.vmctx_ptr();
    let module = Arc::clone(&instance.module);
    for init in &module.table_elements {
        let start = get_table_init_start(init, instance);
        let table = get_table(init, instance);

        for (i, func_idx) in init.elements.iter().enumerate() {
            let callee_sig = instance.module.functions[*func_idx];
            let (callee_ptr, callee_vmctx) =
                if let Some(index) = instance.module.defined_func_index(*func_idx) {
                    (instance.finished_functions[index] as *const _, vmctx)
                } else {
                    let imported_func = instance.imported_function(*func_idx);
                    (imported_func.body, imported_func.vmctx)
                };
            let type_index = instance.signature_id(callee_sig);
            table
                .set(
                    u32::try_from(start + i).unwrap(),
                    VMCallerCheckedAnyfunc {
                        func_ptr: callee_ptr,
                        type_index,
                        vmctx: callee_vmctx,
                    },
                )
                .unwrap();
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
        memories.push(LinearMemory::new(plan).map_err(InstantiationError::Resource)?);
    }
    Ok(memories.into_boxed_slice())
}

/// Initialize the table memory from the provided initializers.
fn initialize_memories(
    instance: &Instance,
    data_initializers: &[DataInitializer<'_>],
) -> Result<(), InstantiationError> {
    for init in data_initializers {
        let start = get_memory_init_start(init, instance);
        unsafe {
            let mem_slice = get_memory_slice(init, instance);
            let to_init = &mut mem_slice[start..start + init.data.len()];
            to_init.copy_from_slice(init.data);
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

fn initialize_globals(instance: &Instance) {
    let module = Arc::clone(&instance.module);
    let num_imports = module.imported_globals.len();
    for (index, global) in module.globals.iter().skip(num_imports) {
        let def_index = module.defined_global_index(index).unwrap();
        unsafe {
            let to = instance.global_ptr(def_index);
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
                GlobalInit::Import => panic!("locally-defined global initialized as import"),
                GlobalInit::RefNullConst | GlobalInit::RefFunc(_) => unimplemented!(),
            }
        }
    }
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

    /// A compilation error occured.
    #[error("Trap occurred while invoking start function")]
    StartTrap(#[source] Trap),
}
