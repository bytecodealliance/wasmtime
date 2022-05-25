//! # `VMExternRef`
//!
//! `VMExternRef` is a reference-counted box for any kind of data that is
//! external and opaque to running Wasm. Sometimes it might hold a Wasmtime
//! thing, other times it might hold something from a Wasmtime embedder and is
//! opaque even to us. It is morally equivalent to `Rc<dyn Any>` in Rust, but
//! additionally always fits in a pointer-sized word. `VMExternRef` is
//! non-nullable, but `Option<VMExternRef>` is a null pointer.
//!
//! The one part of `VMExternRef` that can't ever be opaque to us is the
//! reference count. Even when we don't know what's inside an `VMExternRef`, we
//! need to be able to manipulate its reference count as we add and remove
//! references to it. And we need to do this from compiled Wasm code, so it must
//! be `repr(C)`!
//!
//! ## Memory Layout
//!
//! `VMExternRef` itself is just a pointer to an `VMExternData`, which holds the
//! opaque, boxed value, its reference count, and its vtable pointer.
//!
//! The `VMExternData` struct is *preceded* by the dynamically-sized value boxed
//! up and referenced by one or more `VMExternRef`s:
//!
//! ```text
//!      ,-------------------------------------------------------.
//!      |                                                       |
//!      V                                                       |
//!     +----------------------------+-----------+-----------+   |
//!     | dynamically-sized value... | ref_count | value_ptr |---'
//!     +----------------------------+-----------+-----------+
//!                                  | VMExternData          |
//!                                  +-----------------------+
//!                                   ^
//! +-------------+                   |
//! | VMExternRef |-------------------+
//! +-------------+                   |
//!                                   |
//! +-------------+                   |
//! | VMExternRef |-------------------+
//! +-------------+                   |
//!                                   |
//!   ...                            ===
//!                                   |
//! +-------------+                   |
//! | VMExternRef |-------------------'
//! +-------------+
//! ```
//!
//! The `value_ptr` member always points backwards to the start of the
//! dynamically-sized value (which is also the start of the heap allocation for
//! this value-and-`VMExternData` pair). Because it is a `dyn` pointer, it is
//! fat, and also points to the value's `Any` vtable.
//!
//! The boxed value and the `VMExternRef` footer are held a single heap
//! allocation. The layout described above is used to make satisfying the
//! value's alignment easy: we just need to ensure that the heap allocation used
//! to hold everything satisfies its alignment. It also ensures that we don't
//! need a ton of excess padding between the `VMExternData` and the value for
//! values with large alignment.
//!
//! ## Reference Counting, Wasm Functions, and Garbage Collection
//!
//! For host VM code, we use plain reference counting, where cloning increments
//! the reference count, and dropping decrements it. We can avoid many of the
//! on-stack increment/decrement operations that typically plague the
//! performance of reference counting via Rust's ownership and borrowing system.
//! Moving a `VMExternRef` avoids mutating its reference count, and borrowing it
//! either avoids the reference count increment or delays it until if/when the
//! `VMExternRef` is cloned.
//!
//! When passing a `VMExternRef` into compiled Wasm code, we don't want to do
//! reference count mutations for every compiled `local.{get,set}`, nor for
//! every function call. Therefore, we use a variation of **deferred reference
//! counting**, where we only mutate reference counts when storing
//! `VMExternRef`s somewhere that outlives the activation: into a global or
//! table. Simultaneously, we over-approximate the set of `VMExternRef`s that
//! are inside Wasm function activations. Periodically, we walk the stack at GC
//! safe points, and use stack map information to precisely identify the set of
//! `VMExternRef`s inside Wasm activations. Then we take the difference between
//! this precise set and our over-approximation, and decrement the reference
//! count for each of the `VMExternRef`s that are in our over-approximation but
//! not in the precise set. Finally, the over-approximation is replaced with the
//! precise set.
//!
//! The `VMExternRefActivationsTable` implements the over-approximized set of
//! `VMExternRef`s referenced by Wasm activations. Calling a Wasm function and
//! passing it a `VMExternRef` moves the `VMExternRef` into the table, and the
//! compiled Wasm function logically "borrows" the `VMExternRef` from the
//! table. Similarly, `global.get` and `table.get` operations clone the gotten
//! `VMExternRef` into the `VMExternRefActivationsTable` and then "borrow" the
//! reference out of the table.
//!
//! When a `VMExternRef` is returned to host code from a Wasm function, the host
//! increments the reference count (because the reference is logically
//! "borrowed" from the `VMExternRefActivationsTable` and the reference count
//! from the table will be dropped at the next GC).
//!
//! For more general information on deferred reference counting, see *An
//! Examination of Deferred Reference Counting and Cycle Detection* by Quinane:
//! <https://openresearch-repository.anu.edu.au/bitstream/1885/42030/2/hon-thesis.pdf>

use std::alloc::Layout;
use std::any::Any;
use std::cell::UnsafeCell;
use std::cmp;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::mem;
use std::ops::Deref;
use std::ptr::{self, NonNull};
use std::sync::atomic::{self, AtomicUsize, Ordering};
use wasmtime_environ::StackMap;

/// An external reference to some opaque data.
///
/// `VMExternRef`s dereference to their underlying opaque data as `dyn Any`.
///
/// Unlike the `externref` in the Wasm spec, `VMExternRef`s are non-nullable,
/// and always point to a valid value. You may use `Option<VMExternRef>` to
/// represent nullable references, and `Option<VMExternRef>` is guaranteed to
/// have the same size and alignment as a raw pointer, with `None` represented
/// with the null pointer.
///
/// `VMExternRef`s are reference counted, so cloning is a cheap, shallow
/// operation. It also means they are inherently shared, so you may not get a
/// mutable, exclusive reference to their inner contents, only a shared,
/// immutable reference. You may use interior mutability with `RefCell` or
/// `Mutex` to work around this restriction, if necessary.
///
/// `VMExternRef`s have pointer-equality semantics, not structural-equality
/// semantics. Given two `VMExternRef`s `a` and `b`, `a == b` only if `a` and
/// `b` point to the same allocation. `a` and `b` are considered not equal, even
/// if `a` and `b` are two different identical copies of the same data, if they
/// are in two different allocations. The hashing and ordering implementations
/// also only operate on the pointer.
///
/// # Example
///
/// ```
/// # fn foo() -> Result<(), Box<dyn std::error::Error>> {
/// use std::cell::RefCell;
/// use wasmtime_runtime::VMExternRef;
///
/// // Open a file. Wasm doesn't know about files, but we can let Wasm instances
/// // work with files via opaque `externref` handles.
/// let file = std::fs::File::create("some/file/path")?;
///
/// // Wrap the file up as an `VMExternRef` that can be passed to Wasm.
/// let extern_ref_to_file = VMExternRef::new(file);
///
/// // `VMExternRef`s dereference to `dyn Any`, so you can use `Any` methods to
/// // perform runtime type checks and downcasts.
///
/// assert!(extern_ref_to_file.is::<std::fs::File>());
/// assert!(!extern_ref_to_file.is::<String>());
///
/// if let Some(mut file) = extern_ref_to_file.downcast_ref::<std::fs::File>() {
///     use std::io::Write;
///     writeln!(&mut file, "Hello, `VMExternRef`!")?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
#[repr(transparent)]
pub struct VMExternRef(NonNull<VMExternData>);

// Data contained is always Send+Sync so these should be safe.
unsafe impl Send for VMExternRef {}
unsafe impl Sync for VMExternRef {}

#[repr(C)]
pub(crate) struct VMExternData {
    // Implicit, dynamically-sized member that always preceded an
    // `VMExternData`.
    //
    // value: [u8],
    //
    /// The reference count for this `VMExternData` and value. When it reaches
    /// zero, we can safely destroy the value and free this heap
    /// allocation. This is an `UnsafeCell`, rather than plain `Cell`, because
    /// it can be modified by compiled Wasm code.
    ///
    /// Note: this field's offset must be kept in sync with
    /// `wasmtime_environ::VMOffsets::vm_extern_data_ref_count()` which is
    /// currently always zero.
    ref_count: AtomicUsize,

    /// Always points to the implicit, dynamically-sized `value` member that
    /// precedes this `VMExternData`.
    value_ptr: NonNull<dyn Any + Send + Sync>,
}

impl Clone for VMExternRef {
    #[inline]
    fn clone(&self) -> VMExternRef {
        self.extern_data().increment_ref_count();
        VMExternRef(self.0)
    }
}

impl Drop for VMExternRef {
    #[inline]
    fn drop(&mut self) {
        let data = self.extern_data();

        // Note that the memory orderings here also match the standard library
        // itself. Documentation is more available in the implementation of
        // `Arc`, but the general idea is that this is a special pattern allowed
        // by the C standard with atomic orderings where we "release" for all
        // the decrements and only the final decrementer performs an acquire
        // fence. This properly ensures that the final thread, which actually
        // destroys the data, sees all the updates from all other threads.
        if data.ref_count.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        atomic::fence(Ordering::Acquire);

        // Drop our live reference to `data` before we drop it itself.
        drop(data);
        unsafe {
            VMExternData::drop_and_dealloc(self.0);
        }
    }
}

impl VMExternData {
    /// Get the `Layout` for a value with the given size and alignment, and the
    /// offset within that layout where the `VMExternData` footer resides.
    ///
    /// This doesn't take a `value: &T` because `VMExternRef::new_with` hasn't
    /// constructed a `T` value yet, and it isn't generic over `T` because
    /// `VMExternData::drop_and_dealloc` doesn't know what `T` to use, and has
    /// to use `std::mem::{size,align}_of_val` instead.
    unsafe fn layout_for(value_size: usize, value_align: usize) -> (Layout, usize) {
        let extern_data_size = mem::size_of::<VMExternData>();
        let extern_data_align = mem::align_of::<VMExternData>();

        let value_and_padding_size = round_up_to_align(value_size, extern_data_align).unwrap();

        let alloc_align = std::cmp::max(value_align, extern_data_align);
        let alloc_size = value_and_padding_size + extern_data_size;

        debug_assert!(Layout::from_size_align(alloc_size, alloc_align).is_ok());
        (
            Layout::from_size_align_unchecked(alloc_size, alloc_align),
            value_and_padding_size,
        )
    }

    /// Drop the inner value and then free this `VMExternData` heap allocation.
    pub(crate) unsafe fn drop_and_dealloc(mut data: NonNull<VMExternData>) {
        // Note: we introduce a block scope so that we drop the live
        // reference to the data before we free the heap allocation it
        // resides within after this block.
        let (alloc_ptr, layout) = {
            let data = data.as_mut();
            debug_assert_eq!(data.ref_count.load(Ordering::SeqCst), 0);

            // Same thing, but for the dropping the reference to `value` before
            // we drop it itself.
            let (layout, _) = {
                let value = data.value_ptr.as_ref();
                Self::layout_for(mem::size_of_val(value), mem::align_of_val(value))
            };

            ptr::drop_in_place(data.value_ptr.as_ptr());
            let alloc_ptr = data.value_ptr.cast::<u8>();

            (alloc_ptr, layout)
        };

        ptr::drop_in_place(data.as_ptr());
        std::alloc::dealloc(alloc_ptr.as_ptr(), layout);
    }

    #[inline]
    fn increment_ref_count(&self) {
        // This is only using during cloning operations, and like the standard
        // library we use `Relaxed` here. The rationale is better documented in
        // libstd's implementation of `Arc`, but the general gist is that we're
        // creating a new pointer for our own thread, so there's no need to have
        // any synchronization with orderings. The synchronization with other
        // threads with respect to orderings happens when the pointer is sent to
        // another thread.
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }
}

#[inline]
fn round_up_to_align(n: usize, align: usize) -> Option<usize> {
    debug_assert!(align.is_power_of_two());
    let align_minus_one = align - 1;
    Some(n.checked_add(align_minus_one)? & !align_minus_one)
}

impl VMExternRef {
    /// Wrap the given value inside an `VMExternRef`.
    pub fn new<T>(value: T) -> VMExternRef
    where
        T: 'static + Any + Send + Sync,
    {
        VMExternRef::new_with(|| value)
    }

    /// Construct a new `VMExternRef` in place by invoking `make_value`.
    pub fn new_with<T>(make_value: impl FnOnce() -> T) -> VMExternRef
    where
        T: 'static + Any + Send + Sync,
    {
        unsafe {
            let (layout, footer_offset) =
                VMExternData::layout_for(mem::size_of::<T>(), mem::align_of::<T>());

            let alloc_ptr = std::alloc::alloc(layout);
            let alloc_ptr = NonNull::new(alloc_ptr).unwrap_or_else(|| {
                std::alloc::handle_alloc_error(layout);
            });

            let value_ptr = alloc_ptr.cast::<T>();
            ptr::write(value_ptr.as_ptr(), make_value());

            let extern_data_ptr =
                alloc_ptr.cast::<u8>().as_ptr().add(footer_offset) as *mut VMExternData;
            ptr::write(
                extern_data_ptr,
                VMExternData {
                    ref_count: AtomicUsize::new(1),
                    // Cast from `*mut T` to `*mut dyn Any` here.
                    value_ptr: NonNull::new_unchecked(value_ptr.as_ptr()),
                },
            );

            VMExternRef(NonNull::new_unchecked(extern_data_ptr))
        }
    }

    /// Turn this `VMExternRef` into a raw, untyped pointer.
    ///
    /// Unlike `into_raw`, this does not consume and forget `self`. It is *not*
    /// safe to use `from_raw` on pointers returned from this method; only use
    /// `clone_from_raw`!
    ///
    ///  Nor does this method increment the reference count. You must ensure
    ///  that `self` (or some other clone of `self`) stays alive until
    ///  `clone_from_raw` is called.
    #[inline]
    pub fn as_raw(&self) -> *mut u8 {
        let ptr = self.0.cast::<u8>().as_ptr();
        ptr
    }

    /// Consume this `VMExternRef` into a raw, untyped pointer.
    ///
    /// # Safety
    ///
    /// This method forgets self, so it is possible to create a leak of the
    /// underlying reference counted data if not used carefully.
    ///
    /// Use `from_raw` to recreate the `VMExternRef`.
    pub unsafe fn into_raw(self) -> *mut u8 {
        let ptr = self.0.cast::<u8>().as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Recreate a `VMExternRef` from a pointer returned from a previous call to
    /// `as_raw`.
    ///
    /// # Safety
    ///
    /// Unlike `clone_from_raw`, this does not increment the reference count of the
    /// underlying data.  It is not safe to continue to use the pointer passed to this
    /// function.
    pub unsafe fn from_raw(ptr: *mut u8) -> Self {
        debug_assert!(!ptr.is_null());
        VMExternRef(NonNull::new_unchecked(ptr).cast())
    }

    /// Recreate a `VMExternRef` from a pointer returned from a previous call to
    /// `as_raw`.
    ///
    /// # Safety
    ///
    /// Wildly unsafe to use with anything other than the result of a previous
    /// `as_raw` call!
    ///
    /// Additionally, it is your responsibility to ensure that this raw
    /// `VMExternRef`'s reference count has not dropped to zero. Failure to do
    /// so will result in use after free!
    pub unsafe fn clone_from_raw(ptr: *mut u8) -> Self {
        debug_assert!(!ptr.is_null());
        let x = VMExternRef(NonNull::new_unchecked(ptr).cast());
        x.extern_data().increment_ref_count();
        x
    }

    /// Get the strong reference count for this `VMExternRef`.
    ///
    /// Note that this loads with a `SeqCst` ordering to synchronize with other
    /// threads.
    pub fn strong_count(&self) -> usize {
        self.extern_data().ref_count.load(Ordering::SeqCst)
    }

    #[inline]
    fn extern_data(&self) -> &VMExternData {
        unsafe { self.0.as_ref() }
    }
}

/// Methods that would normally be trait implementations, but aren't to avoid
/// potential footguns around `VMExternRef`'s pointer-equality semantics.
///
/// Note that none of these methods are on `&self`, they all require a
/// fully-qualified `VMExternRef::foo(my_ref)` invocation.
impl VMExternRef {
    /// Check whether two `VMExternRef`s point to the same inner allocation.
    ///
    /// Note that this uses pointer-equality semantics, not structural-equality
    /// semantics, and so only pointers are compared, and doesn't use any `Eq`
    /// or `PartialEq` implementation of the pointed-to values.
    #[inline]
    pub fn eq(a: &Self, b: &Self) -> bool {
        ptr::eq(a.0.as_ptr(), b.0.as_ptr())
    }

    /// Hash a given `VMExternRef`.
    ///
    /// Note that this just hashes the pointer to the inner value, it does *not*
    /// use the inner value's `Hash` implementation (if any).
    #[inline]
    pub fn hash<H>(externref: &Self, hasher: &mut H)
    where
        H: Hasher,
    {
        ptr::hash(externref.0.as_ptr(), hasher);
    }

    /// Compare two `VMExternRef`s.
    ///
    /// Note that this uses pointer-equality semantics, not structural-equality
    /// semantics, and so only pointers are compared, and doesn't use any `Cmp`
    /// or `PartialCmp` implementation of the pointed-to values.
    #[inline]
    pub fn cmp(a: &Self, b: &Self) -> cmp::Ordering {
        let a = a.0.as_ptr() as usize;
        let b = b.0.as_ptr() as usize;
        a.cmp(&b)
    }
}

impl Deref for VMExternRef {
    type Target = dyn Any;

    fn deref(&self) -> &dyn Any {
        unsafe { self.extern_data().value_ptr.as_ref() }
    }
}

/// A wrapper around a `VMExternRef` that implements `Eq` and `Hash` with
/// pointer semantics.
///
/// We use this so that we can morally put `VMExternRef`s inside of `HashSet`s
/// even though they don't implement `Eq` and `Hash` to avoid foot guns.
#[derive(Clone, Debug)]
struct VMExternRefWithTraits(VMExternRef);

impl Hash for VMExternRefWithTraits {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        VMExternRef::hash(&self.0, hasher)
    }
}

impl PartialEq for VMExternRefWithTraits {
    fn eq(&self, other: &Self) -> bool {
        VMExternRef::eq(&self.0, &other.0)
    }
}

impl Eq for VMExternRefWithTraits {}

type TableElem = UnsafeCell<Option<VMExternRef>>;

/// A table that over-approximizes the set of `VMExternRef`s that any Wasm
/// activation on this thread is currently using.
///
/// Under the covers, this is a simple bump allocator that allows duplicate
/// entries. Deduplication happens at GC time.
#[repr(C)] // `alloc` must be the first member, it's accessed from JIT code.
pub struct VMExternRefActivationsTable {
    /// Structures used to perform fast bump allocation of storage of externref
    /// values.
    ///
    /// This is the only member of this structure accessed from JIT code.
    alloc: VMExternRefTableAlloc,

    /// When unioned with `chunk`, this is an over-approximation of the GC roots
    /// on the stack, inside Wasm frames.
    ///
    /// This is used by slow-path insertion, and when a GC cycle finishes, is
    /// re-initialized to the just-discovered precise set of stack roots (which
    /// immediately becomes an over-approximation again as soon as Wasm runs and
    /// potentially drops references).
    over_approximated_stack_roots: HashSet<VMExternRefWithTraits>,

    /// The precise set of on-stack, inside-Wasm GC roots that we discover via
    /// walking the stack and interpreting stack maps.
    ///
    /// This is *only* used inside the `gc` function, and is empty otherwise. It
    /// is just part of this struct so that we can reuse the allocation, rather
    /// than create a new hash set every GC.
    precise_stack_roots: HashSet<VMExternRefWithTraits>,

    /// A pointer to the youngest host stack frame before we called
    /// into Wasm for the first time. When walking the stack in garbage
    /// collection, if we don't find this frame, then we failed to walk every
    /// Wasm stack frame, which means we failed to find all on-stack,
    /// inside-a-Wasm-frame roots, and doing a GC could lead to freeing one of
    /// those missed roots, and use after free.
    stack_canary: Option<usize>,

    /// A debug-only field for asserting that we are in a region of code where
    /// GC is okay to preform.
    #[cfg(debug_assertions)]
    gc_okay: bool,
}

#[repr(C)] // This is accessed from JIT code.
struct VMExternRefTableAlloc {
    /// Bump-allocation finger within the `chunk`.
    ///
    /// NB: this is an `UnsafeCell` because it is written to by compiled Wasm
    /// code.
    next: UnsafeCell<NonNull<TableElem>>,

    /// Pointer to just after the `chunk`.
    ///
    /// This is *not* within the current chunk and therefore is not a valid
    /// place to insert a reference!
    end: NonNull<TableElem>,

    /// Bump allocation chunk that stores fast-path insertions.
    ///
    /// This is not accessed from JIT code.
    chunk: Box<[TableElem]>,
}

// This gets around the usage of `UnsafeCell` throughout the internals of this
// allocator, but the storage should all be Send/Sync and synchronization isn't
// necessary since operations require `&mut self`.
unsafe impl Send for VMExternRefTableAlloc {}
unsafe impl Sync for VMExternRefTableAlloc {}

fn _assert_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<VMExternRefActivationsTable>();
    _assert::<VMExternRef>();
}

impl VMExternRefActivationsTable {
    const CHUNK_SIZE: usize = 4096 / mem::size_of::<usize>();

    /// Create a new `VMExternRefActivationsTable`.
    pub fn new() -> Self {
        // Start with an empty chunk in case this activations table isn't used.
        // This means that there's no space in the bump-allocation area which
        // will force any path trying to use this to the slow gc path. The first
        // time this happens, though, the slow gc path will allocate a new chunk
        // for actual fast-bumping.
        let mut chunk: Box<[TableElem]> = Box::new([]);
        let next = chunk.as_mut_ptr();
        let end = unsafe { next.add(chunk.len()) };

        VMExternRefActivationsTable {
            alloc: VMExternRefTableAlloc {
                next: UnsafeCell::new(NonNull::new(next).unwrap()),
                end: NonNull::new(end).unwrap(),
                chunk,
            },
            over_approximated_stack_roots: HashSet::new(),
            precise_stack_roots: HashSet::new(),
            stack_canary: None,
            #[cfg(debug_assertions)]
            gc_okay: true,
        }
    }

    fn new_chunk(size: usize) -> Box<[UnsafeCell<Option<VMExternRef>>]> {
        assert!(size >= Self::CHUNK_SIZE);
        (0..size).map(|_| UnsafeCell::new(None)).collect()
    }

    /// Get the available capacity in the bump allocation chunk.
    #[inline]
    pub fn bump_capacity_remaining(&self) -> usize {
        let end = self.alloc.end.as_ptr() as usize;
        let next = unsafe { *self.alloc.next.get() };
        end - next.as_ptr() as usize
    }

    /// Try and insert a `VMExternRef` into this table.
    ///
    /// This is a fast path that only succeeds when the bump chunk has the
    /// capacity for the requested insertion.
    ///
    /// If the insertion fails, then the `VMExternRef` is given back. Callers
    /// may attempt a GC to free up space and try again, or may call
    /// `insert_slow_path` to infallibly insert the reference (potentially
    /// allocating additional space in the table to hold it).
    #[inline]
    pub fn try_insert(&mut self, externref: VMExternRef) -> Result<(), VMExternRef> {
        unsafe {
            let next = *self.alloc.next.get();
            if next == self.alloc.end {
                return Err(externref);
            }

            debug_assert!(
                (*next.as_ref().get()).is_none(),
                "slots >= the `next` bump finger are always `None`"
            );
            ptr::write(next.as_ptr(), UnsafeCell::new(Some(externref)));

            let next = NonNull::new_unchecked(next.as_ptr().add(1));
            debug_assert!(next <= self.alloc.end);
            *self.alloc.next.get() = next;

            Ok(())
        }
    }

    /// Insert a reference into the table, falling back on a GC to clear up
    /// space if the table is already full.
    ///
    /// # Unsafety
    ///
    /// The same as `gc`.
    #[inline]
    pub unsafe fn insert_with_gc(
        &mut self,
        externref: VMExternRef,
        module_info_lookup: &dyn ModuleInfoLookup,
    ) {
        #[cfg(debug_assertions)]
        assert!(self.gc_okay);

        if let Err(externref) = self.try_insert(externref) {
            self.gc_and_insert_slow(externref, module_info_lookup);
        }
    }

    #[inline(never)]
    unsafe fn gc_and_insert_slow(
        &mut self,
        externref: VMExternRef,
        module_info_lookup: &dyn ModuleInfoLookup,
    ) {
        gc(module_info_lookup, self);

        // Might as well insert right into the hash set, rather than the bump
        // chunk, since we are already on a slow path and we get de-duplication
        // this way.
        self.over_approximated_stack_roots
            .insert(VMExternRefWithTraits(externref));
    }

    /// Insert a reference into the table, without ever performing GC.
    #[inline]
    pub fn insert_without_gc(&mut self, externref: VMExternRef) {
        if let Err(externref) = self.try_insert(externref) {
            self.insert_slow_without_gc(externref);
        }
    }

    #[inline(never)]
    fn insert_slow_without_gc(&mut self, externref: VMExternRef) {
        self.over_approximated_stack_roots
            .insert(VMExternRefWithTraits(externref));
    }

    fn num_filled_in_bump_chunk(&self) -> usize {
        let next = unsafe { *self.alloc.next.get() };
        let bytes_unused = (self.alloc.end.as_ptr() as usize) - (next.as_ptr() as usize);
        let slots_unused = bytes_unused / mem::size_of::<TableElem>();
        self.alloc.chunk.len().saturating_sub(slots_unused)
    }

    fn elements(&self, mut f: impl FnMut(&VMExternRef)) {
        for elem in self.over_approximated_stack_roots.iter() {
            f(&elem.0);
        }

        // The bump chunk is not all the way full, so we only iterate over its
        // filled-in slots.
        let num_filled = self.num_filled_in_bump_chunk();
        for slot in self.alloc.chunk.iter().take(num_filled) {
            if let Some(elem) = unsafe { &*slot.get() } {
                f(elem);
            }
        }
    }

    fn insert_precise_stack_root(
        precise_stack_roots: &mut HashSet<VMExternRefWithTraits>,
        root: NonNull<VMExternData>,
    ) {
        let root = unsafe { VMExternRef::clone_from_raw(root.as_ptr().cast()) };
        precise_stack_roots.insert(VMExternRefWithTraits(root));
    }

    /// Sweep the bump allocation table after we've discovered our precise stack
    /// roots.
    fn sweep(&mut self) {
        // Sweep our bump chunk.
        let num_filled = self.num_filled_in_bump_chunk();
        unsafe {
            *self.alloc.next.get() = self.alloc.end;
        }
        for slot in self.alloc.chunk.iter().take(num_filled) {
            unsafe {
                *slot.get() = None;
            }
        }
        debug_assert!(
            self.alloc
                .chunk
                .iter()
                .all(|slot| unsafe { (*slot.get()).as_ref().is_none() }),
            "after sweeping the bump chunk, all slots should be `None`"
        );

        // If this is the first instance of gc then the initial chunk is empty,
        // so we lazily allocate space for fast bump-allocation in the future.
        if self.alloc.chunk.is_empty() {
            self.alloc.chunk = Self::new_chunk(Self::CHUNK_SIZE);
            self.alloc.end =
                NonNull::new(unsafe { self.alloc.chunk.as_mut_ptr().add(self.alloc.chunk.len()) })
                    .unwrap();
        }

        // Reset our `next` finger to the start of the bump allocation chunk.
        unsafe {
            let next = self.alloc.chunk.as_mut_ptr();
            debug_assert!(!next.is_null());
            *self.alloc.next.get() = NonNull::new_unchecked(next);
        }

        // The current `precise_stack_roots` becomes our new over-appoximated
        // set for the next GC cycle.
        mem::swap(
            &mut self.precise_stack_roots,
            &mut self.over_approximated_stack_roots,
        );

        // And finally, the new `precise_stack_roots` should be cleared and
        // remain empty until the next GC cycle.
        //
        // Note that this may run arbitrary code as we run externref
        // destructors. Because of our `&mut` borrow above on this table,
        // though, we're guaranteed that nothing will touch this table.
        self.precise_stack_roots.clear();
    }

    /// Fetches the current value of this table's stack canary.
    ///
    /// This should only be used in conjunction with setting the stack canary
    /// below if the return value is `None` typically. This is called from RAII
    /// guards in `wasmtime::func::invoke_wasm_and_catch_traps`.
    ///
    /// For more information on canaries see the gc functions below.
    #[inline]
    pub fn stack_canary(&self) -> Option<usize> {
        self.stack_canary
    }

    /// Sets the current value of the stack canary.
    ///
    /// This is called from RAII guards in
    /// `wasmtime::func::invoke_wasm_and_catch_traps`. This is used to update
    /// the stack canary to a concrete value and then reset it back to `None`
    /// when wasm is finished.
    ///
    /// For more information on canaries see the gc functions below.
    #[inline]
    pub fn set_stack_canary(&mut self, canary: Option<usize>) {
        self.stack_canary = canary;
    }

    /// Set whether it is okay to GC or not right now.
    ///
    /// This is provided as a helper for enabling various debug-only assertions
    /// and checking places where the `wasmtime-runtime` user expects there not
    /// to be any GCs.
    #[inline]
    pub fn set_gc_okay(&mut self, okay: bool) -> bool {
        #[cfg(debug_assertions)]
        {
            return std::mem::replace(&mut self.gc_okay, okay);
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = okay;
            return true;
        }
    }
}

/// Used by the runtime to lookup information about a module given a
/// program counter value.
pub trait ModuleInfoLookup {
    /// Lookup the module information from a program counter value.
    fn lookup(&self, pc: usize) -> Option<&dyn ModuleInfo>;
}

/// Used by the runtime to query module information.
pub trait ModuleInfo {
    /// Lookup the stack map at a program counter value.
    fn lookup_stack_map(&self, pc: usize) -> Option<&StackMap>;
}

#[derive(Debug, Default)]
struct DebugOnly<T> {
    inner: T,
}

impl<T> std::ops::Deref for DebugOnly<T> {
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

impl<T> std::ops::DerefMut for DebugOnly<T> {
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

/// Perform garbage collection of `VMExternRef`s.
///
/// # Unsafety
///
/// You must have called `VMExternRefActivationsTable::set_stack_canary` for at
/// least the oldest host-->Wasm stack frame transition on this thread's stack
/// (it is idempotent to call it more than once) and keep its return value alive
/// across the duration of that host-->Wasm call.
///
/// Additionally, you must have registered the stack maps for every Wasm module
/// that has frames on the stack with the given `stack_maps_registry`.
pub unsafe fn gc(
    module_info_lookup: &dyn ModuleInfoLookup,
    externref_activations_table: &mut VMExternRefActivationsTable,
) {
    log::debug!("start GC");

    #[cfg(debug_assertions)]
    assert!(externref_activations_table.gc_okay);

    debug_assert!({
        // This set is only non-empty within this function. It is built up when
        // walking the stack and interpreting stack maps, and then drained back
        // into the activations table's bump-allocated space at the
        // end. Therefore, it should always be empty upon entering this
        // function.
        externref_activations_table.precise_stack_roots.is_empty()
    });

    // Whenever we call into Wasm from host code for the first time, we set a
    // stack canary. When we return to that host code, we unset the stack
    // canary. If there is *not* a stack canary, then there must be zero Wasm
    // frames on the stack. Therefore, we can simply reset the table without
    // walking the stack.
    let stack_canary = match externref_activations_table.stack_canary {
        None => {
            if cfg!(debug_assertions) {
                // Assert that there aren't any Wasm frames on the stack.
                backtrace::trace(|frame| {
                    assert!(module_info_lookup.lookup(frame.ip() as usize).is_none());
                    true
                });
            }
            externref_activations_table.sweep();
            log::debug!("end GC");
            return;
        }
        Some(canary) => canary,
    };

    // There is a stack canary, so there must be Wasm frames on the stack. The
    // rest of this function consists of:
    //
    // * walking the stack,
    //
    // * finding the precise set of roots inside Wasm frames via our stack maps,
    //   and
    //
    // * resetting our bump-allocated table's over-approximation to the
    //   newly-discovered precise set.

    // The SP of the previous (younger) frame we processed.
    let mut last_sp: Option<usize> = None;

    // Whether we have found our stack canary or not yet.
    let mut found_canary = false;

    // The `activations_table_set` is used for `debug_assert!`s checking that
    // every reference we read out from the stack via stack maps is actually in
    // the table. If that weren't true, than either we forgot to insert a
    // reference in the table when passing it into Wasm (a bug) or we are
    // reading invalid references from the stack (another bug).
    let mut activations_table_set: DebugOnly<HashSet<_>> = Default::default();
    if cfg!(debug_assertions) {
        externref_activations_table.elements(|elem| {
            activations_table_set.insert(elem.as_raw() as *mut VMExternData);
        });
    }

    backtrace::trace(|frame| {
        let pc = frame.ip() as usize;
        let sp = frame.sp() as usize;

        if let Some(module_info) = module_info_lookup.lookup(pc) {
            if let Some(stack_map) = module_info.lookup_stack_map(pc) {
                debug_assert!(sp != 0, "we should always get a valid SP for Wasm frames");

                for i in 0..(stack_map.mapped_words() as usize) {
                    if stack_map.get_bit(i) {
                        // Stack maps have one bit per word in the frame, and the
                        // zero^th bit is the *lowest* addressed word in the frame,
                        // i.e. the closest to the SP. So to get the `i`^th word in
                        // this frame, we add `i * sizeof(word)` to the SP.
                        let ptr_to_ref = sp + i * mem::size_of::<usize>();

                        let r = std::ptr::read(ptr_to_ref as *const *mut VMExternData);
                        debug_assert!(
                            r.is_null() || activations_table_set.contains(&r),
                            "every on-stack externref inside a Wasm frame should \
                            have an entry in the VMExternRefActivationsTable; \
                            {:?} is not in the table",
                            r
                        );
                        if let Some(r) = NonNull::new(r) {
                            VMExternRefActivationsTable::insert_precise_stack_root(
                                &mut externref_activations_table.precise_stack_roots,
                                r,
                            );
                        }
                    }
                }
            }
        }

        if let Some(last_sp) = last_sp {
            // We've found the stack canary when we walk over the frame that it
            // is contained within.
            found_canary |= last_sp <= stack_canary && stack_canary <= sp;
        }
        last_sp = Some(sp);

        // Keep walking the stack until we've found the canary, which is the
        // oldest frame before we ever called into Wasm. We can stop once we've
        // found it because there won't be any more Wasm frames, and therefore
        // there won't be anymore on-stack, inside-a-Wasm-frame roots.
        !found_canary
    });

    // Only sweep and reset the table if we found the stack canary, and
    // therefore know that we discovered all the on-stack, inside-a-Wasm-frame
    // roots. If we did *not* find the stack canary, then `libunwind` failed to
    // walk the whole stack, and we might be missing roots. Reseting the table
    // would free those missing roots while they are still in use, leading to
    // use-after-free.
    if found_canary {
        externref_activations_table.sweep();
    } else {
        log::warn!("did not find stack canary; skipping GC sweep");
        externref_activations_table.precise_stack_roots.clear();
    }

    log::debug!("end GC");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn extern_ref_is_pointer_sized_and_aligned() {
        assert_eq!(mem::size_of::<VMExternRef>(), mem::size_of::<*mut ()>());
        assert_eq!(mem::align_of::<VMExternRef>(), mem::align_of::<*mut ()>());
        assert_eq!(
            mem::size_of::<Option<VMExternRef>>(),
            mem::size_of::<*mut ()>()
        );
        assert_eq!(
            mem::align_of::<Option<VMExternRef>>(),
            mem::align_of::<*mut ()>()
        );
    }

    #[test]
    fn ref_count_is_at_correct_offset() {
        let s = "hi";
        let s: &(dyn Any + Send + Sync) = &s as _;
        let s: *const (dyn Any + Send + Sync) = s as _;
        let s: *mut (dyn Any + Send + Sync) = s as _;

        let extern_data = VMExternData {
            ref_count: AtomicUsize::new(0),
            value_ptr: NonNull::new(s).unwrap(),
        };

        let extern_data_ptr = &extern_data as *const _;
        let ref_count_ptr = &extern_data.ref_count as *const _;

        let actual_offset = (ref_count_ptr as usize) - (extern_data_ptr as usize);

        let offsets = wasmtime_environ::VMOffsets::from(wasmtime_environ::VMOffsetsFields {
            ptr: 8,
            num_imported_functions: 0,
            num_imported_tables: 0,
            num_imported_memories: 0,
            num_imported_globals: 0,
            num_defined_functions: 0,
            num_defined_tables: 0,
            num_defined_memories: 0,
            num_defined_globals: 0,
            num_escaped_funcs: 0,
        });
        assert_eq!(
            offsets.vm_extern_data_ref_count(),
            actual_offset.try_into().unwrap(),
        );
    }

    #[test]
    fn table_next_is_at_correct_offset() {
        let table = VMExternRefActivationsTable::new();

        let table_ptr = &table as *const _;
        let next_ptr = &table.alloc.next as *const _;

        let actual_offset = (next_ptr as usize) - (table_ptr as usize);

        let offsets = wasmtime_environ::VMOffsets::from(wasmtime_environ::VMOffsetsFields {
            ptr: 8,
            num_imported_functions: 0,
            num_imported_tables: 0,
            num_imported_memories: 0,
            num_imported_globals: 0,
            num_defined_functions: 0,
            num_defined_tables: 0,
            num_defined_memories: 0,
            num_defined_globals: 0,
            num_escaped_funcs: 0,
        });
        assert_eq!(
            offsets.vm_extern_ref_activation_table_next() as usize,
            actual_offset
        );
    }

    #[test]
    fn table_end_is_at_correct_offset() {
        let table = VMExternRefActivationsTable::new();

        let table_ptr = &table as *const _;
        let end_ptr = &table.alloc.end as *const _;

        let actual_offset = (end_ptr as usize) - (table_ptr as usize);

        let offsets = wasmtime_environ::VMOffsets::from(wasmtime_environ::VMOffsetsFields {
            ptr: 8,
            num_imported_functions: 0,
            num_imported_tables: 0,
            num_imported_memories: 0,
            num_imported_globals: 0,
            num_defined_functions: 0,
            num_defined_tables: 0,
            num_defined_memories: 0,
            num_defined_globals: 0,
            num_escaped_funcs: 0,
        });
        assert_eq!(
            offsets.vm_extern_ref_activation_table_end() as usize,
            actual_offset
        );
    }
}
