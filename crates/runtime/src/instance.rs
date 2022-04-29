//! An `Instance` contains all the runtime state used by execution of a
//! wasm module (except its callstack and register state). An
//! `InstanceHandle` is a reference-counting handle for an `Instance`.

use crate::export::Export;
use crate::externref::VMExternRefActivationsTable;
use crate::memory::{Memory, RuntimeMemoryCreator};
use crate::table::{Table, TableElement, TableElementType};
use crate::traphandlers::Trap;
use crate::vmcontext::{
    VMBuiltinFunctionsArray, VMCallerCheckedAnyfunc, VMContext, VMFunctionImport,
    VMGlobalDefinition, VMGlobalImport, VMMemoryDefinition, VMMemoryImport, VMRuntimeLimits,
    VMTableDefinition, VMTableImport,
};
use crate::{
    ExportFunction, ExportGlobal, ExportMemory, ExportTable, Imports, ModuleRuntimeInfo, Store,
};
use anyhow::Error;
use memoffset::offset_of;
use more_asserts::assert_lt;
use std::alloc::Layout;
use std::any::Any;
use std::convert::TryFrom;
use std::hash::Hash;
use std::ops::Range;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::{mem, ptr, slice};
use wasmtime_environ::{
    packed_option::ReservedValue, DataIndex, DefinedGlobalIndex, DefinedMemoryIndex,
    DefinedTableIndex, ElemIndex, EntityIndex, EntityRef, EntitySet, FuncIndex, GlobalIndex,
    GlobalInit, HostPtr, MemoryIndex, Module, PrimaryMap, SignatureIndex, TableIndex,
    TableInitialization, TrapCode, VMOffsets, WasmType,
};

mod allocator;

pub use allocator::*;

/// A type that roughly corresponds to a WebAssembly instance, but is also used
/// for host-defined objects.
///
/// This structure is is never allocated directly but is instead managed through
/// an `InstanceHandle`. This structure ends with a `VMContext` which has a
/// dynamic size corresponding to the `module` configured within. Memory
/// management of this structure is always externalized.
///
/// Instances here can correspond to actual instantiated modules, but it's also
/// used ubiquitously for host-defined objects. For example creating a
/// host-defined memory will have a `module` that looks like it exports a single
/// memory (and similar for other constructs).
///
/// This `Instance` type is used as a ubiquitous representation for WebAssembly
/// values, whether or not they were created on the host or through a module.
#[repr(C)] // ensure that the vmctx field is last.
pub(crate) struct Instance {
    /// The runtime info (corresponding to the "compiled module"
    /// abstraction in higher layers) that is retained and needed for
    /// lazy initialization. This provides access to the underlying
    /// Wasm module entities, the compiled JIT code, metadata about
    /// functions, lazy initialization state, etc.
    runtime_info: Arc<dyn ModuleRuntimeInfo>,

    /// Offsets in the `vmctx` region, precomputed from the `module` above.
    offsets: VMOffsets<HostPtr>,

    /// WebAssembly linear memory data.
    ///
    /// This is where all runtime information about defined linear memories in
    /// this module lives.
    memories: PrimaryMap<DefinedMemoryIndex, Memory>,

    /// WebAssembly table data.
    ///
    /// Like memories, this is only for defined tables in the module and
    /// contains all of their runtime state.
    tables: PrimaryMap<DefinedTableIndex, Table>,

    /// Stores the dropped passive element segments in this instantiation by index.
    /// If the index is present in the set, the segment has been dropped.
    dropped_elements: EntitySet<ElemIndex>,

    /// Stores the dropped passive data segments in this instantiation by index.
    /// If the index is present in the set, the segment has been dropped.
    dropped_data: EntitySet<DataIndex>,

    /// Hosts can store arbitrary per-instance information here.
    ///
    /// Most of the time from Wasmtime this is `Box::new(())`, a noop
    /// allocation, but some host-defined objects will store their state here.
    host_state: Box<dyn Any + Send + Sync>,

    /// Additional context used by compiled wasm code. This field is last, and
    /// represents a dynamically-sized array that extends beyond the nominal
    /// end of the struct (similar to a flexible array member).
    vmctx: VMContext,
}

#[allow(clippy::cast_ptr_alignment)]
impl Instance {
    /// Create an instance at the given memory address.
    ///
    /// It is assumed the memory was properly aligned and the
    /// allocation was `alloc_size` in bytes.
    unsafe fn new_at(
        ptr: *mut Instance,
        alloc_size: usize,
        offsets: VMOffsets<HostPtr>,
        req: InstanceAllocationRequest,
        memories: PrimaryMap<DefinedMemoryIndex, Memory>,
        tables: PrimaryMap<DefinedTableIndex, Table>,
    ) {
        // The allocation must be *at least* the size required of `Instance`.
        assert!(alloc_size >= Self::alloc_layout(&offsets).size());

        let module = req.runtime_info.module();
        let dropped_elements = EntitySet::with_capacity(module.passive_elements.len());
        let dropped_data = EntitySet::with_capacity(module.passive_data_map.len());

        ptr::write(
            ptr,
            Instance {
                runtime_info: req.runtime_info.clone(),
                offsets,
                memories,
                tables,
                dropped_elements,
                dropped_data,
                host_state: req.host_state,
                vmctx: VMContext {
                    _marker: std::marker::PhantomPinned,
                },
            },
        );

        (*ptr).initialize_vmctx(module, req.store, req.imports);
    }

    /// Helper function to access various locations offset from our `*mut
    /// VMContext` object.
    unsafe fn vmctx_plus_offset<T>(&self, offset: u32) -> *mut T {
        (self.vmctx_ptr().cast::<u8>())
            .add(usize::try_from(offset).unwrap())
            .cast()
    }

    pub(crate) fn module(&self) -> &Arc<Module> {
        self.runtime_info.module()
    }

    /// Return the indexed `VMFunctionImport`.
    fn imported_function(&self, index: FuncIndex) -> &VMFunctionImport {
        unsafe { &*self.vmctx_plus_offset(self.offsets.vmctx_vmfunction_import(index)) }
    }

    /// Return the index `VMTableImport`.
    fn imported_table(&self, index: TableIndex) -> &VMTableImport {
        unsafe { &*self.vmctx_plus_offset(self.offsets.vmctx_vmtable_import(index)) }
    }

    /// Return the indexed `VMMemoryImport`.
    fn imported_memory(&self, index: MemoryIndex) -> &VMMemoryImport {
        unsafe { &*self.vmctx_plus_offset(self.offsets.vmctx_vmmemory_import(index)) }
    }

    /// Return the indexed `VMGlobalImport`.
    fn imported_global(&self, index: GlobalIndex) -> &VMGlobalImport {
        unsafe { &*self.vmctx_plus_offset(self.offsets.vmctx_vmglobal_import(index)) }
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
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_vmtable_definition(index)) }
    }

    /// Get a locally defined or imported memory.
    pub(crate) fn get_memory(&self, index: MemoryIndex) -> VMMemoryDefinition {
        if let Some(defined_index) = self.module().defined_memory_index(index) {
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
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_vmmemory_definition(index)) }
    }

    /// Return the indexed `VMGlobalDefinition`.
    fn global(&self, index: DefinedGlobalIndex) -> &VMGlobalDefinition {
        unsafe { &*self.global_ptr(index) }
    }

    /// Return the indexed `VMGlobalDefinition`.
    fn global_ptr(&self, index: DefinedGlobalIndex) -> *mut VMGlobalDefinition {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_vmglobal_definition(index)) }
    }

    /// Get a raw pointer to the global at the given index regardless whether it
    /// is defined locally or imported from another module.
    ///
    /// Panics if the index is out of bound or is the reserved value.
    pub(crate) fn defined_or_imported_global_ptr(
        &self,
        index: GlobalIndex,
    ) -> *mut VMGlobalDefinition {
        if let Some(index) = self.module().defined_global_index(index) {
            self.global_ptr(index)
        } else {
            self.imported_global(index).from
        }
    }

    /// Return a pointer to the interrupts structure
    pub fn runtime_limits(&self) -> *mut *const VMRuntimeLimits {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_runtime_limits()) }
    }

    /// Return a pointer to the global epoch counter used by this instance.
    pub fn epoch_ptr(&self) -> *mut *const AtomicU64 {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_epoch_ptr()) }
    }

    /// Return a pointer to the `VMExternRefActivationsTable`.
    pub fn externref_activations_table(&self) -> *mut *mut VMExternRefActivationsTable {
        unsafe { self.vmctx_plus_offset(self.offsets.vmctx_externref_activations_table()) }
    }

    /// Gets a pointer to this instance's `Store` which was originally
    /// configured on creation.
    ///
    /// # Panics
    ///
    /// This will panic if the originally configured store was `None`. That can
    /// happen for host functions so host functions can't be queried what their
    /// original `Store` was since it's just retained as null (since host
    /// functions are shared amongst threads and don't all share the same
    /// store).
    #[inline]
    pub fn store(&self) -> *mut dyn Store {
        let ptr = unsafe { *self.vmctx_plus_offset::<*mut dyn Store>(self.offsets.vmctx_store()) };
        assert!(!ptr.is_null());
        ptr
    }

    pub unsafe fn set_store(&mut self, store: *mut dyn Store) {
        *self.vmctx_plus_offset(self.offsets.vmctx_store()) = store;
    }

    /// Return a reference to the vmctx used by compiled wasm code.
    #[inline]
    pub fn vmctx(&self) -> &VMContext {
        &self.vmctx
    }

    /// Return a raw pointer to the vmctx used by compiled wasm code.
    #[inline]
    pub fn vmctx_ptr(&self) -> *mut VMContext {
        self.vmctx() as *const VMContext as *mut VMContext
    }

    fn get_exported_func(&mut self, index: FuncIndex) -> ExportFunction {
        let anyfunc = self.get_caller_checked_anyfunc(index).unwrap();
        let anyfunc = NonNull::new(anyfunc as *const VMCallerCheckedAnyfunc as *mut _).unwrap();
        ExportFunction { anyfunc }
    }

    fn get_exported_table(&mut self, index: TableIndex) -> ExportTable {
        let (definition, vmctx) = if let Some(def_index) = self.module().defined_table_index(index)
        {
            (self.table_ptr(def_index), self.vmctx_ptr())
        } else {
            let import = self.imported_table(index);
            (import.from, import.vmctx)
        };
        ExportTable {
            definition,
            vmctx,
            table: self.module().table_plans[index].clone(),
        }
    }

    fn get_exported_memory(&mut self, index: MemoryIndex) -> ExportMemory {
        let (definition, vmctx) = if let Some(def_index) = self.module().defined_memory_index(index)
        {
            (self.memory_ptr(def_index), self.vmctx_ptr())
        } else {
            let import = self.imported_memory(index);
            (import.from, import.vmctx)
        };
        ExportMemory {
            definition,
            vmctx,
            memory: self.module().memory_plans[index].clone(),
        }
    }

    fn get_exported_global(&mut self, index: GlobalIndex) -> ExportGlobal {
        ExportGlobal {
            definition: if let Some(def_index) = self.module().defined_global_index(index) {
                self.global_ptr(def_index)
            } else {
                self.imported_global(index).from
            },
            vmctx: self.vmctx_ptr(),
            global: self.module().globals[index],
        }
    }

    /// Return an iterator over the exports of this instance.
    ///
    /// Specifically, it provides access to the key-value pairs, where the keys
    /// are export names, and the values are export declarations which can be
    /// resolved `lookup_by_declaration`.
    pub fn exports(&self) -> indexmap::map::Iter<String, EntityIndex> {
        self.module().exports.iter()
    }

    /// Return a reference to the custom state attached to this instance.
    #[inline]
    pub fn host_state(&self) -> &dyn Any {
        &*self.host_state
    }

    /// Return the offset from the vmctx pointer to its containing Instance.
    #[inline]
    pub(crate) fn vmctx_offset() -> isize {
        offset_of!(Self, vmctx) as isize
    }

    /// Return the table index for the given `VMTableDefinition`.
    unsafe fn table_index(&self, table: &VMTableDefinition) -> DefinedTableIndex {
        let index = DefinedTableIndex::new(
            usize::try_from(
                (table as *const VMTableDefinition)
                    .offset_from(self.table_ptr(DefinedTableIndex::new(0))),
            )
            .unwrap(),
        );
        assert_lt!(index.index(), self.tables.len());
        index
    }

    /// Return the memory index for the given `VMMemoryDefinition`.
    unsafe fn memory_index(&self, memory: &VMMemoryDefinition) -> DefinedMemoryIndex {
        let index = DefinedMemoryIndex::new(
            usize::try_from(
                (memory as *const VMMemoryDefinition)
                    .offset_from(self.memory_ptr(DefinedMemoryIndex::new(0))),
            )
            .unwrap(),
        );
        assert_lt!(index.index(), self.memories.len());
        index
    }

    /// Grow memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages. Returns `Some` with the old size in bytes if growth was
    /// successful.
    pub(crate) fn memory_grow(
        &mut self,
        index: MemoryIndex,
        delta: u64,
    ) -> Result<Option<usize>, Error> {
        let (idx, instance) = if let Some(idx) = self.module().defined_memory_index(index) {
            (idx, self)
        } else {
            let import = self.imported_memory(index);
            unsafe {
                let foreign_instance = (*import.vmctx).instance_mut();
                let foreign_memory_def = &*import.from;
                let foreign_memory_index = foreign_instance.memory_index(foreign_memory_def);
                (foreign_memory_index, foreign_instance)
            }
        };
        let store = unsafe { &mut *instance.store() };
        let memory = &mut instance.memories[idx];

        let result = unsafe { memory.grow(delta, store) };
        let vmmemory = memory.vmmemory();

        // Update the state used by wasm code in case the base pointer and/or
        // the length changed.
        instance.set_memory(idx, vmmemory);

        result
    }

    pub(crate) fn table_element_type(&mut self, table_index: TableIndex) -> TableElementType {
        unsafe { (*self.get_table(table_index)).element_type() }
    }

    /// Grow table by the specified amount of elements, filling them with
    /// `init_value`.
    ///
    /// Returns `None` if table can't be grown by the specified amount of
    /// elements, or if `init_value` is the wrong type of table element.
    pub(crate) fn table_grow(
        &mut self,
        table_index: TableIndex,
        delta: u32,
        init_value: TableElement,
    ) -> Result<Option<u32>, Error> {
        let (defined_table_index, instance) =
            self.get_defined_table_index_and_instance(table_index);
        instance.defined_table_grow(defined_table_index, delta, init_value)
    }

    fn defined_table_grow(
        &mut self,
        table_index: DefinedTableIndex,
        delta: u32,
        init_value: TableElement,
    ) -> Result<Option<u32>, Error> {
        let store = unsafe { &mut *self.store() };
        let table = self
            .tables
            .get_mut(table_index)
            .unwrap_or_else(|| panic!("no table for index {}", table_index.index()));

        let result = unsafe { table.grow(delta, init_value, store) };

        // Keep the `VMContext` pointers used by compiled Wasm code up to
        // date.
        let element = self.tables[table_index].vmtable();
        self.set_table(table_index, element);

        result
    }

    fn alloc_layout(offsets: &VMOffsets<HostPtr>) -> Layout {
        let size = mem::size_of::<Self>()
            .checked_add(usize::try_from(offsets.size_of_vmctx()).unwrap())
            .unwrap();
        let align = mem::align_of::<Self>();
        Layout::from_size_align(size, align).unwrap()
    }

    /// Construct a new VMCallerCheckedAnyfunc for the given function
    /// (imported or defined in this module) and store into the given
    /// location. Used during lazy initialization.
    ///
    /// Note that our current lazy-init scheme actually calls this every
    /// time the anyfunc pointer is fetched; this turns out to be better
    /// than tracking state related to whether it's been initialized
    /// before, because resetting that state on (re)instantiation is
    /// very expensive if there are many anyfuncs.
    fn construct_anyfunc(
        &mut self,
        index: FuncIndex,
        sig: SignatureIndex,
        into: *mut VMCallerCheckedAnyfunc,
    ) {
        let type_index = self.runtime_info.signature(sig);

        let (func_ptr, vmctx) = if let Some(def_index) = self.module().defined_func_index(index) {
            (
                (self.runtime_info.image_base()
                    + self.runtime_info.function_info(def_index).start as usize)
                    as *mut _,
                self.vmctx_ptr(),
            )
        } else {
            let import = self.imported_function(index);
            (import.body.as_ptr(), import.vmctx)
        };

        // Safety: we have a `&mut self`, so we have exclusive access
        // to this Instance.
        unsafe {
            *into = VMCallerCheckedAnyfunc {
                vmctx,
                type_index,
                func_ptr: NonNull::new(func_ptr).expect("Non-null function pointer"),
            };
        }
    }

    /// Get a `&VMCallerCheckedAnyfunc` for the given `FuncIndex`.
    ///
    /// Returns `None` if the index is the reserved index value.
    ///
    /// The returned reference is a stable reference that won't be moved and can
    /// be passed into JIT code.
    pub(crate) fn get_caller_checked_anyfunc(
        &mut self,
        index: FuncIndex,
    ) -> Option<*mut VMCallerCheckedAnyfunc> {
        if index == FuncIndex::reserved_value() {
            return None;
        }

        // Safety: we have a `&mut self`, so we have exclusive access
        // to this Instance.
        unsafe {
            // For now, we eagerly initialize an anyfunc struct in-place
            // whenever asked for a reference to it. This is mostly
            // fine, because in practice each anyfunc is unlikely to be
            // requested more than a few times: once-ish for funcref
            // tables used for call_indirect (the usual compilation
            // strategy places each function in the table at most once),
            // and once or a few times when fetching exports via API.
            // Note that for any case driven by table accesses, the lazy
            // table init behaves like a higher-level cache layer that
            // protects this initialization from happening multiple
            // times, via that particular table at least.
            //
            // When `ref.func` becomes more commonly used or if we
            // otherwise see a use-case where this becomes a hotpath,
            // we can reconsider by using some state to track
            // "uninitialized" explicitly, for example by zeroing the
            // anyfuncs (perhaps together with other
            // zeroed-at-instantiate-time state) or using a separate
            // is-initialized bitmap.
            //
            // We arrived at this design because zeroing memory is
            // expensive, so it's better for instantiation performance
            // if we don't have to track "is-initialized" state at
            // all!
            let func = &self.module().functions[index];
            let sig = func.signature;
            let anyfunc: *mut VMCallerCheckedAnyfunc = self
                .vmctx_plus_offset::<VMCallerCheckedAnyfunc>(
                    self.offsets.vmctx_anyfunc(func.anyfunc),
                );
            self.construct_anyfunc(index, sig, anyfunc);

            Some(anyfunc)
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
        &mut self,
        table_index: TableIndex,
        elem_index: ElemIndex,
        dst: u32,
        src: u32,
        len: u32,
    ) -> Result<(), Trap> {
        // TODO: this `clone()` shouldn't be necessary but is used for now to
        // inform `rustc` that the lifetime of the elements here are
        // disconnected from the lifetime of `self`.
        let module = self.module().clone();

        let elements = match module.passive_elements_map.get(&elem_index) {
            Some(index) if !self.dropped_elements.contains(elem_index) => {
                module.passive_elements[*index].as_ref()
            }
            _ => &[],
        };
        self.table_init_segment(table_index, elements, dst, src, len)
    }

    pub(crate) fn table_init_segment(
        &mut self,
        table_index: TableIndex,
        elements: &[FuncIndex],
        dst: u32,
        src: u32,
        len: u32,
    ) -> Result<(), Trap> {
        // https://webassembly.github.io/bulk-memory-operations/core/exec/instructions.html#exec-table-init

        let table = unsafe { &mut *self.get_table(table_index) };

        let elements = match elements
            .get(usize::try_from(src).unwrap()..)
            .and_then(|s| s.get(..usize::try_from(len).unwrap()))
        {
            Some(elements) => elements,
            None => return Err(Trap::wasm(TrapCode::TableOutOfBounds)),
        };

        match table.element_type() {
            TableElementType::Func => {
                table.init_funcs(
                    dst,
                    elements.iter().map(|idx| {
                        self.get_caller_checked_anyfunc(*idx)
                            .unwrap_or(std::ptr::null_mut())
                    }),
                )?;
            }

            TableElementType::Extern => {
                debug_assert!(elements.iter().all(|e| *e == FuncIndex::reserved_value()));
                table.fill(dst, TableElement::ExternRef(None), len)?;
            }
        }
        Ok(())
    }

    /// Drop an element.
    pub(crate) fn elem_drop(&mut self, elem_index: ElemIndex) {
        // https://webassembly.github.io/reference-types/core/exec/instructions.html#exec-elem-drop

        self.dropped_elements.insert(elem_index);

        // Note that we don't check that we actually removed a segment because
        // dropping a non-passive segment is a no-op (not a trap).
    }

    /// Get a locally-defined memory.
    pub(crate) fn get_defined_memory(&mut self, index: DefinedMemoryIndex) -> *mut Memory {
        ptr::addr_of_mut!(self.memories[index])
    }

    /// Do a `memory.copy`
    ///
    /// # Errors
    ///
    /// Returns a `Trap` error when the source or destination ranges are out of
    /// bounds.
    pub(crate) fn memory_copy(
        &mut self,
        dst_index: MemoryIndex,
        dst: u64,
        src_index: MemoryIndex,
        src: u64,
        len: u64,
    ) -> Result<(), Trap> {
        // https://webassembly.github.io/reference-types/core/exec/instructions.html#exec-memory-copy

        let src_mem = self.get_memory(src_index);
        let dst_mem = self.get_memory(dst_index);

        let src = self.validate_inbounds(src_mem.current_length, src, len)?;
        let dst = self.validate_inbounds(dst_mem.current_length, dst, len)?;

        // Bounds and casts are checked above, by this point we know that
        // everything is safe.
        unsafe {
            let dst = dst_mem.base.add(dst);
            let src = src_mem.base.add(src);
            ptr::copy(src, dst, len as usize);
        }

        Ok(())
    }

    fn validate_inbounds(&self, max: usize, ptr: u64, len: u64) -> Result<usize, Trap> {
        let oob = || Trap::wasm(TrapCode::HeapOutOfBounds);
        let end = ptr
            .checked_add(len)
            .and_then(|i| usize::try_from(i).ok())
            .ok_or_else(oob)?;
        if end > max {
            Err(oob())
        } else {
            Ok(ptr as usize)
        }
    }

    /// Perform the `memory.fill` operation on a locally defined memory.
    ///
    /// # Errors
    ///
    /// Returns a `Trap` error if the memory range is out of bounds.
    pub(crate) fn memory_fill(
        &mut self,
        memory_index: MemoryIndex,
        dst: u64,
        val: u8,
        len: u64,
    ) -> Result<(), Trap> {
        let memory = self.get_memory(memory_index);
        let dst = self.validate_inbounds(memory.current_length, dst, len)?;

        // Bounds and casts are checked above, by this point we know that
        // everything is safe.
        unsafe {
            let dst = memory.base.add(dst);
            ptr::write_bytes(dst, val, len as usize);
        }

        Ok(())
    }

    /// Performs the `memory.init` operation.
    ///
    /// # Errors
    ///
    /// Returns a `Trap` error if the destination range is out of this module's
    /// memory's bounds or if the source range is outside the data segment's
    /// bounds.
    pub(crate) fn memory_init(
        &mut self,
        memory_index: MemoryIndex,
        data_index: DataIndex,
        dst: u64,
        src: u32,
        len: u32,
    ) -> Result<(), Trap> {
        let range = match self.module().passive_data_map.get(&data_index).cloned() {
            Some(range) if !self.dropped_data.contains(data_index) => range,
            _ => 0..0,
        };
        self.memory_init_segment(memory_index, range, dst, src, len)
    }

    pub(crate) fn wasm_data(&self, range: Range<u32>) -> &[u8] {
        &self.runtime_info.wasm_data()[range.start as usize..range.end as usize]
    }

    pub(crate) fn memory_init_segment(
        &mut self,
        memory_index: MemoryIndex,
        range: Range<u32>,
        dst: u64,
        src: u32,
        len: u32,
    ) -> Result<(), Trap> {
        // https://webassembly.github.io/bulk-memory-operations/core/exec/instructions.html#exec-memory-init

        let memory = self.get_memory(memory_index);
        let data = self.wasm_data(range);
        let dst = self.validate_inbounds(memory.current_length, dst, len.into())?;
        let src = self.validate_inbounds(data.len(), src.into(), len.into())?;
        let len = len as usize;

        let src_slice = &data[src..(src + len)];

        unsafe {
            let dst_start = memory.base.add(dst);
            let dst_slice = slice::from_raw_parts_mut(dst_start, len);
            dst_slice.copy_from_slice(src_slice);
        }

        Ok(())
    }

    /// Drop the given data segment, truncating its length to zero.
    pub(crate) fn data_drop(&mut self, data_index: DataIndex) {
        self.dropped_data.insert(data_index);

        // Note that we don't check that we actually removed a segment because
        // dropping a non-passive segment is a no-op (not a trap).
    }

    /// Get a table by index regardless of whether it is locally-defined
    /// or an imported, foreign table. Ensure that the given range of
    /// elements in the table is lazily initialized.  We define this
    /// operation all-in-one for safety, to ensure the lazy-init
    /// happens.
    ///
    /// Takes an `Iterator` for the index-range to lazy-initialize,
    /// for flexibility. This can be a range, single item, or empty
    /// sequence, for example. The iterator should return indices in
    /// increasing order, so that the break-at-out-of-bounds behavior
    /// works correctly.
    pub(crate) fn get_table_with_lazy_init(
        &mut self,
        table_index: TableIndex,
        range: impl Iterator<Item = u32>,
    ) -> *mut Table {
        let (idx, instance) = self.get_defined_table_index_and_instance(table_index);
        let elt_ty = instance.tables[idx].element_type();

        if elt_ty == TableElementType::Func {
            for i in range {
                let value = match instance.tables[idx].get(i) {
                    Some(value) => value,
                    None => {
                        // Out-of-bounds; caller will handle by likely
                        // throwing a trap. No work to do to lazy-init
                        // beyond the end.
                        break;
                    }
                };
                if value.is_uninit() {
                    let table_init = match &instance.module().table_initialization {
                        // We unfortunately can't borrow `tables`
                        // outside the loop because we need to call
                        // `get_caller_checked_anyfunc` (a `&mut`
                        // method) below; so unwrap it dynamically
                        // here.
                        TableInitialization::FuncTable { tables, .. } => tables,
                        _ => break,
                    }
                    .get(table_index);

                    // The TableInitialization::FuncTable elements table may
                    // be smaller than the current size of the table: it
                    // always matches the initial table size, if present. We
                    // want to iterate up through the end of the accessed
                    // index range so that we set an "initialized null" even
                    // if there is no initializer. We do a checked `get()` on
                    // the initializer table below and unwrap to a null if
                    // we're past its end.
                    let func_index =
                        table_init.and_then(|indices| indices.get(i as usize).cloned());
                    let anyfunc = func_index
                        .and_then(|func_index| instance.get_caller_checked_anyfunc(func_index))
                        .unwrap_or(std::ptr::null_mut());

                    let value = TableElement::FuncRef(anyfunc);

                    instance.tables[idx]
                        .set(i, value)
                        .expect("Table type should match and index should be in-bounds");
                }
            }
        }

        ptr::addr_of_mut!(instance.tables[idx])
    }

    /// Get a table by index regardless of whether it is locally-defined or an
    /// imported, foreign table.
    pub(crate) fn get_table(&mut self, table_index: TableIndex) -> *mut Table {
        let (idx, instance) = self.get_defined_table_index_and_instance(table_index);
        ptr::addr_of_mut!(instance.tables[idx])
    }

    /// Get a locally-defined table.
    pub(crate) fn get_defined_table(&mut self, index: DefinedTableIndex) -> *mut Table {
        ptr::addr_of_mut!(self.tables[index])
    }

    pub(crate) fn get_defined_table_index_and_instance(
        &mut self,
        index: TableIndex,
    ) -> (DefinedTableIndex, &mut Instance) {
        if let Some(defined_table_index) = self.module().defined_table_index(index) {
            (defined_table_index, self)
        } else {
            let import = self.imported_table(index);
            unsafe {
                let foreign_instance = (*import.vmctx).instance_mut();
                let foreign_table_def = &*import.from;
                let foreign_table_index = foreign_instance.table_index(foreign_table_def);
                (foreign_table_index, foreign_instance)
            }
        }
    }

    /// Initialize the VMContext data associated with this Instance.
    ///
    /// The `VMContext` memory is assumed to be uninitialized; any field
    /// that we need in a certain state will be explicitly written by this
    /// function.
    unsafe fn initialize_vmctx(&mut self, module: &Module, store: StorePtr, imports: Imports) {
        assert!(std::ptr::eq(module, self.module().as_ref()));

        if let Some(store) = store.as_raw() {
            *self.runtime_limits() = (*store).vmruntime_limits();
            *self.epoch_ptr() = (*store).epoch_ptr();
            *self.externref_activations_table() = (*store).externref_activations_table().0;
            self.set_store(store);
        }

        // Initialize shared signatures
        let signatures = self.runtime_info.signature_ids();
        *self.vmctx_plus_offset(self.offsets.vmctx_signature_ids_array()) = signatures.as_ptr();

        // Initialize the built-in functions
        *self.vmctx_plus_offset(self.offsets.vmctx_builtin_functions()) =
            &VMBuiltinFunctionsArray::INIT;

        // Initialize the imports
        debug_assert_eq!(imports.functions.len(), module.num_imported_funcs);
        ptr::copy_nonoverlapping(
            imports.functions.as_ptr(),
            self.vmctx_plus_offset(self.offsets.vmctx_imported_functions_begin()),
            imports.functions.len(),
        );
        debug_assert_eq!(imports.tables.len(), module.num_imported_tables);
        ptr::copy_nonoverlapping(
            imports.tables.as_ptr(),
            self.vmctx_plus_offset(self.offsets.vmctx_imported_tables_begin()),
            imports.tables.len(),
        );
        debug_assert_eq!(imports.memories.len(), module.num_imported_memories);
        ptr::copy_nonoverlapping(
            imports.memories.as_ptr(),
            self.vmctx_plus_offset(self.offsets.vmctx_imported_memories_begin()),
            imports.memories.len(),
        );
        debug_assert_eq!(imports.globals.len(), module.num_imported_globals);
        ptr::copy_nonoverlapping(
            imports.globals.as_ptr(),
            self.vmctx_plus_offset(self.offsets.vmctx_imported_globals_begin()),
            imports.globals.len(),
        );

        // N.B.: there is no need to initialize the anyfuncs array because
        // we eagerly construct each element in it whenever asked for a
        // reference to that element. In other words, there is no state
        // needed to track the lazy-init, so we don't need to initialize
        // any state now.

        // Initialize the defined tables
        let mut ptr = self.vmctx_plus_offset(self.offsets.vmctx_tables_begin());
        for i in 0..module.table_plans.len() - module.num_imported_tables {
            ptr::write(ptr, self.tables[DefinedTableIndex::new(i)].vmtable());
            ptr = ptr.add(1);
        }

        // Initialize the defined memories
        let mut ptr = self.vmctx_plus_offset(self.offsets.vmctx_memories_begin());
        for i in 0..module.memory_plans.len() - module.num_imported_memories {
            ptr::write(ptr, self.memories[DefinedMemoryIndex::new(i)].vmmemory());
            ptr = ptr.add(1);
        }

        // Initialize the defined globals
        self.initialize_vmctx_globals(module);
    }

    unsafe fn initialize_vmctx_globals(&mut self, module: &Module) {
        let num_imports = module.num_imported_globals;
        for (index, global) in module.globals.iter().skip(num_imports) {
            let def_index = module.defined_global_index(index).unwrap();
            let to = self.global_ptr(def_index);

            // Initialize the global before writing to it
            ptr::write(to, VMGlobalDefinition::new());

            match global.initializer {
                GlobalInit::I32Const(x) => *(*to).as_i32_mut() = x,
                GlobalInit::I64Const(x) => *(*to).as_i64_mut() = x,
                GlobalInit::F32Const(x) => *(*to).as_f32_bits_mut() = x,
                GlobalInit::F64Const(x) => *(*to).as_f64_bits_mut() = x,
                GlobalInit::V128Const(x) => *(*to).as_u128_mut() = x,
                GlobalInit::GetGlobal(x) => {
                    let from = if let Some(def_x) = module.defined_global_index(x) {
                        self.global(def_x)
                    } else {
                        &*self.imported_global(x).from
                    };
                    // Globals of type `externref` need to manage the reference
                    // count as values move between globals, everything else is just
                    // copy-able bits.
                    match global.wasm_ty {
                        WasmType::ExternRef => {
                            *(*to).as_externref_mut() = from.as_externref().clone()
                        }
                        _ => ptr::copy_nonoverlapping(from, to, 1),
                    }
                }
                GlobalInit::RefFunc(f) => {
                    *(*to).as_anyfunc_mut() = self.get_caller_checked_anyfunc(f).unwrap()
                        as *const VMCallerCheckedAnyfunc;
                }
                GlobalInit::RefNullConst => match global.wasm_ty {
                    // `VMGlobalDefinition::new()` already zeroed out the bits
                    WasmType::FuncRef => {}
                    WasmType::ExternRef => {}
                    ty => panic!("unsupported reference type for global: {:?}", ty),
                },
                GlobalInit::Import => panic!("locally-defined global initialized as import"),
            }
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // Drop any defined globals
        for (idx, global) in self.module().globals.iter() {
            let idx = match self.module().defined_global_index(idx) {
                Some(idx) => idx,
                None => continue,
            };
            match global.wasm_ty {
                // For now only externref globals need to get destroyed
                WasmType::ExternRef => {}
                _ => continue,
            }
            unsafe {
                drop((*self.global_ptr(idx)).as_externref_mut().take());
            }
        }
    }
}

/// A handle holding an `Instance` of a WebAssembly module.
#[derive(Hash, PartialEq, Eq)]
pub struct InstanceHandle {
    instance: *mut Instance,
}

// These are only valid if the `Instance` type is send/sync, hence the
// assertion below.
unsafe impl Send for InstanceHandle {}
unsafe impl Sync for InstanceHandle {}

fn _assert_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<Instance>();
}

impl InstanceHandle {
    /// Create a new `InstanceHandle` pointing at the instance
    /// pointed to by the given `VMContext` pointer.
    ///
    /// # Safety
    /// This is unsafe because it doesn't work on just any `VMContext`, it must
    /// be a `VMContext` allocated as part of an `Instance`.
    #[inline]
    pub unsafe fn from_vmctx(vmctx: *mut VMContext) -> Self {
        let instance = (&mut *vmctx).instance();
        Self {
            instance: instance as *const Instance as *mut Instance,
        }
    }

    /// Return a reference to the vmctx used by compiled wasm code.
    pub fn vmctx(&self) -> &VMContext {
        self.instance().vmctx()
    }

    /// Return a raw pointer to the vmctx used by compiled wasm code.
    #[inline]
    pub fn vmctx_ptr(&self) -> *mut VMContext {
        self.instance().vmctx_ptr()
    }

    /// Return a reference to a module.
    pub fn module(&self) -> &Arc<Module> {
        self.instance().module()
    }

    /// Lookup a function by index.
    pub fn get_exported_func(&mut self, export: FuncIndex) -> ExportFunction {
        self.instance_mut().get_exported_func(export)
    }

    /// Lookup a global by index.
    pub fn get_exported_global(&mut self, export: GlobalIndex) -> ExportGlobal {
        self.instance_mut().get_exported_global(export)
    }

    /// Lookup a memory by index.
    pub fn get_exported_memory(&mut self, export: MemoryIndex) -> ExportMemory {
        self.instance_mut().get_exported_memory(export)
    }

    /// Lookup a table by index.
    pub fn get_exported_table(&mut self, export: TableIndex) -> ExportTable {
        self.instance_mut().get_exported_table(export)
    }

    /// Lookup an item with the given index.
    pub fn get_export_by_index(&mut self, export: EntityIndex) -> Export {
        match export {
            EntityIndex::Function(i) => Export::Function(self.get_exported_func(i)),
            EntityIndex::Global(i) => Export::Global(self.get_exported_global(i)),
            EntityIndex::Table(i) => Export::Table(self.get_exported_table(i)),
            EntityIndex::Memory(i) => Export::Memory(self.get_exported_memory(i)),
        }
    }

    /// Return an iterator over the exports of this instance.
    ///
    /// Specifically, it provides access to the key-value pairs, where the keys
    /// are export names, and the values are export declarations which can be
    /// resolved `lookup_by_declaration`.
    pub fn exports(&self) -> indexmap::map::Iter<String, EntityIndex> {
        self.instance().exports()
    }

    /// Return a reference to the custom state attached to this instance.
    pub fn host_state(&self) -> &dyn Any {
        self.instance().host_state()
    }

    /// Return the memory index for the given `VMMemoryDefinition` in this instance.
    pub unsafe fn memory_index(&self, memory: &VMMemoryDefinition) -> DefinedMemoryIndex {
        self.instance().memory_index(memory)
    }

    /// Get a memory defined locally within this module.
    pub fn get_defined_memory(&mut self, index: DefinedMemoryIndex) -> *mut Memory {
        self.instance_mut().get_defined_memory(index)
    }

    /// Return the table index for the given `VMTableDefinition` in this instance.
    pub unsafe fn table_index(&self, table: &VMTableDefinition) -> DefinedTableIndex {
        self.instance().table_index(table)
    }

    /// Get a table defined locally within this module.
    pub fn get_defined_table(&mut self, index: DefinedTableIndex) -> *mut Table {
        self.instance_mut().get_defined_table(index)
    }

    /// Get a table defined locally within this module, lazily
    /// initializing the given range first.
    pub fn get_defined_table_with_lazy_init(
        &mut self,
        index: DefinedTableIndex,
        range: impl Iterator<Item = u32>,
    ) -> *mut Table {
        let index = self.instance().module().table_index(index);
        self.instance_mut().get_table_with_lazy_init(index, range)
    }

    /// Return a reference to the contained `Instance`.
    #[inline]
    pub(crate) fn instance(&self) -> &Instance {
        unsafe { &*(self.instance as *const Instance) }
    }

    pub(crate) fn instance_mut(&mut self) -> &mut Instance {
        unsafe { &mut *self.instance }
    }

    /// Returns the `Store` pointer that was stored on creation
    #[inline]
    pub fn store(&self) -> *mut dyn Store {
        self.instance().store()
    }

    /// Configure the `*mut dyn Store` internal pointer after-the-fact.
    ///
    /// This is provided for the original `Store` itself to configure the first
    /// self-pointer after the original `Box` has been initialized.
    pub unsafe fn set_store(&mut self, store: *mut dyn Store) {
        self.instance_mut().set_store(store);
    }

    /// Returns a clone of this instance.
    ///
    /// This is unsafe because the returned handle here is just a cheap clone
    /// of the internals, there's no lifetime tracking around its validity.
    /// You'll need to ensure that the returned handles all go out of scope at
    /// the same time.
    #[inline]
    pub unsafe fn clone(&self) -> InstanceHandle {
        InstanceHandle {
            instance: self.instance,
        }
    }
}
