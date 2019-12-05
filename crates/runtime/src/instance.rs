//! An `Instance` contains all the runtime state used by execution of a
//! wasm module (except its callstack and register state). An
//! `InstanceHandle` is a reference-counting handle for an `Instance`.

use crate::export::Export;
use crate::imports::Imports;
use crate::jit_int::GdbJitImageRegistration;
use crate::memory::LinearMemory;
use crate::mmap::Mmap;
use crate::signalhandlers::{wasmtime_init_eager, wasmtime_init_finish};
use crate::table::Table;
use crate::traphandlers::wasmtime_call;
use crate::vmcontext::{
    VMBuiltinFunctionsArray, VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMFunctionImport,
    VMGlobalDefinition, VMGlobalImport, VMMemoryDefinition, VMMemoryImport, VMSharedSignatureIndex,
    VMTableDefinition, VMTableImport,
};
use cranelift_entity::{BoxedSlice, EntityRef, PrimaryMap};
use cranelift_wasm::{
    DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex,
    GlobalIndex, GlobalInit, MemoryIndex, SignatureIndex, TableIndex,
};
use memoffset::offset_of;
use more_asserts::assert_lt;
use std::any::Any;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::rc::Rc;
use std::{mem, ptr, slice};
use thiserror::Error;
use wasmtime_environ::{DataInitializer, Module, TableElements, VMOffsets};

fn signature_id(
    vmctx: &VMContext,
    offsets: &VMOffsets,
    index: SignatureIndex,
) -> VMSharedSignatureIndex {
    #[allow(clippy::cast_ptr_alignment)]
    unsafe {
        let ptr = (vmctx as *const VMContext as *const u8)
            .add(usize::try_from(offsets.vmctx_vmshared_signature_id(index)).unwrap());
        *(ptr as *const VMSharedSignatureIndex)
    }
}

fn imported_function<'vmctx>(
    vmctx: &'vmctx VMContext,
    offsets: &VMOffsets,
    index: FuncIndex,
) -> &'vmctx VMFunctionImport {
    #[allow(clippy::cast_ptr_alignment)]
    unsafe {
        let ptr = (vmctx as *const VMContext as *const u8)
            .add(usize::try_from(offsets.vmctx_vmfunction_import(index)).unwrap());
        &*(ptr as *const VMFunctionImport)
    }
}

fn imported_table<'vmctx>(
    vmctx: &'vmctx VMContext,
    offsets: &VMOffsets,
    index: TableIndex,
) -> &'vmctx VMTableImport {
    #[allow(clippy::cast_ptr_alignment)]
    unsafe {
        let ptr = (vmctx as *const VMContext as *const u8)
            .add(usize::try_from(offsets.vmctx_vmtable_import(index)).unwrap());
        &*(ptr as *const VMTableImport)
    }
}

fn imported_memory<'vmctx>(
    vmctx: &'vmctx VMContext,
    offsets: &VMOffsets,
    index: MemoryIndex,
) -> &'vmctx VMMemoryImport {
    #[allow(clippy::cast_ptr_alignment)]
    unsafe {
        let ptr = (vmctx as *const VMContext as *const u8)
            .add(usize::try_from(offsets.vmctx_vmmemory_import(index)).unwrap());
        &*(ptr as *const VMMemoryImport)
    }
}

fn imported_global<'vmctx>(
    vmctx: &'vmctx VMContext,
    offsets: &VMOffsets,
    index: GlobalIndex,
) -> &'vmctx VMGlobalImport {
    unsafe {
        let ptr = (vmctx as *const VMContext as *const u8)
            .add(usize::try_from(offsets.vmctx_vmglobal_import(index)).unwrap());
        #[allow(clippy::cast_ptr_alignment)]
        &*(ptr as *const VMGlobalImport)
    }
}

fn table<'vmctx>(
    vmctx: &'vmctx VMContext,
    offsets: &VMOffsets,
    index: DefinedTableIndex,
) -> &'vmctx VMTableDefinition {
    unsafe {
        let ptr = (vmctx as *const VMContext as *const u8)
            .add(usize::try_from(offsets.vmctx_vmtable_definition(index)).unwrap());
        #[allow(clippy::cast_ptr_alignment)]
        &*(ptr as *const VMTableDefinition)
    }
}

fn table_mut<'vmctx>(
    vmctx: &'vmctx mut VMContext,
    offsets: &VMOffsets,
    index: DefinedTableIndex,
) -> &'vmctx mut VMTableDefinition {
    unsafe {
        let ptr = (vmctx as *mut VMContext as *mut u8)
            .add(usize::try_from(offsets.vmctx_vmtable_definition(index)).unwrap());
        #[allow(clippy::cast_ptr_alignment)]
        &mut *(ptr as *mut VMTableDefinition)
    }
}

fn memory<'vmctx>(
    vmctx: &'vmctx VMContext,
    offsets: &VMOffsets,
    index: DefinedMemoryIndex,
) -> &'vmctx VMMemoryDefinition {
    unsafe {
        let ptr = (vmctx as *const VMContext as *const u8)
            .add(usize::try_from(offsets.vmctx_vmmemory_definition(index)).unwrap());
        #[allow(clippy::cast_ptr_alignment)]
        &*(ptr as *const VMMemoryDefinition)
    }
}

fn memory_mut<'vmctx>(
    vmctx: &'vmctx mut VMContext,
    offsets: &VMOffsets,
    index: DefinedMemoryIndex,
) -> &'vmctx mut VMMemoryDefinition {
    unsafe {
        let ptr = (vmctx as *mut VMContext as *mut u8)
            .add(usize::try_from(offsets.vmctx_vmmemory_definition(index)).unwrap());
        #[allow(clippy::cast_ptr_alignment)]
        &mut *(ptr as *mut VMMemoryDefinition)
    }
}

fn global<'vmctx>(
    vmctx: &'vmctx VMContext,
    offsets: &VMOffsets,
    index: DefinedGlobalIndex,
) -> &'vmctx VMGlobalDefinition {
    unsafe {
        let ptr = (vmctx as *const VMContext as *const u8)
            .add(usize::try_from(offsets.vmctx_vmglobal_definition(index)).unwrap());
        #[allow(clippy::cast_ptr_alignment)]
        &*(ptr as *const VMGlobalDefinition)
    }
}

fn global_mut<'vmctx>(
    vmctx: &'vmctx mut VMContext,
    offsets: &VMOffsets,
    index: DefinedGlobalIndex,
) -> &'vmctx mut VMGlobalDefinition {
    unsafe {
        let ptr = (vmctx as *mut VMContext as *mut u8)
            .add(usize::try_from(offsets.vmctx_vmglobal_definition(index)).unwrap());
        #[allow(clippy::cast_ptr_alignment)]
        &mut *(ptr as *mut VMGlobalDefinition)
    }
}

/// A WebAssembly instance.
///
/// This is repr(C) to ensure that the vmctx field is last.
#[repr(C)]
pub(crate) struct Instance {
    /// The number of references to this `Instance`.
    refcount: usize,

    /// `Instance`s from which this `Instance` imports. These won't
    /// create reference cycles because wasm instances can't cyclically
    /// import from each other.
    dependencies: HashSet<InstanceHandle>,

    /// The underlying mmap that holds this `Instance`.
    mmap: Mmap,

    /// The `Module` this `Instance` was instantiated from.
    module: Rc<Module>,

    /// Offsets in the `vmctx` region.
    offsets: VMOffsets,

    /// A global namespace of exports. This is a temporary mechanism to avoid
    /// cyclic dependencies when one module wants to import from another and
    /// make its memory available too, that will be obviated by host-bindings.
    global_exports: Rc<RefCell<HashMap<String, Option<Export>>>>,

    /// WebAssembly linear memory data.
    memories: BoxedSlice<DefinedMemoryIndex, LinearMemory>,

    /// WebAssembly table data.
    tables: BoxedSlice<DefinedTableIndex, Table>,

    /// Pointers to functions in executable memory.
    finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,

    /// Hosts can store arbitrary per-instance information here.
    host_state: Box<dyn Any>,

    /// Optional image of JIT'ed code for debugger registration.
    dbg_jit_registration: Option<Rc<GdbJitImageRegistration>>,

    /// Additional context used by compiled wasm code. This field is last, and
    /// represents a dynamically-sized array that extends beyond the nominal
    /// end of the struct (similar to a flexible array member).
    vmctx: VMContext,
}

#[allow(clippy::cast_ptr_alignment)]
impl Instance {
    /// Return the indexed `VMSharedSignatureIndex`.
    #[allow(dead_code)]
    fn signature_id(&self, index: SignatureIndex) -> VMSharedSignatureIndex {
        signature_id(&self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMSharedSignatureIndex`s.
    fn signature_ids_ptr(&mut self) -> *mut VMSharedSignatureIndex {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(usize::try_from(self.offsets.vmctx_signature_ids_begin()).unwrap())
                as *mut VMSharedSignatureIndex
        }
    }

    /// Return the indexed `VMFunctionImport`.
    fn imported_function(&self, index: FuncIndex) -> &VMFunctionImport {
        imported_function(&self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMFunctionImport`s.
    fn imported_functions_ptr(&mut self) -> *mut VMFunctionImport {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(usize::try_from(self.offsets.vmctx_imported_functions_begin()).unwrap())
                as *mut VMFunctionImport
        }
    }

    /// Return the index `VMTableImport`.
    #[allow(dead_code)]
    fn imported_table(&self, index: TableIndex) -> &VMTableImport {
        imported_table(&self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMTableImports`s.
    fn imported_tables_ptr(&mut self) -> *mut VMTableImport {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(usize::try_from(self.offsets.vmctx_imported_tables_begin()).unwrap())
                as *mut VMTableImport
        }
    }

    /// Return the indexed `VMMemoryImport`.
    fn imported_memory(&self, index: MemoryIndex) -> &VMMemoryImport {
        imported_memory(&self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMMemoryImport`s.
    fn imported_memories_ptr(&mut self) -> *mut VMMemoryImport {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(usize::try_from(self.offsets.vmctx_imported_memories_begin()).unwrap())
                as *mut VMMemoryImport
        }
    }

    /// Return the indexed `VMGlobalImport`.
    fn imported_global(&self, index: GlobalIndex) -> &VMGlobalImport {
        imported_global(&self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMGlobalImport`s.
    fn imported_globals_ptr(&mut self) -> *mut VMGlobalImport {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(usize::try_from(self.offsets.vmctx_imported_globals_begin()).unwrap())
                as *mut VMGlobalImport
        }
    }

    /// Return the indexed `VMTableDefinition`.
    #[allow(dead_code)]
    fn table(&self, index: DefinedTableIndex) -> &VMTableDefinition {
        table(&self.vmctx, &self.offsets, index)
    }

    /// Return the indexed `VMTableDefinition`.
    #[allow(dead_code)]
    fn table_mut(&mut self, index: DefinedTableIndex) -> &mut VMTableDefinition {
        table_mut(&mut self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMTableDefinition`s.
    fn tables_ptr(&mut self) -> *mut VMTableDefinition {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(usize::try_from(self.offsets.vmctx_tables_begin()).unwrap())
                as *mut VMTableDefinition
        }
    }

    /// Return the indexed `VMMemoryDefinition`.
    fn memory(&self, index: DefinedMemoryIndex) -> &VMMemoryDefinition {
        memory(&self.vmctx, &self.offsets, index)
    }

    /// Return the indexed `VMMemoryDefinition`.
    fn memory_mut(&mut self, index: DefinedMemoryIndex) -> &mut VMMemoryDefinition {
        memory_mut(&mut self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMMemoryDefinition`s.
    fn memories_ptr(&mut self) -> *mut VMMemoryDefinition {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(usize::try_from(self.offsets.vmctx_memories_begin()).unwrap())
                as *mut VMMemoryDefinition
        }
    }

    /// Return the indexed `VMGlobalDefinition`.
    #[allow(dead_code)]
    fn global(&self, index: DefinedGlobalIndex) -> &VMGlobalDefinition {
        global(&self.vmctx, &self.offsets, index)
    }

    /// Return the indexed `VMGlobalDefinition`.
    fn global_mut(&mut self, index: DefinedGlobalIndex) -> &mut VMGlobalDefinition {
        global_mut(&mut self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMGlobalDefinition`s.
    fn globals_ptr(&mut self) -> *mut VMGlobalDefinition {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(usize::try_from(self.offsets.vmctx_globals_begin()).unwrap())
                as *mut VMGlobalDefinition
        }
    }

    /// Return a pointer to the `VMBuiltinFunctionsArray`.
    fn builtin_functions_ptr(&mut self) -> *mut VMBuiltinFunctionsArray {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(usize::try_from(self.offsets.vmctx_builtin_functions_begin()).unwrap())
                as *mut VMBuiltinFunctionsArray
        }
    }

    /// Return a reference to the vmctx used by compiled wasm code.
    pub fn vmctx(&self) -> &VMContext {
        &self.vmctx
    }

    /// Return a raw pointer to the vmctx used by compiled wasm code.
    pub fn vmctx_ptr(&self) -> *const VMContext {
        self.vmctx()
    }

    /// Return a mutable reference to the vmctx used by compiled wasm code.
    pub fn vmctx_mut(&mut self) -> &mut VMContext {
        &mut self.vmctx
    }

    /// Return a mutable raw pointer to the vmctx used by compiled wasm code.
    pub fn vmctx_mut_ptr(&mut self) -> *mut VMContext {
        self.vmctx_mut()
    }

    /// Lookup an export with the given name.
    pub fn lookup(&mut self, field: &str) -> Option<Export> {
        let export = if let Some(export) = self.module.exports.get(field) {
            export.clone()
        } else {
            return None;
        };
        Some(self.lookup_by_declaration(&export))
    }

    /// Lookup an export with the given name. This takes an immutable reference,
    /// and the result is an `Export` that the type system doesn't prevent from
    /// being used to mutate the instance, so this function is unsafe.
    pub unsafe fn lookup_immutable(&self, field: &str) -> Option<Export> {
        #[allow(clippy::cast_ref_to_mut)]
        let temporary_mut = &mut *(self as *const Self as *mut Self);
        temporary_mut.lookup(field)
    }

    /// Lookup an export with the given export declaration.
    pub fn lookup_by_declaration(&mut self, export: &wasmtime_environ::Export) -> Export {
        lookup_by_declaration(
            &self.module,
            &mut self.vmctx,
            &self.offsets,
            &self.finished_functions,
            export,
        )
    }

    /// Lookup an export with the given export declaration. This takes an immutable
    /// reference, and the result is an `Export` that the type system doesn't prevent
    /// from being used to mutate the instance, so this function is unsafe.
    pub unsafe fn lookup_immutable_by_declaration(
        &self,
        export: &wasmtime_environ::Export,
    ) -> Export {
        #[allow(clippy::cast_ref_to_mut)]
        let temporary_mut = &mut *(self as *const Self as *mut Self);
        temporary_mut.lookup_by_declaration(export)
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
    pub fn host_state(&mut self) -> &mut dyn Any {
        &mut *self.host_state
    }

    fn invoke_function(&mut self, index: FuncIndex) -> Result<(), InstantiationError> {
        // TODO: Check that the callee's calling convention matches what we expect.

        let (callee_address, callee_vmctx) = match self.module.defined_func_index(index) {
            Some(defined_index) => {
                let body = *self
                    .finished_functions
                    .get(defined_index)
                    .expect("function index is out of bounds");
                (body, self.vmctx_mut() as *mut VMContext)
            }
            None => {
                assert_lt!(index.index(), self.module.imported_funcs.len());
                let import = self.imported_function(index);
                (import.body, import.vmctx)
            }
        };

        // Make the call.
        unsafe { wasmtime_call(callee_vmctx, callee_address) }
            .map_err(InstantiationError::StartTrap)
    }

    /// Invoke the WebAssembly start function of the instance, if one is present.
    fn invoke_start_function(&mut self) -> Result<(), InstantiationError> {
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

    /// Test whether any of the objects inside this instance require signal
    /// handlers to catch out of bounds accesses.
    pub(crate) fn needs_signal_handlers(&self) -> bool {
        self.memories
            .values()
            .any(|memory| memory.needs_signal_handlers)
    }

    /// Grow memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub(crate) fn memory_grow(
        &mut self,
        memory_index: DefinedMemoryIndex,
        delta: u32,
    ) -> Option<u32> {
        let result = self
            .memories
            .get_mut(memory_index)
            .unwrap_or_else(|| panic!("no memory for index {}", memory_index.index()))
            .grow(delta);

        // Keep current the VMContext pointers used by compiled wasm code.
        *self.memory_mut(memory_index) = self.memories[memory_index].vmmemory();

        result
    }

    /// Grow imported memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    ///
    /// TODO: This and `imported_memory_size` are currently unsafe because
    /// they dereference the memory import's pointers.
    pub(crate) unsafe fn imported_memory_grow(
        &mut self,
        memory_index: MemoryIndex,
        delta: u32,
    ) -> Option<u32> {
        let import = self.imported_memory(memory_index);
        let foreign_instance = (&mut *import.vmctx).instance();
        let foreign_memory = &mut *import.from;
        let foreign_index = foreign_instance.memory_index(foreign_memory);

        foreign_instance.memory_grow(foreign_index, delta)
    }

    /// Returns the number of allocated wasm pages.
    pub(crate) fn memory_size(&mut self, memory_index: DefinedMemoryIndex) -> u32 {
        self.memories
            .get(memory_index)
            .unwrap_or_else(|| panic!("no memory for index {}", memory_index.index()))
            .size()
    }

    /// Returns the number of allocated wasm pages in an imported memory.
    pub(crate) unsafe fn imported_memory_size(&mut self, memory_index: MemoryIndex) -> u32 {
        let import = self.imported_memory(memory_index);
        let foreign_instance = (&mut *import.vmctx).instance();
        let foreign_memory = &mut *import.from;
        let foreign_index = foreign_instance.memory_index(foreign_memory);

        foreign_instance.memory_size(foreign_index)
    }

    pub(crate) fn lookup_global_export(&self, field: &str) -> Option<Export> {
        let cell: &RefCell<HashMap<String, Option<Export>>> = self.global_exports.borrow();
        let map: &mut HashMap<String, Option<Export>> = &mut cell.borrow_mut();
        if let Some(Some(export)) = map.get(field) {
            return Some(export.clone());
        }
        None
    }

    /// Grow table by the specified amount of elements.
    ///
    /// Returns `None` if table can't be grown by the specified amount
    /// of elements.
    pub(crate) fn table_grow(&mut self, table_index: DefinedTableIndex, delta: u32) -> Option<u32> {
        let result = self
            .tables
            .get_mut(table_index)
            .unwrap_or_else(|| panic!("no table for index {}", table_index.index()))
            .grow(delta);

        // Keep current the VMContext pointers used by compiled wasm code.
        *self.table_mut(table_index) = self.tables[table_index].vmtable();

        result
    }

    // Get table element by index.
    pub(crate) fn table_get(
        &self,
        table_index: DefinedTableIndex,
        index: u32,
    ) -> Option<&VMCallerCheckedAnyfunc> {
        self.tables
            .get(table_index)
            .unwrap_or_else(|| panic!("no table for index {}", table_index.index()))
            .get(index)
    }

    // Get table mutable element by index.
    pub(crate) fn table_get_mut(
        &mut self,
        table_index: DefinedTableIndex,
        index: u32,
    ) -> Option<&mut VMCallerCheckedAnyfunc> {
        self.tables
            .get_mut(table_index)
            .unwrap_or_else(|| panic!("no table for index {}", table_index.index()))
            .get_mut(index)
    }
}

/// A handle holding an `Instance` of a WebAssembly module.
#[derive(Hash, PartialEq, Eq)]
pub struct InstanceHandle {
    instance: *mut Instance,
}

impl InstanceHandle {
    /// Create a new `InstanceHandle` pointing at a new `Instance`.
    pub fn new(
        module: Rc<Module>,
        global_exports: Rc<RefCell<HashMap<String, Option<Export>>>>,
        finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
        imports: Imports,
        data_initializers: &[DataInitializer<'_>],
        vmshared_signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
        dbg_jit_registration: Option<Rc<GdbJitImageRegistration>>,
        host_state: Box<dyn Any>,
    ) -> Result<Self, InstantiationError> {
        let mut tables = create_tables(&module);
        let mut memories = create_memories(&module)?;

        let vmctx_tables = tables
            .values_mut()
            .map(Table::vmtable)
            .collect::<PrimaryMap<DefinedTableIndex, _>>()
            .into_boxed_slice();

        let vmctx_memories = memories
            .values_mut()
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

        let instance = {
            #[allow(clippy::cast_ptr_alignment)]
            let instance_ptr = instance_mmap.as_mut_ptr() as *mut Instance;
            let instance = Instance {
                refcount: 1,
                dependencies: imports.dependencies,
                mmap: instance_mmap,
                module,
                global_exports,
                offsets,
                memories,
                tables,
                finished_functions,
                dbg_jit_registration,
                host_state,
                vmctx: VMContext {},
            };
            unsafe {
                ptr::write(instance_ptr, instance);
                &mut *instance_ptr
            }
        };

        unsafe {
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
        }

        // Check initializer bounds before initializing anything.
        check_table_init_bounds(instance)?;
        check_memory_init_bounds(instance, data_initializers)?;

        // Apply the initializers.
        initialize_tables(instance)?;
        initialize_memories(instance, data_initializers)?;
        initialize_globals(instance);

        // Collect the exports for the global export map.
        for (field, decl) in &instance.module.exports {
            use std::collections::hash_map::Entry::*;
            let cell: &RefCell<HashMap<String, Option<Export>>> = instance.global_exports.borrow();
            let map: &mut HashMap<String, Option<Export>> = &mut cell.borrow_mut();
            match map.entry(field.to_string()) {
                Vacant(entry) => {
                    entry.insert(Some(lookup_by_declaration(
                        &instance.module,
                        &mut instance.vmctx,
                        &instance.offsets,
                        &instance.finished_functions,
                        &decl,
                    )));
                }
                Occupied(ref mut entry) => *entry.get_mut() = None,
            }
        }

        // Ensure that our signal handlers are ready for action.
        // TODO: Move these calls out of `InstanceHandle`.
        wasmtime_init_eager();
        wasmtime_init_finish(instance.vmctx_mut());

        // The WebAssembly spec specifies that the start function is
        // invoked automatically at instantiation time.
        instance.invoke_start_function()?;

        Ok(Self { instance })
    }

    /// Create a new `InstanceHandle` pointing at the instance
    /// pointed to by the given `VMContext` pointer.
    pub unsafe fn from_vmctx(vmctx: *mut VMContext) -> Self {
        let instance = (&mut *vmctx).instance();
        instance.refcount += 1;
        Self { instance }
    }

    /// Return a reference to the vmctx used by compiled wasm code.
    pub fn vmctx(&self) -> &VMContext {
        self.instance().vmctx()
    }

    /// Return a raw pointer to the vmctx used by compiled wasm code.
    pub fn vmctx_ptr(&self) -> *const VMContext {
        self.instance().vmctx_ptr()
    }

    /// Return a reference-counting pointer to a module.
    pub fn module(&self) -> Rc<Module> {
        self.instance().module.clone()
    }

    /// Return a reference to a module.
    pub fn module_ref(&self) -> &Module {
        &self.instance().module
    }

    /// Return a mutable reference to the vmctx used by compiled wasm code.
    pub fn vmctx_mut(&mut self) -> &mut VMContext {
        self.instance_mut().vmctx_mut()
    }

    /// Return a mutable raw pointer to the vmctx used by compiled wasm code.
    pub fn vmctx_mut_ptr(&mut self) -> *mut VMContext {
        self.instance_mut().vmctx_mut_ptr()
    }

    /// Lookup an export with the given name.
    pub fn lookup(&mut self, field: &str) -> Option<Export> {
        self.instance_mut().lookup(field)
    }

    /// Lookup an export with the given name. This takes an immutable reference,
    /// and the result is an `Export` that the type system doesn't prevent from
    /// being used to mutate the instance, so this function is unsafe.
    pub unsafe fn lookup_immutable(&self, field: &str) -> Option<Export> {
        self.instance().lookup_immutable(field)
    }

    /// Lookup an export with the given export declaration.
    pub fn lookup_by_declaration(&mut self, export: &wasmtime_environ::Export) -> Export {
        self.instance_mut().lookup_by_declaration(export)
    }

    /// Lookup an export with the given export declaration. This takes an immutable
    /// reference, and the result is an `Export` that the type system doesn't prevent
    /// from being used to mutate the instance, so this function is unsafe.
    pub unsafe fn lookup_immutable_by_declaration(
        &self,
        export: &wasmtime_environ::Export,
    ) -> Export {
        self.instance().lookup_immutable_by_declaration(export)
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
    pub fn host_state(&mut self) -> &mut dyn Any {
        self.instance_mut().host_state()
    }

    /// Return the memory index for the given `VMMemoryDefinition` in this instance.
    pub fn memory_index(&self, memory: &VMMemoryDefinition) -> DefinedMemoryIndex {
        self.instance().memory_index(memory)
    }

    /// Grow memory in this instance by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub fn memory_grow(&mut self, memory_index: DefinedMemoryIndex, delta: u32) -> Option<u32> {
        self.instance_mut().memory_grow(memory_index, delta)
    }

    /// Return the table index for the given `VMTableDefinition` in this instance.
    pub fn table_index(&self, table: &VMTableDefinition) -> DefinedTableIndex {
        self.instance().table_index(table)
    }

    /// Grow table in this instance by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub fn table_grow(&mut self, table_index: DefinedTableIndex, delta: u32) -> Option<u32> {
        self.instance_mut().table_grow(table_index, delta)
    }

    /// Get table element reference.
    ///
    /// Returns `None` if index is out of bounds.
    pub fn table_get(
        &self,
        table_index: DefinedTableIndex,
        index: u32,
    ) -> Option<&VMCallerCheckedAnyfunc> {
        self.instance().table_get(table_index, index)
    }

    /// Get mutable table element reference.
    ///
    /// Returns `None` if index is out of bounds.
    pub fn table_get_mut(
        &mut self,
        table_index: DefinedTableIndex,
        index: u32,
    ) -> Option<&mut VMCallerCheckedAnyfunc> {
        self.instance_mut().table_get_mut(table_index, index)
    }
}

impl InstanceHandle {
    /// Return a reference to the contained `Instance`.
    fn instance(&self) -> &Instance {
        unsafe { &*(self.instance as *const Instance) }
    }

    /// Return a mutable reference to the contained `Instance`.
    fn instance_mut(&mut self) -> &mut Instance {
        unsafe { &mut *(self.instance as *mut Instance) }
    }
}

impl Clone for InstanceHandle {
    fn clone(&self) -> Self {
        unsafe { &mut *(self.instance as *mut Instance) }.refcount += 1;
        Self {
            instance: self.instance,
        }
    }
}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        let instance = self.instance_mut();
        instance.refcount -= 1;
        if instance.refcount == 0 {
            let mmap = mem::replace(&mut instance.mmap, Mmap::new());
            unsafe { ptr::drop_in_place(instance) };
            mem::drop(mmap);
        }
    }
}

fn lookup_by_declaration(
    module: &Module,
    vmctx: &mut VMContext,
    offsets: &VMOffsets,
    finished_functions: &BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
    export: &wasmtime_environ::Export,
) -> Export {
    match export {
        wasmtime_environ::Export::Function(index) => {
            let signature = module.signatures[module.functions[*index]].clone();
            let (address, vmctx) = if let Some(def_index) = module.defined_func_index(*index) {
                (finished_functions[def_index], vmctx as *mut VMContext)
            } else {
                let import = imported_function(vmctx, offsets, *index);
                (import.body, import.vmctx)
            };
            Export::Function {
                address,
                signature,
                vmctx,
            }
        }
        wasmtime_environ::Export::Table(index) => {
            let (definition, vmctx) = if let Some(def_index) = module.defined_table_index(*index) {
                (
                    table_mut(vmctx, offsets, def_index) as *mut VMTableDefinition,
                    vmctx as *mut VMContext,
                )
            } else {
                let import = imported_table(vmctx, offsets, *index);
                (import.from, import.vmctx)
            };
            Export::Table {
                definition,
                vmctx,
                table: module.table_plans[*index].clone(),
            }
        }
        wasmtime_environ::Export::Memory(index) => {
            let (definition, vmctx) = if let Some(def_index) = module.defined_memory_index(*index) {
                (
                    memory_mut(vmctx, offsets, def_index) as *mut VMMemoryDefinition,
                    vmctx as *mut VMContext,
                )
            } else {
                let import = imported_memory(vmctx, offsets, *index);
                (import.from, import.vmctx)
            };
            Export::Memory {
                definition,
                vmctx,
                memory: module.memory_plans[*index].clone(),
            }
        }
        wasmtime_environ::Export::Global(index) => Export::Global {
            definition: if let Some(def_index) = module.defined_global_index(*index) {
                global_mut(vmctx, offsets, def_index)
            } else {
                imported_global(vmctx, offsets, *index).from
            },
            vmctx,
            global: module.globals[*index],
        },
    }
}

fn check_table_init_bounds(instance: &mut Instance) -> Result<(), InstantiationError> {
    let module = Rc::clone(&instance.module);
    for init in &module.table_elements {
        let start = get_table_init_start(init, instance);
        let slice = get_table_slice(
            init,
            &instance.module,
            &mut instance.tables,
            &instance.vmctx,
            &instance.offsets,
        );

        if slice.get_mut(start..start + init.elements.len()).is_none() {
            return Err(InstantiationError::Link(LinkError(
                "elements segment does not fit".to_owned(),
            )));
        }
    }

    Ok(())
}

/// Compute the offset for a memory data initializer.
fn get_memory_init_start(init: &DataInitializer<'_>, instance: &mut Instance) -> usize {
    let mut start = init.location.offset;

    if let Some(base) = init.location.base {
        let global = if let Some(def_index) = instance.module.defined_global_index(base) {
            instance.global_mut(def_index)
        } else {
            instance.imported_global(base).from
        };
        start += usize::try_from(*unsafe { (*global).as_u32() }).unwrap();
    }

    start
}

/// Return a byte-slice view of a memory's data.
fn get_memory_slice<'instance>(
    init: &DataInitializer<'_>,
    instance: &'instance mut Instance,
) -> &'instance mut [u8] {
    let memory = if let Some(defined_memory_index) = instance
        .module
        .defined_memory_index(init.location.memory_index)
    {
        instance.memory(defined_memory_index)
    } else {
        let import = instance.imported_memory(init.location.memory_index);
        let foreign_instance = unsafe { (&mut *(import).vmctx).instance() };
        let foreign_memory = unsafe { &mut *(import).from };
        let foreign_index = foreign_instance.memory_index(foreign_memory);
        foreign_instance.memory(foreign_index)
    };
    unsafe { slice::from_raw_parts_mut(memory.base, memory.current_length) }
}

fn check_memory_init_bounds(
    instance: &mut Instance,
    data_initializers: &[DataInitializer<'_>],
) -> Result<(), InstantiationError> {
    for init in data_initializers {
        let start = get_memory_init_start(init, instance);
        let mem_slice = get_memory_slice(init, instance);

        if mem_slice.get_mut(start..start + init.data.len()).is_none() {
            return Err(InstantiationError::Link(LinkError(
                "data segment does not fit".to_owned(),
            )));
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
fn get_table_init_start(init: &TableElements, instance: &mut Instance) -> usize {
    let mut start = init.offset;

    if let Some(base) = init.base {
        let global = if let Some(def_index) = instance.module.defined_global_index(base) {
            instance.global_mut(def_index)
        } else {
            instance.imported_global(base).from
        };
        start += usize::try_from(*unsafe { (*global).as_u32() }).unwrap();
    }

    start
}

/// Return a byte-slice view of a table's data.
fn get_table_slice<'instance>(
    init: &TableElements,
    module: &Module,
    tables: &'instance mut BoxedSlice<DefinedTableIndex, Table>,
    vmctx: &VMContext,
    offsets: &VMOffsets,
) -> &'instance mut [VMCallerCheckedAnyfunc] {
    if let Some(defined_table_index) = module.defined_table_index(init.table_index) {
        tables[defined_table_index].as_mut()
    } else {
        let import = imported_table(vmctx, offsets, init.table_index);
        let foreign_instance = unsafe { (&mut *(import).vmctx).instance() };
        let foreign_table = unsafe { &mut *(import).from };
        let foreign_index = foreign_instance.table_index(foreign_table);
        foreign_instance.tables[foreign_index].as_mut()
    }
}

/// Initialize the table memory from the provided initializers.
fn initialize_tables(instance: &mut Instance) -> Result<(), InstantiationError> {
    let vmctx: *mut VMContext = instance.vmctx_mut();
    let module = Rc::clone(&instance.module);
    for init in &module.table_elements {
        let start = get_table_init_start(init, instance);
        let slice = get_table_slice(
            init,
            &instance.module,
            &mut instance.tables,
            &instance.vmctx,
            &instance.offsets,
        );

        let subslice = &mut slice[start..start + init.elements.len()];
        for (i, func_idx) in init.elements.iter().enumerate() {
            let callee_sig = instance.module.functions[*func_idx];
            let (callee_ptr, callee_vmctx) =
                if let Some(index) = instance.module.defined_func_index(*func_idx) {
                    (instance.finished_functions[index], vmctx)
                } else {
                    let imported_func =
                        imported_function(&instance.vmctx, &instance.offsets, *func_idx);
                    (imported_func.body, imported_func.vmctx)
                };
            let type_index = signature_id(&instance.vmctx, &instance.offsets, callee_sig);
            subslice[i] = VMCallerCheckedAnyfunc {
                func_ptr: callee_ptr,
                type_index,
                vmctx: callee_vmctx,
            };
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
    instance: &mut Instance,
    data_initializers: &[DataInitializer<'_>],
) -> Result<(), InstantiationError> {
    for init in data_initializers {
        let start = get_memory_init_start(init, instance);
        let mem_slice = get_memory_slice(init, instance);

        let to_init = &mut mem_slice[start..start + init.data.len()];
        to_init.copy_from_slice(init.data);
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
    let module = Rc::clone(&instance.module);
    let num_imports = module.imported_globals.len();
    for (index, global) in module.globals.iter().skip(num_imports) {
        let def_index = module.defined_global_index(index).unwrap();
        let to: *mut VMGlobalDefinition = instance.global_mut(def_index);
        match global.initializer {
            GlobalInit::I32Const(x) => *unsafe { (*to).as_i32_mut() } = x,
            GlobalInit::I64Const(x) => *unsafe { (*to).as_i64_mut() } = x,
            GlobalInit::F32Const(x) => *unsafe { (*to).as_f32_bits_mut() } = x,
            GlobalInit::F64Const(x) => *unsafe { (*to).as_f64_bits_mut() } = x,
            GlobalInit::V128Const(x) => *unsafe { (*to).as_u128_bits_mut() } = x.0,
            GlobalInit::GetGlobal(x) => {
                let from = if let Some(def_x) = module.defined_global_index(x) {
                    instance.global_mut(def_x)
                } else {
                    instance.imported_global(x).from
                };
                unsafe { *to = *from };
            }
            GlobalInit::Import => panic!("locally-defined global initialized as import"),
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
    #[error("Trap occurred while invoking start function: {0}")]
    StartTrap(String),
}
