//! An `Instance` contains all the runtime state used by execution of a
//! wasm module (except its callstack and register state). An
//! `InstanceHandle` is a reference-counting handle for an `Instance`.

use crate::export::Export;
use crate::imports::Imports;
use crate::jit_int::GdbJitImageRegistration;
use crate::memory::{DefaultMemoryCreator, RuntimeLinearMemory, RuntimeMemoryCreator};
use crate::table::Table;
use crate::traphandlers;
use crate::traphandlers::{catch_traps, Trap};
use crate::vmcontext::{
    VMBuiltinFunctionsArray, VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMFunctionImport,
    VMGlobalDefinition, VMGlobalImport, VMMemoryDefinition, VMMemoryImport, VMSharedSignatureIndex,
    VMTableDefinition, VMTableImport, VMTrampoline,
};
use crate::TrapRegistration;
use crate::{ExportFunction, ExportGlobal, ExportMemory, ExportTable};
use memoffset::offset_of;
use more_asserts::assert_lt;
use std::alloc::{self, Layout};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::rc::Rc;
use std::sync::Arc;
use std::{mem, ptr, slice};
use thiserror::Error;
use wasmtime_environ::entity::{packed_option::ReservedValue, BoxedSlice, EntityRef, PrimaryMap};
use wasmtime_environ::wasm::{
    DataIndex, DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex,
    ElemIndex, FuncIndex, GlobalIndex, GlobalInit, MemoryIndex, SignatureIndex, TableIndex,
};
use wasmtime_environ::{ir, DataInitializer, Module, TableElements, VMOffsets};

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

    /// The `Module` this `Instance` was instantiated from.
    module: Arc<Module>,

    /// Offsets in the `vmctx` region.
    offsets: VMOffsets,

    /// WebAssembly linear memory data.
    memories: BoxedSlice<DefinedMemoryIndex, Box<dyn RuntimeLinearMemory>>,

    /// WebAssembly table data.
    tables: BoxedSlice<DefinedTableIndex, Table>,

    /// Passive elements in this instantiation. As `elem.drop`s happen, these
    /// entries get removed. A missing entry is considered equivalent to an
    /// empty slice.
    passive_elements: RefCell<HashMap<ElemIndex, Box<[VMCallerCheckedAnyfunc]>>>,

    /// Passive data segments from our module. As `data.drop`s happen, entries
    /// get removed. A missing entry is considered equivalent to an empty slice.
    passive_data: RefCell<HashMap<DataIndex, Arc<[u8]>>>,

    /// Pointers to functions in executable memory.
    finished_functions: BoxedSlice<DefinedFuncIndex, *mut [VMFunctionBody]>,

    /// Pointers to trampoline functions used to enter particular signatures
    trampolines: HashMap<VMSharedSignatureIndex, VMTrampoline>,

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

    pub(crate) fn module(&self) -> &Arc<Module> {
        &self.module
    }

    pub(crate) fn module_ref(&self) -> &Module {
        &*self.module
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

    /// Get a locally defined or imported memory.
    pub(crate) fn get_memory(&self, index: MemoryIndex) -> VMMemoryDefinition {
        if let Some(defined_index) = self.module.local.defined_memory_index(index) {
            self.memory(defined_index)
        } else {
            let import = self.imported_memory(index);
            *unsafe { import.from.as_ref().unwrap() }
        }
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
                let signature = self.signature_id(self.module.local.functions[*index]);
                let (address, vmctx) =
                    if let Some(def_index) = self.module.local.defined_func_index(*index) {
                        (
                            self.finished_functions[def_index] as *const _,
                            self.vmctx_ptr(),
                        )
                    } else {
                        let import = self.imported_function(*index);
                        (import.body, import.vmctx)
                    };
                ExportFunction {
                    address,
                    signature,
                    vmctx,
                }
                .into()
            }
            wasmtime_environ::Export::Table(index) => {
                let (definition, vmctx) =
                    if let Some(def_index) = self.module.local.defined_table_index(*index) {
                        (self.table_ptr(def_index), self.vmctx_ptr())
                    } else {
                        let import = self.imported_table(*index);
                        (import.from, import.vmctx)
                    };
                ExportTable {
                    definition,
                    vmctx,
                    table: self.module.local.table_plans[*index].clone(),
                }
                .into()
            }
            wasmtime_environ::Export::Memory(index) => {
                let (definition, vmctx) =
                    if let Some(def_index) = self.module.local.defined_memory_index(*index) {
                        (self.memory_ptr(def_index), self.vmctx_ptr())
                    } else {
                        let import = self.imported_memory(*index);
                        (import.from, import.vmctx)
                    };
                ExportMemory {
                    definition,
                    vmctx,
                    memory: self.module.local.memory_plans[*index].clone(),
                }
                .into()
            }
            wasmtime_environ::Export::Global(index) => ExportGlobal {
                definition: if let Some(def_index) = self.module.local.defined_global_index(*index)
                {
                    self.global_ptr(def_index)
                } else {
                    self.imported_global(*index).from
                },
                vmctx: self.vmctx_ptr(),
                global: self.module.local.globals[*index],
            }
            .into(),
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
    #[inline]
    pub fn host_state(&self) -> &dyn Any {
        &*self.host_state
    }

    /// Invoke the WebAssembly start function of the instance, if one is present.
    fn invoke_start_function(&self) -> Result<(), InstantiationError> {
        let start_index = match self.module.start_func {
            Some(idx) => idx,
            None => return Ok(()),
        };

        let (callee_address, callee_vmctx) = match self.module.local.defined_func_index(start_index)
        {
            Some(defined_index) => {
                let body = *self
                    .finished_functions
                    .get(defined_index)
                    .expect("function index is out of bounds");
                (body as *const _, self.vmctx_ptr())
            }
            None => {
                assert_lt!(start_index.index(), self.module.imported_funcs.len());
                let import = self.imported_function(start_index);
                (import.body, import.vmctx)
            }
        };

        // Make the call.
        unsafe {
            catch_traps(callee_vmctx, || {
                mem::transmute::<
                    *const VMFunctionBody,
                    unsafe extern "C" fn(*mut VMContext, *mut VMContext),
                >(callee_address)(callee_vmctx, self.vmctx_ptr())
            })
            .map_err(InstantiationError::StartTrap)
        }
    }

    /// Return the offset from the vmctx pointer to its containing Instance.
    #[inline]
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

    fn alloc_layout(&self) -> Layout {
        let size = mem::size_of_val(self)
            .checked_add(usize::try_from(self.offsets.size_of_vmctx()).unwrap())
            .unwrap();
        let align = mem::align_of_val(self);
        Layout::from_size_align(size, align).unwrap()
    }

    /// Get a `VMCallerCheckedAnyfunc` for the given `FuncIndex`.
    fn get_caller_checked_anyfunc(&self, index: FuncIndex) -> VMCallerCheckedAnyfunc {
        if index == FuncIndex::reserved_value() {
            return VMCallerCheckedAnyfunc::default();
        }

        let sig = self.module.local.functions[index];
        let type_index = self.signature_id(sig);

        let (func_ptr, vmctx) = if let Some(def_index) = self.module.local.defined_func_index(index)
        {
            (
                self.finished_functions[def_index] as *const _,
                self.vmctx_ptr(),
            )
        } else {
            let import = self.imported_function(index);
            (import.body, import.vmctx)
        };
        VMCallerCheckedAnyfunc {
            func_ptr,
            type_index,
            vmctx,
        }
    }

    /// The `table.init` operation: initializes a portion of a table with a
    /// passive element.
    ///
    /// # Errors
    ///
    /// Returns a `Trap` error when the range within the table is out of bounds
    /// or the range within the passive element is out of bounds.
    pub(crate) fn table_init(
        &self,
        table_index: TableIndex,
        elem_index: ElemIndex,
        dst: u32,
        src: u32,
        len: u32,
        source_loc: ir::SourceLoc,
    ) -> Result<(), Trap> {
        // https://webassembly.github.io/bulk-memory-operations/core/exec/instructions.html#exec-table-init

        let table = self.get_table(table_index);
        let passive_elements = self.passive_elements.borrow();
        let elem = passive_elements
            .get(&elem_index)
            .map(|e| &**e)
            .unwrap_or_else(|| &[]);

        if src
            .checked_add(len)
            .map_or(true, |n| n as usize > elem.len())
            || dst.checked_add(len).map_or(true, |m| m > table.size())
        {
            return Err(Trap::wasm(source_loc, ir::TrapCode::TableOutOfBounds));
        }

        // TODO(#983): investigate replacing this get/set loop with a `memcpy`.
        for (dst, src) in (dst..dst + len).zip(src..src + len) {
            table
                .set(dst, elem[src as usize].clone())
                .expect("should never panic because we already did the bounds check above");
        }

        Ok(())
    }

    /// Drop an element.
    pub(crate) fn elem_drop(&self, elem_index: ElemIndex) {
        // https://webassembly.github.io/reference-types/core/exec/instructions.html#exec-elem-drop

        let mut passive_elements = self.passive_elements.borrow_mut();
        passive_elements.remove(&elem_index);
        // Note that we don't check that we actually removed an element because
        // dropping a non-passive element is a no-op (not a trap).
    }

    /// Do a `memory.copy` for a locally defined memory.
    ///
    /// # Errors
    ///
    /// Returns a `Trap` error when the source or destination ranges are out of
    /// bounds.
    pub(crate) fn defined_memory_copy(
        &self,
        memory_index: DefinedMemoryIndex,
        dst: u32,
        src: u32,
        len: u32,
        source_loc: ir::SourceLoc,
    ) -> Result<(), Trap> {
        // https://webassembly.github.io/reference-types/core/exec/instructions.html#exec-memory-copy

        let memory = self.memory(memory_index);

        if src
            .checked_add(len)
            .map_or(true, |n| n as usize > memory.current_length)
            || dst
                .checked_add(len)
                .map_or(true, |m| m as usize > memory.current_length)
        {
            return Err(Trap::wasm(source_loc, ir::TrapCode::HeapOutOfBounds));
        }

        let dst = usize::try_from(dst).unwrap();
        let src = usize::try_from(src).unwrap();

        // Bounds and casts are checked above, by this point we know that
        // everything is safe.
        unsafe {
            let dst = memory.base.add(dst);
            let src = memory.base.add(src);
            ptr::copy(src, dst, len as usize);
        }

        Ok(())
    }

    /// Perform a `memory.copy` on an imported memory.
    pub(crate) fn imported_memory_copy(
        &self,
        memory_index: MemoryIndex,
        dst: u32,
        src: u32,
        len: u32,
        source_loc: ir::SourceLoc,
    ) -> Result<(), Trap> {
        let import = self.imported_memory(memory_index);
        unsafe {
            let foreign_instance = (&*import.vmctx).instance();
            let foreign_memory = &*import.from;
            let foreign_index = foreign_instance.memory_index(foreign_memory);
            foreign_instance.defined_memory_copy(foreign_index, dst, src, len, source_loc)
        }
    }

    /// Perform the `memory.fill` operation on a locally defined memory.
    ///
    /// # Errors
    ///
    /// Returns a `Trap` error if the memory range is out of bounds.
    pub(crate) fn defined_memory_fill(
        &self,
        memory_index: DefinedMemoryIndex,
        dst: u32,
        val: u32,
        len: u32,
        source_loc: ir::SourceLoc,
    ) -> Result<(), Trap> {
        let memory = self.memory(memory_index);

        if dst
            .checked_add(len)
            .map_or(true, |m| m as usize > memory.current_length)
        {
            return Err(Trap::wasm(source_loc, ir::TrapCode::HeapOutOfBounds));
        }

        let dst = isize::try_from(dst).unwrap();
        let val = val as u8;

        // Bounds and casts are checked above, by this point we know that
        // everything is safe.
        unsafe {
            let dst = memory.base.offset(dst);
            ptr::write_bytes(dst, val, len as usize);
        }

        Ok(())
    }

    /// Perform the `memory.fill` operation on an imported memory.
    ///
    /// # Errors
    ///
    /// Returns a `Trap` error if the memory range is out of bounds.
    pub(crate) fn imported_memory_fill(
        &self,
        memory_index: MemoryIndex,
        dst: u32,
        val: u32,
        len: u32,
        source_loc: ir::SourceLoc,
    ) -> Result<(), Trap> {
        let import = self.imported_memory(memory_index);
        unsafe {
            let foreign_instance = (&*import.vmctx).instance();
            let foreign_memory = &*import.from;
            let foreign_index = foreign_instance.memory_index(foreign_memory);
            foreign_instance.defined_memory_fill(foreign_index, dst, val, len, source_loc)
        }
    }

    /// Performs the `memory.init` operation.
    ///
    /// # Errors
    ///
    /// Returns a `Trap` error if the destination range is out of this module's
    /// memory's bounds or if the source range is outside the data segment's
    /// bounds.
    pub(crate) fn memory_init(
        &self,
        memory_index: MemoryIndex,
        data_index: DataIndex,
        dst: u32,
        src: u32,
        len: u32,
        source_loc: ir::SourceLoc,
    ) -> Result<(), Trap> {
        // https://webassembly.github.io/bulk-memory-operations/core/exec/instructions.html#exec-memory-init

        let memory = self.get_memory(memory_index);
        let passive_data = self.passive_data.borrow();
        let data = passive_data
            .get(&data_index)
            .map_or(&[][..], |data| &**data);

        if src
            .checked_add(len)
            .map_or(true, |n| n as usize > data.len())
            || dst
                .checked_add(len)
                .map_or(true, |m| m as usize > memory.current_length)
        {
            return Err(Trap::wasm(source_loc, ir::TrapCode::HeapOutOfBounds));
        }

        let src_slice = &data[src as usize..(src + len) as usize];

        unsafe {
            let dst_start = memory.base.add(dst as usize);
            let dst_slice = slice::from_raw_parts_mut(dst_start, len as usize);
            dst_slice.copy_from_slice(src_slice);
        }

        Ok(())
    }

    /// Drop the given data segment, truncating its length to zero.
    pub(crate) fn data_drop(&self, data_index: DataIndex) {
        let mut passive_data = self.passive_data.borrow_mut();
        passive_data.remove(&data_index);
    }

    /// Get a table by index regardless of whether it is locally-defined or an
    /// imported, foreign table.
    pub(crate) fn get_table(&self, table_index: TableIndex) -> &Table {
        if let Some(defined_table_index) = self.module.local.defined_table_index(table_index) {
            self.get_defined_table(defined_table_index)
        } else {
            self.get_foreign_table(table_index)
        }
    }

    /// Get a locally-defined table.
    pub(crate) fn get_defined_table(&self, index: DefinedTableIndex) -> &Table {
        &self.tables[index]
    }

    /// Get an imported, foreign table.
    pub(crate) fn get_foreign_table(&self, index: TableIndex) -> &Table {
        let import = self.imported_table(index);
        let foreign_instance = unsafe { (&mut *(import).vmctx).instance() };
        let foreign_table = unsafe { &mut *(import).from };
        let foreign_index = foreign_instance.table_index(foreign_table);
        &foreign_instance.tables[foreign_index]
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
        trampolines: HashMap<VMSharedSignatureIndex, VMTrampoline>,
        imports: Imports,
        mem_creator: Option<&dyn RuntimeMemoryCreator>,
        data_initializers: &[DataInitializer<'_>],
        vmshared_signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
        dbg_jit_registration: Option<Rc<GdbJitImageRegistration>>,
        is_bulk_memory: bool,
        host_state: Box<dyn Any>,
    ) -> Result<Self, InstantiationError> {
        let tables = create_tables(&module);
        let memories = create_memories(&module, mem_creator.unwrap_or(&DefaultMemoryCreator {}))?;

        let vmctx_tables = tables
            .values()
            .map(Table::vmtable)
            .collect::<PrimaryMap<DefinedTableIndex, _>>()
            .into_boxed_slice();

        let vmctx_memories = memories
            .values()
            .map(|a| a.vmmemory())
            .collect::<PrimaryMap<DefinedMemoryIndex, _>>()
            .into_boxed_slice();

        let vmctx_globals = create_globals(&module);

        let offsets = VMOffsets::new(mem::size_of::<*const u8>() as u8, &module.local);

        let passive_data = RefCell::new(module.passive_data.clone());

        let handle = {
            let instance = Instance {
                refcount: Cell::new(1),
                dependencies: imports.dependencies,
                module,
                offsets,
                memories,
                tables,
                passive_elements: Default::default(),
                passive_data,
                finished_functions,
                trampolines,
                dbg_jit_registration,
                host_state,
                signal_handler: Cell::new(None),
                trap_registration,
                vmctx: VMContext {},
            };
            let layout = instance.alloc_layout();
            let instance_ptr = alloc::alloc(layout) as *mut Instance;
            if instance_ptr.is_null() {
                alloc::handle_alloc_error(layout);
            }
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

        // Check initializer bounds before initializing anything. Only do this
        // when bulk memory is disabled, since the bulk memory proposal changes
        // instantiation such that the intermediate results of failed
        // initializations are visible.
        if !is_bulk_memory {
            check_table_init_bounds(instance)?;
            check_memory_init_bounds(instance, data_initializers)?;
        }

        // Apply the initializers.
        initialize_tables(instance)?;
        initialize_passive_elements(instance);
        initialize_memories(instance, data_initializers)?;
        initialize_globals(instance);

        // Ensure that our signal handlers are ready for action.
        // TODO: Move these calls out of `InstanceHandle`.
        traphandlers::init();

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
        self.instance().module()
    }

    /// Return a reference to a module.
    pub fn module_ref(&self) -> &Module {
        self.instance().module_ref()
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

    /// Get a table defined locally within this module.
    pub fn get_defined_table(&self, index: DefinedTableIndex) -> &Table {
        self.instance().get_defined_table(index)
    }

    /// Gets the trampoline pre-registered for a particular signature
    pub fn trampoline(&self, sig: VMSharedSignatureIndex) -> Option<VMTrampoline> {
        self.instance().trampolines.get(&sig).cloned()
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
            let layout = instance.alloc_layout();
            unsafe {
                ptr::drop_in_place(self.instance);
                alloc::dealloc(self.instance.cast(), layout);
            }
        }
    }
}

fn check_table_init_bounds(instance: &Instance) -> Result<(), InstantiationError> {
    let module = Arc::clone(&instance.module);
    for init in &module.table_elements {
        let start = get_table_init_start(init, instance);
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

/// Compute the offset for a memory data initializer.
fn get_memory_init_start(init: &DataInitializer<'_>, instance: &Instance) -> usize {
    let mut start = init.location.offset;

    if let Some(base) = init.location.base {
        let val = unsafe {
            if let Some(def_index) = instance.module.local.defined_global_index(base) {
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
        .local
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
                    "memory out of bounds: data segment does not fit".into(),
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
        PrimaryMap::with_capacity(module.local.table_plans.len() - num_imports);
    for table in &module.local.table_plans.values().as_slice()[num_imports..] {
        tables.push(Table::new(table));
    }
    tables.into_boxed_slice()
}

/// Compute the offset for a table element initializer.
fn get_table_init_start(init: &TableElements, instance: &Instance) -> usize {
    let mut start = init.offset;

    if let Some(base) = init.base {
        let val = unsafe {
            if let Some(def_index) = instance.module.local.defined_global_index(base) {
                *instance.global(def_index).as_u32()
            } else {
                *(*instance.imported_global(base).from).as_u32()
            }
        };
        start += usize::try_from(val).unwrap();
    }

    start
}

/// Initialize the table memory from the provided initializers.
fn initialize_tables(instance: &Instance) -> Result<(), InstantiationError> {
    let module = Arc::clone(&instance.module);
    for init in &module.table_elements {
        let start = get_table_init_start(init, instance);
        let table = instance.get_table(init.table_index);

        if start
            .checked_add(init.elements.len())
            .map_or(true, |end| end > table.size() as usize)
        {
            return Err(InstantiationError::Trap(Trap::wasm(
                ir::SourceLoc::default(),
                ir::TrapCode::HeapOutOfBounds,
            )));
        }

        for (i, func_idx) in init.elements.iter().enumerate() {
            let anyfunc = instance.get_caller_checked_anyfunc(*func_idx);
            table
                .set(u32::try_from(start + i).unwrap(), anyfunc)
                .unwrap();
        }
    }

    Ok(())
}

/// Initialize the `Instance::passive_elements` map by resolving the
/// `Module::passive_elements`'s `FuncIndex`s into `VMCallerCheckedAnyfunc`s for
/// this instance.
fn initialize_passive_elements(instance: &Instance) {
    let mut passive_elements = instance.passive_elements.borrow_mut();
    debug_assert!(
        passive_elements.is_empty(),
        "should only be called once, at initialization time"
    );

    passive_elements.extend(
        instance
            .module
            .passive_elements
            .iter()
            .filter(|(_, segments)| !segments.is_empty())
            .map(|(idx, segments)| {
                (
                    *idx,
                    segments
                        .iter()
                        .map(|s| instance.get_caller_checked_anyfunc(*s))
                        .collect(),
                )
            }),
    );
}

/// Allocate memory for just the memories of the current module.
fn create_memories(
    module: &Module,
    mem_creator: &dyn RuntimeMemoryCreator,
) -> Result<BoxedSlice<DefinedMemoryIndex, Box<dyn RuntimeLinearMemory>>, InstantiationError> {
    let num_imports = module.imported_memories.len();
    let mut memories: PrimaryMap<DefinedMemoryIndex, _> =
        PrimaryMap::with_capacity(module.local.memory_plans.len() - num_imports);
    for plan in &module.local.memory_plans.values().as_slice()[num_imports..] {
        memories.push(
            mem_creator
                .new_memory(plan)
                .map_err(InstantiationError::Resource)?,
        );
    }
    Ok(memories.into_boxed_slice())
}

/// Initialize the table memory from the provided initializers.
fn initialize_memories(
    instance: &Instance,
    data_initializers: &[DataInitializer<'_>],
) -> Result<(), InstantiationError> {
    for init in data_initializers {
        let memory = instance.get_memory(init.location.memory_index);

        let start = get_memory_init_start(init, instance);
        if start
            .checked_add(init.data.len())
            .map_or(true, |end| end > memory.current_length)
        {
            return Err(InstantiationError::Trap(Trap::wasm(
                ir::SourceLoc::default(),
                ir::TrapCode::HeapOutOfBounds,
            )));
        }

        unsafe {
            let mem_slice = get_memory_slice(init, instance);
            let end = start + init.data.len();
            let to_init = &mut mem_slice[start..end];
            to_init.copy_from_slice(init.data);
        }
    }

    Ok(())
}

/// Allocate memory for just the globals of the current module,
/// with initializers applied.
fn create_globals(module: &Module) -> BoxedSlice<DefinedGlobalIndex, VMGlobalDefinition> {
    let num_imports = module.imported_globals.len();
    let mut vmctx_globals = PrimaryMap::with_capacity(module.local.globals.len() - num_imports);

    for _ in &module.local.globals.values().as_slice()[num_imports..] {
        vmctx_globals.push(VMGlobalDefinition::new());
    }

    vmctx_globals.into_boxed_slice()
}

fn initialize_globals(instance: &Instance) {
    let module = Arc::clone(&instance.module);
    let num_imports = module.imported_globals.len();
    for (index, global) in module.local.globals.iter().skip(num_imports) {
        let def_index = module.local.defined_global_index(index).unwrap();
        unsafe {
            let to = instance.global_ptr(def_index);
            match global.initializer {
                GlobalInit::I32Const(x) => *(*to).as_i32_mut() = x,
                GlobalInit::I64Const(x) => *(*to).as_i64_mut() = x,
                GlobalInit::F32Const(x) => *(*to).as_f32_bits_mut() = x,
                GlobalInit::F64Const(x) => *(*to).as_f64_bits_mut() = x,
                GlobalInit::V128Const(x) => *(*to).as_u128_bits_mut() = x.0,
                GlobalInit::GetGlobal(x) => {
                    let from = if let Some(def_x) = module.local.defined_global_index(x) {
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

    /// A trap ocurred during instantiation, after linking.
    #[error("Trap occurred during instantiation")]
    Trap(#[source] Trap),

    /// A compilation error occured.
    #[error("Trap occurred while invoking start function")]
    StartTrap(#[source] Trap),
}
