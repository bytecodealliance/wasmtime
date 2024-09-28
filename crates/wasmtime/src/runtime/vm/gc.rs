#[cfg(feature = "gc")]
mod enabled;
#[cfg(feature = "gc")]
pub use enabled::*;

#[cfg(not(feature = "gc"))]
mod disabled;
#[cfg(not(feature = "gc"))]
pub use disabled::*;

mod func_ref;
mod gc_ref;
mod gc_runtime;
mod host_data;
mod i31;

pub use func_ref::*;
pub use gc_ref::*;
pub use gc_runtime::*;
pub use host_data::*;
pub use i31::*;

use crate::prelude::*;
use crate::runtime::vm::GcHeapAllocationIndex;
use core::alloc::Layout;
use core::ptr;
use core::{any::Any, num::NonZeroUsize};
use wasmtime_environ::{GcArrayLayout, GcStructLayout, VMGcKind, VMSharedTypeIndex};

/// GC-related data that is one-to-one with a `wasmtime::Store`.
///
/// Contains everything we need to do collections, invoke barriers, etc...
///
/// In general, exposes a very similar interface to `GcHeap`, but fills in some
/// of the context arguments for callers (such as the `ExternRefHostDataTable`)
/// since they are all stored together inside `GcStore`.
pub struct GcStore {
    /// This GC heap's allocation index (primarily used for integrating with the
    /// pooling allocator).
    pub allocation_index: GcHeapAllocationIndex,

    /// The actual GC heap.
    pub gc_heap: Box<dyn GcHeap>,

    /// The `externref` host data table for this GC heap.
    pub host_data_table: ExternRefHostDataTable,

    /// The function-references table for this GC heap.
    pub func_ref_table: FuncRefTable,
}

impl GcStore {
    /// Create a new `GcStore`.
    pub fn new(allocation_index: GcHeapAllocationIndex, gc_heap: Box<dyn GcHeap>) -> Self {
        let host_data_table = ExternRefHostDataTable::default();
        let func_ref_table = FuncRefTable::default();
        Self {
            allocation_index,
            gc_heap,
            host_data_table,
            func_ref_table,
        }
    }

    /// Perform garbage collection within this heap.
    pub fn gc(&mut self, roots: GcRootsIter<'_>) {
        let mut collection = self.gc_heap.gc(roots, &mut self.host_data_table);
        collection.collect();
    }

    /// Asynchronously perform garbage collection within this heap.
    #[cfg(feature = "async")]
    pub async fn gc_async(&mut self, roots: GcRootsIter<'_>) {
        let collection = self.gc_heap.gc(roots, &mut self.host_data_table);
        collect_async(collection).await;
    }

    /// Get the kind of the given GC reference.
    pub fn kind(&self, gc_ref: &VMGcRef) -> VMGcKind {
        debug_assert!(!gc_ref.is_i31());
        self.header(gc_ref).kind()
    }

    /// Get the header of the given GC reference.
    pub fn header(&self, gc_ref: &VMGcRef) -> &VMGcHeader {
        debug_assert!(!gc_ref.is_i31());
        self.gc_heap.header(gc_ref)
    }

    /// Clone a GC reference, calling GC write barriers as necessary.
    pub fn clone_gc_ref(&mut self, gc_ref: &VMGcRef) -> VMGcRef {
        if gc_ref.is_i31() {
            gc_ref.unchecked_copy()
        } else {
            self.gc_heap.clone_gc_ref(gc_ref)
        }
    }

    /// Write the `source` GC reference into the `destination` slot, performing
    /// write barriers as necessary.
    pub fn write_gc_ref(&mut self, destination: &mut Option<VMGcRef>, source: Option<&VMGcRef>) {
        // If neither the source nor destination actually point to a GC object
        // (that is, they are both either null or `i31ref`s) then we can skip
        // the GC barrier.
        if destination.as_ref().map_or(true, |d| d.is_i31())
            && source.as_ref().map_or(true, |s| s.is_i31())
        {
            *destination = source.map(|s| s.unchecked_copy());
            return;
        }

        self.gc_heap
            .write_gc_ref(&mut self.host_data_table, destination, source);
    }

    /// Drop the given GC reference, performing drop barriers as necessary.
    pub fn drop_gc_ref(&mut self, gc_ref: VMGcRef) {
        if !gc_ref.is_i31() {
            self.gc_heap.drop_gc_ref(&mut self.host_data_table, gc_ref);
        }
    }

    /// Hook to call whenever a GC reference is about to be exposed to Wasm.
    pub fn expose_gc_ref_to_wasm(&mut self, gc_ref: VMGcRef) {
        if !gc_ref.is_i31() {
            log::trace!("exposing GC ref to Wasm: {gc_ref:p}");
            self.gc_heap.expose_gc_ref_to_wasm(gc_ref);
        }
    }

    /// Allocate a new `externref`.
    ///
    /// Returns:
    ///
    /// * `Ok(Ok(_))`: Successfully allocated the `externref`.
    ///
    /// * `Ok(Err(value))`: Failed to allocate the `externref`, but doing a GC
    ///   and then trying again may succeed. Returns the given `value` as the
    ///   error payload.
    ///
    /// * `Err(_)`: Unrecoverable allocation failure.
    pub fn alloc_externref(
        &mut self,
        value: Box<dyn Any + Send + Sync>,
    ) -> Result<Result<VMExternRef, Box<dyn Any + Send + Sync>>> {
        let host_data_id = self.host_data_table.alloc(value);
        match self.gc_heap.alloc_externref(host_data_id)? {
            #[cfg_attr(not(feature = "gc"), allow(unreachable_patterns))]
            Some(x) => Ok(Ok(x)),
            None => Ok(Err(self.host_data_table.dealloc(host_data_id))),
        }
    }

    /// Get a shared borrow of the given `externref`'s host data.
    ///
    /// Passing invalid `VMExternRef`s (eg garbage values or `externref`s
    /// associated with a different heap is memory safe but will lead to general
    /// incorrectness such as panics and wrong results.
    pub fn externref_host_data(&self, externref: &VMExternRef) -> &(dyn Any + Send + Sync) {
        let host_data_id = self.gc_heap.externref_host_data(externref);
        self.host_data_table.get(host_data_id)
    }

    /// Get a mutable borrow of the given `externref`'s host data.
    ///
    /// Passing invalid `VMExternRef`s (eg garbage values or `externref`s
    /// associated with a different heap is memory safe but will lead to general
    /// incorrectness such as panics and wrong results.
    pub fn externref_host_data_mut(
        &mut self,
        externref: &VMExternRef,
    ) -> &mut (dyn Any + Send + Sync) {
        let host_data_id = self.gc_heap.externref_host_data(externref);
        self.host_data_table.get_mut(host_data_id)
    }

    /// Allocate a raw object with the given header and layout.
    pub fn alloc_raw(&mut self, header: VMGcHeader, layout: Layout) -> Result<Option<VMGcRef>> {
        self.gc_heap.alloc_raw(header, layout)
    }

    /// Allocate an uninitialized struct with the given type index and layout.
    ///
    /// This does NOT check that the index is currently allocated in the types
    /// registry or that the layout matches the index's type. Failure to uphold
    /// those invariants is memory safe, but will lead to general incorrectness
    /// such as panics and wrong results.
    pub fn alloc_uninit_struct(
        &mut self,
        ty: VMSharedTypeIndex,
        layout: &GcStructLayout,
    ) -> Result<Option<VMStructRef>> {
        self.gc_heap.alloc_uninit_struct(ty, layout)
    }

    /// Deallocate an uninitialized struct.
    pub fn dealloc_uninit_struct(&mut self, structref: VMStructRef) {
        self.gc_heap.dealloc_uninit_struct(structref);
    }

    /// Get the data for the given object reference.
    ///
    /// Panics when the structref and its size is out of the GC heap bounds.
    pub fn gc_object_data(&mut self, gc_ref: &VMGcRef) -> VMGcObjectDataMut<'_> {
        self.gc_heap.gc_object_data(gc_ref)
    }

    /// Allocate an uninitialized array with the given type index.
    ///
    /// This does NOT check that the index is currently allocated in the types
    /// registry or that the layout matches the index's type. Failure to uphold
    /// those invariants is memory safe, but will lead to general incorrectness
    /// such as panics and wrong results.
    pub fn alloc_uninit_array(
        &mut self,
        ty: VMSharedTypeIndex,
        len: u32,
        layout: &GcArrayLayout,
    ) -> Result<Option<VMArrayRef>> {
        self.gc_heap.alloc_uninit_array(ty, len, layout)
    }

    /// Deallocate an uninitialized array.
    pub fn dealloc_uninit_array(&mut self, arrayref: VMArrayRef) {
        self.gc_heap.dealloc_uninit_array(arrayref);
    }

    /// Get the length of the given array.
    pub fn array_len(&self, arrayref: &VMArrayRef) -> u32 {
        self.gc_heap.array_len(arrayref)
    }
}

/// Get a no-op GC heap for when GC is disabled (either statically at compile
/// time or dynamically due to it being turned off in the `wasmtime::Config`).
pub fn disabled_gc_heap() -> Box<dyn GcHeap> {
    return Box::new(DisabledGcHeap);
}

pub(crate) struct DisabledGcHeap;

unsafe impl GcHeap for DisabledGcHeap {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn enter_no_gc_scope(&mut self) {}
    fn exit_no_gc_scope(&mut self) {}
    fn header(&self, _gc_ref: &VMGcRef) -> &VMGcHeader {
        unreachable!()
    }
    fn clone_gc_ref(&mut self, _gc_ref: &VMGcRef) -> VMGcRef {
        unreachable!()
    }
    fn write_gc_ref(
        &mut self,
        _host_data_table: &mut ExternRefHostDataTable,
        _destination: &mut Option<VMGcRef>,
        _source: Option<&VMGcRef>,
    ) {
        unreachable!()
    }
    fn expose_gc_ref_to_wasm(&mut self, _gc_ref: VMGcRef) {
        unreachable!()
    }
    fn need_gc_before_entering_wasm(&self, _num_gc_refs: NonZeroUsize) -> bool {
        unreachable!()
    }
    fn alloc_externref(&mut self, _host_data: ExternRefHostDataId) -> Result<Option<VMExternRef>> {
        bail!(
            "GC support disabled either in the `Config` or at compile time \
             because the `gc` cargo feature was not enabled"
        )
    }
    fn externref_host_data(&self, _externref: &VMExternRef) -> ExternRefHostDataId {
        unreachable!()
    }
    fn alloc_raw(&mut self, _header: VMGcHeader, _layout: Layout) -> Result<Option<VMGcRef>> {
        bail!(
            "GC support disabled either in the `Config` or at compile time \
             because the `gc` cargo feature was not enabled"
        )
    }
    fn alloc_uninit_struct(
        &mut self,
        _ty: wasmtime_environ::VMSharedTypeIndex,
        _layout: &GcStructLayout,
    ) -> Result<Option<VMStructRef>> {
        bail!(
            "GC support disabled either in the `Config` or at compile time \
             because the `gc` cargo feature was not enabled"
        )
    }
    fn dealloc_uninit_struct(&mut self, _structref: VMStructRef) {
        unreachable!()
    }
    fn gc_object_data(&mut self, _gc_ref: &VMGcRef) -> VMGcObjectDataMut<'_> {
        unreachable!()
    }
    fn alloc_uninit_array(
        &mut self,
        _ty: VMSharedTypeIndex,
        _len: u32,
        _layout: &GcArrayLayout,
    ) -> Result<Option<VMArrayRef>> {
        bail!(
            "GC support disabled either in the `Config` or at compile time \
             because the `gc` cargo feature was not enabled"
        )
    }
    fn dealloc_uninit_array(&mut self, _structref: VMArrayRef) {
        unreachable!()
    }
    fn array_len(&self, _arrayref: &VMArrayRef) -> u32 {
        unreachable!()
    }
    fn gc<'a>(
        &'a mut self,
        _roots: GcRootsIter<'a>,
        _host_data_table: &'a mut ExternRefHostDataTable,
    ) -> Box<dyn GarbageCollection<'a> + 'a> {
        return Box::new(NoGc);

        struct NoGc;

        impl<'a> GarbageCollection<'a> for NoGc {
            fn collect_increment(&mut self) -> GcProgress {
                GcProgress::Complete
            }
        }
    }
    unsafe fn vmctx_gc_heap_base(&self) -> *mut u8 {
        ptr::null_mut()
    }
    unsafe fn vmctx_gc_heap_bound(&self) -> usize {
        0
    }
    unsafe fn vmctx_gc_heap_data(&self) -> *mut u8 {
        ptr::null_mut()
    }
    #[cfg(feature = "pooling-allocator")]
    fn reset(&mut self) {}
}
