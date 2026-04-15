//! The copying (semi-space) garbage collector.
//!
//! This implements a Cheney-style semi-space copying collector. The GC heap is
//! divided into two halves: the "active" semi-space where new objects are
//! allocated, and the "idle" semi-space. During collection, live objects are
//! copied from the idle space (which was the previous active space) to the new
//! active space, and all roots are updated to point to the new locations.
//!
//! Allocation is a simple bump pointer within the active semi-space.
//!
//! This collector does not require any read or write barriers.

use super::VMArrayRef;
use super::trace_info::{TraceInfo, TraceInfos};
use crate::runtime::vm::{
    ExternRefHostDataId, ExternRefHostDataTable, GarbageCollection, GcHeap, GcHeapObject,
    GcProgress, GcRootsIter, GcRuntime, TypedGcRef, VMExternRef, VMGcHeader, VMGcRef,
};
use crate::vm::VMMemoryDefinition;
use crate::{Engine, prelude::*};
use core::sync::atomic::AtomicUsize;
use core::{alloc::Layout, any::Any, mem, num::NonZeroU32, ptr::NonNull};
use wasmtime_environ::copying::{
    ALIGN, ARRAY_LENGTH_OFFSET, CopyingTypeLayouts, HEADER_COPIED_BIT,
};
use wasmtime_environ::{
    GcArrayLayout, GcStructLayout, GcTypeLayouts, POISON, VMGcKind, VMSharedTypeIndex, gc_assert,
};

#[expect(clippy::cast_possible_truncation, reason = "known to not overflow")]
const GC_REF_ARRAY_ELEMS_OFFSET: u32 = ARRAY_LENGTH_OFFSET + (mem::size_of::<u32>() as u32);

/// The copying collector.
#[derive(Default)]
pub struct CopyingCollector {
    layouts: CopyingTypeLayouts,
}

unsafe impl GcRuntime for CopyingCollector {
    fn layouts(&self) -> &dyn GcTypeLayouts {
        &self.layouts
    }

    fn new_gc_heap(&self, engine: &Engine) -> Result<Box<dyn GcHeap>> {
        let heap = CopyingHeap::new(engine)?;
        Ok(Box::new(heap) as _)
    }
}

/// The common header for all objects in the copying collector.
#[repr(C)]
struct VMCopyingHeader {
    header: VMGcHeader,
    object_size: u32,
}

// Safety: All copying collector objects have a `VMCopyingHeader`.
unsafe impl GcHeapObject for VMCopyingHeader {
    #[inline]
    fn is(_header: &VMGcHeader) -> bool {
        true
    }
}

impl VMCopyingHeader {
    /// Get the size of this object in the GC heap.
    #[inline]
    fn object_size(&self) -> u32 {
        self.object_size
    }

    /// Check whether this object has been copied to the new semi-space during
    /// a collection.
    #[inline]
    fn copied(&self) -> bool {
        self.header.reserved_u26() & HEADER_COPIED_BIT != 0
    }

    /// Mark this object as having been copied to the new semi-space.
    #[inline]
    fn set_copied(&mut self) {
        let reserved = self.header.reserved_u26();
        self.header.set_reserved_u26(reserved | HEADER_COPIED_BIT);
    }
}

/// A copying collector header together with a forwarding reference.
///
/// During collection, after an object has been copied to the new semi-space,
/// its old location is overwritten with the forwarding reference pointing to
/// the new location.
#[repr(C)]
struct VMCopyingHeaderAndForwardingRef {
    header: VMCopyingHeader,
    forwarding_ref: Option<VMGcRef>,
}

// Safety: All copying collector objects have a `VMCopyingHeader` and space for
// the forwarding reference.
unsafe impl GcHeapObject for VMCopyingHeaderAndForwardingRef {
    #[inline]
    fn is(_header: &VMGcHeader) -> bool {
        true
    }
}

impl VMCopyingHeaderAndForwardingRef {
    /// Get the forwarding reference for this object, if it has been copied
    /// during the current collection.
    fn forwarding_ref(&self) -> Option<VMGcRef> {
        debug_assert!(
            self.header.object_size()
                >= u32::try_from(mem::size_of::<VMCopyingHeaderAndForwardingRef>()).unwrap()
        );
        if self.header.copied() {
            Some(
                self.forwarding_ref
                    .as_ref()
                    .expect("should always have a forwarding ref if the copied bit is set")
                    .unchecked_copy(),
            )
        } else {
            None
        }
    }

    /// Set the forwarding reference for this object and mark it as copied.
    fn set_forwarding_ref(&mut self, forwarding_ref: VMGcRef) {
        debug_assert!(!self.header.copied());
        debug_assert!(
            self.header.object_size()
                >= u32::try_from(mem::size_of::<VMCopyingHeaderAndForwardingRef>()).unwrap()
        );
        self.header.set_copied();
        self.forwarding_ref = Some(forwarding_ref);
    }
}

/// The header for an array in the copying collector.
#[repr(C)]
struct VMCopyingArrayHeader {
    header: VMCopyingHeader,
    length: u32,
}

unsafe impl GcHeapObject for VMCopyingArrayHeader {
    #[inline]
    fn is(header: &VMGcHeader) -> bool {
        header.kind() == VMGcKind::ArrayRef
    }
}

/// The representation of an `externref` in the copying collector.
#[repr(C)]
struct VMCopyingExternRef {
    /// NB: Explicitly leave room for the forwarding ref so that our other
    /// fields aren't overwritten after copying to the new semi-space.
    header: VMCopyingHeaderAndForwardingRef,

    /// The ID of this ref's data in the `ExternRefHostDataTable`.
    host_data: ExternRefHostDataId,

    /// Link to the next `externref` in this semi-space.
    next_extern_ref: Option<VMExternRef>,
}

unsafe impl GcHeapObject for VMCopyingExternRef {
    #[inline]
    fn is(header: &VMGcHeader) -> bool {
        header.kind() == VMGcKind::ExternRef
    }
}

/// Get a typed reference to a copying-collector object from a raw `VMGcRef`.
fn copying_ref(gc_ref: &VMGcRef) -> &TypedGcRef<VMCopyingHeader> {
    debug_assert!(!gc_ref.is_i31());
    gc_ref.as_typed_unchecked()
}

/// Get a typed reference to a forwarding-ref header from a raw `VMGcRef`.
fn header_and_forwarding_ref(gc_ref: &VMGcRef) -> &TypedGcRef<VMCopyingHeaderAndForwardingRef> {
    debug_assert!(!gc_ref.is_i31());
    gc_ref.as_typed_unchecked()
}

fn externref_to_copying(externref: &VMExternRef) -> &TypedGcRef<VMCopyingExternRef> {
    let gc_ref = externref.as_gc_ref();
    debug_assert!(!gc_ref.is_i31());
    gc_ref.as_typed_unchecked()
}

/// A copying (semi-space) heap.
struct CopyingHeap {
    /// For every type that we have allocated in this heap, how do we trace it?
    trace_infos: TraceInfos,

    /// Count of how many no-gc scopes we are currently within.
    no_gc_count: u64,

    /// The storage for the GC heap itself.
    memory: Option<crate::vm::Memory>,

    /// The cached `VMMemoryDefinition` for `self.memory` so that we don't have
    /// to make indirect calls through a `dyn RuntimeLinearMemory` object.
    ///
    /// Must be updated and kept in sync with `self.memory`, cleared when the
    /// memory is taken and updated when the memory is replaced.
    vmmemory: Option<VMMemoryDefinition>,

    /// The bump "pointer" (really an index) for allocating new objects.
    ///
    /// This is always within the active semi-space.
    bump_ptr: u32,

    /// The start of the active semi-space.
    active_space_start: u32,

    /// The end of the active semi-space.
    active_space_end: u32,

    /// The start of the idle semi-space.
    idle_space_start: u32,

    /// The end of the idle semi-space.
    idle_space_end: u32,

    /// "Pointer" (really an index) to the start of the worklist.
    ///
    /// This is always within the active semi-space and is always less than or
    /// equal to `bump_ptr`.
    ///
    /// This is used to implement a Cheney-style worklist: grey objects (the set
    /// of objects that have been copied to the new semi-space but have not yet
    /// been scanned) are always within `worklist_ptr..bump_ptr`. The worklist
    /// is empty when `worklist_ptr == bump_ptr` and we can pop from the
    /// worklist by advancing `worklist_ptr`.
    worklist_ptr: u32,

    /// The set of `externref`s in the active semi-space.
    ///
    /// The set is implemented as an intrusive linked-list, and this is the
    /// head of the list.
    active_extern_ref_set_head: Option<VMExternRef>,

    /// Like `active_extern_ref_set_head` but for the idle semi-space.
    idle_extern_ref_set_head: Option<VMExternRef>,
}

impl CopyingHeap {
    fn new(engine: &Engine) -> Result<Self> {
        log::trace!("allocating new copying heap");
        Ok(Self {
            trace_infos: TraceInfos::new(engine, GC_REF_ARRAY_ELEMS_OFFSET),
            no_gc_count: 0,
            memory: None,
            vmmemory: None,
            bump_ptr: 0,
            active_space_start: 0,
            active_space_end: 0,
            idle_space_start: 0,
            idle_space_end: 0,
            worklist_ptr: 0,
            active_extern_ref_set_head: None,
            idle_extern_ref_set_head: None,
        })
    }

    fn capacity(&self) -> u32 {
        let len = self.vmmemory.as_ref().unwrap().current_length();
        let len = u32::try_from(len).unwrap_or(u32::MAX);
        // Round down to a multiple of `ALIGN` so our semi-spaces are
        // equal-sized.
        let len = len & !(ALIGN - 1);
        len
    }

    /// Initialize the semi-spaces for a heap of the given capacity.
    fn initialize_semi_spaces(&mut self) {
        debug_assert_eq!(self.active_space_start, 0);
        debug_assert_eq!(self.active_space_end, 0);
        debug_assert_eq!(self.idle_space_start, 0);
        debug_assert_eq!(self.idle_space_end, 0);
        debug_assert_eq!(self.bump_ptr, 0);

        self.resize_semi_spaces();
        self.reset_bump_ptr();
    }

    fn resize_semi_spaces(&mut self) {
        debug_assert_eq!(
            self.active_space_end - self.active_space_start,
            self.idle_space_end - self.idle_space_start,
            "the active and idle spaces should be the same size"
        );

        // We only adjust the semi-space regions for new memory capacity if the
        // active semi-space is the first half of the GC heap. Else, when the
        // the second half of the GC heap is the active semi-space, wait until
        // the next collection to automatically update the regions.
        if self.idle_space_start < self.active_space_start {
            return;
        }

        let capacity = self.capacity();
        let halfway = capacity / 2;

        debug_assert!(self.bump_ptr <= halfway);
        debug_assert!(self.idle_space_start <= halfway);
        debug_assert!(self.active_space_end <= halfway);

        self.active_space_end = halfway;
        self.idle_space_start = halfway;
        self.idle_space_end = capacity;

        debug_assert_eq!(
            self.active_space_end - self.active_space_start,
            self.idle_space_end - self.idle_space_start,
            "the active and idle spaces should be the same size"
        );
    }

    fn reset_bump_ptr(&mut self) {
        // We always need to keep `bump_ptr` aligned to `ALIGN`.
        //
        // When the active space is the first half of the GC heap, we need to
        // skip past index 0, since `VMGcRef` is a `NonZeroU32`.
        //
        // When the active space is the second half of the GC heap, we *also*
        // skip the first `ALIGN` bytes. This ensures that the active and idle
        // spaces are always equally sized, which is required to guarantee that
        // evacuating objects from one to the other will succeed.
        self.bump_ptr = self.active_space_start;
        if self.active_space_end - self.active_space_start >= ALIGN {
            self.bump_ptr += ALIGN;
        }
        debug_assert!(self.bump_ptr.is_multiple_of(ALIGN));
    }

    /// Ensure that we have tracing information for the given type.
    fn ensure_trace_info(&mut self, ty: VMSharedTypeIndex) {
        self.trace_infos.ensure(ty);
    }

    /// Allocate `size` bytes from the active semi-space bump pointer.
    ///
    /// Returns `None` if there isn't enough room.
    fn allocate(&mut self, size: u32) -> Option<u32> {
        debug_assert!(size.is_multiple_of(ALIGN));
        debug_assert!(self.bump_ptr.is_multiple_of(ALIGN));
        debug_assert!(self.bump_ptr >= self.active_space_start);
        debug_assert!(self.bump_ptr <= self.active_space_end);

        let result = self.bump_ptr;
        let new_bump_ptr = result.checked_add(size)?;
        if new_bump_ptr > self.active_space_end {
            return None;
        }

        self.bump_ptr = new_bump_ptr;
        debug_assert!(self.bump_ptr.is_multiple_of(ALIGN));
        debug_assert!(self.bump_ptr >= self.active_space_start);
        debug_assert!(self.bump_ptr <= self.active_space_end);

        Some(result)
    }

    /// Check whether an index is within the active semi-space.
    fn is_in_active_space(&self, index: u32) -> bool {
        index >= self.active_space_start && index < self.active_space_end
    }

    /// Check whether an index is within the idle semi-space.
    fn is_in_idle_space(&self, index: u32) -> bool {
        index >= self.idle_space_start && index < self.idle_space_end
    }

    /// Swap the active and idle semi-spaces.
    fn flip(&mut self) {
        debug_assert_eq!(
            self.active_space_end - self.active_space_start,
            self.idle_space_end - self.idle_space_start,
            "the active and idle spaces should be the same size"
        );

        mem::swap(&mut self.active_space_start, &mut self.idle_space_start);
        mem::swap(&mut self.active_space_end, &mut self.idle_space_end);
        self.reset_bump_ptr();

        // The active idle list becomes the old active list; the active list is
        // cleared because we are starting fresh in the new space.
        self.idle_extern_ref_set_head = self.active_extern_ref_set_head.take();
    }

    /// Initialize the worklist at the start of a collection.
    fn initialize_worklist(&mut self) {
        self.worklist_ptr = self.bump_ptr;
    }

    /// Pop the next item off the worklist, or return `None` if the worklist is
    /// empty.
    fn worklist_pop(&mut self) -> Option<VMGcRef> {
        debug_assert!(
            self.is_in_active_space(self.worklist_ptr)
                || self.worklist_ptr == self.active_space_end
        );
        debug_assert!(
            self.is_in_active_space(self.bump_ptr) || self.bump_ptr == self.active_space_end
        );
        debug_assert!(self.worklist_ptr <= self.bump_ptr);

        if self.worklist_ptr == self.bump_ptr {
            return None;
        }

        let result = self.worklist_ptr;
        let result = NonZeroU32::new(result).unwrap();
        let result = VMGcRef::from_heap_index(result).unwrap();

        let obj_size = self.index(copying_ref(&result)).object_size();

        self.worklist_ptr += obj_size;
        debug_assert!(self.worklist_ptr <= self.bump_ptr);

        Some(result)
    }

    /// Insert `gc_ref`, which points to a grey object, into the worklist.
    fn worklist_insert(&self, gc_ref: &VMGcRef) {
        // This is a no-op: insertion happens implicitly in `copy` when
        // allocating space for the copied object and advancing the
        // `bump_ptr`. But we still define and call this method just for the
        // debug assertions.
        if !cfg!(debug_assertions) {
            return;
        }

        let index = gc_ref.as_heap_index().unwrap().get();
        debug_assert!(self.is_in_active_space(index));
        let obj_size = self.index(copying_ref(gc_ref)).object_size();
        debug_assert_eq!(index + obj_size, self.bump_ptr);
        debug_assert!(self.worklist_ptr <= index);
    }

    /// Get-or-create the location of this idle-space ref in the new active
    /// semi-space.
    fn forward(&mut self, from_ref: &VMGcRef) -> VMGcRef {
        debug_assert!(!from_ref.is_i31());
        debug_assert!(self.is_in_idle_space(from_ref.as_heap_index().unwrap().get()));

        if let Some(to_ref) = self
            .index(header_and_forwarding_ref(from_ref))
            .forwarding_ref()
        {
            return to_ref;
        }
        self.copy(from_ref)
    }

    /// Copy this idle-space ref into the new active semi-space and return its
    /// new location.
    fn copy(&mut self, from_ref: &VMGcRef) -> VMGcRef {
        debug_assert!(!from_ref.is_i31());
        let from_index = from_ref.as_heap_index().unwrap().get();
        debug_assert!(self.is_in_idle_space(from_index));
        debug_assert!(!self.index(copying_ref(from_ref)).copied());

        let size = self.index(copying_ref(from_ref)).object_size();
        let to_index = self.allocate(size).expect(
            "there should always be enough room in the active semi-space for objects that \
             survived collection, since the active space is the same size as the idle space",
        );
        debug_assert!(self.is_in_active_space(to_index));

        let to_ref =
            VMGcRef::from_heap_index(NonZeroU32::new(to_index).unwrap()).expect("valid heap index");

        // Copy the object bytes.
        let from_start = usize::try_from(from_index).unwrap();
        let to_start = usize::try_from(to_index).unwrap();
        let size_usize = usize::try_from(size).unwrap();
        self.heap_slice_mut()
            .copy_within(from_start..from_start + size_usize, to_start);

        // Set the forwarding ref in the old (idle-space) object.
        self.index_mut(header_and_forwarding_ref(from_ref))
            .set_forwarding_ref(to_ref.unchecked_copy());

        // If this is an externref, insert it into the active externref list.
        if self
            .index(copying_ref(&to_ref))
            .header
            .kind()
            .matches(VMGcKind::ExternRef)
        {
            let old_head = self.active_extern_ref_set_head.take();
            self.index_mut::<VMCopyingExternRef>(to_ref.as_typed_unchecked())
                .next_extern_ref = old_head;
            self.active_extern_ref_set_head =
                Some(to_ref.unchecked_copy().into_externref_unchecked());
        }

        self.worklist_insert(&to_ref);
        to_ref
    }

    /// Trace a grey object's outgoing edges, copying their referents into the
    /// new semi-space if necessary, and updating the object's references with
    /// their forwarded locations in the new semi-space.
    fn scan(&mut self, gc_ref: &VMGcRef, trace_infos: &TraceInfos) {
        debug_assert!(!gc_ref.is_i31());
        let index = gc_ref.as_heap_index().unwrap().get();
        debug_assert!(self.is_in_active_space(index));

        let ty = self.index(copying_ref(gc_ref)).header.ty();

        // `externref`s have no GC edges.
        let Some(ty) = ty else {
            return;
        };

        let object_start = usize::try_from(index).unwrap();
        match trace_infos.trace_info(&ty) {
            TraceInfo::Struct { gc_ref_offsets } => {
                for &offset in gc_ref_offsets {
                    self.scan_field(object_start, offset);
                }
            }
            TraceInfo::Array { gc_ref_elems } => {
                if *gc_ref_elems {
                    let array_ref = gc_ref.as_arrayref_unchecked();
                    let len = self.array_len(array_ref);

                    for i in 0..len {
                        let elem_offset = GC_REF_ARRAY_ELEMS_OFFSET
                            + i * u32::try_from(mem::size_of::<u32>()).unwrap();
                        self.scan_field(object_start, elem_offset);
                    }
                }
            }
        }
    }

    #[inline]
    fn scan_field(&mut self, object_start: usize, offset: u32) {
        let offset = usize::try_from(offset).unwrap();
        let field_start = object_start + offset;
        let field_end = field_start + mem::size_of::<u32>();

        let raw: [u8; 4] = self.heap_slice()[field_start..field_end]
            .try_into()
            .unwrap();
        let raw = u32::from_le_bytes(raw);

        if let Some(child) = VMGcRef::from_raw_u32(raw)
            && !child.is_i31()
        {
            debug_assert!(self.is_in_idle_space(child.as_heap_index().unwrap().get()));
            let new_ref = self.forward(&child);
            debug_assert!(self.is_in_active_space(new_ref.as_heap_index().unwrap().get()));
            // Write the new reference back.
            let new_raw = new_ref.as_raw_u32().to_le_bytes();
            self.heap_slice_mut()[field_start..field_end].copy_from_slice(&new_raw);
        }
    }
}

unsafe impl GcHeap for CopyingHeap {
    fn is_attached(&self) -> bool {
        debug_assert_eq!(self.memory.is_some(), self.vmmemory.is_some());
        self.memory.is_some()
    }

    fn attach(&mut self, memory: crate::vm::Memory) {
        assert!(!self.is_attached());
        assert!(!memory.is_shared_memory());
        self.vmmemory = Some(memory.vmmemory());
        self.memory = Some(memory);
        self.initialize_semi_spaces();

        if cfg!(gc_zeal) {
            self.heap_slice_mut().fill(POISON);
        }
    }

    fn detach(&mut self) -> crate::vm::Memory {
        assert!(self.is_attached());

        let CopyingHeap {
            no_gc_count,
            memory,
            vmmemory,
            bump_ptr,
            active_space_start,
            active_space_end,
            idle_space_start,
            idle_space_end,
            worklist_ptr,
            active_extern_ref_set_head,
            idle_extern_ref_set_head,
            // NB: we will only ever be reused with the same engine, so no need
            // to clear out our tracing info just to fill it back in with the
            // same exact stuff.
            trace_infos: _,
        } = self;

        *no_gc_count = 0;
        *vmmemory = None;
        *bump_ptr = 0;
        *active_space_start = 0;
        *active_space_end = 0;
        *idle_space_start = 0;
        *idle_space_end = 0;
        *worklist_ptr = 0;
        *active_extern_ref_set_head = None;
        *idle_extern_ref_set_head = None;

        memory.take().unwrap()
    }

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

    fn clone_gc_ref(&mut self, gc_ref: &VMGcRef) -> VMGcRef {
        // The copying collector doesn't use reference counting; cloning is a
        // simple copy.
        gc_ref.unchecked_copy()
    }

    fn write_gc_ref(
        &mut self,
        _host_data_table: &mut ExternRefHostDataTable,
        destination: &mut Option<VMGcRef>,
        source: Option<&VMGcRef>,
    ) {
        // The copying collector doesn't use reference counting; writes are
        // simple overwrites.
        *destination = source.map(|s| s.unchecked_copy());
    }

    fn expose_gc_ref_to_wasm(&mut self, _gc_ref: VMGcRef) {
        // The copying collector doesn't need any special handling when exposing
        // a GC ref to Wasm. There is no over-approximated-stack-roots list.
    }

    fn alloc_externref(
        &mut self,
        host_data: ExternRefHostDataId,
    ) -> Result<Result<VMExternRef, u64>> {
        let align = usize::try_from(ALIGN).unwrap();
        let size = core::mem::size_of::<VMCopyingExternRef>();
        let size = (size + align - 1) & !(align - 1);
        let gc_ref = match self.alloc_raw(
            VMGcHeader::externref(),
            Layout::from_size_align(size, align).unwrap(),
        )? {
            Err(n) => return Ok(Err(n)),
            Ok(gc_ref) => gc_ref,
        };
        // Take the old list head before borrowing self mutably through index_mut.
        let old_head = self.active_extern_ref_set_head.take();
        let externref_obj = self.index_mut::<VMCopyingExternRef>(gc_ref.as_typed_unchecked());
        externref_obj.host_data = host_data;
        externref_obj.next_extern_ref = old_head;
        let externref = gc_ref.into_externref_unchecked();
        self.active_extern_ref_set_head = Some(externref.unchecked_copy());
        Ok(Ok(externref))
    }

    fn externref_host_data(&self, externref: &VMExternRef) -> ExternRefHostDataId {
        let typed_ref = externref_to_copying(externref);
        self.index(typed_ref).host_data
    }

    fn header(&self, gc_ref: &VMGcRef) -> &VMGcHeader {
        let header: &VMGcHeader = self.index(gc_ref.as_typed_unchecked());

        debug_assert!(
            VMGcKind::try_from_u32(header.kind().as_u32()).is_some(),
            "header: invalid VMGcKind {:#010x} at gc_ref {gc_ref:#p}",
            header.kind().as_u32(),
        );

        header
    }

    fn header_mut(&mut self, gc_ref: &VMGcRef) -> &mut VMGcHeader {
        let header: &mut VMGcHeader = self.index_mut(gc_ref.as_typed_unchecked());

        debug_assert!(
            VMGcKind::try_from_u32(header.kind().as_u32()).is_some(),
            "header_mut: invalid VMGcKind {:#010x} at gc_ref {gc_ref:#p}",
            header.kind().as_u32(),
        );

        header
    }

    fn object_size(&self, gc_ref: &VMGcRef) -> usize {
        usize::try_from(self.index(copying_ref(gc_ref)).object_size()).unwrap()
    }

    fn alloc_raw(&mut self, header: VMGcHeader, layout: Layout) -> Result<Result<VMGcRef, u64>> {
        let align = u32::try_from(layout.align()).unwrap();
        ensure!(
            align == ALIGN,
            "copying collector requires all allocations to have alignment {ALIGN}, \
             but got alignment {align}",
        );

        debug_assert!(layout.size() >= core::mem::size_of::<VMCopyingHeader>());
        debug_assert_eq!(self.bump_ptr % ALIGN, 0, "bump_ptr is not aligned to ALIGN");
        debug_assert_eq!(header.reserved_u26(), 0);

        // We must have trace info for every GC type that we allocate in this
        // heap.
        if let Some(ty) = header.ty() {
            self.ensure_trace_info(ty);
        } else {
            debug_assert_eq!(header.kind(), VMGcKind::ExternRef);
        }

        let size = u32::try_from(layout.size()).unwrap();
        // Round up the allocation size to ALIGN so that the next bump-pointer
        // allocation is also aligned.
        let size = (size + ALIGN - 1) & !(ALIGN - 1);

        let gc_ref = match self.allocate(size) {
            None => return Ok(Err(u64::try_from(layout.size()).unwrap())),
            Some(index) => {
                debug_assert_ne!(index, 0, "index 0 is reserved; bump_ptr should skip it");
                VMGcRef::from_heap_index(NonZeroU32::new(index).unwrap()).unwrap()
            }
        };

        // Assert that the newly-allocated memory is still filled with the
        // poison pattern.
        if cfg!(gc_zeal) {
            let start = usize::try_from(gc_ref.as_heap_index().unwrap().get()).unwrap();
            let slice = &self.heap_slice()[start..][..layout.size()];
            gc_assert!(
                slice.iter().all(|&b| b == POISON),
                "newly allocated GC object at index {start} is not fully poisoned; \
                 freed memory was corrupted",
            );
        }

        let object_size = size;
        *self.index_mut(copying_ref(&gc_ref)) = VMCopyingHeader {
            header,
            object_size,
        };
        Ok(Ok(gc_ref))
    }

    fn alloc_uninit_struct_or_exn(
        &mut self,
        ty: VMSharedTypeIndex,
        layout: &GcStructLayout,
    ) -> Result<Result<VMGcRef, u64>> {
        let kind = if layout.is_exception {
            VMGcKind::ExnRef
        } else {
            VMGcKind::StructRef
        };
        let gc_ref =
            match self.alloc_raw(VMGcHeader::from_kind_and_index(kind, ty), layout.layout())? {
                Err(n) => return Ok(Err(n)),
                Ok(gc_ref) => gc_ref,
            };

        Ok(Ok(gc_ref))
    }

    fn dealloc_uninit_struct_or_exn(&mut self, _gcref: VMGcRef) {
        // The copying collector doesn't support individual deallocation; memory
        // is reclaimed during collection.
    }

    fn alloc_uninit_array(
        &mut self,
        ty: VMSharedTypeIndex,
        length: u32,
        layout: &GcArrayLayout,
    ) -> Result<Result<VMArrayRef, u64>> {
        let gc_ref = match self.alloc_raw(
            VMGcHeader::from_kind_and_index(VMGcKind::ArrayRef, ty),
            layout.layout(length),
        )? {
            Err(n) => return Ok(Err(n)),
            Ok(gc_ref) => gc_ref,
        };

        self.index_mut(gc_ref.as_typed_unchecked::<VMCopyingArrayHeader>())
            .length = length;

        Ok(Ok(gc_ref.into_arrayref_unchecked()))
    }

    fn dealloc_uninit_array(&mut self, _arrayref: VMArrayRef) {
        // The copying collector doesn't support individual deallocation; memory
        // is reclaimed during collection.
    }

    fn array_len(&self, arrayref: &VMArrayRef) -> u32 {
        debug_assert!(arrayref.as_gc_ref().is_typed::<VMCopyingArrayHeader>(self));
        self.index::<VMCopyingArrayHeader>(arrayref.as_gc_ref().as_typed_unchecked())
            .length
    }

    fn allocated_bytes(&self) -> usize {
        usize::try_from(self.bump_ptr - self.active_space_start).unwrap()
    }

    fn gc<'a>(
        &'a mut self,
        roots: GcRootsIter<'a>,
        host_data_table: &'a mut ExternRefHostDataTable,
    ) -> Box<dyn GarbageCollection<'a> + 'a> {
        assert_eq!(self.no_gc_count, 0, "Cannot GC inside a no-GC scope!");
        Box::new(CopyingCollection {
            roots: Some(roots),
            host_data_table,
            heap: self,
            phase: CopyingCollectionPhase::Collect,
        })
    }

    unsafe fn vmctx_gc_heap_data(&self) -> NonNull<u8> {
        // The copying collector doesn't currently have vmctx GC heap
        // data. Return a dangling pointer.
        NonNull::dangling()
    }

    fn take_memory(&mut self) -> crate::vm::Memory {
        debug_assert!(self.is_attached());
        self.vmmemory.take();
        self.memory.take().unwrap()
    }

    fn needs_gc_before_next_growth(&self) -> bool {
        // We need a GC before growth when the active space is the second half
        // of the GC heap, because we cannot safely extend the active space in
        // that configuration without making it larger than the idle space.
        self.idle_space_start < self.active_space_start
    }

    unsafe fn replace_memory(&mut self, memory: crate::vm::Memory, delta_bytes_grown: u64) {
        debug_assert!(self.memory.is_none());
        debug_assert!(!memory.is_shared_memory());
        self.vmmemory = Some(memory.vmmemory());
        self.memory = Some(memory);

        // If the heap was previously empty, reinitialize the semi-spaces from
        // scratch.
        if self.active_space_end == 0 && self.idle_space_end == 0 {
            self.initialize_semi_spaces();
        } else {
            // Otherwise the memory was grown: try to resize the semi-spaces
            // accordingly.
            self.resize_semi_spaces();
        }

        // Poison the newly-grown region.
        if cfg!(gc_zeal) && delta_bytes_grown > 0 {
            let slice = self.heap_slice_mut();
            let len = slice.len();
            let delta_bytes_grown = usize::try_from(delta_bytes_grown).unwrap();
            slice[len - delta_bytes_grown..].fill(POISON);
        }
    }

    #[inline]
    fn vmmemory(&self) -> VMMemoryDefinition {
        debug_assert!(self.is_attached());
        debug_assert!(!self.memory.as_ref().unwrap().is_shared_memory());
        let vmmemory = self.vmmemory.as_ref().unwrap();
        VMMemoryDefinition {
            base: vmmemory.base,
            current_length: AtomicUsize::new(vmmemory.current_length()),
        }
    }
}

struct CopyingCollection<'a> {
    roots: Option<GcRootsIter<'a>>,
    host_data_table: &'a mut ExternRefHostDataTable,
    heap: &'a mut CopyingHeap,
    phase: CopyingCollectionPhase,
}

enum CopyingCollectionPhase {
    Collect,
    Done,
}

impl CopyingCollection<'_> {
    /// Forward all GC roots from the idle space to the active space.
    fn process_roots(&mut self) {
        log::trace!("Begin processing GC roots");
        let roots = self.roots.take().unwrap();
        for mut root in roots {
            let gc_ref = root.get();
            if gc_ref.is_i31() {
                continue;
            }
            let old_index = gc_ref.as_heap_index().unwrap().get();
            debug_assert!(self.heap.is_in_idle_space(old_index));
            let new_ref = self.heap.forward(&gc_ref);
            root.set(new_ref);
        }
        log::trace!("End processing GC roots");
    }

    /// Scan all grey objects until the worklist is empty.
    fn process_worklist(&mut self) {
        log::trace!("Begin processing worklist");
        let trace_infos = mem::take(&mut self.heap.trace_infos);
        while let Some(gc_ref) = self.heap.worklist_pop() {
            debug_assert!(
                self.heap
                    .is_in_active_space(gc_ref.as_heap_index().unwrap().get())
            );
            self.heap.scan(&gc_ref, &trace_infos);
        }
        self.heap.trace_infos = trace_infos;
        log::trace!("End processing worklist");
    }

    /// Clean up dead externrefs by iterating the idle semi-space's externref
    /// linked list and deallocating host data for any that were not forwarded.
    fn sweep_extern_refs(&mut self) {
        log::trace!("Begin sweeping `externref`s");
        let mut link = self.heap.idle_extern_ref_set_head.take();
        while let Some(externref) = link {
            let gc_ref = externref.as_gc_ref();
            debug_assert!(
                self.heap
                    .is_in_idle_space(gc_ref.as_heap_index().unwrap().get())
            );
            let header = self.heap.index(copying_ref(gc_ref));
            if !header.copied() {
                let typed: &TypedGcRef<VMCopyingExternRef> = gc_ref.as_typed_unchecked();
                let host_data_id = self.heap.index(typed).host_data;
                self.host_data_table.dealloc(host_data_id);
            }
            link = self
                .heap
                .index::<VMCopyingExternRef>(gc_ref.as_typed_unchecked())
                .next_extern_ref
                .as_ref()
                .map(|e| e.unchecked_copy());
        }
        log::trace!("End sweeping `externref`s");
    }
}

impl GarbageCollection<'_> for CopyingCollection<'_> {
    fn collect_increment(&mut self) -> GcProgress {
        match self.phase {
            CopyingCollectionPhase::Collect => {
                log::trace!("Begin copying collection");

                assert!(self.heap.active_space_start <= self.heap.bump_ptr);
                assert!(self.heap.bump_ptr <= self.heap.active_space_end);
                assert!(self.heap.idle_space_start <= self.heap.idle_space_end);
                assert!(
                    self.heap.active_space_end <= self.heap.idle_space_start
                        || self.heap.idle_space_end <= self.heap.active_space_start
                );

                // Flip the semi-spaces.
                self.heap.flip();
                self.heap.initialize_worklist();

                self.process_roots();
                self.process_worklist();

                assert!(self.heap.active_space_start <= self.heap.bump_ptr);
                assert!(self.heap.bump_ptr <= self.heap.active_space_end);
                assert!(self.heap.idle_space_start <= self.heap.idle_space_end);
                assert!(
                    self.heap.active_space_end <= self.heap.idle_space_start
                        || self.heap.idle_space_end <= self.heap.active_space_start
                );

                self.sweep_extern_refs();
                self.heap.resize_semi_spaces();

                debug_assert_eq!(
                    self.heap.active_space_end - self.heap.active_space_start,
                    self.heap.idle_space_end - self.heap.idle_space_start,
                    "the active and idle spaces should be the same size"
                );

                // Poison the idle space so stale accesses are detectable.
                if cfg!(gc_zeal) {
                    let idle_start = usize::try_from(self.heap.idle_space_start).unwrap();
                    let idle_end = usize::try_from(self.heap.idle_space_end).unwrap();
                    self.heap.heap_slice_mut()[idle_start..idle_end].fill(POISON);
                }

                log::trace!("End copying collection");
                self.phase = CopyingCollectionPhase::Done;
                GcProgress::Complete
            }
            CopyingCollectionPhase::Done => GcProgress::Complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_copying_header_size_align() {
        assert_eq!(
            wasmtime_environ::copying::HEADER_SIZE as usize,
            core::mem::size_of::<VMCopyingHeader>(),
        );
        // The struct's natural alignment must not exceed ALIGN, which is
        // enforced by the bump allocator.
        assert!(
            core::mem::align_of::<VMCopyingHeader>() <= wasmtime_environ::copying::ALIGN as usize,
        );
    }

    #[test]
    fn vm_copying_array_header_length_offset() {
        assert_eq!(
            wasmtime_environ::copying::ARRAY_LENGTH_OFFSET,
            u32::try_from(core::mem::offset_of!(VMCopyingArrayHeader, length)).unwrap(),
        );
    }

    #[test]
    fn vm_copying_header_object_size_offset() {
        assert_eq!(
            // object_size comes right after the VMGcHeader
            wasmtime_environ::VM_GC_HEADER_SIZE,
            u32::try_from(core::mem::offset_of!(VMCopyingHeader, object_size)).unwrap(),
        );
    }

    #[test]
    fn vm_copying_forwarding_ref_offset() {
        assert_eq!(
            wasmtime_environ::copying::FORWARDING_REF_OFFSET as usize,
            core::mem::offset_of!(VMCopyingHeaderAndForwardingRef, forwarding_ref),
        );
    }

    #[test]
    fn vm_copying_header_and_forwarding_ref_size() {
        // The forwarding ref data (at FORWARDING_REF_OFFSET) plus its size
        // must fit within MIN_OBJECT_SIZE, so every object has room for
        // the forwarding pointer during collection.
        assert!(
            wasmtime_environ::copying::FORWARDING_REF_OFFSET as usize
                + core::mem::size_of::<Option<VMGcRef>>()
                <= wasmtime_environ::copying::MIN_OBJECT_SIZE as usize,
        );
    }
}
