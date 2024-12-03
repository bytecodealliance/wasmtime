//! The null collector.
//!
//! The null collector bump allocates objects until it runs out of space, at
//! which point it returns an out-of-memory error. It never collects garbage.
//! It does not require any GC barriers.

use super::*;
use crate::{
    prelude::*,
    vm::{
        mmap::AlignedLength, ExternRefHostDataId, ExternRefHostDataTable, GarbageCollection,
        GcHeap, GcHeapObject, GcProgress, GcRootsIter, Mmap, SendSyncUnsafeCell, TypedGcRef,
        VMGcHeader, VMGcRef,
    },
    GcHeapOutOfMemory,
};
use core::{
    alloc::Layout,
    any::Any,
    cell::UnsafeCell,
    num::{NonZeroU32, NonZeroUsize},
};
use wasmtime_environ::{
    null::NullTypeLayouts, GcArrayLayout, GcStructLayout, GcTypeLayouts, VMGcKind,
    VMSharedTypeIndex,
};

/// The null collector.
#[derive(Default)]
pub struct NullCollector {
    layouts: NullTypeLayouts,
}

unsafe impl GcRuntime for NullCollector {
    fn layouts(&self) -> &dyn GcTypeLayouts {
        &self.layouts
    }

    fn new_gc_heap(&self) -> Result<Box<dyn GcHeap>> {
        let heap = NullHeap::new()?;
        Ok(Box::new(heap) as _)
    }
}

/// A GC heap for the null collector.
#[repr(C)]
struct NullHeap {
    /// Bump-allocation finger indexing within `1..self.heap.len()`.
    ///
    /// NB: this is an `UnsafeCell` because it is written to by compiled Wasm
    /// code.
    next: SendSyncUnsafeCell<NonZeroU32>,

    /// The number of active no-gc scopes at the current moment.
    no_gc_count: usize,

    /// The actual GC heap.
    heap: Mmap<AlignedLength>,
}

/// The common header for all arrays in the null collector.
#[repr(C)]
struct VMNullArrayHeader {
    header: VMGcHeader,
    length: u32,
}

unsafe impl GcHeapObject for VMNullArrayHeader {
    #[inline]
    fn is(header: &VMGcHeader) -> bool {
        header.kind() == VMGcKind::ArrayRef
    }
}

impl VMNullArrayHeader {
    fn typed_ref<'a>(
        gc_heap: &NullHeap,
        array: &'a VMArrayRef,
    ) -> &'a TypedGcRef<VMNullArrayHeader> {
        let gc_ref = array.as_gc_ref();
        debug_assert!(gc_ref.is_typed::<VMNullArrayHeader>(gc_heap));
        gc_ref.as_typed_unchecked()
    }
}

/// The representation of an `externref` in the null collector.
#[repr(C)]
struct VMNullExternRef {
    header: VMGcHeader,
    host_data: ExternRefHostDataId,
}

unsafe impl GcHeapObject for VMNullExternRef {
    #[inline]
    fn is(header: &VMGcHeader) -> bool {
        header.kind() == VMGcKind::ExternRef
    }
}

impl VMNullExternRef {
    /// Convert a generic `externref` to a typed reference to our concrete
    /// `externref` type.
    fn typed_ref<'a>(
        gc_heap: &NullHeap,
        externref: &'a VMExternRef,
    ) -> &'a TypedGcRef<VMNullExternRef> {
        let gc_ref = externref.as_gc_ref();
        debug_assert!(gc_ref.is_typed::<VMNullExternRef>(gc_heap));
        gc_ref.as_typed_unchecked()
    }
}

fn oom() -> Error {
    GcHeapOutOfMemory::new(()).into()
}

impl NullHeap {
    /// Construct a new, default heap for the null collector.
    fn new() -> Result<Self> {
        Self::with_capacity(super::DEFAULT_GC_HEAP_CAPACITY)
    }

    /// Create a new DRC heap with the given capacity.
    fn with_capacity(capacity: usize) -> Result<Self> {
        let heap = Mmap::with_at_least(capacity)?;
        Ok(Self {
            no_gc_count: 0,
            next: SendSyncUnsafeCell::new(NonZeroU32::new(1).unwrap()),
            heap,
        })
    }

    fn alloc(&mut self, mut header: VMGcHeader, layout: Layout) -> Result<VMGcRef> {
        debug_assert!(layout.size() >= core::mem::size_of::<VMGcHeader>());
        debug_assert!(layout.align() >= core::mem::align_of::<VMGcHeader>());

        // Make sure that the requested allocation's size fits in the GC
        // header's unused bits.
        let size = match u32::try_from(layout.size()).ok().and_then(|size| {
            if VMGcKind::value_fits_in_unused_bits(size) {
                Some(size)
            } else {
                None
            }
        }) {
            Some(size) => size,
            None => return Err(crate::Trap::AllocationTooLarge.into()),
        };

        let next = *self.next.get_mut();

        // Increment the bump pointer to the layout's requested alignment.
        let aligned = match u32::try_from(layout.align())
            .ok()
            .and_then(|align| next.get().checked_next_multiple_of(align))
        {
            Some(aligned) => aligned,
            None => return Err(oom()),
        };

        // Check whether the allocation fits in the heap space we have left.
        let end_of_object = match aligned.checked_add(size) {
            Some(end) => end,
            None => return Err(oom()),
        };
        if u32::try_from(self.heap.len())
            .ok()
            .map_or(true, |heap_len| end_of_object > heap_len)
        {
            return Err(oom());
        }

        // Update the bump pointer, write the header, and return the GC ref.
        *self.next.get_mut() = NonZeroU32::new(end_of_object).unwrap();

        let aligned = NonZeroU32::new(aligned).unwrap();
        let gc_ref = VMGcRef::from_heap_index(aligned).unwrap();

        debug_assert_eq!(header.reserved_u27(), 0);
        header.set_reserved_u27(size);
        *self.header_mut(&gc_ref) = header;

        Ok(gc_ref)
    }
}

unsafe impl GcHeap for NullHeap {
    fn as_any(&self) -> &dyn Any {
        self as _
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as _
    }

    fn enter_no_gc_scope(&mut self) {
        self.no_gc_count += 1;
    }

    fn exit_no_gc_scope(&mut self) {
        self.no_gc_count -= 1;
    }

    fn heap_slice(&self) -> &[UnsafeCell<u8>] {
        let ptr = self.heap.as_ptr().cast();
        let len = self.heap.len();
        unsafe { core::slice::from_raw_parts(ptr, len) }
    }

    fn heap_slice_mut(&mut self) -> &mut [u8] {
        let ptr = self.heap.as_mut_ptr();
        let len = self.heap.len();
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    }

    fn clone_gc_ref(&mut self, gc_ref: &VMGcRef) -> VMGcRef {
        gc_ref.unchecked_copy()
    }

    fn write_gc_ref(
        &mut self,
        _host_data_table: &mut ExternRefHostDataTable,
        destination: &mut Option<VMGcRef>,
        source: Option<&VMGcRef>,
    ) {
        *destination = source.map(|s| s.unchecked_copy());
    }

    fn expose_gc_ref_to_wasm(&mut self, _gc_ref: VMGcRef) {
        // Don't need to do anything special here.
    }

    fn need_gc_before_entering_wasm(&self, _num_gc_refs: NonZeroUsize) -> bool {
        // Never need to GC before entering Wasm.
        false
    }

    fn alloc_externref(&mut self, host_data: ExternRefHostDataId) -> Result<Option<VMExternRef>> {
        let gc_ref = self.alloc(VMGcHeader::externref(), Layout::new::<VMNullExternRef>())?;
        self.index_mut::<VMNullExternRef>(gc_ref.as_typed_unchecked())
            .host_data = host_data;
        Ok(Some(gc_ref.into_externref_unchecked()))
    }

    fn externref_host_data(&self, externref: &VMExternRef) -> ExternRefHostDataId {
        let typed_ref = VMNullExternRef::typed_ref(self, externref);
        self.index(typed_ref).host_data
    }

    fn object_size(&self, gc_ref: &VMGcRef) -> usize {
        let size = self.header(gc_ref).reserved_u27();
        usize::try_from(size).unwrap()
    }

    fn header(&self, gc_ref: &VMGcRef) -> &VMGcHeader {
        self.index(gc_ref.as_typed_unchecked())
    }

    fn header_mut(&mut self, gc_ref: &VMGcRef) -> &mut VMGcHeader {
        self.index_mut(gc_ref.as_typed_unchecked())
    }

    fn alloc_raw(&mut self, header: VMGcHeader, layout: Layout) -> Result<Option<VMGcRef>> {
        self.alloc(header, layout).map(Some)
    }

    fn alloc_uninit_struct(
        &mut self,
        ty: VMSharedTypeIndex,
        layout: &GcStructLayout,
    ) -> Result<Option<VMStructRef>> {
        let gc_ref = self.alloc(
            VMGcHeader::from_kind_and_index(VMGcKind::StructRef, ty),
            layout.layout(),
        )?;
        Ok(Some(gc_ref.into_structref_unchecked()))
    }

    fn dealloc_uninit_struct(&mut self, _struct_ref: VMStructRef) {}

    fn alloc_uninit_array(
        &mut self,
        ty: VMSharedTypeIndex,
        length: u32,
        layout: &GcArrayLayout,
    ) -> Result<Option<VMArrayRef>> {
        let gc_ref = self.alloc(
            VMGcHeader::from_kind_and_index(VMGcKind::ArrayRef, ty),
            layout.layout(length),
        )?;
        self.index_mut::<VMNullArrayHeader>(gc_ref.as_typed_unchecked())
            .length = length;
        Ok(Some(gc_ref.into_arrayref_unchecked()))
    }

    fn dealloc_uninit_array(&mut self, _array_ref: VMArrayRef) {}

    fn array_len(&self, arrayref: &VMArrayRef) -> u32 {
        let arrayref = VMNullArrayHeader::typed_ref(self, arrayref);
        self.index(arrayref).length
    }

    fn gc<'a>(
        &'a mut self,
        _roots: GcRootsIter<'a>,
        _host_data_table: &'a mut ExternRefHostDataTable,
    ) -> Box<dyn GarbageCollection<'a> + 'a> {
        assert_eq!(self.no_gc_count, 0, "Cannot GC inside a no-GC scope!");
        Box::new(NullCollection {})
    }

    unsafe fn vmctx_gc_heap_data(&self) -> *mut u8 {
        self.next.get().cast()
    }

    #[cfg(feature = "pooling-allocator")]
    fn reset(&mut self) {
        let NullHeap {
            next,
            no_gc_count,
            heap: _,
        } = self;

        *next.get_mut() = NonZeroU32::new(1).unwrap();
        *no_gc_count = 0;
    }
}

struct NullCollection {}

impl<'a> GarbageCollection<'a> for NullCollection {
    fn collect_increment(&mut self) -> GcProgress {
        GcProgress::Complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_gc_null_header_size_align() {
        assert_eq!(
            (wasmtime_environ::null::HEADER_SIZE as usize),
            core::mem::size_of::<VMGcHeader>()
        );
        assert_eq!(
            (wasmtime_environ::null::HEADER_ALIGN as usize),
            core::mem::align_of::<VMGcHeader>()
        );
    }

    #[test]
    fn vm_null_array_header_length_offset() {
        assert_eq!(
            wasmtime_environ::null::ARRAY_LENGTH_OFFSET,
            u32::try_from(core::mem::offset_of!(VMNullArrayHeader, length)).unwrap(),
        );
    }
}
