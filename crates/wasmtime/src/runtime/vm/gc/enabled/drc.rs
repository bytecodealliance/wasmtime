//! The deferred reference-counting (DRC) collector.
//!
//! Warning: this ref-counting collector does not have a tracing cycle
//! collector, and therefore cannot collect cycles between GC objects!
//!
//! For host VM code, we use plain reference counting, where cloning increments
//! the reference count, and dropping decrements it. We can avoid many of the
//! on-stack increment/decrement operations that typically plague the
//! performance of reference counting via Rust's ownership and borrowing system.
//! Moving a `VMGcRef` avoids mutating its reference count, and borrowing it
//! either avoids the reference count increment or delays it until if/when the
//! `VMGcRef` is cloned.
//!
//! When passing a `VMGcRef` into compiled Wasm code, we don't want to do
//! reference count mutations for every compiled `local.{get,set}`, nor for
//! every function call. Therefore, we use a variation of **deferred reference
//! counting**, where we only mutate reference counts when storing `VMGcRef`s
//! somewhere that outlives the Wasm activation: into a global or
//! table. Simultaneously, we over-approximate the set of `VMGcRef`s that are
//! inside Wasm function activations. Periodically, we walk the stack at GC safe
//! points, and use stack map information to precisely identify the set of
//! `VMGcRef`s inside Wasm activations. Then we take the difference between this
//! precise set and our over-approximation, and decrement the reference count
//! for each of the `VMGcRef`s that are in our over-approximation but not in the
//! precise set. Finally, the over-approximation is replaced with the precise
//! set.
//!
//! The `VMGcRefActivationsTable` implements the over-approximized set of
//! `VMGcRef`s referenced by Wasm activations. Calling a Wasm function and
//! passing it a `VMGcRef` moves the `VMGcRef` into the table, and the compiled
//! Wasm function logically "borrows" the `VMGcRef` from the table. Similarly,
//! `global.get` and `table.get` operations clone the gotten `VMGcRef` into the
//! `VMGcRefActivationsTable` and then "borrow" the reference out of the table.
//!
//! When a `VMGcRef` is returned to host code from a Wasm function, the host
//! increments the reference count (because the reference is logically
//! "borrowed" from the `VMGcRefActivationsTable` and the reference count from
//! the table will be dropped at the next GC).
//!
//! For more general information on deferred reference counting, see *An
//! Examination of Deferred Reference Counting and Cycle Detection* by Quinane:
//! <https://openresearch-repository.anu.edu.au/bitstream/1885/42030/2/hon-thesis.pdf>

use super::free_list::FreeList;
use super::{VMArrayRef, VMStructRef};
use crate::hash_set::HashSet;
use crate::prelude::*;
use crate::runtime::vm::{
    mmap::AlignedLength, ExternRefHostDataId, ExternRefHostDataTable, GarbageCollection, GcHeap,
    GcHeapObject, GcProgress, GcRootsIter, GcRuntime, Mmap, TypedGcRef, VMExternRef, VMGcHeader,
    VMGcRef,
};
use crate::vm::SendSyncPtr;
use core::{
    alloc::Layout,
    any::Any,
    mem,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Range},
    ptr::NonNull,
};
use wasmtime_environ::drc::DrcTypeLayouts;
use wasmtime_environ::{GcArrayLayout, GcStructLayout, GcTypeLayouts, VMGcKind, VMSharedTypeIndex};

/// The deferred reference-counting (DRC) collector.
///
/// This reference-counting collector does not have a cycle collector, and so it
/// will not be able to reclaim garbage cycles.
///
/// This is not a moving collector; it doesn't have a nursery or do any
/// compaction.
#[derive(Default)]
pub struct DrcCollector {
    layouts: DrcTypeLayouts,
}

unsafe impl GcRuntime for DrcCollector {
    fn layouts(&self) -> &dyn GcTypeLayouts {
        &self.layouts
    }

    fn new_gc_heap(&self) -> Result<Box<dyn GcHeap>> {
        let heap = DrcHeap::new()?;
        Ok(Box::new(heap) as _)
    }
}

/// A deferred reference-counting (DRC) heap.
struct DrcHeap {
    /// Count of how many no-gc scopes we are currently within.
    no_gc_count: u64,

    /// This heap's bump table for GC refs entering the Wasm stack. This is
    /// mutated directly by Wasm and a pointer to it is stored inside the
    /// `VMContext`.
    ///
    /// NB: this box isn't strictly necessary (because the `DrcHeap` is itself
    /// boxed up) but it makes upholding the safety invariants of the
    /// `vmctx_gc_heap_data` more-obviously correct without needing to reason
    /// about less-local system properties.
    activations_table: Box<VMGcRefActivationsTable>,

    /// The storage for the GC heap itself.
    heap: Mmap<AlignedLength>,

    /// A free list describing which ranges of the heap are available for use.
    free_list: FreeList,

    /// An explicit stack to avoid recursion when deallocating one object needs
    /// to dec-ref another object, which can then be deallocated and dec-refs
    /// yet another object, etc...
    ///
    /// We store this stack here to reuse the storage and avoid repeated
    /// allocations.
    ///
    /// Note that the `Option` is perhaps technically unnecessary (we could
    /// remove the `Option` and, when we take the stack out of `self`, leave
    /// behind an empty vec instead of `None`) but we keep it because it will
    /// help us catch unexpected re-entry, similar to how a `RefCell` would.
    dec_ref_stack: Option<Vec<VMGcRef>>,
}

impl DrcHeap {
    /// Construct a new, default DRC heap.
    fn new() -> Result<Self> {
        Self::with_capacity(super::DEFAULT_GC_HEAP_CAPACITY)
    }

    /// Create a new DRC heap with the given capacity.
    fn with_capacity(capacity: usize) -> Result<Self> {
        log::trace!("allocating new DRC heap with capacity {capacity:#x}");
        let heap = Mmap::with_at_least(capacity)?;
        let free_list = FreeList::new(heap.len());
        Ok(Self {
            no_gc_count: 0,
            activations_table: Box::new(VMGcRefActivationsTable::default()),
            heap,
            free_list,
            dec_ref_stack: Some(vec![]),
        })
    }

    fn dealloc(&mut self, gc_ref: VMGcRef) {
        let drc_ref = drc_ref(&gc_ref);
        let size = self.index(drc_ref).object_size();
        let layout = FreeList::layout(size);
        self.free_list
            .dealloc(gc_ref.as_heap_index().unwrap(), layout);
    }

    fn object_range(&self, gc_ref: &VMGcRef) -> Range<usize> {
        let start = gc_ref.as_heap_index().unwrap().get();
        let start = usize::try_from(start).unwrap();
        let size = self
            .index::<VMDrcHeader>(gc_ref.as_typed_unchecked())
            .object_size();
        let end = start.checked_add(size).unwrap();
        start..end
    }

    /// Increment the ref count for the associated object.
    fn inc_ref(&mut self, gc_ref: &VMGcRef) {
        if gc_ref.is_i31() {
            return;
        }

        let drc_ref = drc_ref(gc_ref);
        let header = self.index_mut(&drc_ref);
        debug_assert_ne!(
            header.ref_count, 0,
            "{:#p} is supposedly live; should have nonzero ref count",
            *gc_ref
        );
        header.ref_count += 1;
        log::trace!("increment {:#p} ref count -> {}", *gc_ref, header.ref_count);
    }

    /// Decrement the ref count for the associated object.
    ///
    /// Returns `true` if the ref count reached zero and the object should be
    /// deallocated.
    fn dec_ref(&mut self, gc_ref: &VMGcRef) -> bool {
        if gc_ref.is_i31() {
            return false;
        }

        let drc_ref = drc_ref(gc_ref);
        let header = self.index_mut(drc_ref);
        debug_assert_ne!(
            header.ref_count, 0,
            "{:#p} is supposedly live; should have nonzero ref count",
            *gc_ref
        );
        header.ref_count -= 1;
        log::trace!("decrement {:#p} ref count -> {}", *gc_ref, header.ref_count);
        header.ref_count == 0
    }

    /// Decrement the ref count for the associated object.
    ///
    /// If the ref count reached zero, then deallocate the object and remove its
    /// associated entry from the `host_data_table` if necessary.
    ///
    /// This uses an explicit stack, rather than recursion, for the scenario
    /// where dropping one object means that the ref count for another object
    /// that it referenced reaches zero.
    fn dec_ref_and_maybe_dealloc(
        &mut self,
        host_data_table: &mut ExternRefHostDataTable,
        gc_ref: &VMGcRef,
    ) {
        let mut stack = self.dec_ref_stack.take().unwrap();
        debug_assert!(stack.is_empty());
        stack.push(gc_ref.unchecked_copy());

        while let Some(gc_ref) = stack.pop() {
            if self.dec_ref(&gc_ref) {
                // The object's reference count reached zero.
                //
                // Enqueue any other objects it references for dec-ref'ing.
                stack.extend(self.trace_gc_ref(&gc_ref));

                // If this object was an `externref`, remove its associated
                // entry from the host-data table.
                if let Some(externref) = gc_ref.as_typed::<VMDrcExternRef>(self) {
                    let host_data_id = self.index(externref).host_data;
                    host_data_table.dealloc(host_data_id);
                }

                // Deallocate this GC object!
                self.dealloc(gc_ref.unchecked_copy());
            }
        }

        debug_assert!(stack.is_empty());
        debug_assert!(self.dec_ref_stack.is_none());
        self.dec_ref_stack = Some(stack);
    }

    /// Enumerate all of the given `VMGcRef`'s outgoing edges.
    fn trace_gc_ref(&self, gc_ref: &VMGcRef) -> impl Iterator<Item = VMGcRef> + use<'_> {
        debug_assert!(!gc_ref.is_i31());

        let len = self.index(drc_ref(gc_ref)).num_gc_refs();

        let is_array = self.header(gc_ref).kind() == VMGcKind::ArrayRef;
        let start_in_obj = mem::size_of::<VMDrcHeader>() +
            // Skip over the array's `length` field, if necessary.
            usize::from(is_array) * mem::size_of::<u32>();

        let end_in_obj = start_in_obj + len * mem::size_of::<u32>();

        let object_range = self.object_range(gc_ref);
        let object_data = &self.heap_slice()[object_range];
        let gc_refs_data = &object_data[start_in_obj..end_in_obj];

        gc_refs_data
            .chunks(mem::size_of::<u32>())
            .filter_map(|chunk| {
                assert_eq!(chunk.len(), 4);
                let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];

                // Values are always little-endian inside the GC heap.
                let raw = u32::from_le_bytes(bytes);

                VMGcRef::from_raw_u32(raw)
            })
    }

    fn trace(&mut self, roots: &mut GcRootsIter<'_>) {
        debug_assert!({
            // This set is only non-empty during collection. It is built up when
            // tracing roots, and then drained back into the activations table's
            // bump-allocated space at the end. Therefore, it should always be
            // empty upon beginning tracing, which is the start of collection.
            self.activations_table.precise_stack_roots.is_empty()
        });

        // The `activations_table_set` is used for `debug_assert!`s checking that
        // every reference we read out from the stack via stack maps is actually in
        // the table. If that weren't true, than either we forgot to insert a
        // reference in the table when passing it into Wasm (a bug) or we are
        // reading invalid references from the stack (another bug).
        let mut activations_table_set: DebugOnly<HashSet<_>> = Default::default();
        if cfg!(debug_assertions) {
            self.activations_table.elements(|elem| {
                activations_table_set.insert(elem.unchecked_copy());
            });
        }

        for root in roots {
            if !root.is_on_wasm_stack() {
                // We only trace on-Wasm-stack GC roots. These are the
                // GC references that we do deferred ref counting for
                // and that get inserted into our activations
                // table. Other GC roots are managed purely with naive
                // ref counting.
                continue;
            }

            let gc_ref = root.get();
            debug_assert!(
                gc_ref.is_i31() || activations_table_set.contains(&gc_ref),
                "every on-stack gc_ref inside a Wasm frame should \
                 have an entry in the VMGcRefActivationsTable; \
                 {gc_ref:#p} is not in the table",
            );
            if gc_ref.is_i31() {
                continue;
            }

            debug_assert_ne!(
                self.index_mut(drc_ref(&gc_ref)).ref_count,
                0,
                "{gc_ref:#p} is on the Wasm stack and therefore should be held \
                 by the activations table; should have nonzero ref count",
            );

            log::trace!("Found GC reference on the stack: {:#p}", gc_ref);
            let is_new = self
                .activations_table
                .precise_stack_roots
                .insert(gc_ref.unchecked_copy());
            if is_new {
                self.inc_ref(&gc_ref);
            }
        }
    }

    fn iter_bump_chunk(&mut self) -> impl Iterator<Item = VMGcRef> + '_ {
        let num_filled = self.activations_table.num_filled_in_bump_chunk();
        self.activations_table
            .alloc
            .chunk
            .iter_mut()
            .take(num_filled)
            .map(|slot| VMGcRef::from_raw_u32(*slot).expect("non-null"))
    }

    #[inline(never)]
    #[cold]
    fn log_gc_ref_set(prefix: &str, items: impl Iterator<Item = VMGcRef>) {
        assert!(log::log_enabled!(log::Level::Trace));
        let mut set = "{".to_string();
        let mut any = false;
        for gc_ref in items {
            any = true;
            set += &format!("\n  {gc_ref:#p},");
        }
        if any {
            set.push('\n');
        }
        set.push('}');
        log::trace!("{prefix}: {set}");
    }

    fn drain_bump_chunk(&mut self, mut f: impl FnMut(&mut Self, VMGcRef)) {
        let num_filled = self.activations_table.num_filled_in_bump_chunk();

        // Temporarily take the allocation out of `self` to avoid conflicting
        // borrows.
        let mut alloc = mem::take(&mut self.activations_table.alloc);
        for slot in alloc.chunk.iter_mut().take(num_filled) {
            let raw = mem::take(slot);
            let gc_ref = VMGcRef::from_raw_u32(raw).expect("non-null");
            f(self, gc_ref);
            *slot = 0;
        }

        debug_assert!(
            alloc.chunk.iter().all(|slot| *slot == 0),
            "after sweeping the bump chunk, all slots should be empty",
        );

        debug_assert!(self.activations_table.alloc.chunk.is_empty());
        self.activations_table.alloc = alloc;
    }

    /// Sweep the bump allocation table after we've discovered our precise stack
    /// roots.
    fn sweep(&mut self, host_data_table: &mut ExternRefHostDataTable) {
        if log::log_enabled!(log::Level::Trace) {
            Self::log_gc_ref_set("bump chunk before sweeping", self.iter_bump_chunk());
        }

        // Sweep our bump chunk.
        log::trace!("Begin sweeping bump chunk");
        self.drain_bump_chunk(|heap, gc_ref| {
            heap.dec_ref_and_maybe_dealloc(host_data_table, &gc_ref);
        });
        log::trace!("Done sweeping bump chunk");

        if self.activations_table.alloc.chunk.is_empty() {
            // If this is the first collection, then the bump chunk is empty
            // since we lazily allocate it. Force that lazy allocation now so we
            // have fast bump-allocation in the future.
            self.activations_table.alloc.force_allocation();
        } else {
            // Reset our `next` finger to the start of the bump allocation chunk.
            self.activations_table.alloc.reset();
        }

        if log::log_enabled!(log::Level::Trace) {
            Self::log_gc_ref_set(
                "hash set before sweeping",
                self.activations_table
                    .over_approximated_stack_roots
                    .iter()
                    .map(|r| r.unchecked_copy()),
            );
        }

        // The current `precise_stack_roots` becomes our new over-appoximated
        // set for the next GC cycle.
        mem::swap(
            &mut self.activations_table.precise_stack_roots,
            &mut self.activations_table.over_approximated_stack_roots,
        );

        // And finally, the new `precise_stack_roots` should be cleared and
        // remain empty until the next GC cycle.
        //
        // Note that this may run arbitrary code as we run gc_ref
        // destructors. Because of our `&mut` borrow above on this table,
        // though, we're guaranteed that nothing will touch this table.
        log::trace!("Begin sweeping hash set");
        let mut precise_stack_roots = mem::take(&mut self.activations_table.precise_stack_roots);
        for gc_ref in precise_stack_roots.drain() {
            self.dec_ref_and_maybe_dealloc(host_data_table, &gc_ref);
        }
        log::trace!("Done sweeping hash set");

        // Make sure to replace the `precise_stack_roots` so that we reuse any
        // allocated capacity.
        self.activations_table.precise_stack_roots = precise_stack_roots;

        if log::log_enabled!(log::Level::Trace) {
            Self::log_gc_ref_set(
                "hash set after sweeping",
                self.activations_table
                    .over_approximated_stack_roots
                    .iter()
                    .map(|r| r.unchecked_copy()),
            );
        }
    }
}

/// Convert the given GC reference as a typed GC reference pointing to a
/// `VMDrcHeader`.
fn drc_ref(gc_ref: &VMGcRef) -> &TypedGcRef<VMDrcHeader> {
    debug_assert!(!gc_ref.is_i31());
    gc_ref.as_typed_unchecked()
}

/// Convert a generic `externref` to a typed reference to our concrete
/// `externref` type.
fn externref_to_drc(externref: &VMExternRef) -> &TypedGcRef<VMDrcExternRef> {
    let gc_ref = externref.as_gc_ref();
    debug_assert!(!gc_ref.is_i31());
    gc_ref.as_typed_unchecked()
}

/// The common header for all objects in the DRC collector.
///
/// This adds a ref count on top collector-agnostic `VMGcHeader`.
///
/// This is accessed by JIT code.
#[repr(C)]
struct VMDrcHeader {
    header: VMGcHeader,
    ref_count: u64,
    object_size: u32,
}

unsafe impl GcHeapObject for VMDrcHeader {
    #[inline]
    fn is(_header: &VMGcHeader) -> bool {
        // All DRC objects have a DRC header.
        true
    }
}

impl VMDrcHeader {
    /// The size of this header's object.
    fn object_size(&self) -> usize {
        usize::try_from(self.object_size).unwrap()
    }

    /// The number of GC references this object contains.
    ///
    /// This is stored in the inner `VMGcHeader`'s reserved bits.
    fn num_gc_refs(&self) -> usize {
        let size = self.header.reserved_u27();
        usize::try_from(size).unwrap()
    }
}

/// The common header for all arrays in the DRC collector.
#[repr(C)]
struct VMDrcArrayHeader {
    header: VMDrcHeader,
    length: u32,
}

unsafe impl GcHeapObject for VMDrcArrayHeader {
    #[inline]
    fn is(header: &VMGcHeader) -> bool {
        header.kind() == VMGcKind::ArrayRef
    }
}

/// The representation of an `externref` in the DRC collector.
#[repr(C)]
struct VMDrcExternRef {
    header: VMDrcHeader,
    host_data: ExternRefHostDataId,
}

unsafe impl GcHeapObject for VMDrcExternRef {
    #[inline]
    fn is(header: &VMGcHeader) -> bool {
        header.kind() == VMGcKind::ExternRef
    }
}

unsafe impl GcHeap for DrcHeap {
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
        self.inc_ref(gc_ref);
        gc_ref.unchecked_copy()
    }

    fn write_gc_ref(
        &mut self,
        host_data_table: &mut ExternRefHostDataTable,
        destination: &mut Option<VMGcRef>,
        source: Option<&VMGcRef>,
    ) {
        // Increment the ref count of the object being written into the slot.
        if let Some(src) = source {
            self.inc_ref(src);
        }

        // Decrement the ref count of the value being overwritten and, if
        // necessary, deallocate the GC object.
        if let Some(dest) = destination {
            self.dec_ref_and_maybe_dealloc(host_data_table, dest);
        }

        // Do the actual write.
        *destination = source.map(|s| s.unchecked_copy());
    }

    fn expose_gc_ref_to_wasm(&mut self, gc_ref: VMGcRef) {
        self.activations_table.insert_without_gc(gc_ref);
    }

    fn need_gc_before_entering_wasm(&self, num_gc_refs: NonZeroUsize) -> bool {
        num_gc_refs.get() > self.activations_table.bump_capacity_remaining()
    }

    fn alloc_externref(&mut self, host_data: ExternRefHostDataId) -> Result<Option<VMExternRef>> {
        let gc_ref =
            match self.alloc_raw(VMGcHeader::externref(), Layout::new::<VMDrcExternRef>())? {
                None => return Ok(None),
                Some(gc_ref) => gc_ref,
            };
        self.index_mut::<VMDrcExternRef>(gc_ref.as_typed_unchecked())
            .host_data = host_data;
        debug_assert_eq!(self.index(drc_ref(&gc_ref)).num_gc_refs(), 0);
        Ok(Some(gc_ref.into_externref_unchecked()))
    }

    fn externref_host_data(&self, externref: &VMExternRef) -> ExternRefHostDataId {
        let typed_ref = externref_to_drc(externref);
        self.index(typed_ref).host_data
    }

    fn header(&self, gc_ref: &VMGcRef) -> &VMGcHeader {
        self.index(gc_ref.as_typed_unchecked())
    }

    fn header_mut(&mut self, gc_ref: &VMGcRef) -> &mut VMGcHeader {
        self.index_mut(gc_ref.as_typed_unchecked())
    }

    fn object_size(&self, gc_ref: &VMGcRef) -> usize {
        self.index(drc_ref(gc_ref)).object_size()
    }

    fn alloc_raw(&mut self, header: VMGcHeader, layout: Layout) -> Result<Option<VMGcRef>> {
        debug_assert!(layout.size() >= core::mem::size_of::<VMDrcHeader>());
        debug_assert!(layout.align() >= core::mem::align_of::<VMDrcHeader>());

        let size = u32::try_from(layout.size()).unwrap();
        let gc_ref = match self.free_list.alloc(layout)? {
            None => return Ok(None),
            Some(index) => VMGcRef::from_heap_index(index).unwrap(),
        };

        *self.index_mut(drc_ref(&gc_ref)) = VMDrcHeader {
            header,
            ref_count: 1,
            object_size: size,
        };
        log::trace!("new object: increment {gc_ref:#p} ref count -> 1");
        Ok(Some(gc_ref))
    }

    fn alloc_uninit_struct(
        &mut self,
        ty: VMSharedTypeIndex,
        layout: &GcStructLayout,
    ) -> Result<Option<VMStructRef>> {
        let gc_ref = match self.alloc_raw(
            VMGcHeader::from_kind_and_index(VMGcKind::StructRef, ty),
            layout.layout(),
        )? {
            None => return Ok(None),
            Some(gc_ref) => gc_ref,
        };

        let num_gc_refs = layout.fields.iter().filter(|f| f.is_gc_ref).count();
        if cfg!(debug_assertions) {
            let mut fields_by_offset = layout.fields.clone();
            fields_by_offset.sort_by_key(|f| f.offset);
            debug_assert_eq!(
                fields_by_offset.iter().take_while(|f| f.is_gc_ref).count(),
                num_gc_refs,
                "all GC refs should be placed at the start of the struct",
            );
        }

        let num_gc_refs = u32::try_from(num_gc_refs).unwrap();
        if !VMGcKind::value_fits_in_unused_bits(num_gc_refs) {
            self.dealloc_uninit_struct(gc_ref.into_structref_unchecked());
            return Err(crate::Trap::AllocationTooLarge.into());
        }
        self.header_mut(&gc_ref).set_reserved_u27(num_gc_refs);

        Ok(Some(gc_ref.into_structref_unchecked()))
    }

    fn dealloc_uninit_struct(&mut self, structref: VMStructRef) {
        self.dealloc(structref.into());
    }

    fn alloc_uninit_array(
        &mut self,
        ty: VMSharedTypeIndex,
        length: u32,
        layout: &GcArrayLayout,
    ) -> Result<Option<VMArrayRef>> {
        let gc_ref = match self.alloc_raw(
            VMGcHeader::from_kind_and_index(VMGcKind::ArrayRef, ty),
            layout.layout(length),
        )? {
            None => return Ok(None),
            Some(gc_ref) => gc_ref,
        };

        self.index_mut(gc_ref.as_typed_unchecked::<VMDrcArrayHeader>())
            .length = length;

        let num_gc_refs = if layout.elems_are_gc_refs { length } else { 0 };
        let num_gc_refs = u32::try_from(num_gc_refs).unwrap();
        if !VMGcKind::value_fits_in_unused_bits(num_gc_refs) {
            self.dealloc_uninit_array(gc_ref.into_arrayref_unchecked());
            return Err(crate::Trap::AllocationTooLarge.into());
        }
        self.header_mut(&gc_ref).set_reserved_u27(num_gc_refs);

        Ok(Some(gc_ref.into_arrayref_unchecked()))
    }

    fn dealloc_uninit_array(&mut self, arrayref: VMArrayRef) {
        self.dealloc(arrayref.into())
    }

    fn array_len(&self, arrayref: &VMArrayRef) -> u32 {
        debug_assert!(arrayref.as_gc_ref().is_typed::<VMDrcArrayHeader>(self));
        self.index::<VMDrcArrayHeader>(arrayref.as_gc_ref().as_typed_unchecked())
            .length
    }

    fn gc<'a>(
        &'a mut self,
        roots: GcRootsIter<'a>,
        host_data_table: &'a mut ExternRefHostDataTable,
    ) -> Box<dyn GarbageCollection<'a> + 'a> {
        assert_eq!(self.no_gc_count, 0, "Cannot GC inside a no-GC scope!");
        Box::new(DrcCollection {
            roots,
            host_data_table,
            heap: self,
            phase: DrcCollectionPhase::Trace,
        })
    }

    unsafe fn vmctx_gc_heap_data(&self) -> NonNull<u8> {
        let ptr: NonNull<VMGcRefActivationsTable> = NonNull::from(&*self.activations_table);
        ptr.cast()
    }

    #[cfg(feature = "pooling-allocator")]
    fn reset(&mut self) {
        let DrcHeap {
            no_gc_count,
            activations_table,
            free_list,
            dec_ref_stack,
            heap: _,
        } = self;

        *no_gc_count = 0;
        free_list.reset();
        activations_table.reset();
        debug_assert!(dec_ref_stack.as_ref().is_some_and(|s| s.is_empty()));
    }

    fn heap_slice(&self) -> &[u8] {
        let len = self.heap.len();
        unsafe { self.heap.slice(0..len) }
    }

    fn heap_slice_mut(&mut self) -> &mut [u8] {
        let len = self.heap.len();
        unsafe { self.heap.slice_mut(0..len) }
    }
}

struct DrcCollection<'a> {
    roots: GcRootsIter<'a>,
    host_data_table: &'a mut ExternRefHostDataTable,
    heap: &'a mut DrcHeap,
    phase: DrcCollectionPhase,
}

enum DrcCollectionPhase {
    Trace,
    Sweep,
    Done,
}

impl<'a> GarbageCollection<'a> for DrcCollection<'a> {
    fn collect_increment(&mut self) -> GcProgress {
        match self.phase {
            DrcCollectionPhase::Trace => {
                log::trace!("Begin DRC trace");
                self.heap.trace(&mut self.roots);
                log::trace!("End DRC trace");
                self.phase = DrcCollectionPhase::Sweep;
                GcProgress::Continue
            }
            DrcCollectionPhase::Sweep => {
                log::trace!("Begin DRC sweep");
                self.heap.sweep(self.host_data_table);
                log::trace!("End DRC sweep");
                self.phase = DrcCollectionPhase::Done;
                GcProgress::Complete
            }
            DrcCollectionPhase::Done => GcProgress::Complete,
        }
    }
}

/// The type of `VMGcRefActivationsTable`'s bump region's elements.
///
/// These are written to by Wasm.
type TableElem = u32;

/// A table that over-approximizes the set of `VMGcRef`s that any Wasm
/// activation on this thread is currently using.
///
/// Under the covers, this is a simple bump allocator that allows duplicate
/// entries. Deduplication happens at GC time.
//
// `alloc` must be the first member, it's accessed from JIT code.
#[repr(C)]
struct VMGcRefActivationsTable {
    /// Structures used to perform fast bump allocation of storage of externref
    /// values.
    ///
    /// This is the only member of this structure accessed from JIT code.
    alloc: VMGcRefTableAlloc,

    /// When unioned with `chunk`, this is an over-approximation of the GC roots
    /// on the stack, inside Wasm frames.
    ///
    /// This is used by slow-path insertion, and when a GC cycle finishes, is
    /// re-initialized to the just-discovered precise set of stack roots (which
    /// immediately becomes an over-approximation again as soon as Wasm runs and
    /// potentially drops references).
    over_approximated_stack_roots: HashSet<VMGcRef>,

    /// The precise set of on-stack, inside-Wasm GC roots that we discover via
    /// walking the stack and interpreting stack maps.
    ///
    /// This is *only* used inside the `gc` function, and is empty otherwise. It
    /// is just part of this struct so that we can reuse the allocation, rather
    /// than create a new hash set every GC.
    precise_stack_roots: HashSet<VMGcRef>,
}

/// The chunk of memory that we bump-allocate into for the fast path of
/// inserting into the `VMGcRefActivationsTable`.
///
/// This is accessed from compiled Wasm code.
#[repr(C)]
struct VMGcRefTableAlloc {
    /// Bump-allocation finger within the `chunk`.
    ///
    /// NB: this is written to by compiled Wasm code.
    next: SendSyncPtr<TableElem>,

    /// Pointer to just after the `chunk`.
    ///
    /// This is *not* within the current chunk and therefore is not a valid
    /// place to insert a reference!
    end: SendSyncPtr<TableElem>,

    /// Bump allocation chunk that stores fast-path insertions.
    ///
    /// This is not accessed from JIT code.
    chunk: Box<[TableElem]>,
}

impl Default for VMGcRefTableAlloc {
    fn default() -> Self {
        // Start with an empty chunk, just in case this activations table isn't
        // ever used. This means that there's no space in the bump-allocation
        // area which will force any path trying to use this to the slow GC
        // path. The first time this happens, though, the slow GC path will
        // allocate a new chunk for actual fast-bumping.
        let mut chunk: Box<[TableElem]> = Box::new([]);
        let next = chunk.as_mut_ptr();
        let end = unsafe { next.add(chunk.len()) };
        VMGcRefTableAlloc {
            next: SendSyncPtr::new(NonNull::new(next).unwrap()),
            end: SendSyncPtr::new(NonNull::new(end).unwrap()),
            chunk,
        }
    }
}

impl VMGcRefTableAlloc {
    /// Create a new, empty bump region.
    const CHUNK_SIZE: usize = 4096 / mem::size_of::<TableElem>();

    /// Force the lazy allocation of this bump region.
    fn force_allocation(&mut self) {
        assert!(self.chunk.is_empty());
        self.chunk = (0..Self::CHUNK_SIZE).map(|_| 0).collect();
        self.reset();
    }

    /// Reset this bump region, retaining any underlying allocation, but moving
    /// the bump pointer and limit to their default positions.
    fn reset(&mut self) {
        self.next = SendSyncPtr::new(NonNull::new(self.chunk.as_mut_ptr()).unwrap());
        self.end = SendSyncPtr::new(
            NonNull::new(unsafe { self.chunk.as_mut_ptr().add(self.chunk.len()) }).unwrap(),
        );
    }
}

fn _assert_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<VMGcRefActivationsTable>();
}

impl Default for VMGcRefActivationsTable {
    fn default() -> Self {
        Self::new()
    }
}

impl VMGcRefActivationsTable {
    /// Create a new `VMGcRefActivationsTable`.
    fn new() -> Self {
        VMGcRefActivationsTable {
            alloc: VMGcRefTableAlloc::default(),
            over_approximated_stack_roots: HashSet::new(),
            precise_stack_roots: HashSet::new(),
        }
    }

    #[cfg(feature = "pooling-allocator")]
    fn reset(&mut self) {
        let VMGcRefActivationsTable {
            alloc,
            over_approximated_stack_roots,
            precise_stack_roots,
        } = self;

        alloc.reset();
        over_approximated_stack_roots.clear();
        precise_stack_roots.clear();
    }

    /// Get the available capacity in the bump allocation chunk.
    #[inline]
    fn bump_capacity_remaining(&self) -> usize {
        let end = self.alloc.end.as_ptr() as usize;
        let next = self.alloc.next.as_ptr() as usize;
        end - next
    }

    /// Try and insert a `VMGcRef` into this table.
    ///
    /// This is a fast path that only succeeds when the bump chunk has the
    /// capacity for the requested insertion.
    ///
    /// If the insertion fails, then the `VMGcRef` is given back. Callers
    /// may attempt a GC to free up space and try again, or may call
    /// `insert_slow_path` to infallibly insert the reference (potentially
    /// allocating additional space in the table to hold it).
    #[inline]
    fn try_insert(&mut self, gc_ref: VMGcRef) -> Result<(), VMGcRef> {
        unsafe {
            if self.alloc.next == self.alloc.end {
                return Err(gc_ref);
            }

            debug_assert_eq!(
                self.alloc.next.as_non_null().read(),
                0,
                "slots >= the `next` bump finger are always `None`"
            );
            self.alloc.next.as_non_null().write(gc_ref.as_raw_u32());

            let next = SendSyncPtr::new(NonNull::new(self.alloc.next.as_ptr().add(1)).unwrap());
            debug_assert!(next.as_ptr() <= self.alloc.end.as_ptr());
            self.alloc.next = next;

            Ok(())
        }
    }

    /// Insert a reference into the table, without ever performing GC.
    #[inline]
    fn insert_without_gc(&mut self, gc_ref: VMGcRef) {
        if let Err(gc_ref) = self.try_insert(gc_ref) {
            self.insert_slow_without_gc(gc_ref);
        }
    }

    #[inline(never)]
    fn insert_slow_without_gc(&mut self, gc_ref: VMGcRef) {
        self.over_approximated_stack_roots.insert(gc_ref);
    }

    fn num_filled_in_bump_chunk(&self) -> usize {
        let next = self.alloc.next;
        let bytes_unused = (self.alloc.end.as_ptr() as usize) - (next.as_ptr() as usize);
        let slots_unused = bytes_unused / mem::size_of::<TableElem>();
        self.alloc.chunk.len().saturating_sub(slots_unused)
    }

    fn elements(&self, mut f: impl FnMut(&VMGcRef)) {
        for elem in self.over_approximated_stack_roots.iter() {
            f(elem);
        }

        // The bump chunk is not all the way full, so we only iterate over its
        // filled-in slots.
        let num_filled = self.num_filled_in_bump_chunk();
        for slot in self.alloc.chunk.iter().take(num_filled) {
            if let Some(elem) = VMGcRef::from_raw_u32(*slot) {
                f(&elem);
            }
        }
    }
}

#[derive(Debug, Default)]
struct DebugOnly<T> {
    inner: T,
}

impl<T> Deref for DebugOnly<T> {
    type Target = T;

    fn deref(&self) -> &T {
        if cfg!(debug_assertions) {
            &self.inner
        } else {
            panic!(
                "only deref `DebugOnly` when `cfg(debug_assertions)` or \
                 inside a `debug_assert!(..)`"
            )
        }
    }
}

impl<T> DerefMut for DebugOnly<T> {
    fn deref_mut(&mut self) -> &mut T {
        if cfg!(debug_assertions) {
            &mut self.inner
        } else {
            panic!(
                "only deref `DebugOnly` when `cfg(debug_assertions)` or \
                 inside a `debug_assert!(..)`"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime_environ::HostPtr;

    #[test]
    fn vm_drc_header_size_align() {
        assert_eq!(
            (wasmtime_environ::drc::HEADER_SIZE as usize),
            core::mem::size_of::<VMDrcHeader>()
        );
        assert_eq!(
            (wasmtime_environ::drc::HEADER_ALIGN as usize),
            core::mem::align_of::<VMDrcHeader>()
        );
    }

    #[test]
    fn vm_drc_array_header_length_offset() {
        assert_eq!(
            wasmtime_environ::drc::ARRAY_LENGTH_OFFSET,
            u32::try_from(core::mem::offset_of!(VMDrcArrayHeader, length)).unwrap(),
        );
    }

    #[test]
    fn ref_count_is_at_correct_offset() {
        let extern_data = VMDrcHeader {
            header: VMGcHeader::externref(),
            ref_count: 0,
            object_size: 0,
        };

        let extern_data_ptr = &extern_data as *const _;
        let ref_count_ptr = &extern_data.ref_count as *const _;

        let actual_offset = (ref_count_ptr as usize) - (extern_data_ptr as usize);

        let offsets = wasmtime_environ::VMOffsets::from(wasmtime_environ::VMOffsetsFields {
            ptr: HostPtr,
            num_imported_functions: 0,
            num_imported_tables: 0,
            num_imported_memories: 0,
            num_imported_globals: 0,
            num_imported_tags: 0,
            num_defined_tables: 0,
            num_defined_memories: 0,
            num_owned_memories: 0,
            num_defined_globals: 0,
            num_defined_tags: 0,
            num_escaped_funcs: 0,
        });

        assert_eq!(
            offsets.vm_drc_header_ref_count(),
            u32::try_from(actual_offset).unwrap(),
        );
    }

    #[test]
    fn table_next_is_at_correct_offset() {
        let table = VMGcRefActivationsTable::new();

        let table_ptr = &table as *const _;
        let next_ptr = &table.alloc.next as *const _;

        let actual_offset = (next_ptr as usize) - (table_ptr as usize);

        let offsets = wasmtime_environ::VMOffsets::from(wasmtime_environ::VMOffsetsFields {
            ptr: HostPtr,
            num_imported_functions: 0,
            num_imported_tables: 0,
            num_imported_memories: 0,
            num_imported_globals: 0,
            num_imported_tags: 0,
            num_defined_tables: 0,
            num_defined_memories: 0,
            num_owned_memories: 0,
            num_defined_globals: 0,
            num_defined_tags: 0,
            num_escaped_funcs: 0,
        });
        assert_eq!(
            offsets.vm_gc_ref_activation_table_next() as usize,
            actual_offset
        );
    }

    #[test]
    fn table_end_is_at_correct_offset() {
        let table = VMGcRefActivationsTable::new();

        let table_ptr = &table as *const _;
        let end_ptr = &table.alloc.end as *const _;

        let actual_offset = (end_ptr as usize) - (table_ptr as usize);

        let offsets = wasmtime_environ::VMOffsets::from(wasmtime_environ::VMOffsetsFields {
            ptr: HostPtr,
            num_imported_functions: 0,
            num_imported_tables: 0,
            num_imported_memories: 0,
            num_imported_globals: 0,
            num_imported_tags: 0,
            num_defined_tables: 0,
            num_defined_memories: 0,
            num_owned_memories: 0,
            num_defined_globals: 0,
            num_defined_tags: 0,
            num_escaped_funcs: 0,
        });
        assert_eq!(
            offsets.vm_gc_ref_activation_table_end() as usize,
            actual_offset
        );
    }
}
