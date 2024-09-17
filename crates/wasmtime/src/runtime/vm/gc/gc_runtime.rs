//! Traits for abstracting over our different garbage collectors.

use crate::prelude::*;
use crate::runtime::vm::{
    ExternRefHostDataId, ExternRefHostDataTable, SendSyncPtr, VMArrayRef, VMExternRef, VMGcHeader,
    VMGcRef, VMStructRef,
};
use core::alloc::Layout;
use core::marker;
use core::ptr;
use core::{any::Any, num::NonZeroUsize};
use wasmtime_environ::{GcArrayLayout, GcStructLayout, GcTypeLayouts, VMSharedTypeIndex};

use super::VMGcObjectDataMut;

/// Trait for integrating a garbage collector with the runtime.
///
/// This trait is responsible for:
///
/// * GC barriers used by runtime code (as opposed to compiled Wasm code)
///
/// * Creating and managing GC heaps for individual stores
///
/// * Running garbage collection
///
/// # Safety
///
/// The collector, its GC heaps, and GC barriers when taken together as a whole
/// must be safe. Additionally, they must work with the GC barriers emitted into
/// compiled Wasm code via the collector's corresponding `GcCompiler`
/// implementation. That is, if callers only call safe methods on this trait
/// (while pairing it with its associated `GcCompiler`, `GcHeap`, and etc...)
/// and uphold all the documented safety invariants of this trait's unsafe
/// methods, then it must be impossible for callers to violate memory
/// safety. Implementations of this trait may not add new safety invariants, not
/// already documented in this trait's interface, that callers need to uphold.
pub unsafe trait GcRuntime: 'static + Send + Sync {
    /// Get this collector's GC type layouts.
    fn layouts(&self) -> &dyn GcTypeLayouts;

    /// Construct a new GC heap.
    fn new_gc_heap(&self) -> Result<Box<dyn GcHeap>>;
}

/// A heap that manages garbage-collected objects.
///
/// Each `wasmtime::Store` is associated with a single `GcHeap`, and a `GcHeap`
/// is only ever used with one store at a time, but `GcHeap`s may be reused with
/// new stores after its original store is dropped. The `reset` method will be
/// called in between each such reuse. (This reuse allows for better integration
/// with the pooling allocator).
///
/// If a `GcHeap` mapped any memory, its `Drop` implementation should unmap that
/// memory.
///
/// # Safety
///
/// The trait methods below are all safe: implementations of this trait must
/// ensure that these methods cannot be misused to create memory unsafety. The
/// expectation is that -- given that `VMGcRef` is a newtype over an index --
/// implementations perform similar tricks as Wasm linear memory
/// implementations. The heap should internally be a contiguous region of memory
/// and `VMGcRef` indices into the heap must be bounds checked (explicitly or
/// implicitly via virtual memory tricks).
///
/// Furthermore, if heap corruption occurs because (for example) a `VMGcRef`
/// from a different heap is used with this heap, then that corruption must be
/// limited to within this heap. Every heap is a mini sandbox. It follows that
/// native pointers should never be written into or read out from the GC heap,
/// since that could spread corruption from inside the GC heap out to the native
/// host heap. The host data for an `externref`, therefore, is stored in a side
/// table (`ExternRefHostDataTable`) and never inside the heap. Only an id
/// referencing a slot in that table should ever be written into the GC heap.
///
/// These constraints give us great amounts of safety compared to working with
/// raw pointers. The worst that could happen is corruption local to heap and a
/// panic, or perhaps reading stale heap data from a previous Wasm instance. A
/// corrupt `GcHeap` can *never* result in the native host's corruption.
///
/// The downside is that we are introducing `heap_base + index` computations and
/// bounds checking to access GC memory, adding performance overhead. This is
/// deemed to be a worthy trade off. Furthermore, it isn't even a clear cut
/// performance degradation since this allows us to use 32-bit "pointers",
/// giving us more compact data representations and the improved cache
/// utilization that implies.
pub unsafe trait GcHeap: 'static + Send + Sync {
    ////////////////////////////////////////////////////////////////////////////
    // `Any` methods

    /// Get this heap as an `&Any`.
    fn as_any(&self) -> &dyn Any;

    /// Get this heap as an `&mut Any`.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    ////////////////////////////////////////////////////////////////////////////
    // No-GC Scope Methods

    /// Enter a no-GC scope.
    ///
    /// Calling the `gc` method when we are inside a no-GC scope should panic.
    ///
    /// We can enter multiple, nested no-GC scopes and this method should
    /// account for that.
    fn enter_no_gc_scope(&mut self);

    /// Exit a no-GC scope.
    ///
    /// Dual to `enter_no_gc_scope`.
    fn exit_no_gc_scope(&mut self);

    ////////////////////////////////////////////////////////////////////////////
    // GC Object Header Methods

    /// Get a shared borrow of the `VMGcHeader` that this GC reference is
    /// pointing to.
    fn header(&self, gc_ref: &VMGcRef) -> &VMGcHeader;

    ////////////////////////////////////////////////////////////////////////////
    // GC Barriers

    /// Read barrier called every time the runtime clones a GC reference.
    ///
    /// Callers should pass a valid `VMGcRef` that belongs to the given
    /// heap. Failure to do so is memory safe, but may result in general
    /// failures such as panics or incorrect results.
    fn clone_gc_ref(&mut self, gc_ref: &VMGcRef) -> VMGcRef;

    /// Write barrier called whenever the runtime is nulling out a GC reference.
    ///
    /// Default implemented in terms of the `write_gc_ref` barrier.
    ///
    /// If an `externref` is reclaimed, then its associated entry in the
    /// `host_data_table` should be removed.
    ///
    /// Callers should pass a valid `VMGcRef` that belongs to the given
    /// heap. Failure to do so is memory safe, but may result in general
    /// failures such as panics or incorrect results.
    ///
    /// The given `gc_ref` should not be used again.
    fn drop_gc_ref(&mut self, host_data_table: &mut ExternRefHostDataTable, gc_ref: VMGcRef) {
        let mut dest = Some(gc_ref);
        self.write_gc_ref(host_data_table, &mut dest, None);
    }

    /// Write barrier called every time the runtime overwrites a GC reference.
    ///
    /// The `source` is a borrowed GC reference, and should not have been cloned
    /// already for this write operation. This allows implementations to fuse
    /// the `source`'s read barrier into this write barrier.
    ///
    /// If an `externref` is reclaimed, then its associated entry in the
    /// `host_data_table` should be removed.
    ///
    /// Callers should pass a valid `VMGcRef` that belongs to the given heap for
    /// both the `source` and `destination`. Failure to do so is memory safe,
    /// but may result in general failures such as panics or incorrect results.
    fn write_gc_ref(
        &mut self,
        host_data_table: &mut ExternRefHostDataTable,
        destination: &mut Option<VMGcRef>,
        source: Option<&VMGcRef>,
    );

    /// Read barrier called whenever a GC reference is passed from the runtime
    /// to Wasm: an argument to a host-to-Wasm call, or a return from a
    /// Wasm-to-host call.
    ///
    /// Callers should pass a valid `VMGcRef` that belongs to the given
    /// heap. Failure to do so is memory safe, but may result in general
    /// failures such as panics or incorrect results.
    fn expose_gc_ref_to_wasm(&mut self, gc_ref: VMGcRef);

    /// Predicate invoked before calling into or returning to Wasm to determine
    /// whether we should GC first.
    ///
    /// `num_gc_refs` is the number of non-`i31ref` GC references that will be
    /// passed into Wasm.
    fn need_gc_before_entering_wasm(&self, num_gc_refs: NonZeroUsize) -> bool;

    ////////////////////////////////////////////////////////////////////////////
    // `externref` Methods

    /// Allocate a `VMExternRef` with space for host data described by the given
    /// layout.
    ///
    /// Return values:
    ///
    /// * `Ok(Some(_))`: The allocation was successful.
    ///
    /// * `Ok(None)`: There is currently no available space for this
    ///   allocation. The caller should call `self.gc()`, run the GC to
    ///   completion so the collector can reclaim space, and then try allocating
    ///   again.
    ///
    /// * `Err(_)`: The collector cannot satisfy this allocation request, and
    ///   would not be able to even after the caller were to trigger a
    ///   collection. This could be because, for example, the requested
    ///   allocation is larger than this collector's implementation limit for
    ///   object size.
    fn alloc_externref(&mut self, host_data: ExternRefHostDataId) -> Result<Option<VMExternRef>>;

    /// Get the host data ID associated with the given `externref`.
    ///
    /// Callers should pass a valid `externref` that belongs to the given
    /// heap. Failure to do so is memory safe, but may result in general
    /// failures such as panics or incorrect results.
    fn externref_host_data(&self, externref: &VMExternRef) -> ExternRefHostDataId;

    ////////////////////////////////////////////////////////////////////////////
    // Struct and Array methods

    /// Allocate a raw, uninitialized GC-managed object with the given header
    /// and layout.
    ///
    /// The object's fields and elements are left uninitialized. It is the
    /// caller's responsibility to initialize them before exposing the struct to
    /// Wasm or triggering a GC.
    ///
    /// The header's described type and layout must match *for this
    /// collector*. That is, if this collector adds an extra header word to all
    /// objects, the given layout must already include space for that header
    /// word. Therefore, this method is effectively only usable with layouts
    /// derived from a `Gc{Struct,Array}Layout` returned by this collector.
    ///
    /// Failure to uphold any of the above is memory safe, but may result in
    /// general failures such as panics or incorrect results.
    ///
    /// Return values:
    ///
    /// * `Ok(Some(_))`: The allocation was successful.
    ///
    /// * `Ok(None)`: There is currently no available space for this
    ///   allocation. The caller should call `self.gc()`, run the GC to
    ///   completion so the collector can reclaim space, and then try allocating
    ///   again.
    ///
    /// * `Err(_)`: The collector cannot satisfy this allocation request, and
    ///   would not be able to even after the caller were to trigger a
    ///   collection. This could be because, for example, the requested
    ///   alignment is larger than this collector's implementation limit.
    fn alloc_raw(&mut self, header: VMGcHeader, layout: Layout) -> Result<Option<VMGcRef>>;

    /// Allocate a GC-managed struct of the given type and layout.
    ///
    /// The struct's fields are left uninitialized. It is the caller's
    /// responsibility to initialize them before exposing the struct to Wasm or
    /// triggering a GC.
    ///
    /// The `ty` and `layout` must match.
    ///
    /// Failure to do either of the above is memory safe, but may result in
    /// general failures such as panics or incorrect results.
    ///
    /// Return values:
    ///
    /// * `Ok(Some(_))`: The allocation was successful.
    ///
    /// * `Ok(None)`: There is currently no available space for this
    ///   allocation. The caller should call `self.gc()`, run the GC to
    ///   completion so the collector can reclaim space, and then try allocating
    ///   again.
    ///
    /// * `Err(_)`: The collector cannot satisfy this allocation request, and
    ///   would not be able to even after the caller were to trigger a
    ///   collection. This could be because, for example, the requested
    ///   allocation is larger than this collector's implementation limit for
    ///   object size.
    fn alloc_uninit_struct(
        &mut self,
        ty: VMSharedTypeIndex,
        layout: &GcStructLayout,
    ) -> Result<Option<VMStructRef>>;

    /// Deallocate an uninitialized, GC-managed struct.
    ///
    /// This is useful for if initialization of the struct's fields fails, so
    /// that the struct's allocation can be eagerly reclaimed, and so that the
    /// collector doesn't attempt to treat any of the uninitialized fields as
    /// valid GC references, or something like that.
    fn dealloc_uninit_struct(&mut self, structref: VMStructRef);

    /// Get a mutable borrow of the the given object's data.
    ///
    /// Panics on out-of-bounds accesses.
    fn gc_object_data(&mut self, gc_ref: &VMGcRef) -> VMGcObjectDataMut<'_>;

    /// Allocate a GC-managed array of the given type and length.
    ///
    /// The array's elements are left uninitialized. It is the caller's
    /// responsibility to initialize them before exposing the array to Wasm or
    /// triggering a GC. Failure to do this is memory safe, but may result in
    /// general failures such as panics or incorrect results.
    ///
    /// Return values:
    ///
    /// * `Ok(Some(_))`: The allocation was successful.
    ///
    /// * `Ok(None)`: There is currently no available space for this
    ///   allocation. The caller should call `self.gc()`, run the GC to
    ///   completion so the collector can reclaim space, and then try allocating
    ///   again.
    ///
    /// * `Err(_)`: The collector cannot satisfy this allocation request, and
    ///   would not be able to even after the caller were to trigger a
    ///   collection. This could be because, for example, the requested
    ///   allocation is larger than this collector's implementation limit for
    ///   object size.
    fn alloc_uninit_array(
        &mut self,
        ty: VMSharedTypeIndex,
        len: u32,
        layout: &GcArrayLayout,
    ) -> Result<Option<VMArrayRef>>;

    /// Deallocate an uninitialized, GC-managed array.
    ///
    /// This is useful for if initialization of the array's fields fails, so
    /// that the array's allocation can be eagerly reclaimed, and so that the
    /// collector doesn't attempt to treat any of the uninitialized fields as
    /// valid GC references, or something like that.
    fn dealloc_uninit_array(&mut self, arrayref: VMArrayRef);

    /// Get the length of the given array.
    ///
    /// Panics on out-of-bounds accesses.
    ///
    /// The given `arrayref` should be valid and of the given size. Failure to
    /// do so is memory safe, but may result in general failures such as panics
    /// or incorrect results.
    fn array_len(&self, arrayref: &VMArrayRef) -> u32;

    ////////////////////////////////////////////////////////////////////////////
    // Garbage Collection Methods

    /// Start a new garbage collection process.
    ///
    /// The given `roots` are GC roots and should not be collected (nor anything
    /// transitively reachable from them).
    ///
    /// Upon reclaiming an `externref`, its associated entry in the
    /// `host_data_table` is removed.
    ///
    /// Callers should pass valid GC roots that belongs to this heap, and the
    /// host data table associated with this heap's `externref`s. Failure to do
    /// so is memory safe, but may result in general failures such as panics or
    /// incorrect results.
    ///
    /// This method should panic if we are in a no-GC scope.
    fn gc<'a>(
        &'a mut self,
        roots: GcRootsIter<'a>,
        host_data_table: &'a mut ExternRefHostDataTable,
    ) -> Box<dyn GarbageCollection<'a> + 'a>;

    ////////////////////////////////////////////////////////////////////////////
    // JIT-Code Interaction Methods

    /// Get the GC heap's base pointer.
    ///
    /// # Safety
    ///
    /// The memory region
    ///
    /// ```ignore
    /// self.vmctx_gc_heap_base..self.vmctx_gc_heap_base + self.vmctx_gc_heap_bound
    /// ```
    ///
    /// must be the GC heap region, and must remain valid for JIT code as long
    /// as `self` is not dropped.
    unsafe fn vmctx_gc_heap_base(&self) -> *mut u8;

    /// Get the GC heap's bound.
    ///
    /// # Safety
    ///
    /// The memory region
    ///
    /// ```ignore
    /// self.vmctx_gc_heap_base..self.vmctx_gc_heap_base + self.vmctx_gc_heap_bound
    /// ```
    ///
    /// must be the GC heap region, and must remain valid for JIT code as long
    /// as `self` is not dropped.
    unsafe fn vmctx_gc_heap_bound(&self) -> usize;

    /// Get the pointer that will be stored in the `VMContext::gc_heap_data`
    /// field and be accessible from JIT code via collaboration with the
    /// corresponding `GcCompiler` trait.
    ///
    /// # Safety
    ///
    /// The returned pointer, if any, must remain valid as long as `self` is not
    /// dropped.
    unsafe fn vmctx_gc_heap_data(&self) -> *mut u8;

    ////////////////////////////////////////////////////////////////////////////
    // Recycling GC Heap Methods

    /// Reset this heap.
    ///
    /// Calling this method unassociates this heap with the store that it has
    /// been associated with, making it available to be associated with a new
    /// heap.
    ///
    /// This should refill free lists, reset bump pointers, and etc... as if
    /// nothing were allocated in this heap (because nothing is allocated in
    /// this heap anymore).
    ///
    /// This should retain any allocated memory from the global allocator and
    /// any virtual memory mappings.
    ///
    /// This method is only used with the pooling allocator.
    #[cfg(feature = "pooling-allocator")]
    fn reset(&mut self);
}

/// A list of GC roots.
///
/// This is effectively a builder for a `GcRootsIter` that will be given to a GC
/// heap when it is time to perform garbage collection.
#[derive(Default)]
pub struct GcRootsList(Vec<RawGcRoot>);

// Ideally these `*mut`s would be `&mut`s and we wouldn't need as much of this
// machinery around `GcRootsList`, `RawGcRoot`, `GcRoot`, and `GcRootIter` but
// if we try that then we run into two different kinds of lifetime issues:
//
// 1. When collecting the various roots from a `&mut StoreOpaque`, we borrow
//    from `self` to push new GC roots onto the roots list. But then we want to
//    call helper methods like `self.for_each_global(...)`, but we can't because
//    there are active borrows of `self` preventing it.
//
// 2. We want to reuse the roots list and its backing storage across GCs, rather
//    than reallocate on every GC. But the only place for the roots list to live
//    such that it is easily reusable across GCs is in the store itself. But the
//    contents of the roots list (when it is non-empty, during GCs) borrow from
//    the store, which creates self-references.
#[derive(Clone, Copy, Debug)]
enum RawGcRoot {
    Stack(SendSyncPtr<u32>),
    NonStack(SendSyncPtr<VMGcRef>),
}

impl GcRootsList {
    /// Add a GC root that is inside a Wasm stack frame to this list.
    #[inline]
    pub unsafe fn add_wasm_stack_root(&mut self, ptr_to_root: SendSyncPtr<u32>) {
        log::trace!(
            "Adding Wasm stack root: {:#p} -> {:#p}",
            ptr_to_root,
            VMGcRef::from_raw_u32(*ptr_to_root.as_ref()).unwrap()
        );
        debug_assert!(VMGcRef::from_raw_u32(*ptr_to_root.as_ref()).is_some());
        self.0.push(RawGcRoot::Stack(ptr_to_root));
    }

    /// Add a GC root to this list.
    #[inline]
    pub unsafe fn add_root(&mut self, ptr_to_root: SendSyncPtr<VMGcRef>, why: &str) {
        log::trace!(
            "Adding non-stack root: {why}: {:#p}",
            ptr_to_root.as_ref().unchecked_copy()
        );
        self.0.push(RawGcRoot::NonStack(ptr_to_root))
    }

    /// Get an iterator over all roots in this list.
    ///
    /// # Safety
    ///
    /// Callers must ensure that all the pointers to GC roots that have been
    /// added to this list are valid for the duration of the `'a` lifetime.
    #[inline]
    pub unsafe fn iter<'a>(&'a mut self) -> GcRootsIter<'a> {
        GcRootsIter {
            list: self,
            index: 0,
        }
    }

    /// Is this list empty?
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clear this GC roots list.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// An iterator over all the roots in a `GcRootsList`.
pub struct GcRootsIter<'a> {
    list: &'a mut GcRootsList,
    index: usize,
}

impl<'a> Iterator for GcRootsIter<'a> {
    type Item = GcRoot<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let root = GcRoot {
            raw: self.list.0.get(self.index).copied()?,
            _phantom: marker::PhantomData,
        };
        self.index += 1;
        Some(root)
    }
}

/// A GC root.
///
/// This is, effectively, a mutable reference to a `VMGcRef`.
///
/// Collector implementations should update the `VMGcRef` if they move the
/// `VMGcRef`'s referent during the course of a GC.
#[derive(Debug)]
pub struct GcRoot<'a> {
    raw: RawGcRoot,
    _phantom: marker::PhantomData<&'a mut VMGcRef>,
}

impl GcRoot<'_> {
    /// Is this root from inside a Wasm stack frame?
    #[inline]
    pub fn is_on_wasm_stack(&self) -> bool {
        matches!(self.raw, RawGcRoot::Stack(_))
    }

    /// Get this GC root.
    ///
    /// Does NOT run GC barriers.
    #[inline]
    pub fn get(&self) -> VMGcRef {
        match self.raw {
            RawGcRoot::NonStack(ptr) => unsafe { ptr::read(ptr.as_ptr()) },
            RawGcRoot::Stack(ptr) => unsafe {
                let raw: u32 = ptr::read(ptr.as_ptr());
                VMGcRef::from_raw_u32(raw).expect("non-null")
            },
        }
    }

    /// Set this GC root.
    ///
    /// Does NOT run GC barriers.
    ///
    /// Collector implementations should use this method to update GC root
    /// pointers after the collector moves the GC object that the root is
    /// referencing.
    pub fn set(&mut self, new_ref: VMGcRef) {
        match self.raw {
            RawGcRoot::NonStack(ptr) => unsafe {
                ptr::write(ptr.as_ptr(), new_ref);
            },
            RawGcRoot::Stack(ptr) => unsafe {
                ptr::write(ptr.as_ptr(), new_ref.as_raw_u32());
            },
        }
    }
}

/// A garbage collection process.
///
/// Implementations define the `collect_increment` method, and then consumers
/// can either use
///
/// * `GarbageCollection::collect` for synchronous code, or
///
/// * `collect_async(Box<dyn GarbageCollection>)` for async code.
///
/// When using fuel and/or epochs, consumers can also use `collect_increment`
/// directly and choose to abandon further execution in this GC's heap's whole
/// store if the GC is taking too long to complete.
pub trait GarbageCollection<'a>: Send + Sync {
    /// Perform an incremental slice of this garbage collection process.
    ///
    /// Upon completion of the slice, a `GcProgress` is returned which informs
    /// the caller whether to continue driving this GC process forward and
    /// executing more slices (`GcProgress::Continue`) or whether the GC process
    /// has finished (`GcProgress::Complete`).
    ///
    /// The mutator does *not* run in between increments. This method exists
    /// solely to allow cooperative yielding
    fn collect_increment(&mut self) -> GcProgress;

    /// Run this GC process to completion.
    ///
    /// Keeps calling `collect_increment` in a loop until the GC process is
    /// complete.
    fn collect(&mut self) {
        loop {
            match self.collect_increment() {
                GcProgress::Continue => continue,
                GcProgress::Complete => return,
            }
        }
    }
}

/// The result of doing an incremental amount of GC.
pub enum GcProgress {
    /// There is still more work to do.
    Continue,
    /// The GC is complete.
    Complete,
}

/// Asynchronously run the given garbage collection process to completion,
/// cooperatively yielding back to the event loop after each increment of work.
#[cfg(feature = "async")]
pub async fn collect_async<'a>(mut collection: Box<dyn GarbageCollection<'a> + 'a>) {
    loop {
        match collection.collect_increment() {
            GcProgress::Continue => crate::runtime::vm::Yield::new().await,
            GcProgress::Complete => return,
        }
    }
}

#[cfg(all(test, feature = "async"))]
mod collect_async_tests {
    use super::*;

    #[test]
    fn is_send_and_sync() {
        fn _assert_send_sync<T: Send + Sync>(_: T) {}

        fn _foo<'a>(collection: Box<dyn GarbageCollection<'a>>) {
            _assert_send_sync(collect_async(collection));
        }
    }
}
