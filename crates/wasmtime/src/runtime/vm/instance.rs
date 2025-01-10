//! An `Instance` contains all the runtime state used by execution of a
//! wasm module (except its callstack and register state). An
//! `InstanceHandle` is a reference-counting handle for an `Instance`.

use crate::runtime::vm::const_expr::{ConstEvalContext, ConstExprEvaluator};
use crate::runtime::vm::export::Export;
use crate::runtime::vm::memory::{Memory, RuntimeMemoryCreator};
use crate::runtime::vm::table::{Table, TableElement, TableElementType};
use crate::runtime::vm::vmcontext::{
    VMBuiltinFunctionsArray, VMContext, VMFuncRef, VMFunctionImport, VMGlobalDefinition,
    VMGlobalImport, VMMemoryDefinition, VMMemoryImport, VMOpaqueContext, VMRuntimeLimits,
    VMTableDefinition, VMTableImport,
};
use crate::runtime::vm::{
    ExportFunction, ExportGlobal, ExportMemory, ExportTable, GcStore, Imports, ModuleRuntimeInfo,
    SendSyncPtr, VMFunctionBody, VMGcRef, VMStore, WasmFault,
};
use crate::store::{StoreInner, StoreOpaque};
use crate::{StoreContextMut, prelude::*};
use alloc::sync::Arc;
use core::alloc::Layout;
use core::any::Any;
use core::ops::Range;
use core::ptr::NonNull;
use core::sync::atomic::AtomicU64;
use core::{mem, ptr};
use sptr::Strict;
use wasmtime_environ::{
    DataIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, ElemIndex, EntityIndex,
    EntityRef, EntitySet, FuncIndex, GlobalIndex, HostPtr, MemoryIndex, Module,
    ModuleInternedTypeIndex, PrimaryMap, PtrSize, TableIndex, TableInitialValue,
    TableSegmentElements, Trap, VMCONTEXT_MAGIC, VMOffsets, VMSharedTypeIndex, WasmHeapTopType,
    packed_option::ReservedValue,
};
#[cfg(feature = "wmemcheck")]
use wasmtime_wmemcheck::Wmemcheck;

mod allocator;
pub use allocator::*;

/// The pair of an instance and a raw pointer its associated store.
///
/// ### Safety
///
/// Getting a borrow of a vmctx's store is one of the fundamental bits of unsafe
/// code in Wasmtime. No matter how we architect the runtime, some kind of
/// unsafe conversion from a raw vmctx pointer that Wasm is using into a Rust
/// struct must happen.
///
/// It is our responsibility to ensure that multiple (exclusive) borrows of the
/// vmctx's store never exist at the same time. The distinction between the
/// `Instance` type (which doesn't expose its underlying vmctx pointer or a way
/// to get a borrow of its associated store) and this type (which does) is
/// designed to help with that.
///
/// Going from a `*mut VMContext` to a `&mut StoreInner<T>` is naturally unsafe
/// due to the raw pointer usage, but additionally the `T` type parameter needs
/// to be the same `T` that was used to define the `dyn VMStore` trait object
/// that was stuffed into the vmctx.
///
/// ### Usage
///
/// Usage generally looks like:
///
/// 1. You get a raw `*mut VMContext` from Wasm
///
/// 2. You call `InstanceAndStore::from_vmctx` on that raw pointer
///
/// 3. You then call `InstanceAndStore::unpack_mut` (or another helper) to get
///    the underlying `&mut Instance` and `&mut dyn VMStore` (or `&mut
///    StoreInner<T>`).
///
/// 4. You then use whatever `Instance` methods you need to, each of which take
///    a store argument as necessary.
///
/// In step (4) you no longer need to worry about double exclusive borrows of
/// the store, so long as you don't do (1-2) again. Note also that the borrow
/// checker prevents repeating step (3) if you never repeat (1-2). In general,
/// steps (1-3) should be done in a single, common, internally-unsafe,
/// plumbing-code bottleneck and the raw pointer should never be exposed to Rust
/// code that does (4) after the `InstanceAndStore` is created. Follow this
/// pattern, and everything using the resulting `Instance` and `Store` can be
/// safe code (at least, with regards to accessing the store itself).
///
/// As an illustrative example, the common plumbing code for our various
/// libcalls performs steps (1-3) before calling into each actual libcall
/// implementation function that does (4). The plumbing code hides the raw vmctx
/// pointer and never gives out access to it to the libcall implementation
/// functions, nor does an `Instance` expose its internal vmctx pointer, which
/// would allow unsafely repeating steps (1-2).
#[repr(transparent)]
pub struct InstanceAndStore {
    instance: Instance,
}

impl InstanceAndStore {
    /// Converts the provided `*mut VMContext` to an `InstanceAndStore`
    /// reference and calls the provided closure with it.
    ///
    /// This method will move the `vmctx` pointer backwards to point to the
    /// original `Instance` that precedes it. The closure is provided a
    /// temporary reference to the `InstanceAndStore` with a constrained
    /// lifetime to ensure that it doesn't accidentally escape.
    ///
    /// # Safety
    ///
    /// Callers must validate that the `vmctx` pointer is a valid allocation and
    /// that it's valid to acquire `&mut InstanceAndStore` at this time. For
    /// example this can't be called twice on the same `VMContext` to get two
    /// active mutable borrows to the same `InstanceAndStore`.
    ///
    /// See also the safety discussion in this type's documentation.
    #[inline]
    pub(crate) unsafe fn from_vmctx<R>(
        vmctx: *mut VMContext,
        f: impl for<'a> FnOnce(&'a mut Self) -> R,
    ) -> R {
        debug_assert!(!vmctx.is_null());

        const _: () = assert!(mem::size_of::<InstanceAndStore>() == mem::size_of::<Instance>());
        let ptr = vmctx
            .byte_sub(mem::size_of::<Instance>())
            .cast::<InstanceAndStore>();

        f(&mut *ptr)
    }

    /// Unpacks this `InstanceAndStore` into its underlying `Instance` and `dyn
    /// VMStore`.
    #[inline]
    pub(crate) fn unpack_mut(&mut self) -> (&mut Instance, &mut dyn VMStore) {
        unsafe {
            let store = &mut *self.store_ptr();
            (&mut self.instance, store)
        }
    }

    /// Unpacks this `InstanceAndStore` into its underlying `Instance` and
    /// `StoreInner<T>`.
    ///
    /// # Safety
    ///
    /// The `T` must be the same `T` that was used to define this store's
    /// instance.
    #[inline]
    pub(crate) unsafe fn unpack_context_mut<T>(
        &mut self,
    ) -> (&mut Instance, StoreContextMut<'_, T>) {
        let store_ptr = self.store_ptr().cast::<StoreInner<T>>();
        (&mut self.instance, StoreContextMut(&mut *store_ptr))
    }

    /// Gets a pointer to this instance's `Store` which was originally
    /// configured on creation.
    ///
    /// # Panics
    ///
    /// May panic if the originally configured store was `None`. That can happen
    /// for host functions so host functions can't be queried what their
    /// original `Store` was since it's just retained as null (since host
    /// functions are shared amongst threads and don't all share the same
    /// store).
    #[inline]
    fn store_ptr(&self) -> *mut dyn VMStore {
        let ptr = unsafe {
            *self
                .instance
                .vmctx_plus_offset::<*mut dyn VMStore>(self.instance.offsets().ptr.vmctx_store())
        };
        debug_assert!(!ptr.is_null());
        ptr
    }
}

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
pub struct Instance {
    /// The runtime info (corresponding to the "compiled module"
    /// abstraction in higher layers) that is retained and needed for
    /// lazy initialization. This provides access to the underlying
    /// Wasm module entities, the compiled JIT code, metadata about
    /// functions, lazy initialization state, etc.
    runtime_info: ModuleRuntimeInfo,

    /// WebAssembly linear memory data.
    ///
    /// This is where all runtime information about defined linear memories in
    /// this module lives.
    ///
    /// The `MemoryAllocationIndex` was given from our `InstanceAllocator` and
    /// must be given back to the instance allocator when deallocating each
    /// memory.
    memories: PrimaryMap<DefinedMemoryIndex, (MemoryAllocationIndex, Memory)>,

    /// WebAssembly table data.
    ///
    /// Like memories, this is only for defined tables in the module and
    /// contains all of their runtime state.
    ///
    /// The `TableAllocationIndex` was given from our `InstanceAllocator` and
    /// must be given back to the instance allocator when deallocating each
    /// table.
    tables: PrimaryMap<DefinedTableIndex, (TableAllocationIndex, Table)>,

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

    /// A pointer to the `vmctx` field at the end of the `Instance`.
    ///
    /// If you're looking at this a reasonable question would be "why do we need
    /// a pointer to ourselves?" because after all the pointer's value is
    /// trivially derivable from any `&Instance` pointer. The rationale for this
    /// field's existence is subtle, but it's required for correctness. The
    /// short version is "this makes miri happy".
    ///
    /// The long version of why this field exists is that the rules that MIRI
    /// uses to ensure pointers are used correctly have various conditions on
    /// them depend on how pointers are used. More specifically if `*mut T` is
    /// derived from `&mut T`, then that invalidates all prior pointers drived
    /// from the `&mut T`. This means that while we liberally want to re-acquire
    /// a `*mut VMContext` throughout the implementation of `Instance` the
    /// trivial way, a function `fn vmctx(&mut Instance) -> *mut VMContext`
    /// would effectively invalidate all prior `*mut VMContext` pointers
    /// acquired. The purpose of this field is to serve as a sort of
    /// source-of-truth for where `*mut VMContext` pointers come from.
    ///
    /// This field is initialized when the `Instance` is created with the
    /// original allocation's pointer. That means that the provenance of this
    /// pointer contains the entire allocation (both instance and `VMContext`).
    /// This provenance bit is then "carried through" where `fn vmctx` will base
    /// all returned pointers on this pointer itself. This provides the means of
    /// never invalidating this pointer throughout MIRI and additionally being
    /// able to still temporarily have `&mut Instance` methods and such.
    ///
    /// It's important to note, though, that this is not here purely for MIRI.
    /// The careful construction of the `fn vmctx` method has ramifications on
    /// the LLVM IR generated, for example. A historical CVE on Wasmtime,
    /// GHSA-ch89-5g45-qwc7, was caused due to relying on undefined behavior. By
    /// deriving VMContext pointers from this pointer it specifically hints to
    /// LLVM that trickery is afoot and it properly informs `noalias` and such
    /// annotations and analysis. More-or-less this pointer is actually loaded
    /// in LLVM IR which helps defeat otherwise present aliasing optimizations,
    /// which we want, since writes to this should basically never be optimized
    /// out.
    ///
    /// As a final note it's worth pointing out that the machine code generated
    /// for accessing `fn vmctx` is still as one would expect. This member isn't
    /// actually ever loaded at runtime (or at least shouldn't be). Perhaps in
    /// the future if the memory consumption of this field is a problem we could
    /// shrink it slightly, but for now one extra pointer per wasm instance
    /// seems not too bad.
    vmctx_self_reference: SendSyncPtr<VMContext>,

    // TODO: add support for multiple memories; `wmemcheck_state` corresponds to
    // memory 0.
    #[cfg(feature = "wmemcheck")]
    pub(crate) wmemcheck_state: Option<Wmemcheck>,

    /// Additional context used by compiled wasm code. This field is last, and
    /// represents a dynamically-sized array that extends beyond the nominal
    /// end of the struct (similar to a flexible array member).
    vmctx: VMContext,
}

impl Instance {
    /// Create an instance at the given memory address.
    ///
    /// It is assumed the memory was properly aligned and the
    /// allocation was `alloc_size` in bytes.
    unsafe fn new(
        req: InstanceAllocationRequest,
        memories: PrimaryMap<DefinedMemoryIndex, (MemoryAllocationIndex, Memory)>,
        tables: PrimaryMap<DefinedTableIndex, (TableAllocationIndex, Table)>,
        memory_tys: &PrimaryMap<MemoryIndex, wasmtime_environ::Memory>,
    ) -> InstanceHandle {
        // The allocation must be *at least* the size required of `Instance`.
        let layout = Self::alloc_layout(req.runtime_info.offsets());
        let ptr = alloc::alloc::alloc(layout);
        if ptr.is_null() {
            alloc::alloc::handle_alloc_error(layout);
        }
        let ptr = ptr.cast::<Instance>();

        let module = req.runtime_info.env_module();
        let dropped_elements = EntitySet::with_capacity(module.passive_elements.len());
        let dropped_data = EntitySet::with_capacity(module.passive_data_map.len());

        #[cfg(not(feature = "wmemcheck"))]
        let _ = memory_tys;

        ptr::write(ptr, Instance {
            runtime_info: req.runtime_info.clone(),
            memories,
            tables,
            dropped_elements,
            dropped_data,
            host_state: req.host_state,
            vmctx_self_reference: SendSyncPtr::new(NonNull::new(ptr.add(1).cast()).unwrap()),
            vmctx: VMContext {
                _marker: core::marker::PhantomPinned,
            },
            #[cfg(feature = "wmemcheck")]
            wmemcheck_state: {
                if req.wmemcheck {
                    let size = memory_tys
                        .iter()
                        .next()
                        .map(|memory| memory.1.limits.min)
                        .unwrap_or(0)
                        * 64
                        * 1024;
                    Some(Wmemcheck::new(size as usize))
                } else {
                    None
                }
            },
        });

        (*ptr).initialize_vmctx(module, req.runtime_info.offsets(), req.store, req.imports);
        InstanceHandle {
            instance: Some(SendSyncPtr::new(NonNull::new(ptr).unwrap())),
        }
    }

    /// Converts the provided `*mut VMContext` to an `Instance` pointer and runs
    /// the provided closure with the instance.
    ///
    /// This method will move the `vmctx` pointer backwards to point to the
    /// original `Instance` that precedes it. The closure is provided a
    /// temporary version of the `Instance` pointer with a constrained lifetime
    /// to the closure to ensure it doesn't accidentally escape.
    ///
    /// # Unsafety
    ///
    /// Callers must validate that the `vmctx` pointer is a valid allocation
    /// and that it's valid to acquire `&mut Instance` at this time. For example
    /// this can't be called twice on the same `VMContext` to get two active
    /// pointers to the same `Instance`.
    #[inline]
    pub unsafe fn from_vmctx<R>(vmctx: *mut VMContext, f: impl FnOnce(&mut Instance) -> R) -> R {
        debug_assert!(!vmctx.is_null());
        let ptr = vmctx
            .byte_sub(mem::size_of::<Instance>())
            .cast::<Instance>();
        f(&mut *ptr)
    }

    /// Helper function to access various locations offset from our `*mut
    /// VMContext` object.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the `offset` must be within bounds of the
    /// `VMContext` object trailing this instance.
    unsafe fn vmctx_plus_offset<T>(&self, offset: impl Into<u32>) -> *const T {
        self.vmctx()
            .byte_add(usize::try_from(offset.into()).unwrap())
            .cast()
    }

    /// Dual of `vmctx_plus_offset`, but for mutability.
    unsafe fn vmctx_plus_offset_mut<T>(&mut self, offset: impl Into<u32>) -> *mut T {
        self.vmctx()
            .byte_add(usize::try_from(offset.into()).unwrap())
            .cast()
    }

    pub(crate) fn env_module(&self) -> &Arc<wasmtime_environ::Module> {
        self.runtime_info.env_module()
    }

    pub(crate) fn runtime_module(&self) -> Option<&crate::Module> {
        match &self.runtime_info {
            ModuleRuntimeInfo::Module(m) => Some(m),
            ModuleRuntimeInfo::Bare(_) => None,
        }
    }

    /// Translate a module-level interned type index into an engine-level
    /// interned type index.
    pub fn engine_type_index(&self, module_index: ModuleInternedTypeIndex) -> VMSharedTypeIndex {
        self.runtime_info.engine_type_index(module_index)
    }

    #[inline]
    fn offsets(&self) -> &VMOffsets<HostPtr> {
        self.runtime_info.offsets()
    }

    /// Return the indexed `VMFunctionImport`.
    fn imported_function(&self, index: FuncIndex) -> &VMFunctionImport {
        unsafe { &*self.vmctx_plus_offset(self.offsets().vmctx_vmfunction_import(index)) }
    }

    /// Return the index `VMTableImport`.
    fn imported_table(&self, index: TableIndex) -> &VMTableImport {
        unsafe { &*self.vmctx_plus_offset(self.offsets().vmctx_vmtable_import(index)) }
    }

    /// Return the indexed `VMMemoryImport`.
    fn imported_memory(&self, index: MemoryIndex) -> &VMMemoryImport {
        unsafe { &*self.vmctx_plus_offset(self.offsets().vmctx_vmmemory_import(index)) }
    }

    /// Return the indexed `VMGlobalImport`.
    fn imported_global(&self, index: GlobalIndex) -> &VMGlobalImport {
        unsafe { &*self.vmctx_plus_offset(self.offsets().vmctx_vmglobal_import(index)) }
    }

    /// Return the indexed `VMTableDefinition`.
    #[allow(dead_code)]
    fn table(&mut self, index: DefinedTableIndex) -> VMTableDefinition {
        unsafe { *self.table_ptr(index) }
    }

    /// Updates the value for a defined table to `VMTableDefinition`.
    fn set_table(&mut self, index: DefinedTableIndex, table: VMTableDefinition) {
        unsafe {
            *self.table_ptr(index) = table;
        }
    }

    /// Return the indexed `VMTableDefinition`.
    fn table_ptr(&mut self, index: DefinedTableIndex) -> *mut VMTableDefinition {
        unsafe { self.vmctx_plus_offset_mut(self.offsets().vmctx_vmtable_definition(index)) }
    }

    /// Get a locally defined or imported memory.
    pub(crate) fn get_memory(&self, index: MemoryIndex) -> VMMemoryDefinition {
        if let Some(defined_index) = self.env_module().defined_memory_index(index) {
            self.memory(defined_index)
        } else {
            let import = self.imported_memory(index);
            unsafe { VMMemoryDefinition::load(import.from) }
        }
    }

    /// Get a locally defined or imported memory.
    #[cfg(feature = "threads")]
    pub(crate) fn get_runtime_memory(&mut self, index: MemoryIndex) -> &mut Memory {
        if let Some(defined_index) = self.env_module().defined_memory_index(index) {
            unsafe { &mut *self.get_defined_memory(defined_index) }
        } else {
            let import = self.imported_memory(index);
            unsafe {
                let ptr =
                    Instance::from_vmctx(import.vmctx, |i| i.get_defined_memory(import.index));
                &mut *ptr
            }
        }
    }

    /// Return the indexed `VMMemoryDefinition`.
    fn memory(&self, index: DefinedMemoryIndex) -> VMMemoryDefinition {
        unsafe { VMMemoryDefinition::load(self.memory_ptr(index)) }
    }

    /// Set the indexed memory to `VMMemoryDefinition`.
    fn set_memory(&self, index: DefinedMemoryIndex, mem: VMMemoryDefinition) {
        unsafe {
            *self.memory_ptr(index) = mem;
        }
    }

    /// Return the indexed `VMMemoryDefinition`.
    fn memory_ptr(&self, index: DefinedMemoryIndex) -> *mut VMMemoryDefinition {
        unsafe { *self.vmctx_plus_offset(self.offsets().vmctx_vmmemory_pointer(index)) }
    }

    /// Return the indexed `VMGlobalDefinition`.
    fn global_ptr(&mut self, index: DefinedGlobalIndex) -> *mut VMGlobalDefinition {
        unsafe { self.vmctx_plus_offset_mut(self.offsets().vmctx_vmglobal_definition(index)) }
    }

    /// Get a raw pointer to the global at the given index regardless whether it
    /// is defined locally or imported from another module.
    ///
    /// Panics if the index is out of bound or is the reserved value.
    pub(crate) fn defined_or_imported_global_ptr(
        &mut self,
        index: GlobalIndex,
    ) -> *mut VMGlobalDefinition {
        if let Some(index) = self.env_module().defined_global_index(index) {
            self.global_ptr(index)
        } else {
            self.imported_global(index).from
        }
    }

    /// Get all globals within this instance.
    ///
    /// Returns both import and defined globals.
    ///
    /// Returns both exported and non-exported globals.
    ///
    /// Gives access to the full globals space.
    pub fn all_globals<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator<Item = (GlobalIndex, ExportGlobal)> + 'a {
        let module = self.env_module().clone();
        module.globals.keys().map(move |idx| {
            (idx, ExportGlobal {
                definition: self.defined_or_imported_global_ptr(idx),
                vmctx: self.vmctx(),
                global: self.env_module().globals[idx],
            })
        })
    }

    /// Get the globals defined in this instance (not imported).
    pub fn defined_globals<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator<Item = (DefinedGlobalIndex, ExportGlobal)> + 'a {
        let module = self.env_module().clone();
        module
            .globals
            .keys()
            .skip(module.num_imported_globals)
            .map(move |global_idx| {
                let def_idx = module.defined_global_index(global_idx).unwrap();
                let global = ExportGlobal {
                    definition: self.global_ptr(def_idx),
                    vmctx: self.vmctx(),
                    global: self.env_module().globals[global_idx],
                };
                (def_idx, global)
            })
    }

    /// Return a pointer to the interrupts structure
    #[inline]
    pub fn runtime_limits(&mut self) -> *mut *const VMRuntimeLimits {
        unsafe { self.vmctx_plus_offset_mut(self.offsets().ptr.vmctx_runtime_limits()) }
    }

    /// Return a pointer to the global epoch counter used by this instance.
    pub fn epoch_ptr(&mut self) -> *mut *const AtomicU64 {
        unsafe { self.vmctx_plus_offset_mut(self.offsets().ptr.vmctx_epoch_ptr()) }
    }

    /// Return a pointer to the GC heap base pointer.
    pub fn gc_heap_base(&mut self) -> *mut *mut u8 {
        unsafe { self.vmctx_plus_offset_mut(self.offsets().ptr.vmctx_gc_heap_base()) }
    }

    /// Return a pointer to the GC heap bound.
    pub fn gc_heap_bound(&mut self) -> *mut usize {
        unsafe { self.vmctx_plus_offset_mut(self.offsets().ptr.vmctx_gc_heap_bound()) }
    }

    /// Return a pointer to the collector-specific heap data.
    pub fn gc_heap_data(&mut self) -> *mut *mut u8 {
        unsafe { self.vmctx_plus_offset_mut(self.offsets().ptr.vmctx_gc_heap_data()) }
    }

    pub(crate) unsafe fn set_store(&mut self, store: Option<*mut dyn VMStore>) {
        if let Some(store) = store {
            *self.vmctx_plus_offset_mut(self.offsets().ptr.vmctx_store()) = store;
            *self.runtime_limits() = (*store).vmruntime_limits();
            *self.epoch_ptr() = (*store).engine().epoch_counter();
            self.set_gc_heap((*store).gc_store_mut().ok());
        } else {
            assert_eq!(
                mem::size_of::<*mut dyn VMStore>(),
                mem::size_of::<[*mut (); 2]>()
            );
            *self.vmctx_plus_offset_mut::<[*mut (); 2]>(self.offsets().ptr.vmctx_store()) =
                [ptr::null_mut(), ptr::null_mut()];
            *self.runtime_limits() = ptr::null_mut();
            *self.epoch_ptr() = ptr::null_mut();
            self.set_gc_heap(None);
        }
    }

    unsafe fn set_gc_heap(&mut self, gc_store: Option<&mut GcStore>) {
        if let Some(gc_store) = gc_store {
            let heap = gc_store.gc_heap.heap_slice_mut();
            *self.gc_heap_base() = heap.as_mut_ptr();
            *self.gc_heap_bound() = heap.len();
            *self.gc_heap_data() = gc_store.gc_heap.vmctx_gc_heap_data();
        } else {
            *self.gc_heap_base() = ptr::null_mut();
            *self.gc_heap_bound() = 0;
            *self.gc_heap_data() = ptr::null_mut();
        }
    }

    pub(crate) unsafe fn set_callee(&mut self, callee: Option<NonNull<VMFunctionBody>>) {
        *self.vmctx_plus_offset_mut(self.offsets().ptr.vmctx_callee()) =
            callee.map_or(ptr::null_mut(), |c| c.as_ptr());
    }

    /// Return a reference to the vmctx used by compiled wasm code.
    #[inline]
    pub fn vmctx(&self) -> *mut VMContext {
        // The definition of this method is subtle but intentional. The goal
        // here is that effectively this should return `&mut self.vmctx`, but
        // it's not quite so simple. Some more documentation is available on the
        // `vmctx_self_reference` field, but the general idea is that we're
        // creating a pointer to return with proper provenance. Provenance is
        // still in the works in Rust at the time of this writing but the load
        // of the `self.vmctx_self_reference` field is important here as it
        // affects how LLVM thinks about aliasing with respect to the returned
        // pointer.
        //
        // The intention of this method is to codegen to machine code as `&mut
        // self.vmctx`, however. While it doesn't show up like this in LLVM IR
        // (there's an actual load of the field) it does look like that by the
        // time the backend runs. (that's magic to me, the backend removing
        // loads...)
        //
        // As a final minor note, strict provenance APIs are not stable on Rust
        // today so the `sptr` crate is used. This crate provides the extension
        // trait `Strict` but the method names conflict with the nightly methods
        // so a different syntax is used to invoke methods here.
        let addr = &raw const self.vmctx;
        Strict::with_addr(self.vmctx_self_reference.as_ptr(), Strict::addr(addr))
    }

    fn get_exported_func(&mut self, index: FuncIndex) -> ExportFunction {
        let func_ref = self.get_func_ref(index).unwrap();
        ExportFunction { func_ref }
    }

    fn get_exported_table(&mut self, index: TableIndex) -> ExportTable {
        let (definition, vmctx) =
            if let Some(def_index) = self.env_module().defined_table_index(index) {
                (self.table_ptr(def_index), self.vmctx())
            } else {
                let import = self.imported_table(index);
                (import.from, import.vmctx)
            };
        ExportTable {
            definition,
            vmctx,
            table: self.env_module().tables[index],
        }
    }

    fn get_exported_memory(&mut self, index: MemoryIndex) -> ExportMemory {
        let (definition, vmctx, def_index) =
            if let Some(def_index) = self.env_module().defined_memory_index(index) {
                (self.memory_ptr(def_index), self.vmctx(), def_index)
            } else {
                let import = self.imported_memory(index);
                (import.from, import.vmctx, import.index)
            };
        ExportMemory {
            definition,
            vmctx,
            memory: self.env_module().memories[index],
            index: def_index,
        }
    }

    fn get_exported_global(&mut self, index: GlobalIndex) -> ExportGlobal {
        ExportGlobal {
            definition: if let Some(def_index) = self.env_module().defined_global_index(index) {
                self.global_ptr(def_index)
            } else {
                self.imported_global(index).from
            },
            vmctx: self.vmctx(),
            global: self.env_module().globals[index],
        }
    }

    /// Return an iterator over the exports of this instance.
    ///
    /// Specifically, it provides access to the key-value pairs, where the keys
    /// are export names, and the values are export declarations which can be
    /// resolved `lookup_by_declaration`.
    pub fn exports(&self) -> wasmparser::collections::index_map::Iter<String, EntityIndex> {
        self.env_module().exports.iter()
    }

    /// Return a reference to the custom state attached to this instance.
    #[inline]
    pub fn host_state(&self) -> &dyn Any {
        &*self.host_state
    }

    /// Return the table index for the given `VMTableDefinition`.
    pub unsafe fn table_index(&mut self, table: &VMTableDefinition) -> DefinedTableIndex {
        let index = DefinedTableIndex::new(
            usize::try_from(
                (table as *const VMTableDefinition)
                    .offset_from(self.table_ptr(DefinedTableIndex::new(0))),
            )
            .unwrap(),
        );
        assert!(index.index() < self.tables.len());
        index
    }

    /// Get the given memory's page size, in bytes.
    pub(crate) fn memory_page_size(&self, index: MemoryIndex) -> usize {
        usize::try_from(self.env_module().memories[index].page_size()).unwrap()
    }

    /// Grow memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages. Returns `Some` with the old size in bytes if growth was
    /// successful.
    pub(crate) fn memory_grow(
        &mut self,
        store: &mut dyn VMStore,
        index: MemoryIndex,
        delta: u64,
    ) -> Result<Option<usize>, Error> {
        match self.env_module().defined_memory_index(index) {
            Some(idx) => self.defined_memory_grow(store, idx, delta),
            None => {
                let import = self.imported_memory(index);
                unsafe {
                    Instance::from_vmctx(import.vmctx, |i| {
                        i.defined_memory_grow(store, import.index, delta)
                    })
                }
            }
        }
    }

    fn defined_memory_grow(
        &mut self,
        store: &mut dyn VMStore,
        idx: DefinedMemoryIndex,
        delta: u64,
    ) -> Result<Option<usize>, Error> {
        let memory = &mut self.memories[idx].1;

        let result = unsafe { memory.grow(delta, Some(store)) };

        // Update the state used by a non-shared Wasm memory in case the base
        // pointer and/or the length changed.
        if memory.as_shared_memory().is_none() {
            let vmmemory = memory.vmmemory();
            self.set_memory(idx, vmmemory);
        }

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
        store: &mut dyn VMStore,
        table_index: TableIndex,
        delta: u64,
        init_value: TableElement,
    ) -> Result<Option<usize>, Error> {
        self.with_defined_table_index_and_instance(table_index, |i, instance| {
            instance.defined_table_grow(store, i, delta, init_value)
        })
    }

    fn defined_table_grow(
        &mut self,
        store: &mut dyn VMStore,
        table_index: DefinedTableIndex,
        delta: u64,
        init_value: TableElement,
    ) -> Result<Option<usize>, Error> {
        let table = &mut self
            .tables
            .get_mut(table_index)
            .unwrap_or_else(|| panic!("no table for index {}", table_index.index()))
            .1;

        let result = unsafe { table.grow(delta, init_value, store) };

        // Keep the `VMContext` pointers used by compiled Wasm code up to
        // date.
        let element = self.tables[table_index].1.vmtable();
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

    /// Construct a new VMFuncRef for the given function
    /// (imported or defined in this module) and store into the given
    /// location. Used during lazy initialization.
    ///
    /// Note that our current lazy-init scheme actually calls this every
    /// time the funcref pointer is fetched; this turns out to be better
    /// than tracking state related to whether it's been initialized
    /// before, because resetting that state on (re)instantiation is
    /// very expensive if there are many funcrefs.
    fn construct_func_ref(
        &mut self,
        index: FuncIndex,
        sig: ModuleInternedTypeIndex,
        into: *mut VMFuncRef,
    ) {
        let type_index = unsafe {
            let base: *const VMSharedTypeIndex =
                *self.vmctx_plus_offset_mut(self.offsets().ptr.vmctx_type_ids_array());
            *base.add(sig.index())
        };

        let func_ref = if let Some(def_index) = self.env_module().defined_func_index(index) {
            VMFuncRef {
                array_call: self
                    .runtime_info
                    .array_to_wasm_trampoline(def_index)
                    .expect("should have array-to-Wasm trampoline for escaping function"),
                wasm_call: Some(self.runtime_info.function(def_index)),
                vmctx: VMOpaqueContext::from_vmcontext(self.vmctx()),
                type_index,
            }
        } else {
            let import = self.imported_function(index);
            VMFuncRef {
                array_call: import.array_call,
                wasm_call: Some(import.wasm_call),
                vmctx: import.vmctx,
                type_index,
            }
        };

        // Safety: we have a `&mut self`, so we have exclusive access
        // to this Instance.
        unsafe {
            ptr::write(into, func_ref);
        }
    }

    /// Get a `&VMFuncRef` for the given `FuncIndex`.
    ///
    /// Returns `None` if the index is the reserved index value.
    ///
    /// The returned reference is a stable reference that won't be moved and can
    /// be passed into JIT code.
    pub(crate) fn get_func_ref(&mut self, index: FuncIndex) -> Option<NonNull<VMFuncRef>> {
        if index == FuncIndex::reserved_value() {
            return None;
        }

        // Safety: we have a `&mut self`, so we have exclusive access
        // to this Instance.
        unsafe {
            // For now, we eagerly initialize an funcref struct in-place
            // whenever asked for a reference to it. This is mostly
            // fine, because in practice each funcref is unlikely to be
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
            // funcrefs (perhaps together with other
            // zeroed-at-instantiate-time state) or using a separate
            // is-initialized bitmap.
            //
            // We arrived at this design because zeroing memory is
            // expensive, so it's better for instantiation performance
            // if we don't have to track "is-initialized" state at
            // all!
            let func = &self.env_module().functions[index];
            let sig = func.signature;
            let func_ref: *mut VMFuncRef = self
                .vmctx_plus_offset_mut::<VMFuncRef>(self.offsets().vmctx_func_ref(func.func_ref));
            self.construct_func_ref(index, sig, func_ref);

            Some(NonNull::new(func_ref).unwrap())
        }
    }

    /// Get the passive elements segment at the given index.
    ///
    /// Returns an empty segment if the index is out of bounds or if the segment
    /// has been dropped.
    ///
    /// The `storage` parameter should always be `None`; it is a bit of a hack
    /// to work around lifetime issues.
    pub(crate) fn passive_element_segment<'a>(
        &self,
        storage: &'a mut Option<(Arc<wasmtime_environ::Module>, TableSegmentElements)>,
        elem_index: ElemIndex,
    ) -> &'a TableSegmentElements {
        debug_assert!(storage.is_none());
        *storage = Some((
            // TODO: this `clone()` shouldn't be necessary but is used for now to
            // inform `rustc` that the lifetime of the elements here are
            // disconnected from the lifetime of `self`.
            self.env_module().clone(),
            // NB: fall back to an expressions-based list of elements which
            // doesn't have static type information (as opposed to
            // `TableSegmentElements::Functions`) since we don't know what type
            // is needed in the caller's context. Let the type be inferred by
            // how they use the segment.
            TableSegmentElements::Expressions(Box::new([])),
        ));
        let (module, empty) = storage.as_ref().unwrap();

        match module.passive_elements_map.get(&elem_index) {
            Some(index) if !self.dropped_elements.contains(elem_index) => {
                &module.passive_elements[*index]
            }
            _ => empty,
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
        store: &mut StoreOpaque,
        table_index: TableIndex,
        elem_index: ElemIndex,
        dst: u64,
        src: u64,
        len: u64,
    ) -> Result<(), Trap> {
        let mut storage = None;
        let elements = self.passive_element_segment(&mut storage, elem_index);
        let mut const_evaluator = ConstExprEvaluator::default();
        self.table_init_segment(
            store,
            &mut const_evaluator,
            table_index,
            elements,
            dst,
            src,
            len,
        )
    }

    pub(crate) fn table_init_segment(
        &mut self,
        store: &mut StoreOpaque,
        const_evaluator: &mut ConstExprEvaluator,
        table_index: TableIndex,
        elements: &TableSegmentElements,
        dst: u64,
        src: u64,
        len: u64,
    ) -> Result<(), Trap> {
        // https://webassembly.github.io/bulk-memory-operations/core/exec/instructions.html#exec-table-init

        let table = unsafe { &mut *self.get_table(table_index) };
        let src = usize::try_from(src).map_err(|_| Trap::TableOutOfBounds)?;
        let len = usize::try_from(len).map_err(|_| Trap::TableOutOfBounds)?;
        let module = self.env_module().clone();

        match elements {
            TableSegmentElements::Functions(funcs) => {
                let elements = funcs
                    .get(src..)
                    .and_then(|s| s.get(..len))
                    .ok_or(Trap::TableOutOfBounds)?;
                table.init_func(dst, elements.iter().map(|idx| self.get_func_ref(*idx)))?;
            }
            TableSegmentElements::Expressions(exprs) => {
                let exprs = exprs
                    .get(src..)
                    .and_then(|s| s.get(..len))
                    .ok_or(Trap::TableOutOfBounds)?;
                let mut context = ConstEvalContext::new(self);
                match module.tables[table_index].ref_type.heap_type.top() {
                    WasmHeapTopType::Extern => table.init_gc_refs(
                        dst,
                        exprs.iter().map(|expr| unsafe {
                            let raw = const_evaluator
                                .eval(store, &mut context, expr)
                                .expect("const expr should be valid");
                            VMGcRef::from_raw_u32(raw.get_externref())
                        }),
                    )?,
                    WasmHeapTopType::Any => table.init_gc_refs(
                        dst,
                        exprs.iter().map(|expr| unsafe {
                            let raw = const_evaluator
                                .eval(store, &mut context, expr)
                                .expect("const expr should be valid");
                            VMGcRef::from_raw_u32(raw.get_anyref())
                        }),
                    )?,
                    WasmHeapTopType::Func => table.init_func(
                        dst,
                        exprs.iter().map(|expr| unsafe {
                            NonNull::new(
                                const_evaluator
                                    .eval(store, &mut context, expr)
                                    .expect("const expr should be valid")
                                    .get_funcref()
                                    .cast(),
                            )
                        }),
                    )?,
                }
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
    pub fn get_defined_memory(&mut self, index: DefinedMemoryIndex) -> *mut Memory {
        &raw mut self.memories[index].1
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

        let src = self.validate_inbounds(src_mem.current_length(), src, len)?;
        let dst = self.validate_inbounds(dst_mem.current_length(), dst, len)?;
        let len = usize::try_from(len).unwrap();

        // Bounds and casts are checked above, by this point we know that
        // everything is safe.
        unsafe {
            let dst = dst_mem.base.add(dst);
            let src = src_mem.base.add(src);
            // FIXME audit whether this is safe in the presence of shared memory
            // (https://github.com/bytecodealliance/wasmtime/issues/4203).
            ptr::copy(src, dst, len);
        }

        Ok(())
    }

    fn validate_inbounds(&self, max: usize, ptr: u64, len: u64) -> Result<usize, Trap> {
        let oob = || Trap::MemoryOutOfBounds;
        let end = ptr
            .checked_add(len)
            .and_then(|i| usize::try_from(i).ok())
            .ok_or_else(oob)?;
        if end > max {
            Err(oob())
        } else {
            Ok(ptr.try_into().unwrap())
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
        let dst = self.validate_inbounds(memory.current_length(), dst, len)?;
        let len = usize::try_from(len).unwrap();

        // Bounds and casts are checked above, by this point we know that
        // everything is safe.
        unsafe {
            let dst = memory.base.add(dst);
            // FIXME audit whether this is safe in the presence of shared memory
            // (https://github.com/bytecodealliance/wasmtime/issues/4203).
            ptr::write_bytes(dst, val, len);
        }

        Ok(())
    }

    /// Get the internal storage range of a particular Wasm data segment.
    pub(crate) fn wasm_data_range(&self, index: DataIndex) -> Range<u32> {
        match self.env_module().passive_data_map.get(&index) {
            Some(range) if !self.dropped_data.contains(index) => range.clone(),
            _ => 0..0,
        }
    }

    /// Given an internal storage range of a Wasm data segment (or subset of a
    /// Wasm data segment), get the data's raw bytes.
    pub(crate) fn wasm_data(&self, range: Range<u32>) -> &[u8] {
        let start = usize::try_from(range.start).unwrap();
        let end = usize::try_from(range.end).unwrap();
        &self.runtime_info.wasm_data()[start..end]
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
        let range = self.wasm_data_range(data_index);
        self.memory_init_segment(memory_index, range, dst, src, len)
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
        let dst = self.validate_inbounds(memory.current_length(), dst, len.into())?;
        let src = self.validate_inbounds(data.len(), src.into(), len.into())?;
        let len = len as usize;

        unsafe {
            let src_start = data.as_ptr().add(src);
            let dst_start = memory.base.add(dst);
            // FIXME audit whether this is safe in the presence of shared memory
            // (https://github.com/bytecodealliance/wasmtime/issues/4203).
            ptr::copy_nonoverlapping(src_start, dst_start, len);
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
        range: impl Iterator<Item = u64>,
    ) -> *mut Table {
        self.with_defined_table_index_and_instance(table_index, |idx, instance| {
            instance.get_defined_table_with_lazy_init(idx, range)
        })
    }

    /// Gets the raw runtime table data structure owned by this instance
    /// given the provided `idx`.
    ///
    /// The `range` specified is eagerly initialized for funcref tables.
    pub fn get_defined_table_with_lazy_init(
        &mut self,
        idx: DefinedTableIndex,
        range: impl Iterator<Item = u64>,
    ) -> *mut Table {
        let elt_ty = self.tables[idx].1.element_type();

        if elt_ty == TableElementType::Func {
            for i in range {
                let value = match self.tables[idx].1.get(None, i) {
                    Some(value) => value,
                    None => {
                        // Out-of-bounds; caller will handle by likely
                        // throwing a trap. No work to do to lazy-init
                        // beyond the end.
                        break;
                    }
                };

                if !value.is_uninit() {
                    continue;
                }

                // The table element `i` is uninitialized and is now being
                // initialized. This must imply that a `precompiled` list of
                // function indices is available for this table. The precompiled
                // list is extracted and then it is consulted with `i` to
                // determine the function that is going to be initialized. Note
                // that `i` may be outside the limits of the static
                // initialization so it's a fallible `get` instead of an index.
                let module = self.env_module();
                let precomputed = match &module.table_initialization.initial_values[idx] {
                    TableInitialValue::Null { precomputed } => precomputed,
                    TableInitialValue::Expr(_) => unreachable!(),
                };
                // Panicking here helps catch bugs rather than silently truncating by accident.
                let func_index = precomputed.get(usize::try_from(i).unwrap()).cloned();
                let func_ref = func_index.and_then(|func_index| self.get_func_ref(func_index));
                self.tables[idx]
                    .1
                    .set(i, TableElement::FuncRef(func_ref))
                    .expect("Table type should match and index should be in-bounds");
            }
        }

        &raw mut self.tables[idx].1
    }

    /// Get a table by index regardless of whether it is locally-defined or an
    /// imported, foreign table.
    pub(crate) fn get_table(&mut self, table_index: TableIndex) -> *mut Table {
        self.with_defined_table_index_and_instance(table_index, |idx, instance| {
            &raw mut instance.tables[idx].1
        })
    }

    /// Get a locally-defined table.
    pub(crate) fn get_defined_table(&mut self, index: DefinedTableIndex) -> *mut Table {
        &raw mut self.tables[index].1
    }

    pub(crate) fn with_defined_table_index_and_instance<R>(
        &mut self,
        index: TableIndex,
        f: impl FnOnce(DefinedTableIndex, &mut Instance) -> R,
    ) -> R {
        if let Some(defined_table_index) = self.env_module().defined_table_index(index) {
            f(defined_table_index, self)
        } else {
            let import = self.imported_table(index);
            unsafe {
                Instance::from_vmctx(import.vmctx, |foreign_instance| {
                    let foreign_table_def = import.from;
                    let foreign_table_index = foreign_instance.table_index(&*foreign_table_def);
                    f(foreign_table_index, foreign_instance)
                })
            }
        }
    }

    /// Initialize the VMContext data associated with this Instance.
    ///
    /// The `VMContext` memory is assumed to be uninitialized; any field
    /// that we need in a certain state will be explicitly written by this
    /// function.
    unsafe fn initialize_vmctx(
        &mut self,
        module: &Module,
        offsets: &VMOffsets<HostPtr>,
        store: StorePtr,
        imports: Imports,
    ) {
        assert!(ptr::eq(module, self.env_module().as_ref()));

        *self.vmctx_plus_offset_mut(offsets.ptr.vmctx_magic()) = VMCONTEXT_MAGIC;
        self.set_callee(None);
        self.set_store(store.as_raw());

        // Initialize shared types
        let types = self.runtime_info.type_ids();
        *self.vmctx_plus_offset_mut(offsets.ptr.vmctx_type_ids_array()) = types.as_ptr();

        // Initialize the built-in functions
        *self.vmctx_plus_offset_mut(offsets.ptr.vmctx_builtin_functions()) =
            &VMBuiltinFunctionsArray::INIT;

        // Initialize the imports
        debug_assert_eq!(imports.functions.len(), module.num_imported_funcs);
        ptr::copy_nonoverlapping(
            imports.functions.as_ptr(),
            self.vmctx_plus_offset_mut(offsets.vmctx_imported_functions_begin()),
            imports.functions.len(),
        );
        debug_assert_eq!(imports.tables.len(), module.num_imported_tables);
        ptr::copy_nonoverlapping(
            imports.tables.as_ptr(),
            self.vmctx_plus_offset_mut(offsets.vmctx_imported_tables_begin()),
            imports.tables.len(),
        );
        debug_assert_eq!(imports.memories.len(), module.num_imported_memories);
        ptr::copy_nonoverlapping(
            imports.memories.as_ptr(),
            self.vmctx_plus_offset_mut(offsets.vmctx_imported_memories_begin()),
            imports.memories.len(),
        );
        debug_assert_eq!(imports.globals.len(), module.num_imported_globals);
        ptr::copy_nonoverlapping(
            imports.globals.as_ptr(),
            self.vmctx_plus_offset_mut(offsets.vmctx_imported_globals_begin()),
            imports.globals.len(),
        );

        // N.B.: there is no need to initialize the funcrefs array because we
        // eagerly construct each element in it whenever asked for a reference
        // to that element. In other words, there is no state needed to track
        // the lazy-init, so we don't need to initialize any state now.

        // Initialize the defined tables
        let mut ptr = self.vmctx_plus_offset_mut(offsets.vmctx_tables_begin());
        for i in 0..module.num_defined_tables() {
            ptr::write(ptr, self.tables[DefinedTableIndex::new(i)].1.vmtable());
            ptr = ptr.add(1);
        }

        // Initialize the defined memories. This fills in both the
        // `defined_memories` table and the `owned_memories` table at the same
        // time. Entries in `defined_memories` hold a pointer to a definition
        // (all memories) whereas the `owned_memories` hold the actual
        // definitions of memories owned (not shared) in the module.
        let mut ptr = self.vmctx_plus_offset_mut(offsets.vmctx_memories_begin());
        let mut owned_ptr = self.vmctx_plus_offset_mut(offsets.vmctx_owned_memories_begin());
        for i in 0..module.num_defined_memories() {
            let defined_memory_index = DefinedMemoryIndex::new(i);
            let memory_index = module.memory_index(defined_memory_index);
            if module.memories[memory_index].shared {
                let def_ptr = self.memories[defined_memory_index]
                    .1
                    .as_shared_memory()
                    .unwrap()
                    .vmmemory_ptr();
                ptr::write(ptr, def_ptr.cast_mut());
            } else {
                ptr::write(owned_ptr, self.memories[defined_memory_index].1.vmmemory());
                ptr::write(ptr, owned_ptr);
                owned_ptr = owned_ptr.add(1);
            }
            ptr = ptr.add(1);
        }

        // Zero-initialize the globals so that nothing is uninitialized memory
        // after this function returns. The globals are actually initialized
        // with their const expression initializers after the instance is fully
        // allocated.
        for (index, _init) in module.global_initializers.iter() {
            ptr::write(self.global_ptr(index), VMGlobalDefinition::new());
        }
    }

    fn wasm_fault(&self, addr: usize) -> Option<WasmFault> {
        let mut fault = None;
        for (_, (_, memory)) in self.memories.iter() {
            let accessible = memory.wasm_accessible();
            if accessible.start <= addr && addr < accessible.end {
                // All linear memories should be disjoint so assert that no
                // prior fault has been found.
                assert!(fault.is_none());
                fault = Some(WasmFault {
                    memory_size: memory.byte_size(),
                    wasm_address: u64::try_from(addr - accessible.start).unwrap(),
                });
            }
        }
        fault
    }
}

/// A handle holding an `Instance` of a WebAssembly module.
#[derive(Debug)]
pub struct InstanceHandle {
    instance: Option<SendSyncPtr<Instance>>,
}

impl InstanceHandle {
    /// Creates an "empty" instance handle which internally has a null pointer
    /// to an instance.
    pub fn null() -> InstanceHandle {
        InstanceHandle { instance: None }
    }

    /// Return a raw pointer to the vmctx used by compiled wasm code.
    #[inline]
    pub fn vmctx(&self) -> *mut VMContext {
        self.instance().vmctx()
    }

    /// Return a reference to a module.
    pub fn module(&self) -> &Arc<Module> {
        self.instance().env_module()
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
    pub fn exports(&self) -> wasmparser::collections::index_map::Iter<String, EntityIndex> {
        self.instance().exports()
    }

    /// Return a reference to the custom state attached to this instance.
    pub fn host_state(&self) -> &dyn Any {
        self.instance().host_state()
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
        range: impl Iterator<Item = u64>,
    ) -> *mut Table {
        let index = self.instance().env_module().table_index(index);
        self.instance_mut().get_table_with_lazy_init(index, range)
    }

    /// Get all tables within this instance.
    ///
    /// Returns both import and defined tables.
    ///
    /// Returns both exported and non-exported tables.
    ///
    /// Gives access to the full tables space.
    pub fn all_tables<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator<Item = (TableIndex, ExportTable)> + 'a {
        let indices = (0..self.module().tables.len())
            .map(|i| TableIndex::new(i))
            .collect::<Vec<_>>();
        indices.into_iter().map(|i| (i, self.get_exported_table(i)))
    }

    /// Return the tables defined in this instance (not imported).
    pub fn defined_tables<'a>(&'a mut self) -> impl ExactSizeIterator<Item = ExportTable> + 'a {
        let num_imported = self.module().num_imported_tables;
        self.all_tables()
            .skip(num_imported)
            .map(|(_i, table)| table)
    }

    /// Get all memories within this instance.
    ///
    /// Returns both import and defined memories.
    ///
    /// Returns both exported and non-exported memories.
    ///
    /// Gives access to the full memories space.
    pub fn all_memories<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator<Item = (MemoryIndex, ExportMemory)> + 'a {
        let indices = (0..self.module().memories.len())
            .map(|i| MemoryIndex::new(i))
            .collect::<Vec<_>>();
        indices
            .into_iter()
            .map(|i| (i, self.get_exported_memory(i)))
    }

    /// Return the memories defined in this instance (not imported).
    pub fn defined_memories<'a>(&'a mut self) -> impl ExactSizeIterator<Item = ExportMemory> + 'a {
        let num_imported = self.module().num_imported_memories;
        self.all_memories()
            .skip(num_imported)
            .map(|(_i, memory)| memory)
    }

    /// Get all globals within this instance.
    ///
    /// Returns both import and defined globals.
    ///
    /// Returns both exported and non-exported globals.
    ///
    /// Gives access to the full globals space.
    pub fn all_globals<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator<Item = (GlobalIndex, ExportGlobal)> + 'a {
        self.instance_mut().all_globals()
    }

    /// Get the globals defined in this instance (not imported).
    pub fn defined_globals<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator<Item = (DefinedGlobalIndex, ExportGlobal)> + 'a {
        self.instance_mut().defined_globals()
    }

    /// Return a reference to the contained `Instance`.
    #[inline]
    pub(crate) fn instance(&self) -> &Instance {
        unsafe { &*self.instance.unwrap().as_ptr() }
    }

    pub(crate) fn instance_mut(&mut self) -> &mut Instance {
        unsafe { &mut *self.instance.unwrap().as_ptr() }
    }

    /// Get this instance's `dyn VMStore` trait object.
    ///
    /// This should only be used for initializing a vmctx's store pointer. It
    /// should never be used to access the store itself. Use `InstanceAndStore`
    /// for that instead.
    pub fn traitobj(&self, store: &StoreOpaque) -> *mut dyn VMStore {
        // By requiring a store argument, we are ensuring that callers aren't
        // getting this trait object in order to access the store, since they
        // already have access. See `InstanceAndStore` and its documentation for
        // details about the store access patterns we want to restrict host code
        // to.
        let _ = store;

        let ptr = unsafe {
            *self
                .instance()
                .vmctx_plus_offset::<*mut dyn VMStore>(self.instance().offsets().ptr.vmctx_store())
        };
        debug_assert!(!ptr.is_null());
        ptr
    }

    /// Configure the `*mut dyn Store` internal pointer after-the-fact.
    ///
    /// This is provided for the original `Store` itself to configure the first
    /// self-pointer after the original `Box` has been initialized.
    pub unsafe fn set_store(&mut self, store: *mut dyn VMStore) {
        self.instance_mut().set_store(Some(store));
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

    /// Performs post-initialization of an instance after its handle has been
    /// created and registered with a store.
    ///
    /// Failure of this function means that the instance still must persist
    /// within the store since failure may indicate partial failure, or some
    /// state could be referenced by other instances.
    pub fn initialize(
        &mut self,
        store: &mut StoreOpaque,
        module: &Module,
        is_bulk_memory: bool,
    ) -> Result<()> {
        allocator::initialize_instance(store, self.instance_mut(), module, is_bulk_memory)
    }

    /// Attempts to convert from the host `addr` specified to a WebAssembly
    /// based address recorded in `WasmFault`.
    ///
    /// This method will check all linear memories that this instance contains
    /// to see if any of them contain `addr`. If one does then `Some` is
    /// returned with metadata about the wasm fault. Otherwise `None` is
    /// returned and `addr` doesn't belong to this instance.
    pub fn wasm_fault(&self, addr: usize) -> Option<WasmFault> {
        self.instance().wasm_fault(addr)
    }
}
