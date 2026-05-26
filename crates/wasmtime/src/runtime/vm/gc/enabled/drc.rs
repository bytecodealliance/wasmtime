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
//! precise set. Finally, the over-approximation is reset to the precise set.
//!
//! An intrusive, singly-linked list in the object header implements the
//! over-approximated set of `VMGcRef`s referenced by Wasm activations. Calling
//! a Wasm function and passing it a `VMGcRef` inserts the `VMGcRef` into that
//! list if it is not already present, and the compiled Wasm function logically
//! "borrows" the `VMGcRef` from the list. Similarly, `global.get` and
//! `table.get` operations logically clone the gotten `VMGcRef` into that list
//! and then "borrow" the reference out of the list.
//!
//! When a `VMGcRef` is returned to host code from a Wasm function, the host
//! increments the reference count (because the reference is logically
//! "borrowed" from the list and the reference count from
//! the table will be dropped at the next GC).
//!
//! The precise set of stack roots is implemented with a mark bit in the object
//! header. See the `trace` and `sweep` methods for more details.
//!
//! For more general information on deferred reference counting, see *An
//! Examination of Deferred Reference Counting and Cycle Detection* by Quinane:
//! <https://openresearch-repository.anu.edu.au/bitstream/1885/42030/2/hon-thesis.pdf>

use super::VMArrayRef;
use super::free_list::FreeList;
use super::trace_info::{TraceInfo, TraceInfos};
use crate::hash_set::HashSet;
use crate::runtime::vm::{
    ExternRefHostDataId, ExternRefHostDataTable, GarbageCollection, GcHeap, GcHeapObject,
    GcProgress, GcRootsIter, GcRuntime, SendSyncUnsafeCell, TypedGcRef, VMExternRef, VMGcHeader,
    VMGcObjectData, VMGcRef,
};
use crate::vm::VMMemoryDefinition;
use crate::{Engine, Trap, bail_bug, prelude::*};
use core::sync::atomic::AtomicUsize;
use core::{
    alloc::Layout,
    any::Any,
    mem,
    ops::{Deref, DerefMut, Range},
    ptr::NonNull,
};
use wasmtime_core::undo::Undo;
use wasmtime_environ::drc::{ARRAY_LENGTH_OFFSET, DrcTypeLayouts};
use wasmtime_environ::{
    GcArrayLayout, GcStructLayout, GcTypeLayouts, POISON, VMGcKind, VMSharedTypeIndex, gc_assert,
};

#[expect(clippy::cast_possible_truncation, reason = "known to not overflow")]
const GC_REF_ARRAY_ELEMS_OFFSET: u32 = ARRAY_LENGTH_OFFSET + (mem::size_of::<u32>() as u32);

const MAX_ARRAY_STACK_DEPTH: usize = 1024;

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

    fn new_gc_heap(&self, engine: &Engine) -> Result<Box<dyn GcHeap>> {
        let heap = DrcHeap::new(engine)?;
        Ok(Box::new(heap) as _)
    }
}

/// JIT-accessible DRC heap data.
#[derive(Default)]
#[repr(C)]
struct VMDrcHeapDataInner {
    /// The head of the over-approximated-stack-roots list.
    over_approximated_stack_roots: Option<VMGcRef>,

    /// The current size of the over-approximated-stack-roots list.
    current_over_approximated_stack_roots_len: u32,

    /// The size of the over-approximated-stack-roots list immediately after the
    /// last GC.
    over_approximated_stack_roots_len_after_last_gc: u32,
}

#[derive(Default)]
#[repr(transparent)]
struct VMDrcHeapData {
    inner: SendSyncUnsafeCell<VMDrcHeapDataInner>,
}

impl VMDrcHeapData {
    fn over_approximated_stack_roots(&self) -> Option<VMGcRef> {
        // Safety: `inner` is valid to read from.
        unsafe {
            (*self.inner.get())
                .over_approximated_stack_roots
                .as_ref()
                .map(|r: &VMGcRef| r.unchecked_copy())
        }
    }

    fn set_over_approximated_stack_roots(&mut self, gc_ref: Option<VMGcRef>) {
        self.inner.get_mut().over_approximated_stack_roots = gc_ref;
    }

    fn current_over_approximated_stack_roots_len(&self) -> u32 {
        // Safety: `inner` is valid to read from.
        unsafe { (*self.inner.get()).current_over_approximated_stack_roots_len }
    }

    fn increment_current_over_approximated_stack_roots_len(&mut self) {
        self.inner
            .get_mut()
            .current_over_approximated_stack_roots_len += 1;
    }

    fn decrement_current_over_approximated_stack_roots_len(&mut self) {
        let len = &mut self
            .inner
            .get_mut()
            .current_over_approximated_stack_roots_len;
        debug_assert!(*len > 0);
        *len -= 1;
    }

    fn over_approximated_stack_roots_len_after_last_gc(&self) -> u32 {
        // Safety: `inner` is valid to read from.
        unsafe { (*self.inner.get()).over_approximated_stack_roots_len_after_last_gc }
    }

    fn set_over_approximated_stack_roots_len_after_last_gc(&mut self, len: u32) {
        self.inner
            .get_mut()
            .over_approximated_stack_roots_len_after_last_gc = len;
    }
}

/// A deferred reference-counting (DRC) heap.
struct DrcHeap {
    /// For every type that we have allocated in this heap, how do we trace it?
    trace_infos: TraceInfos,

    /// Count of how many no-gc scopes we are currently within.
    no_gc_count: u64,

    /// The head of the over-approximated-stack-roots list.
    ///
    /// Note that this is exposed directly to compiled Wasm code through the
    /// vmctx, so must not move.
    vmctx_data: Box<VMDrcHeapData>,

    /// The storage for the GC heap itself.
    memory: Option<crate::vm::Memory>,

    /// The cached `VMMemoryDefinition` for `self.memory` so that we don't have
    /// to make indirect calls through a `dyn RuntimeLinearMemory` object.
    ///
    /// Must be updated and kept in sync with `self.memory`, cleared when the
    /// memory is taken and updated when the memory is replaced.
    vmmemory: Option<VMMemoryDefinition>,

    /// A free list describing which ranges of the heap are available for use.
    free_list: Option<FreeList>,

    /// Allocations used during tracing, temporarily removed from `self` for
    /// easier borrow-checker management.
    tracing_allocs: Option<TracingAllocs>,

    /// Running total of bytes currently allocated (live objects) in this heap.
    allocated_bytes: usize,
}

struct TracingAllocs {
    /// An explicit stack to avoid recursion when deallocating one object needs
    /// to dec-ref another object, which can then be deallocated and dec-refs
    /// yet another object, etc...
    ///
    /// We store this stack here to reuse the storage and avoid repeated
    /// allocations.
    dec_ref_stack: Vec<VMGcRef>,

    /// An explicit stack for arrays that are too large to push all their
    /// elements onto `dec_ref_stack` at once. Each entry is an array GC
    /// reference and the range of element indices remaining to process.
    large_array_dec_ref_stack: Vec<(VMGcRef, Range<u32>)>,

    /// A batched set of GC refs to deallocate all at once.
    to_dealloc: Vec<VMGcRef>,
}

impl DrcHeap {
    /// Construct a new, default DRC heap.
    fn new(engine: &Engine) -> Result<Self> {
        log::trace!("allocating new DRC heap");
        Ok(Self {
            trace_infos: TraceInfos::new(engine, GC_REF_ARRAY_ELEMS_OFFSET),
            no_gc_count: 0,
            vmctx_data: Box::default(),
            memory: None,
            vmmemory: None,
            free_list: None,
            tracing_allocs: Some(TracingAllocs {
                dec_ref_stack: Vec::with_capacity(1),
                large_array_dec_ref_stack: Vec::with_capacity(1),
                to_dealloc: Vec::with_capacity(1),
            }),
            allocated_bytes: 0,
        })
    }

    fn dealloc(&mut self, gc_ref: VMGcRef) -> Result<()> {
        let drc_ref = drc_ref(&gc_ref);
        let size = self.index(drc_ref)?.object_size;
        let alloc_size = match FreeList::aligned_size(size) {
            Some(size) => size,
            None => bail_bug!("aligned size overflow"),
        };
        let index = gc_ref.heap_index()?;

        // Poison the freed memory so that any stale access is detectable.
        if cfg!(gc_zeal) {
            let index = usize::try_from(index.get())?;
            let alloc_size = usize::try_from(alloc_size)?;
            self.heap_slice_mut()[index..][..alloc_size].fill(POISON);
        }

        self.allocated_bytes -= usize::try_from(alloc_size)?;
        self.free_list
            .as_mut()
            .unwrap()
            .dealloc_fast(index, alloc_size);
        Ok(())
    }

    /// Increment the ref count for the associated object.
    fn inc_ref(&mut self, gc_ref: &VMGcRef) -> Result<()> {
        if gc_ref.is_i31() {
            return Ok(());
        }

        let drc_ref = drc_ref(gc_ref);
        let header = self.index_mut(&drc_ref)?;
        header.inc_ref();
        log::trace!("increment {:#p} ref count -> {}", *gc_ref, header.ref_count);
        Ok(())
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
    ) -> Result<()> {
        if gc_ref.is_i31() {
            return Ok(());
        }

        let allocs = match self.tracing_allocs.take() {
            Some(allocs) => allocs,
            None => bail_bug!("allocs missing during tracing"),
        };
        let mut undo = Undo::new((self, allocs), |(this, allocs)| {
            debug_assert!(this.tracing_allocs.is_none());
            this.tracing_allocs = Some(allocs);
        });
        let (this, allocs) = &mut *undo;
        let stack = &mut allocs.dec_ref_stack;
        let large_array_stack = &mut allocs.large_array_dec_ref_stack;
        let to_dealloc = &mut allocs.to_dealloc;

        debug_assert!(stack.is_empty());
        debug_assert!(large_array_stack.is_empty());
        debug_assert!(to_dealloc.is_empty());

        stack.push(gc_ref.unchecked_copy());

        while !stack.is_empty() || !large_array_stack.is_empty() {
            while let Some(gc_ref) = stack.pop() {
                debug_assert!(!gc_ref.is_i31());

                // Read the DRC header once to get ref_count, type, and object_size.
                let drc_header = this.index_mut(drc_ref(&gc_ref))?;
                log::trace!(
                    "decrement {:#p} ref count -> {}",
                    gc_ref,
                    drc_header.ref_count - 1
                );
                if !drc_header.dec_ref() {
                    continue;
                }

                // Extract type and size from the header we already read (avoiding
                // re-reading from heap).
                let ty = drc_header.header.ty();

                // Trace: enqueue child GC refs for dec-ref'ing.
                if let Some(ty) = ty {
                    match this.trace_infos.trace_info(&ty) {
                        TraceInfo::Struct { gc_ref_offsets } => {
                            stack.reserve(gc_ref_offsets.len());
                            let data = this.gc_object_data(&gc_ref)?;
                            for offset in gc_ref_offsets {
                                Self::trace_offset(stack, data, *offset)?;
                            }
                        }
                        TraceInfo::Array { gc_ref_elems: true } => {
                            let len = this.array_len(gc_ref.as_arrayref_unchecked())?;
                            let len_usize = usize::try_from(len)?;

                            if stack.len() + len_usize <= MAX_ARRAY_STACK_DEPTH {
                                let data = this.gc_object_data(&gc_ref)?;
                                stack.reserve(len_usize);
                                for i in 0..len {
                                    Self::trace_array_elem(stack, data, i)?;
                                }
                            } else {
                                // Only push the first `n` elements onto the
                                // stack; process the rest via the
                                // `large_array_stack`.
                                let n = MAX_ARRAY_STACK_DEPTH.saturating_sub(stack.len());
                                let n = u32::try_from(n)?;
                                let data = this.gc_object_data(&gc_ref)?;
                                for i in 0..n {
                                    Self::trace_array_elem(stack, data, i)?;
                                }
                                large_array_stack.push((gc_ref.unchecked_copy(), n..len));

                                // Don't fallthrough and push onto `to_dealloc`
                                // yet; only do that after we've processed all
                                // elements. This ensures we don't push it
                                // multiple times.
                                continue;
                            }
                        }
                        TraceInfo::Array {
                            gc_ref_elems: false,
                        } => {}
                    }
                } else {
                    // Handle `externref` host data. Only `externref`s have host
                    // data, and `ty` is `None` only for `externref`s, so we skip
                    // this for `struct` and `array` objects entirely.
                    debug_assert!(drc_header.header.kind().matches(VMGcKind::ExternRef));
                    let externref = match gc_ref.as_typed::<VMDrcExternRef>(*this) {
                        Some(r) => r,
                        None => bail_bug!("expected externref"),
                    };
                    let host_data_id = this.index(externref)?.host_data;
                    host_data_table.dealloc(host_data_id)?;
                }

                to_dealloc.push(gc_ref);
            }

            if let Some((gc_ref, mut elems)) = large_array_stack.pop() {
                // Add the next chunk of array elements onto the stack.
                let data = this.gc_object_data(&gc_ref)?;
                for i in elems.by_ref().take(MAX_ARRAY_STACK_DEPTH) {
                    Self::trace_array_elem(stack, data, i)?;
                }

                // If we are done processing this array, then enqueue it for
                // deallocation. Otherwise, push it back onto the
                // `large_array_stack` for continued processing once the regular
                // stack is exhausted again.
                if elems.is_empty() {
                    to_dealloc.push(gc_ref);
                } else {
                    large_array_stack.push((gc_ref, elems));
                }
            }
        }

        // Deallocate the dead objects and return their memory blocks to the
        // free list.
        for gc_ref in to_dealloc.drain(..) {
            this.dealloc(gc_ref)?;
        }

        debug_assert!(stack.is_empty());
        debug_assert!(large_array_stack.is_empty());
        debug_assert!(to_dealloc.is_empty());

        Ok(())
    }

    #[inline]
    fn trace_array_elem(stack: &mut Vec<VMGcRef>, data: &VMGcObjectData, i: u32) -> Result<()> {
        let elem_offset = GC_REF_ARRAY_ELEMS_OFFSET + i * u32::try_from(mem::size_of::<u32>())?;
        Self::trace_offset(stack, data, elem_offset)
    }

    #[inline]
    fn trace_offset(stack: &mut Vec<VMGcRef>, data: &VMGcObjectData, offset: u32) -> Result<()> {
        let raw = data.read_u32(offset)?;
        if let Some(gc_ref) = VMGcRef::from_raw_u32(raw)
            && !gc_ref.is_i31()
        {
            stack.push(gc_ref);
        }
        Ok(())
    }

    /// Iterate over the over-approximated-stack-roots list.
    fn iter_over_approximated_stack_roots(&self) -> impl Iterator<Item = VMGcRef> + '_ {
        let mut link = self.vmctx_data.over_approximated_stack_roots();

        core::iter::from_fn(move || {
            let r = link.as_ref()?.unchecked_copy();
            link = self
                .index(drc_ref(&r))
                .ok()?
                .next_over_approximated_stack_root();
            Some(r)
        })
    }

    /// Assert the integrity of the over-approximated stack roots list.
    fn assert_over_approximated_stack_roots_integrity(&self) -> Result<()> {
        if !cfg!(gc_zeal) {
            return Ok(());
        }

        let mut visited = HashSet::new();
        for gc_ref in self.iter_over_approximated_stack_roots() {
            let idx = gc_ref.heap_index()?.get();

            // Each entry must have a valid `VMGcKind`.
            let header = self.header(&gc_ref)?;
            let kind = header.kind().as_u32();
            assert!(
                VMGcKind::try_from_u32(kind).is_some(),
                "over-approx list: entry at heap index {idx} has invalid VMGcKind {kind:#034b}",
            );

            // Each entry must have its in-list bit set.
            let drc_header = self.index(drc_ref(&gc_ref))?;
            assert!(
                drc_header.is_in_over_approximated_stack_roots(),
                "over-approx list: entry at heap index {idx} does not have in-list bit set",
            );

            // Each entry must have a nonzero ref count.
            assert_ne!(
                drc_header.ref_count, 0,
                "over-approx list: entry at heap index {idx} has zero ref count",
            );

            // No cycles or duplicates.
            assert!(
                visited.insert(idx),
                "over-approx list: cycle or duplicate detected at heap index {idx}",
            );
        }

        assert_eq!(
            self.vmctx_data.current_over_approximated_stack_roots_len() as usize,
            visited.len(),
            "over-approx list: tracked size does not match actual size",
        );
        Ok(())
    }

    /// Assert that every free block in the free list is filled with the poison
    /// pattern.
    fn assert_free_blocks_are_poisoned(&self) {
        if !cfg!(gc_zeal) {
            return;
        }

        let free_list = self.free_list.as_ref().unwrap();
        for (index, len) in free_list.iter_free_blocks() {
            let start = usize::try_from(index).unwrap();
            let size = usize::try_from(len).unwrap();
            let slice = &self.heap_slice()[start..][..size];
            assert!(
                slice.iter().all(|&b| b == POISON),
                "free block at heap index {start} (size {size}) is not fully poisoned",
            );
        }
    }

    fn trace(&mut self, roots: &mut GcRootsIter<'_>) -> Result<()> {
        // The `over_approx_set` is used for `debug_assert!`s checking that
        // every reference we read out from the stack via stack maps is actually
        // in the table. If that weren't true, than either we forgot to insert a
        // reference in the table when passing it into Wasm (a bug) or we are
        // reading invalid references from the stack (another bug).
        let mut over_approx_set: DebugOnly<HashSet<_>> = Default::default();
        if cfg!(debug_assertions) {
            over_approx_set.extend(self.iter_over_approximated_stack_roots());
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

            let gc_ref = root.get()?;

            if gc_ref.is_i31() {
                continue;
            }

            log::trace!("Found GC reference on the stack: {gc_ref:#p}");

            debug_assert!(
                over_approx_set.contains(&gc_ref),
                "every on-stack gc ref inside a Wasm frame should \
                 have be in our over-approximated stack roots set, \
                 but {gc_ref:#p} is not in the set",
            );
            debug_assert!(
                self.index(drc_ref(&gc_ref))?
                    .is_in_over_approximated_stack_roots(),
                "every on-stack gc ref inside a Wasm frame should have \
                 its in-the-over-approximated-stack-roots-list bit set",
            );
            debug_assert_ne!(
                self.index_mut(drc_ref(&gc_ref))?.ref_count,
                0,
                "{gc_ref:#p} is on the Wasm stack and therefore should be held \
                 alive by the over-approximated-stack-roots set; should have \
                 nonzero ref count",
            );

            self.index_mut(drc_ref(&gc_ref))?.set_marked();
        }
        Ok(())
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

    /// Sweep the bump allocation table after we've discovered our precise stack
    /// roots.
    fn sweep(&mut self, host_data_table: &mut ExternRefHostDataTable) -> Result<()> {
        if log::log_enabled!(log::Level::Trace) {
            Self::log_gc_ref_set(
                "over-approximated-stack-roots set before sweeping",
                self.iter_over_approximated_stack_roots(),
            );
        }

        // Logically, we are taking the difference between
        // over-approximated-stack-roots set and the precise-stack-roots set,
        // decrementing the ref count for each object in that difference
        // (because they are no longer live on the stack), and then resetting
        // the over-approximated-stack-roots set to the precise set. In our
        // actual implementation, the over-approximated-stack-roots set is
        // implemented as an intrusive, singly-linked list in the object
        // headers, and the precise-stack-roots set is implemented via the mark
        // bits in the object headers. Therefore, we walk the
        // over-approximated-stack-roots list, checking whether each object has
        // its mark bit set.
        //
        // * If the mark bit is set, then it is in the precise-stack-roots set
        //   and is still on the stack, so we keep it in the
        //   over-approximated-stack-roots list and do not modify its ref count.
        //
        // * If the mark bit is not set, then it is not in the
        //   precise-stack-roots set and is no longer on the stack, so we remove
        //   it from the over-approximated-stack-roots set and decrement its ref
        //   count.
        //
        // We also clear the mark bits as we do this traversal.
        //
        // Finally, note that decrementing ref counts may run `Drop`
        // implementations, which may run arbitrary user code. However, because
        // of our `&mut` borrow on this heap (which ultimately comes from a
        // `&mut Store`) we're guaranteed that nothing will reentrantly touch
        // this heap or run Wasm code in this store.
        log::trace!("Begin sweeping");

        // The `VMGcRef` of the previous object in the
        // over-approximated-stack-roots list, if any.
        let mut prev = None;

        // The `VMGcRef` of the next object in the over-approximated-stack-roots
        // list, if any.
        let mut next = self.vmctx_data.over_approximated_stack_roots();

        while let Some(gc_ref) = next {
            log::trace!("sweeping gc ref: {gc_ref:#p}");

            let header = self.index_mut(drc_ref(&gc_ref))?;
            debug_assert!(header.is_in_over_approximated_stack_roots());

            if header.clear_marked() {
                // This GC ref was marked, meaning it is still on the stack, so
                // keep it in the over-approximated-stack-roots list and move on
                // to the next object in the list.
                log::trace!(
                    "  -> {gc_ref:#p} is marked, leaving it in the over-approximated-\
                     stack-roots list"
                );
                next = header.next_over_approximated_stack_root();
                prev = Some(gc_ref);
                continue;
            }

            // This GC ref was not marked, meaning it is no longer on the stack,
            // so remove it from the over-approximated-stack-roots list and
            // decrement its reference count.
            log::trace!(
                "  -> {gc_ref:#p} is not marked, removing it from over-approximated-\
                 stack-roots list and decrementing its ref count"
            );
            next = header.next_over_approximated_stack_root();
            let prev_next = header.next_over_approximated_stack_root();
            header.set_in_over_approximated_stack_roots_bit(false);
            match &prev {
                None => self.vmctx_data.set_over_approximated_stack_roots(prev_next),
                Some(prev) => self
                    .index_mut(drc_ref(prev))?
                    .set_next_over_approximated_stack_root(prev_next),
            }
            self.vmctx_data
                .decrement_current_over_approximated_stack_roots_len();
            self.dec_ref_and_maybe_dealloc(host_data_table, &gc_ref)?;
        }

        self.vmctx_data
            .set_over_approximated_stack_roots_len_after_last_gc(
                self.vmctx_data.current_over_approximated_stack_roots_len(),
            );

        log::trace!("Done sweeping");

        if log::log_enabled!(log::Level::Trace) {
            Self::log_gc_ref_set(
                "over-approximated-stack-roots set after sweeping",
                self.iter_over_approximated_stack_roots(),
            );
        }

        Ok(())
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
    next_over_approximated_stack_root: Option<VMGcRef>,
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
    #[inline]
    fn object_size(&self) -> usize {
        usize::try_from(self.object_size).unwrap()
    }

    /// Is this object in the over-approximated stack roots list?
    #[inline]
    fn is_in_over_approximated_stack_roots(&self) -> bool {
        self.header.reserved_u26() & wasmtime_environ::drc::HEADER_IN_OVER_APPROX_LIST_BIT != 0
    }

    /// Set whether this object is in the over-approximated stack roots list.
    #[inline]
    fn set_in_over_approximated_stack_roots_bit(&mut self, bit: bool) {
        let reserved = self.header.reserved_u26();
        let new_reserved = if bit {
            reserved | wasmtime_environ::drc::HEADER_IN_OVER_APPROX_LIST_BIT
        } else {
            reserved & !wasmtime_environ::drc::HEADER_IN_OVER_APPROX_LIST_BIT
        };
        self.header.set_reserved_u26(new_reserved);
    }

    /// Get the next object after this one in the over-approximated-stack-roots
    /// list, if any.
    #[inline]
    fn next_over_approximated_stack_root(&self) -> Option<VMGcRef> {
        debug_assert!(self.is_in_over_approximated_stack_roots());
        self.next_over_approximated_stack_root
            .as_ref()
            .map(|r| r.unchecked_copy())
    }

    /// Set the next object after this one in the over-approximated-stack-roots
    /// list.
    #[inline]
    fn set_next_over_approximated_stack_root(&mut self, next: Option<VMGcRef>) {
        debug_assert!(self.is_in_over_approximated_stack_roots());
        self.next_over_approximated_stack_root = next;
    }

    /// Is this object marked?
    #[inline]
    fn is_marked(&self) -> bool {
        self.header.reserved_u26() & wasmtime_environ::drc::HEADER_MARK_BIT != 0
    }

    /// Mark this object.
    ///
    /// Returns `true` if this object was newly marked (i.e. `is_marked()` would
    /// have returned `false` before this call was made).
    #[inline]
    fn set_marked(&mut self) {
        let reserved = self.header.reserved_u26();
        self.header
            .set_reserved_u26(reserved | wasmtime_environ::drc::HEADER_MARK_BIT);
    }

    /// Clear the mark bit for this object.
    ///
    /// Returns `true` if this object was marked before the mark bit was
    /// cleared.
    #[inline]
    fn clear_marked(&mut self) -> bool {
        if self.is_marked() {
            let reserved = self.header.reserved_u26();
            self.header
                .set_reserved_u26(reserved & !wasmtime_environ::drc::HEADER_MARK_BIT);
            debug_assert!(!self.is_marked());
            true
        } else {
            false
        }
    }

    /// Increment the ref count for this object.
    fn inc_ref(&mut self) {
        debug_assert!(self.ref_count > 0);
        self.ref_count += 1;
    }

    /// Decrement the ref count for this object.
    ///
    /// Returns `true` if the ref count reached zero and the object should be
    /// deallocated.
    fn dec_ref(&mut self) -> bool {
        debug_assert!(self.ref_count > 0);
        self.ref_count -= 1;
        self.ref_count == 0
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
    fn is_attached(&self) -> bool {
        debug_assert_eq!(self.memory.is_some(), self.free_list.is_some());
        debug_assert_eq!(self.memory.is_some(), self.vmmemory.is_some());
        self.memory.is_some()
    }

    fn attach(&mut self, memory: crate::vm::Memory) {
        assert!(!self.is_attached());
        assert!(!memory.is_shared_memory());
        debug_assert!(self.vmctx_data.over_approximated_stack_roots().is_none());
        debug_assert_eq!(
            self.vmctx_data.current_over_approximated_stack_roots_len(),
            0
        );
        debug_assert_eq!(
            self.vmctx_data
                .over_approximated_stack_roots_len_after_last_gc(),
            0
        );
        let len = memory.vmmemory().current_length();
        self.free_list = Some(FreeList::new(len));
        self.vmmemory = Some(memory.vmmemory());
        self.memory = Some(memory);

        // Poison the entire heap so any access to uninitialized memory is
        // detectable.
        if cfg!(gc_zeal) {
            self.heap_slice_mut().fill(POISON);
        }
    }

    fn detach(&mut self) -> crate::vm::Memory {
        assert!(self.is_attached());

        let DrcHeap {
            no_gc_count,
            vmctx_data,
            free_list,
            tracing_allocs,
            memory,
            vmmemory,
            allocated_bytes,
            trace_infos,
        } = self;

        *no_gc_count = 0;
        **vmctx_data = VMDrcHeapData::default();
        *free_list = None;
        *vmmemory = None;
        *allocated_bytes = 0;
        trace_infos.clear();

        debug_assert!(tracing_allocs.as_ref().is_some_and(|allocs| {
            allocs.dec_ref_stack.is_empty()
                && allocs.large_array_dec_ref_stack.is_empty()
                && allocs.to_dealloc.is_empty()
        }));

        memory.take().unwrap()
    }

    fn ensure_trace_info(&mut self, ty: VMSharedTypeIndex) {
        self.trace_infos.ensure(ty);
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
        // If incrementing the reference count fails then that means that the GC
        // heap is corrupted. Plumbing this result all throughout Wasmtime has
        // quite large implications which aren't necessarily worth the tradeoff.
        // This is the only collector where this is a fallible operation, for
        // example. For now catch this in debug mode but otherwise just leave
        // the corruption to get detected later. This corrupted reference will
        // trigger an error later on instead.
        if let Err(e) = self.inc_ref(gc_ref) {
            if cfg!(debug_assertions) {
                panic!("gc heap corrupted: {e}");
            }
        }
        gc_ref.unchecked_copy()
    }

    fn write_gc_ref(
        &mut self,
        host_data_table: &mut ExternRefHostDataTable,
        destination: &mut Option<VMGcRef>,
        source: Option<&VMGcRef>,
    ) -> Result<()> {
        // Increment the ref count of the object being written into the slot.
        if let Some(src) = source {
            self.inc_ref(src)?;
        }

        // Decrement the ref count of the value being overwritten and, if
        // necessary, deallocate the GC object.
        if let Some(dest) = destination {
            self.dec_ref_and_maybe_dealloc(host_data_table, dest)?;
        }

        // Do the actual write.
        *destination = source.map(|s| s.unchecked_copy());
        Ok(())
    }

    fn expose_gc_ref_to_wasm(&mut self, gc_ref: VMGcRef) -> Result<()> {
        // Read the current list head before borrowing through index_mut.
        let next = self.vmctx_data.over_approximated_stack_roots();

        let header = self.index_mut(drc_ref(&gc_ref))?;
        if header.is_in_over_approximated_stack_roots() {
            // Already in the over-approximated-stack-roots list. Decrement the
            // object's ref count because the OASR list can't hold multiple
            // copies of the same GC reference.
            let ref_count_is_zero = header.dec_ref();
            debug_assert!(
                !ref_count_is_zero,
                "should not have reached refcount == 0 because the OASR list \
                 is holding a reference"
            );
            return Ok(());
        }

        // Push this object onto the head of the over-approximated-stack-roots
        // list using a single index_mut call.
        header.set_in_over_approximated_stack_roots_bit(true);
        header.set_next_over_approximated_stack_root(next);
        self.vmctx_data
            .set_over_approximated_stack_roots(Some(gc_ref));
        self.vmctx_data
            .increment_current_over_approximated_stack_roots_len();
        Ok(())
    }

    fn alloc_externref(
        &mut self,
        host_data: ExternRefHostDataId,
    ) -> Result<Result<VMExternRef, u64>> {
        let gc_ref =
            match self.alloc_raw(VMGcHeader::externref(), Layout::new::<VMDrcExternRef>())? {
                Err(n) => return Ok(Err(n)),
                Ok(gc_ref) => gc_ref,
            };
        self.index_mut::<VMDrcExternRef>(gc_ref.as_typed_unchecked())?
            .host_data = host_data;
        Ok(Ok(gc_ref.into_externref_unchecked()))
    }

    fn externref_host_data(&self, externref: &VMExternRef) -> Result<ExternRefHostDataId> {
        let typed_ref = externref_to_drc(externref);
        Ok(self.index(typed_ref)?.host_data)
    }

    fn header(&self, gc_ref: &VMGcRef) -> Result<&VMGcHeader> {
        let header: &VMGcHeader = self.index(gc_ref.as_typed_unchecked())?;

        debug_assert!(
            VMGcKind::try_from_u32(header.kind().as_u32()).is_some(),
            "header: invalid VMGcKind {:#010x} at gc_ref {gc_ref:#p}",
            header.kind().as_u32(),
        );

        Ok(header)
    }

    fn header_mut(&mut self, gc_ref: &VMGcRef) -> Result<&mut VMGcHeader> {
        let header: &mut VMGcHeader = self.index_mut(gc_ref.as_typed_unchecked())?;

        debug_assert!(
            VMGcKind::try_from_u32(header.kind().as_u32()).is_some(),
            "header_mut: invalid VMGcKind {:#010x} at gc_ref {gc_ref:#p}",
            header.kind().as_u32(),
        );

        Ok(header)
    }

    fn object_size(&self, gc_ref: &VMGcRef) -> Result<usize> {
        Ok(self.index(drc_ref(gc_ref))?.object_size())
    }

    fn alloc_raw(&mut self, header: VMGcHeader, layout: Layout) -> Result<Result<VMGcRef, u64>> {
        debug_assert!(layout.size() >= core::mem::size_of::<VMDrcHeader>());
        debug_assert!(layout.align() >= core::mem::align_of::<VMDrcHeader>());
        debug_assert!(FreeList::can_align_to(layout.align()));
        debug_assert_eq!(header.reserved_u26(), 0);

        // We must have trace info for every GC type that we allocate in this
        // heap. Trace info is eagerly registered during module instantiation
        // and `StructRefPre`/`ArrayRefPre` construction. The only kinds of GC
        // objects we allocate that do not have an associated
        // `VMSharedTypeIndex` are `externref`s, and they don't have any GC
        // edges.
        if let Some(ty) = header.ty() {
            debug_assert!(
                self.trace_infos.contains(&ty),
                "trace info for {ty:?} should have been eagerly registered",
            );
        } else {
            debug_assert_eq!(header.kind(), VMGcKind::ExternRef);
        }

        let object_size = u32::try_from(layout.size()).unwrap();
        let alloc_size = FreeList::aligned_size(object_size).ok_or(Trap::AllocationTooLarge)?;

        let gc_ref = match self.free_list.as_mut().unwrap().alloc_fast(alloc_size) {
            None => return Ok(Err(u64::try_from(layout.size())?)),
            Some(index) => match VMGcRef::from_heap_index(index) {
                Some(r) => r,
                None => {
                    bail_bug!("invalid GC heap index returned from free list alloc: {index:#x}")
                }
            },
        };

        // Assert that the newly-allocated memory is still filled with the
        // poison pattern, and hasn't been corrupted since deallocation (or
        // initial heap creation).
        if cfg!(gc_zeal) {
            let start = usize::try_from(gc_ref.heap_index()?.get())?;
            let slice = &self.heap_slice()[start..][..layout.size()];
            gc_assert!(
                slice.iter().all(|&b| b == POISON),
                "newly allocated GC object at index {start} is not fully poisoned; \
                 freed memory was corrupted",
            );
        }

        *self.index_mut(drc_ref(&gc_ref))? = VMDrcHeader {
            header,
            ref_count: 1,
            next_over_approximated_stack_root: None,
            object_size,
        };
        self.allocated_bytes += usize::try_from(alloc_size)?;
        log::trace!("new object: increment {gc_ref:#p} ref count -> 1");
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

    fn dealloc_uninit_struct_or_exn(&mut self, gcref: VMGcRef) -> Result<()> {
        self.dealloc(gcref)
    }

    fn alloc_uninit_array(
        &mut self,
        ty: VMSharedTypeIndex,
        length: u32,
        layout: &GcArrayLayout,
    ) -> Result<Result<VMArrayRef, u64>> {
        let layout = layout.layout(length).ok_or(Trap::AllocationTooLarge)?;
        let gc_ref = match self.alloc_raw(
            VMGcHeader::from_kind_and_index(VMGcKind::ArrayRef, ty),
            layout,
        )? {
            Err(n) => return Ok(Err(n)),
            Ok(gc_ref) => gc_ref,
        };

        self.index_mut(gc_ref.as_typed_unchecked::<VMDrcArrayHeader>())?
            .length = length;

        Ok(Ok(gc_ref.into_arrayref_unchecked()))
    }

    fn dealloc_uninit_array(&mut self, arrayref: VMArrayRef) -> Result<()> {
        self.dealloc(arrayref.into())
    }

    fn array_len(&self, arrayref: &VMArrayRef) -> Result<u32> {
        debug_assert!(arrayref.as_gc_ref().is_typed::<VMDrcArrayHeader>(self));
        Ok(self
            .index::<VMDrcArrayHeader>(arrayref.as_gc_ref().as_typed_unchecked())?
            .length)
    }

    fn allocated_bytes(&self) -> usize {
        self.allocated_bytes
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
        let ptr: NonNull<VMDrcHeapData> = NonNull::from(&*self.vmctx_data);
        ptr.cast()
    }

    fn take_memory(&mut self) -> crate::vm::Memory {
        debug_assert!(self.is_attached());
        self.vmmemory.take();
        self.memory.take().unwrap()
    }

    unsafe fn replace_memory(&mut self, memory: crate::vm::Memory, delta_bytes_grown: u64) {
        debug_assert!(self.memory.is_none());
        debug_assert!(!memory.is_shared_memory());
        self.vmmemory = Some(memory.vmmemory());
        self.memory = Some(memory);

        // Poison the newly-grown region so stale accesses are detectable.
        if cfg!(gc_zeal) {
            let old_cap = self.free_list.as_ref().unwrap().current_capacity();
            let new_bytes = usize::try_from(delta_bytes_grown).unwrap();
            let slice = self.heap_slice_mut();
            if old_cap + new_bytes <= slice.len() {
                slice[old_cap..old_cap + new_bytes].fill(POISON);
            }
        }

        self.free_list
            .as_mut()
            .unwrap()
            .add_capacity(usize::try_from(delta_bytes_grown).unwrap())
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
    fn collect_increment(&mut self) -> Result<GcProgress> {
        match self.phase {
            DrcCollectionPhase::Trace => {
                #[cfg(feature = "std")]
                let start = std::time::Instant::now();
                log::debug!("Begin DRC trace");

                self.heap.assert_over_approximated_stack_roots_integrity()?;
                self.heap.assert_free_blocks_are_poisoned();

                self.heap.trace(&mut self.roots)?;

                self.heap.assert_over_approximated_stack_roots_integrity()?;
                self.heap.assert_free_blocks_are_poisoned();

                log::debug!("End DRC trace");
                #[cfg(feature = "std")]
                log::debug!("  -> {:.3} seconds", start.elapsed().as_secs_f64());

                self.phase = DrcCollectionPhase::Sweep;
                Ok(GcProgress::Continue)
            }
            DrcCollectionPhase::Sweep => {
                #[cfg(feature = "std")]
                let start = std::time::Instant::now();
                log::debug!("Begin DRC sweep");

                self.heap.assert_over_approximated_stack_roots_integrity()?;
                self.heap.assert_free_blocks_are_poisoned();

                self.heap.sweep(self.host_data_table)?;

                self.heap.assert_over_approximated_stack_roots_integrity()?;
                self.heap.assert_free_blocks_are_poisoned();

                log::debug!("End DRC sweep");
                #[cfg(feature = "std")]
                log::debug!("  -> {:.3} seconds", start.elapsed().as_secs_f64());

                self.phase = DrcCollectionPhase::Done;
                Ok(GcProgress::Complete)
            }
            DrcCollectionPhase::Done => Ok(GcProgress::Complete),
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
    use wasmtime_environ::{HostPtr, PtrSize};

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
            next_over_approximated_stack_root: None,
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
            num_runtime_data: 0,
            has_startup_func: false,
        });

        assert_eq!(
            offsets.vm_drc_header_ref_count(),
            u32::try_from(actual_offset).unwrap(),
        );
    }

    #[test]
    fn vm_drc_heap_data_over_approximated_stack_roots_offset() {
        assert_eq!(
            HostPtr.vmdrc_heap_data_over_approximated_stack_roots() as usize,
            core::mem::offset_of!(VMDrcHeapDataInner, over_approximated_stack_roots),
        );
    }

    #[test]
    fn vm_drc_heap_data_current_over_approximated_stack_roots_len_offset() {
        assert_eq!(
            HostPtr.vmdrc_heap_data_current_over_approximated_stack_roots_len() as usize,
            core::mem::offset_of!(
                VMDrcHeapDataInner,
                current_over_approximated_stack_roots_len
            ),
        );
    }

    #[test]
    fn vm_drc_heap_data_over_approximated_stack_roots_len_after_last_gc_offset() {
        assert_eq!(
            HostPtr.vmdrc_heap_data_over_approximated_stack_roots_len_after_last_gc() as usize,
            core::mem::offset_of!(
                VMDrcHeapDataInner,
                over_approximated_stack_roots_len_after_last_gc
            ),
        );
    }

    #[test]
    fn vm_drc_heap_data_size() {
        assert_eq!(
            HostPtr.size_of_vmdrc_heap_data() as usize,
            core::mem::size_of::<VMDrcHeapData>(),
        );
        assert_eq!(
            HostPtr.size_of_vmdrc_heap_data() as usize,
            core::mem::size_of::<VMDrcHeapDataInner>(),
        );
    }

    #[test]
    fn vm_drc_heap_data_align() {
        assert_eq!(
            HostPtr.align_of_vmdrc_heap_data() as usize,
            core::mem::align_of::<VMDrcHeapData>(),
        );
        assert_eq!(
            HostPtr.align_of_vmdrc_heap_data() as usize,
            core::mem::align_of::<VMDrcHeapDataInner>(),
        );
    }
}
