//! An `Instance` contains all the runtime state used by execution of a wasm
//! module.

use crate::export::Export;
use crate::imports::Imports;
use crate::memory::LinearMemory;
use crate::mmap::Mmap;
use crate::signalhandlers::{wasmtime_init_eager, wasmtime_init_finish};
use crate::table::Table;
use crate::traphandlers::wasmtime_call;
use crate::vmcontext::{
    VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMFunctionImport, VMGlobalDefinition,
    VMGlobalImport, VMMemoryDefinition, VMMemoryImport, VMSharedSignatureIndex, VMTableDefinition,
    VMTableImport,
};
use core::slice;
use core::{mem, ptr};
use cranelift_entity::EntityRef;
use cranelift_entity::{BoxedSlice, PrimaryMap};
use cranelift_wasm::{
    DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex,
    GlobalIndex, GlobalInit, MemoryIndex, SignatureIndex, TableIndex,
};
use std::borrow::ToOwned;
use std::rc::Rc;
use std::string::String;
use wasmtime_environ::{DataInitializer, Module, TableElements, VMOffsets};

fn signature_id(
    vmctx: &VMContext,
    offsets: &VMOffsets,
    index: SignatureIndex,
) -> VMSharedSignatureIndex {
    #[allow(clippy::cast_ptr_alignment)]
    unsafe {
        let ptr = (vmctx as *const VMContext as *const u8)
            .add(cast::usize(offsets.vmctx_vmshared_signature_id(index)));
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
            .add(cast::usize(offsets.vmctx_vmfunction_import(index)));
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
            .add(cast::usize(offsets.vmctx_vmtable_import(index)));
        &*(ptr as *const VMTableImport)
    }
}

/// The actual contents of an instance.
///
/// `Instance` is just a handle containing a pointer to an `InstanceContents`,
/// which is specially allocated.
///
/// This is repr(C) to ensure that the vmctx field is last.
#[repr(C)]
pub(crate) struct InstanceContents {
    /// Offsets in the `vmctx` region.
    offsets: VMOffsets,

    /// WebAssembly linear memory data.
    memories: BoxedSlice<DefinedMemoryIndex, LinearMemory>,

    /// WebAssembly table data.
    tables: BoxedSlice<DefinedTableIndex, Table>,

    /// Pointers to functions in executable memory.
    finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,

    /// Context pointer used by compiled wasm code.
    vmctx: VMContext,
}

#[allow(clippy::cast_ptr_alignment)]
impl InstanceContents {
    /// Return the indexed `VMSharedSignatureIndex`.
    #[allow(dead_code)]
    fn signature_id(&self, index: SignatureIndex) -> VMSharedSignatureIndex {
        signature_id(&self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMSharedSignatureIndex`s.
    fn signature_ids_ptr(&mut self) -> *mut VMSharedSignatureIndex {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_signature_ids_begin()))
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
                .add(cast::usize(self.offsets.vmctx_imported_functions_begin()))
                as *mut VMFunctionImport
        }
    }

    /// Return the index `VMTableImport`.
    fn imported_table(&self, index: TableIndex) -> &VMTableImport {
        imported_table(&self.vmctx, &self.offsets, index)
    }

    /// Return a pointer to the `VMTableImports`s.
    fn imported_tables_ptr(&mut self) -> *mut VMTableImport {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_imported_tables_begin()))
                as *mut VMTableImport
        }
    }

    /// Return the indexed `VMMemoryImport`.
    fn imported_memory(&self, index: MemoryIndex) -> &VMMemoryImport {
        unsafe {
            let ptr = (&self.vmctx as *const VMContext as *const u8)
                .add(cast::usize(self.offsets.vmctx_vmmemory_import(index)));
            &*(ptr as *const VMMemoryImport)
        }
    }

    /// Return a pointer to the `VMMemoryImport`s.
    fn imported_memories_ptr(&mut self) -> *mut VMMemoryImport {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_imported_memories_begin()))
                as *mut VMMemoryImport
        }
    }

    /// Return the indexed `VMGlobalImport`.
    fn imported_global(&self, index: GlobalIndex) -> &VMGlobalImport {
        unsafe {
            let ptr = (&self.vmctx as *const VMContext as *const u8)
                .add(cast::usize(self.offsets.vmctx_vmglobal_import(index)));
            &*(ptr as *const VMGlobalImport)
        }
    }

    /// Return a pointer to the `VMGlobalImport`s.
    fn imported_globals_ptr(&mut self) -> *mut VMGlobalImport {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_imported_globals_begin()))
                as *mut VMGlobalImport
        }
    }

    /// Return the indexed `VMTableDefinition`.
    #[allow(dead_code)]
    fn table(&self, index: DefinedTableIndex) -> &VMTableDefinition {
        unsafe {
            let ptr = (&self.vmctx as *const VMContext as *const u8)
                .add(cast::usize(self.offsets.vmctx_vmtable_definition(index)));
            &*(ptr as *const VMTableDefinition)
        }
    }

    /// Return the indexed `VMTableDefinition`.
    fn table_mut(&mut self, index: DefinedTableIndex) -> &mut VMTableDefinition {
        unsafe {
            let ptr = (&self.vmctx as *const VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_vmtable_definition(index)));
            &mut *(ptr as *mut VMTableDefinition)
        }
    }

    /// Return a pointer to the `VMTableDefinition`s.
    fn tables_ptr(&mut self) -> *mut VMTableDefinition {
        unsafe {
            (&self.vmctx as *const VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_tables_begin()))
                as *mut VMTableDefinition
        }
    }

    /// Return the indexed `VMMemoryDefinition`.
    fn memory(&self, index: DefinedMemoryIndex) -> &VMMemoryDefinition {
        unsafe {
            let ptr = (&self.vmctx as *const VMContext as *const u8)
                .add(cast::usize(self.offsets.vmctx_vmmemory_definition(index)));
            &*(ptr as *const VMMemoryDefinition)
        }
    }

    /// Return the indexed `VMMemoryDefinition`.
    fn memory_mut(&mut self, index: DefinedMemoryIndex) -> &mut VMMemoryDefinition {
        unsafe {
            let ptr = (&self.vmctx as *const VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_vmmemory_definition(index)));
            &mut *(ptr as *mut VMMemoryDefinition)
        }
    }

    /// Return a pointer to the `VMMemoryDefinition`s.
    fn memories_ptr(&mut self) -> *mut VMMemoryDefinition {
        unsafe {
            (&self.vmctx as *const VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_memories_begin()))
                as *mut VMMemoryDefinition
        }
    }

    /// Return the indexed `VMGlobalDefinition`.
    #[allow(dead_code)]
    fn global(&self, index: DefinedGlobalIndex) -> &VMGlobalDefinition {
        unsafe {
            let ptr = (&self.vmctx as *const VMContext as *const u8)
                .add(cast::usize(self.offsets.vmctx_vmglobal_definition(index)));
            &*(ptr as *const VMGlobalDefinition)
        }
    }

    /// Return the indexed `VMGlobalDefinition`.
    fn global_mut(&mut self, index: DefinedGlobalIndex) -> &mut VMGlobalDefinition {
        unsafe {
            let ptr = (&self.vmctx as *const VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_vmglobal_definition(index)));
            &mut *(ptr as *mut VMGlobalDefinition)
        }
    }

    /// Return a pointer to the `VMGlobalDefinition`s.
    fn globals_ptr(&mut self) -> *mut VMGlobalDefinition {
        unsafe {
            (&mut self.vmctx as *mut VMContext as *mut u8)
                .add(cast::usize(self.offsets.vmctx_globals_begin()))
                as *mut VMGlobalDefinition
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

    /// Invoke the WebAssembly start function of the instance, if one is present.
    fn invoke_start_function(&mut self, module: &Module) -> Result<(), InstantiationError> {
        if let Some(start_index) = module.start_func {
            let (callee_address, callee_vmctx) = match module.defined_func_index(start_index) {
                Some(defined_start_index) => {
                    let body = *self
                        .finished_functions
                        .get(defined_start_index)
                        .expect("start function index is out of bounds");
                    (body, self.vmctx_mut() as *mut VMContext)
                }
                None => {
                    assert!(start_index.index() < module.imported_funcs.len());
                    let import = self.imported_function(start_index);
                    (import.body, import.vmctx)
                }
            };

            // Make the call.
            unsafe { wasmtime_call(callee_address, callee_vmctx) }
                .map_err(InstantiationError::StartTrap)?;
        }

        Ok(())
    }

    /// Return the offset from the vmctx pointer to its containing Instance.
    pub(crate) fn vmctx_offset() -> isize {
        offset_of!(Self, vmctx) as isize
    }

    /// Return the table index for the given `VMTableDefinition`.
    pub(crate) fn table_index(&self, table: &mut VMTableDefinition) -> DefinedTableIndex {
        let offsets = &self.offsets;
        let begin = unsafe {
            (&self.vmctx as *const VMContext as *mut u8)
                .add(cast::usize(offsets.vmctx_tables_begin()))
        } as *mut VMTableDefinition;
        let end: *mut VMTableDefinition = table;
        // TODO: Use `offset_from` once it stablizes.
        let index = DefinedTableIndex::new(
            (end as usize - begin as usize) / mem::size_of::<VMTableDefinition>(),
        );
        assert!(index.index() < self.tables.len());
        index
    }

    /// Return the memory index for the given `VMMemoryDefinition`.
    pub(crate) fn memory_index(&self, memory: &mut VMMemoryDefinition) -> DefinedMemoryIndex {
        let offsets = &self.offsets;
        let begin = unsafe {
            (&self.vmctx as *const VMContext as *mut u8)
                .add(cast::usize(offsets.vmctx_memories_begin()))
        } as *mut VMMemoryDefinition;
        let end: *mut VMMemoryDefinition = memory;
        // TODO: Use `offset_from` once it stablizes.
        let index = DefinedMemoryIndex::new(
            (end as usize - begin as usize) / mem::size_of::<VMMemoryDefinition>(),
        );
        assert!(index.index() < self.memories.len());
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
        let foreign_instance_contents = (&mut *import.vmctx).instance_contents();
        let foreign_memory = &mut *import.from;
        let foreign_index = foreign_instance_contents.memory_index(foreign_memory);

        foreign_instance_contents.memory_grow(foreign_index, delta)
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
        let foreign_instance_contents = (&mut *import.vmctx).instance_contents();
        let foreign_memory = &mut *import.from;
        let foreign_index = foreign_instance_contents.memory_index(foreign_memory);

        foreign_instance_contents.memory_size(foreign_index)
    }
}

/// A wrapper around an `Mmap` holding an `InstanceContents`.
struct MmapField {
    /// The allocated contents.
    mmap: Mmap,
}

#[allow(clippy::cast_ptr_alignment)]
impl MmapField {
    /// Return the contained contents.
    fn contents(&self) -> &InstanceContents {
        assert!(self.mmap.len() >= mem::size_of::<InstanceContents>());
        unsafe { &*(self.mmap.as_ptr() as *const InstanceContents) }
    }

    /// Return the contained contents.
    fn contents_mut(&mut self) -> &mut InstanceContents {
        assert!(self.mmap.len() >= mem::size_of::<InstanceContents>());
        unsafe { &mut *(self.mmap.as_mut_ptr() as *mut InstanceContents) }
    }
}

impl Drop for MmapField {
    fn drop(&mut self) {
        /// Drop the `InstanceContents`.
        assert!(self.mmap.len() >= mem::size_of::<InstanceContents>());
        mem::drop(mem::replace(self.contents_mut(), unsafe { mem::zeroed() }));
    }
}

/// An Instance of a WebAssembly module.
///
/// Note that compiled wasm code passes around raw pointers to `Instance`, so
/// this shouldn't be moved.
pub struct Instance {
    /// The `Module` this `Instance` was instantiated from.
    module: Rc<Module>,

    /// The `Mmap` containing the contents of the instance.
    mmap_field: MmapField,
}

impl Instance {
    /// Create a new `Instance`.
    pub fn new(
        module: Rc<Module>,
        finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
        imports: Imports,
        data_initializers: &[DataInitializer<'_>],
        vmshared_signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
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

        let offsets = VMOffsets {
            pointer_size: mem::size_of::<*const u8>() as u8,
            num_signature_ids: vmshared_signatures.len() as u64,
            num_imported_functions: imports.functions.len() as u64,
            num_imported_tables: imports.tables.len() as u64,
            num_imported_memories: imports.memories.len() as u64,
            num_imported_globals: imports.globals.len() as u64,
            num_defined_tables: tables.len() as u64,
            num_defined_memories: memories.len() as u64,
            num_defined_globals: vmctx_globals.len() as u64,
        };

        let mut contents_mmap = Mmap::with_size(
            mem::size_of::<InstanceContents>()
                .checked_add(cast::usize(offsets.size_of_vmctx()))
                .unwrap(),
        )
        .map_err(InstantiationError::Resource)?;

        let contents = {
            #[allow(clippy::cast_ptr_alignment)]
            let contents_ptr = contents_mmap.as_mut_ptr() as *mut InstanceContents;
            let contents = InstanceContents {
                offsets,
                memories,
                tables,
                finished_functions,
                vmctx: VMContext {},
            };
            unsafe {
                ptr::write(contents_ptr, contents);
                &mut *contents_ptr
            }
        };

        unsafe {
            ptr::copy(
                vmshared_signatures.values().as_slice().as_ptr(),
                contents.signature_ids_ptr() as *mut VMSharedSignatureIndex,
                vmshared_signatures.len(),
            );
            ptr::copy(
                imports.functions.values().as_slice().as_ptr(),
                contents.imported_functions_ptr() as *mut VMFunctionImport,
                imports.functions.len(),
            );
            ptr::copy(
                imports.tables.values().as_slice().as_ptr(),
                contents.imported_tables_ptr() as *mut VMTableImport,
                imports.tables.len(),
            );
            ptr::copy(
                imports.memories.values().as_slice().as_ptr(),
                contents.imported_memories_ptr() as *mut VMMemoryImport,
                imports.memories.len(),
            );
            ptr::copy(
                imports.globals.values().as_slice().as_ptr(),
                contents.imported_globals_ptr() as *mut VMGlobalImport,
                imports.globals.len(),
            );
            ptr::copy(
                vmctx_tables.values().as_slice().as_ptr(),
                contents.tables_ptr() as *mut VMTableDefinition,
                vmctx_tables.len(),
            );
            ptr::copy(
                vmctx_memories.values().as_slice().as_ptr(),
                contents.memories_ptr() as *mut VMMemoryDefinition,
                vmctx_memories.len(),
            );
            ptr::copy(
                vmctx_globals.values().as_slice().as_ptr(),
                contents.globals_ptr() as *mut VMGlobalDefinition,
                vmctx_globals.len(),
            );
        }

        // Check initializer bounds before initializing anything.
        check_table_init_bounds(&*module, contents)?;
        check_memory_init_bounds(&*module, contents, data_initializers)?;

        // Apply the initializers.
        initialize_tables(&*module, contents)?;
        initialize_memories(&*module, contents, data_initializers)?;
        initialize_globals(&*module, contents);

        // Rather than writing inline assembly to jump to the code region, we use the fact that
        // the Rust ABI for calling a function with no arguments and no return values matches the
        // one of the generated code. Thanks to this, we can transmute the code region into a
        // first-class Rust function and call it.
        // Ensure that our signal handlers are ready for action.
        // TODO: Move these calls out of `Instance`.
        wasmtime_init_eager();
        wasmtime_init_finish(contents.vmctx_mut());

        // The WebAssembly spec specifies that the start function is
        // invoked automatically at instantiation time.
        contents.invoke_start_function(&*module)?;

        Ok(Instance {
            module,
            mmap_field: MmapField {
                mmap: contents_mmap,
            },
        })
    }

    /// Return a reference to the vmctx used by compiled wasm code.
    pub fn vmctx(&self) -> &VMContext {
        self.mmap_field.contents().vmctx()
    }

    /// Return a raw pointer to the vmctx used by compiled wasm code.
    pub fn vmctx_ptr(&self) -> *const VMContext {
        self.mmap_field.contents().vmctx_ptr()
    }

    /// Return a mutable reference to the vmctx used by compiled wasm code.
    pub fn vmctx_mut(&mut self) -> &mut VMContext {
        self.mmap_field.contents_mut().vmctx_mut()
    }

    /// Return a mutable raw pointer to the vmctx used by compiled wasm code.
    pub fn vmctx_mut_ptr(&mut self) -> *mut VMContext {
        self.mmap_field.contents_mut().vmctx_mut_ptr()
    }

    /// Lookup an export with the given name.
    pub fn lookup(&mut self, field: &str) -> Option<Export> {
        let contents = self.mmap_field.contents_mut();
        if let Some(export) = self.module.exports.get(field) {
            Some(match export {
                wasmtime_environ::Export::Function(index) => {
                    let signature = self.module.signatures[self.module.functions[*index]].clone();
                    let (address, vmctx) =
                        if let Some(def_index) = self.module.defined_func_index(*index) {
                            (
                                contents.finished_functions[def_index],
                                &mut contents.vmctx as *mut VMContext,
                            )
                        } else {
                            let import = contents.imported_function(*index);
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
                            (
                                contents.table_mut(def_index) as *mut VMTableDefinition,
                                &mut contents.vmctx as *mut VMContext,
                            )
                        } else {
                            let import = contents.imported_table(*index);
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
                            (
                                contents.memory_mut(def_index) as *mut VMMemoryDefinition,
                                &mut contents.vmctx as *mut VMContext,
                            )
                        } else {
                            let import = contents.imported_memory(*index);
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
                        contents.global_mut(def_index)
                    } else {
                        contents.imported_global(*index).from
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
        let temporary_mut = &mut *(self as *const Self as *mut Self);
        temporary_mut.lookup(field)
    }
}

fn check_table_init_bounds(
    module: &Module,
    contents: &mut InstanceContents,
) -> Result<(), InstantiationError> {
    for init in &module.table_elements {
        let start = get_table_init_start(init, module, contents);
        let slice = get_table_slice(
            init,
            module,
            &mut contents.tables,
            &contents.vmctx,
            &contents.offsets,
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
fn get_memory_init_start(
    init: &DataInitializer<'_>,
    module: &Module,
    contents: &mut InstanceContents,
) -> usize {
    let mut start = init.location.offset;

    if let Some(base) = init.location.base {
        let global = if let Some(def_index) = module.defined_global_index(base) {
            contents.global_mut(def_index)
        } else {
            contents.imported_global(base).from
        };
        start += cast::usize(*unsafe { (*global).as_u32() });
    }

    start
}

/// Return a byte-slice view of a memory's data.
fn get_memory_slice<'contents>(
    init: &DataInitializer<'_>,
    module: &Module,
    contents: &'contents mut InstanceContents,
) -> &'contents mut [u8] {
    let memory = if let Some(defined_memory_index) =
        module.defined_memory_index(init.location.memory_index)
    {
        contents.memory(defined_memory_index)
    } else {
        let import = contents.imported_memory(init.location.memory_index);
        let foreign_contents = unsafe { (&mut *(import).vmctx).instance_contents() };
        let foreign_memory = unsafe { &mut *(import).from };
        let foreign_index = foreign_contents.memory_index(foreign_memory);
        foreign_contents.memory(foreign_index)
    };
    unsafe { slice::from_raw_parts_mut(memory.base, memory.current_length) }
}

fn check_memory_init_bounds(
    module: &Module,
    contents: &mut InstanceContents,
    data_initializers: &[DataInitializer<'_>],
) -> Result<(), InstantiationError> {
    for init in data_initializers {
        let start = get_memory_init_start(init, module, contents);
        let mem_slice = get_memory_slice(init, module, contents);

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
fn get_table_init_start(
    init: &TableElements,
    module: &Module,
    contents: &mut InstanceContents,
) -> usize {
    let mut start = init.offset;

    if let Some(base) = init.base {
        let global = if let Some(def_index) = module.defined_global_index(base) {
            contents.global_mut(def_index)
        } else {
            contents.imported_global(base).from
        };
        start += cast::usize(*unsafe { (*global).as_u32() });
    }

    start
}

/// Return a byte-slice view of a table's data.
fn get_table_slice<'contents>(
    init: &TableElements,
    module: &Module,
    tables: &'contents mut BoxedSlice<DefinedTableIndex, Table>,
    vmctx: &VMContext,
    offsets: &VMOffsets,
) -> &'contents mut [VMCallerCheckedAnyfunc] {
    if let Some(defined_table_index) = module.defined_table_index(init.table_index) {
        tables[defined_table_index].as_mut()
    } else {
        let import = imported_table(vmctx, offsets, init.table_index);
        let foreign_contents = unsafe { (&mut *(import).vmctx).instance_contents() };
        let foreign_table = unsafe { &mut *(import).from };
        let foreign_index = foreign_contents.table_index(foreign_table);
        foreign_contents.tables[foreign_index].as_mut()
    }
}

/// Initialize the table memory from the provided initializers.
fn initialize_tables(
    module: &Module,
    contents: &mut InstanceContents,
) -> Result<(), InstantiationError> {
    let vmctx: *mut VMContext = contents.vmctx_mut();
    for init in &module.table_elements {
        let start = get_table_init_start(init, module, contents);
        let slice = get_table_slice(
            init,
            module,
            &mut contents.tables,
            &contents.vmctx,
            &contents.offsets,
        );

        let subslice = &mut slice[start..start + init.elements.len()];
        for (i, func_idx) in init.elements.iter().enumerate() {
            let callee_sig = module.functions[*func_idx];
            let (callee_ptr, callee_vmctx) =
                if let Some(index) = module.defined_func_index(*func_idx) {
                    (contents.finished_functions[index], vmctx)
                } else {
                    let imported_func =
                        imported_function(&contents.vmctx, &contents.offsets, *func_idx);
                    (imported_func.body, imported_func.vmctx)
                };
            let type_index = signature_id(&contents.vmctx, &contents.offsets, callee_sig);
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
    module: &Module,
    contents: &mut InstanceContents,
    data_initializers: &[DataInitializer<'_>],
) -> Result<(), InstantiationError> {
    for init in data_initializers {
        let start = get_memory_init_start(init, module, contents);
        let mem_slice = get_memory_slice(init, module, contents);

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

fn initialize_globals(module: &Module, contents: &mut InstanceContents) {
    let num_imports = module.imported_globals.len();
    for (index, global) in module.globals.iter().skip(num_imports) {
        let def_index = module.defined_global_index(index).unwrap();
        let to: *mut VMGlobalDefinition = contents.global_mut(def_index);
        match global.initializer {
            GlobalInit::I32Const(x) => *unsafe { (*to).as_i32_mut() } = x,
            GlobalInit::I64Const(x) => *unsafe { (*to).as_i64_mut() } = x,
            GlobalInit::F32Const(x) => *unsafe { (*to).as_f32_bits_mut() } = x,
            GlobalInit::F64Const(x) => *unsafe { (*to).as_f64_bits_mut() } = x,
            GlobalInit::GetGlobal(x) => {
                let from = if let Some(def_x) = module.defined_global_index(x) {
                    contents.global_mut(def_x)
                } else {
                    contents.imported_global(x).from
                };
                unsafe { *to = *from };
            }
            GlobalInit::Import => panic!("locally-defined global initialized as import"),
        }
    }
}

/// An link error while instantiating a module.
#[derive(Fail, Debug)]
#[fail(display = "Link error: {}", _0)]
pub struct LinkError(pub String);

/// An error while instantiating a module.
#[derive(Fail, Debug)]
pub enum InstantiationError {
    /// Insufficient resources available for execution.
    #[fail(display = "Insufficient resources: {}", _0)]
    Resource(String),

    /// A wasm link error occured.
    #[fail(display = "{}", _0)]
    Link(LinkError),

    /// A compilation error occured.
    #[fail(display = "Trap occurred while invoking start function: {}", _0)]
    StartTrap(String),
}
