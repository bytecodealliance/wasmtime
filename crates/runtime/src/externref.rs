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
//! ```ignore
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
//! https://openresearch-repository.anu.edu.au/bitstream/1885/42030/2/hon-thesis.pdf

use std::alloc::Layout;
use std::any::Any;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::hash::Hasher;
use std::mem;
use std::ops::Deref;
use std::ptr::{self, NonNull};
use std::sync::{Arc, RwLock};
use wasmtime_environ::{ir::Stackmap, StackMapInformation};

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
/// let extern_ref_to_file = VMExternRef::new(RefCell::new(file));
///
/// // `VMExternRef`s dereference to `dyn Any`, so you can use `Any` methods to
/// // perform runtime type checks and downcasts.
///
/// assert!(extern_ref_to_file.is::<RefCell<std::fs::File>>());
/// assert!(!extern_ref_to_file.is::<String>());
///
/// if let Some(file) = extern_ref_to_file.downcast_ref::<RefCell<std::fs::File>>() {
///     use std::io::Write;
///     let mut file = file.borrow_mut();
///     writeln!(&mut file, "Hello, `VMExternRef`!")?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
#[repr(transparent)]
pub struct VMExternRef(NonNull<VMExternData>);

#[repr(C)]
struct VMExternData {
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
    ref_count: UnsafeCell<usize>,

    /// Always points to the implicit, dynamically-sized `value` member that
    /// precedes this `VMExternData`.
    value_ptr: NonNull<dyn Any>,
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
        data.decrement_ref_count();
        if data.get_ref_count() == 0 {
            // Drop our live reference to `data` before we drop it itself.
            drop(data);
            unsafe {
                VMExternData::drop_and_dealloc(self.0);
            }
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
    unsafe fn drop_and_dealloc(mut data: NonNull<VMExternData>) {
        // Note: we introduce a block scope so that we drop the live
        // reference to the data before we free the heap allocation it
        // resides within after this block.
        let (alloc_ptr, layout) = {
            let data = data.as_mut();
            debug_assert_eq!(data.get_ref_count(), 0);

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
    fn get_ref_count(&self) -> usize {
        unsafe { *self.ref_count.get() }
    }

    #[inline]
    fn increment_ref_count(&self) {
        unsafe {
            let count = self.ref_count.get();
            *count += 1;
        }
    }

    #[inline]
    fn decrement_ref_count(&self) {
        unsafe {
            let count = self.ref_count.get();
            *count -= 1;
        }
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
        T: 'static + Any,
    {
        VMExternRef::new_with(|| value)
    }

    /// Construct a new `VMExternRef` in place by invoking `make_value`.
    pub fn new_with<T>(make_value: impl FnOnce() -> T) -> VMExternRef
    where
        T: 'static + Any,
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

            let value_ref: &T = value_ptr.as_ref();
            let value_ref: &dyn Any = value_ref as _;
            let value_ptr: *const dyn Any = value_ref as _;
            let value_ptr: *mut dyn Any = value_ptr as _;
            let value_ptr = NonNull::new_unchecked(value_ptr);

            let extern_data_ptr =
                alloc_ptr.cast::<u8>().as_ptr().add(footer_offset) as *mut VMExternData;
            ptr::write(
                extern_data_ptr,
                VMExternData {
                    ref_count: UnsafeCell::new(1),
                    value_ptr,
                },
            );

            VMExternRef(NonNull::new_unchecked(extern_data_ptr))
        }
    }

    // /// Turn this `VMExternRef` into a raw, untyped pointer.
    // ///
    // /// This forgets `self` and does *not* decrement the reference count on the
    // /// pointed-to data.
    // ///
    // /// This `VMExternRef` may be recovered with `VMExternRef::from_raw`.
    // pub fn into_raw(self) -> *mut u8 {
    //     let ptr = self.as_raw();
    //     mem::forget(self);
    //     ptr
    // }

    // /// Recreate a `VMExternRef` from a pointer returned from a previous call to
    // /// `VMExternRef::into_raw`.
    // ///
    // /// # Safety
    // ///
    // /// Wildly unsafe to use with anything other than the result of a previous
    // /// `into_raw` call!
    // ///
    // /// This method does *not* increment the reference count on the pointed-to
    // /// data, so `from_raw` must be called at most *once* on the result of a
    // /// previous `into_raw` call. (Ideally, every `into_raw` is later followed
    // /// by a `from_raw`, but it is technically memory safe to never call
    // /// `from_raw` after `into_raw`: it will leak the pointed-to value, which is
    // /// memory safe).
    // pub unsafe fn from_raw(ptr: *mut u8) -> Self {
    //     debug_assert!(!ptr.is_null());
    //     VMExternRef(NonNull::new_unchecked(ptr).cast())
    // }

    /// Turn this `VMExternRef` into a raw, untyped pointer.
    ///
    /// Unlike `into_raw`, this does not consume and forget `self`. It is *not*
    /// safe to use `from_raw` on pointers returned from this method; only use
    /// `clone_from_raw`!
    ///
    ///  Nor does this method increment the reference count. You must ensure
    ///  that `self` (or some other clone of `self`) stays alive until
    ///  `clone_from_raw` is called.
    pub fn as_raw(&self) -> *mut u8 {
        let ptr = self.0.cast::<u8>().as_ptr();
        mem::forget(self);
        ptr
    }

    /// Recreate a `VMExternRef` from a pointer returned from a previous call to
    /// `VMExternRef::as_raw`.
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

    /// Get the reference count for this `VMExternRef`.
    pub fn get_reference_count(&self) -> usize {
        self.extern_data().get_ref_count()
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
        ptr::eq(a.0.as_ptr() as *const _, b.0.as_ptr() as *const _)
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
        ptr::hash(externref.0.as_ptr() as *const _, hasher);
    }

    /// Compare two `VMExternRef`s.
    ///
    /// Note that this uses pointer-equality semantics, not structural-equality
    /// semantics, and so only pointers are compared, and doesn't use any `Cmp`
    /// or `PartialCmp` implementation of the pointed-to values.
    #[inline]
    pub fn cmp(a: &Self, b: &Self) -> Ordering {
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

type TableElem = UnsafeCell<Option<VMExternRef>>;

/// A table that over-approximizes the set of `VMExternRef`s that any Wasm
/// activation on this thread is currently using.
///
/// Under the covers, this is a simple bump allocator that allows duplicate
/// entries. Deduplication happens at GC time.
#[repr(C)]
pub struct VMExternRefActivationsTable {
    /// Bump-allocation finger within the current chunk.
    ///
    /// NB: this is an `UnsafeCell` because it is read from and written to by
    /// compiled Wasm code.
    next: UnsafeCell<NonNull<TableElem>>,

    /// Pointer to just after the current chunk.
    ///
    /// This is *not* within the current chunk and therefore is not a valid
    /// place to insert a reference!
    ///
    /// This is only updated from host code.
    end: Cell<NonNull<TableElem>>,

    /// The chunks within which we are bump allocating.
    ///
    /// This is only updated from host code.
    chunks: RefCell<Vec<Box<[TableElem]>>>,

    /// The precise set of on-stack, inside-Wasm GC roots that we discover via
    /// walking the stack and interpreting stack maps.
    ///
    /// That is, this is the precise set that the bump allocation table is
    /// over-approximating.
    ///
    /// This is *only* used inside the `gc` function, and is empty otherwise. It
    /// is just part of this struct so that we can reuse the allocation, rather
    /// than create a new hash set every GC.
    precise_stack_roots: RefCell<HashSet<NonNull<VMExternData>>>,

    /// A pointer to a `u8` on the youngest host stack frame before we called
    /// into Wasm for the first time. When walking the stack in garbage
    /// collection, if we don't find this frame, then we failed to walk every
    /// Wasm stack frame, which means we failed to find all on-stack,
    /// inside-a-Wasm-frame roots, and doing a GC could lead to freeing one of
    /// those missed roots, and use after free.
    stack_canary: Cell<Option<NonNull<u8>>>,
}

impl VMExternRefActivationsTable {
    const INITIAL_CHUNK_SIZE: usize = 4096 / mem::size_of::<usize>();

    /// Create a new `VMExternRefActivationsTable`.
    pub fn new() -> Self {
        let chunk = Self::new_chunk(Self::INITIAL_CHUNK_SIZE);
        let next = chunk.as_ptr() as *mut TableElem;
        let end = unsafe { next.add(chunk.len()) };

        VMExternRefActivationsTable {
            next: UnsafeCell::new(NonNull::new(next).unwrap()),
            end: Cell::new(NonNull::new(end).unwrap()),
            chunks: RefCell::new(vec![chunk]),
            precise_stack_roots: RefCell::new(HashSet::with_capacity(Self::INITIAL_CHUNK_SIZE)),
            stack_canary: Cell::new(None),
        }
    }

    fn new_chunk(size: usize) -> Box<[UnsafeCell<Option<VMExternRef>>]> {
        assert!(size >= Self::INITIAL_CHUNK_SIZE);
        let mut chunk = Vec::with_capacity(size);
        for _ in 0..size {
            chunk.push(UnsafeCell::new(None));
        }
        chunk.into_boxed_slice()
    }

    /// Try and insert a `VMExternRef` into this table.
    ///
    /// This is a fast path that only succeeds when the current chunk has the
    /// capacity for the requested insertion.
    ///
    /// If the insertion fails, then the `VMExternRef` is given back. Callers
    /// may attempt a GC to free up space and try again, or may call
    /// `insert_slow_path` to allocate a new bump chunk for this insertion.
    #[inline]
    pub fn try_insert(&self, externref: VMExternRef) -> Result<(), VMExternRef> {
        unsafe {
            let next = *self.next.get();
            let end = self.end.get();
            if next == end {
                return Err(externref);
            }

            debug_assert!((*next.as_ref().get()).is_none());
            ptr::write(next.as_ptr(), UnsafeCell::new(Some(externref)));
            let next = NonNull::new_unchecked(next.as_ptr().add(1));
            debug_assert!(next <= end);
            *self.next.get() = next;
            Ok(())
        }
    }

    /// This is a slow path for inserting a reference into the table when the
    /// current bump chunk is full.
    ///
    /// This method is infallible, and will allocate an additional bump chunk if
    /// necessary.
    #[inline(never)]
    pub fn insert_slow_path(&self, externref: VMExternRef) {
        let externref = match self.try_insert(externref) {
            Ok(()) => return,
            Err(x) => x,
        };

        {
            let mut chunks = self.chunks.borrow_mut();

            let new_size = chunks.last().unwrap().len() * 2;
            let new_chunk = Self::new_chunk(new_size);

            unsafe {
                let next = new_chunk.as_ptr() as *mut TableElem;
                debug_assert!(!next.is_null());
                *self.next.get() = NonNull::new_unchecked(next);

                let end = next.add(new_chunk.len());
                debug_assert!(!end.is_null());
                self.end.set(NonNull::new_unchecked(end));
            }

            chunks.push(new_chunk);
        }

        self.try_insert(externref)
            .expect("insertion should always succeed after we allocate a new chunk");
    }

    fn num_filled_in_last_chunk(&self, chunks: &[Box<[TableElem]>]) -> usize {
        let last_chunk = chunks.last().unwrap();
        let next = unsafe { *self.next.get() };
        let end = self.end.get();
        let num_unused_in_last_chunk =
            ((end.as_ptr() as usize) - (next.as_ptr() as usize)) / mem::size_of::<usize>();
        last_chunk.len().saturating_sub(num_unused_in_last_chunk)
    }

    fn elements(&self, mut f: impl FnMut(&VMExternRef)) {
        // Every chunk except the last one is full, so we can simply iterate
        // over all of their elements.
        let chunks = self.chunks.borrow();
        for chunk in chunks.iter().take(chunks.len() - 1) {
            for elem in chunk.iter() {
                if let Some(elem) = unsafe { &*elem.get() } {
                    f(elem);
                }
            }
        }

        // The last chunk is not all the way full, so we only iterate over its
        // full parts.
        let num_filled_in_last_chunk = self.num_filled_in_last_chunk(&chunks);
        for elem in chunks.last().unwrap().iter().take(num_filled_in_last_chunk) {
            if let Some(elem) = unsafe { &*elem.get() } {
                f(elem);
            }
        }
    }

    fn insert_precise_stack_root(&self, root: NonNull<VMExternData>) {
        let mut precise_stack_roots = self.precise_stack_roots.borrow_mut();
        if precise_stack_roots.insert(root) {
            // If this root was not already in the set, then we need to
            // increment its reference count, so that it doesn't get freed in
            // `reset` when we're overwriting old bump allocation table entries
            // with new ones.
            unsafe {
                root.as_ref().increment_ref_count();
            }
        }
    }

    /// Refill the bump allocation table with our precise stack roots, and sweep
    /// away everything else.
    fn reset(&self) {
        let mut chunks = self.chunks.borrow_mut();

        let mut precise_roots = self.precise_stack_roots.borrow_mut();
        if precise_roots.is_empty() {
            // Get rid of all but our first bump chunk, and set our `next` and
            // `end` bump allocation fingers into it.
            unsafe {
                let chunk = chunks.first().unwrap();

                let next = chunk.as_ptr() as *mut TableElem;
                debug_assert!(!next.is_null());
                *self.next.get() = NonNull::new_unchecked(next);

                let end = next.add(chunk.len());
                debug_assert!(!end.is_null());
                self.end.set(NonNull::new_unchecked(end));
            }
            chunks.truncate(1);
        } else {
            // Drain our precise stack roots into the bump allocation table.
            //
            // This overwrites old entries, which drops them and decrements their
            // reference counts. Safety relies on the reference count increment in
            // `insert_precise_stack_root` to avoid over-eagerly dropping references
            // that are in `self.precise_stack_roots` but haven't been inserted into
            // the bump allocation table yet.
            let mut precise_roots = precise_roots.drain();
            'outer: for (chunk_index, chunk) in chunks.iter().enumerate() {
                for (slot_index, slot) in chunk.iter().enumerate() {
                    if let Some(root) = precise_roots.next() {
                        unsafe {
                            // NB: there is no reference count increment here
                            // because everything in `self.precise_stack_roots`
                            // already had its reference count incremented for us,
                            // and this is logically a move out from there, rather
                            // than a clone.
                            *slot.get() = Some(VMExternRef(root));
                        }
                    } else {
                        // We've inserted all of our precise, on-stack roots back
                        // into the bump allocation table. Update our `next` and
                        // `end` bump pointer members for the new current chunk, and
                        // free any excess chunks.
                        let start = chunk.as_ptr() as *mut TableElem;
                        unsafe {
                            let next = start.add(slot_index + 1);
                            debug_assert!(!next.is_null());
                            *self.next.get() = NonNull::new_unchecked(next);

                            let end = start.add(chunk.len());
                            debug_assert!(!end.is_null());
                            self.end.set(NonNull::new_unchecked(end));
                        }
                        chunks.truncate(chunk_index + 1);
                        break 'outer;
                    }
                }
            }

            debug_assert!(
                precise_roots.next().is_none(),
                "should always have enough capacity in the bump allocations table \
                 to hold all of our precise, on-stack roots"
            );
        }

        // Finally, sweep away excess capacity within our new last/current
        // chunk, so that old, no-longer-live roots get dropped.
        let num_filled_in_last_chunk = self.num_filled_in_last_chunk(&chunks);
        for slot in chunks.last().unwrap().iter().skip(num_filled_in_last_chunk) {
            unsafe {
                *slot.get() = None;
            }
        }
    }

    /// Set the stack canary around a call into Wasm.
    ///
    /// The return value should not be dropped until after the Wasm call has
    /// returned.
    ///
    /// While this method is always safe to call (or not call), it is unsafe to
    /// call the `wasmtime_runtime::gc` function unless this method is called at
    /// the proper times and its return value properly outlives its Wasm call.
    ///
    /// For `gc` to be safe, this is only *strictly required* to surround the
    /// oldest host-->Wasm stack frame transition on this thread, but repeatedly
    /// calling it is idempotent and cheap, so it is recommended to call this
    /// for every host-->Wasm call.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use wasmtime_runtime::*;
    ///
    /// #let get_table_from_somewhere = || unimplemented!();
    /// let table: &VMExternRefActivationsTable = get_table_from_somewhere();
    ///
    /// // Set the canary before a Wasm call. The canary should always be a
    /// // local on the stack.
    /// let canary = 0;
    /// let auto_reset_canary = table.set_stack_canary(&canary);
    ///
    /// // Do the call into Wasm.
    /// #let call_into_wasm = || unimplemented!();
    /// call_into_wasm();
    ///
    /// // Only drop the value returned by `set_stack_canary` after the Wasm
    /// // call has returned.
    /// drop(auto_reset_canary);
    /// ```
    pub fn set_stack_canary<'a>(&'a self, canary: &u8) -> impl Drop + 'a {
        let should_reset = if self.stack_canary.get().is_none() {
            let canary = canary as *const u8 as *mut u8;
            self.stack_canary.set(Some(unsafe {
                debug_assert!(!canary.is_null());
                NonNull::new_unchecked(canary)
            }));
            true
        } else {
            false
        };

        return AutoResetCanary {
            table: self,
            should_reset,
        };

        struct AutoResetCanary<'a> {
            table: &'a VMExternRefActivationsTable,
            should_reset: bool,
        }

        impl Drop for AutoResetCanary<'_> {
            fn drop(&mut self) {
                if self.should_reset {
                    debug_assert!(self.table.stack_canary.get().is_some());
                    self.table.stack_canary.set(None);
                }
            }
        }
    }
}

/// A registry of stack maps for currently active Wasm modules.
#[derive(Default)]
pub struct StackMapRegistry {
    inner: RwLock<StackMapRegistryInner>,
}

#[derive(Default)]
struct StackMapRegistryInner {
    /// A map from the highest pc in a module, to its stack maps.
    ///
    /// For details, see the comment above `GlobalFrameInfo::ranges`.
    ranges: BTreeMap<usize, ModuleStackMaps>,
}

#[derive(Debug)]
struct ModuleStackMaps {
    /// The range of PCs that this module covers. Different modules should
    /// always have distinct ranges.
    range: std::ops::Range<usize>,

    /// A map from a PC in this module (that is a GC safepoint) to its
    /// associated stack map.
    pc_to_stack_map: Vec<(usize, Arc<Stackmap>)>,
}

impl StackMapRegistry {
    /// Register the stack maps for a given module.
    ///
    /// The stack maps should be given as an iterator over a function's PC range
    /// in memory (that is, where the JIT actually allocated and emitted the
    /// function's code at), and the stack maps and code offsets within that
    /// range for each of its GC safepoints.
    ///
    /// The return value is an RAII registration for the stack maps. The
    /// registration should not be dropped until its associated module is
    /// dropped. Dropping the registration will unregister its stack
    /// maps.
    ///
    /// # Safety
    ///
    /// Dropping the returned registration before the module is dropped, or when
    /// there are still active frames from the module on the stack, means we
    /// will no longer be able to find GC roots for the module's frames anymore,
    /// which could lead to freeing still-in-use objects and use-after-free!
    pub unsafe fn register_stack_maps<'a>(
        self: &Arc<Self>,
        stack_maps: impl IntoIterator<Item = (std::ops::Range<usize>, &'a [StackMapInformation])>,
    ) -> Option<StackMapRegistration> {
        let mut min = usize::max_value();
        let mut max = 0;
        let mut pc_to_stack_map = vec![];

        for (range, infos) in stack_maps {
            let len = range.end - range.start;

            min = std::cmp::min(min, range.start);
            max = std::cmp::max(max, range.end);

            for info in infos {
                assert!((info.code_offset as usize) < len);
                pc_to_stack_map.push((
                    range.start + (info.code_offset as usize),
                    Arc::new(info.stack_map.clone()),
                ));
            }
        }

        if pc_to_stack_map.is_empty() {
            // Nothing to register.
            return None;
        }

        let module_stack_maps = ModuleStackMaps {
            range: min..max,
            pc_to_stack_map,
        };

        let mut inner = self.inner.write().unwrap();

        // Assert that this chunk of ranges doesn't collide with any other known
        // chunks.
        if let Some((_, prev)) = inner.ranges.range(max..).next() {
            assert!(prev.range.start > max);
        }
        if let Some((prev_end, _)) = inner.ranges.range(..=min).next_back() {
            assert!(*prev_end < min);
        }

        let old = inner.ranges.insert(max, module_stack_maps);
        assert!(old.is_none());

        Some(StackMapRegistration {
            key: max,
            registry: self.clone(),
        })
    }

    /// Lookup the stack map for the given PC, if any.
    pub fn lookup_stack_map(&self, pc: usize) -> Option<Arc<Stackmap>> {
        let inner = self.inner.read().unwrap();
        let stack_maps = inner.module_stack_maps(pc)?;

        // Do a binary search to find the stack map for the given PC.
        //
        // Because GC safepoints are technically only associated with a single
        // PC, we should ideally only care about `Ok(index)` values returned
        // from the binary search. However, safepoints are inserted right before
        // calls, and there are two things that can disturb the PC/offset
        // associated with the safepoint versus the PC we actually use to query
        // for the stack map:
        //
        // 1. The `backtrace` crate gives us the PC in a frame that will be
        //    *returned to*, and where execution will continue from, rather than
        //    the PC of the call we are currently at. So we would need to
        //    disassemble one instruction backwards to query the actual PC for
        //    the stack map.
        //
        //    TODO: One thing we *could* do to make this a little less error
        //    prone, would be to assert/check that the nearest GC safepoint
        //    found is within `max_encoded_size(any kind of call instruction)`
        //    our queried PC for the target architecture.
        //
        // 2. Cranelift's stack maps only handle the stack, not
        //    registers. However, some references that are arguments to a call
        //    may need to be in registers. In these cases, what Cranelift will
        //    do is:
        //
        //      a. spill all the live references,
        //      b. insert a GC safepoint for those references,
        //      c. reload the references into registers, and finally
        //      d. make the call.
        //
        //    Step (c) adds drift between the GC safepoint and the location of
        //    the call, which is where we actually walk the stack frame and
        //    collect its live references.
        //
        //    Luckily, the spill stack slots for the live references are still
        //    up to date, so we can still find all the on-stack roots.
        //    Furthermore, we do not have a moving GC, so we don't need to worry
        //    whether the following code will reuse the references in registers
        //    (which would not have been updated to point to the moved objects)
        //    or reload from the stack slots (which would have been updated to
        //    point to the moved objects).
        let index = match stack_maps
            .pc_to_stack_map
            .binary_search_by_key(&pc, |(pc, _stack_map)| *pc)
        {
            // Exact hit.
            Ok(i) => i,

            Err(n) => {
                // `Err(0)` means that the associated stack map would have been
                // the first element in the array if this pc had an associated
                // stack map, but this pc does not have an associated stack
                // map. That doesn't make sense since every call and trap inside
                // Wasm is a GC safepoint and should have a stack map, and the
                // only way to have Wasm frames under this native frame is if we
                // are at a call or a trap.
                debug_assert!(n != 0);

                n - 1
            }
        };

        let stack_map = stack_maps.pc_to_stack_map[index].1.clone();
        Some(stack_map)
    }
}

impl StackMapRegistryInner {
    fn module_stack_maps(&self, pc: usize) -> Option<&ModuleStackMaps> {
        let (end, stack_maps) = self.ranges.range(pc..).next()?;
        if pc < stack_maps.range.start || *end < pc {
            None
        } else {
            Some(stack_maps)
        }
    }
}

/// The registration for a module's stack maps.
///
/// Unsafe to drop earlier than its module is dropped. See
/// `StackMapRegistry::register_stack_maps` for details.
pub struct StackMapRegistration {
    key: usize,
    registry: Arc<StackMapRegistry>,
}

impl Drop for StackMapRegistration {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.registry.inner.write() {
            inner.ranges.remove(&self.key);
        }
    }
}

#[cfg(debug_assertions)]
#[derive(Debug, Default)]
struct DebugOnly<T> {
    inner: T,
}

#[cfg(debug_assertions)]
impl<T> std::ops::Deref for DebugOnly<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

#[cfg(debug_assertions)]
impl<T> std::ops::DerefMut for DebugOnly<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

#[cfg(not(debug_assertions))]
#[derive(Debug, Default)]
struct DebugOnly<T> {
    _phantom: PhantomData<T>,
}

#[cfg(not(debug_assertions))]
impl<T> std::ops::Deref for DebugOnly<T> {
    type Target = T;

    fn deref(&self) -> &T {
        panic!("only deref `DebugOnly` inside `debug_assert!`s")
    }
}

#[cfg(not(debug_assertions))]
impl<T> std::ops::DerefMut for DebugOnly<T> {
    fn deref_mut(&mut self) -> &mut T {
        panic!("only deref `DebugOnly` inside `debug_assert!`s")
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
pub unsafe fn gc(
    stack_maps_registry: &StackMapRegistry,
    externref_activations_table: &VMExternRefActivationsTable,
) {
    log::debug!("start GC");

    debug_assert!({
        // This set is only non-empty within this function. It is built up when
        // walking the stack and interpreting stack maps, and then drained back
        // into the activations table's bump-allocated space at the
        // end. Therefore, it should always be empty upon entering this
        // function.
        let precise_stack_roots = externref_activations_table.precise_stack_roots.borrow();
        precise_stack_roots.is_empty()
    });

    // Whenever we call into Wasm from host code for the first time, we set a
    // stack canary. When we return to that host code, we unset the stack
    // canary. If there is *not* a stack canary, then there must be zero Wasm
    // frames on the stack. Therefore, we can simply reset the table without
    // walking the stack.
    let stack_canary = match externref_activations_table.stack_canary.get() {
        None => {
            if cfg!(debug_assertions) {
                // Assert that there aren't any Wasm frames on the stack.
                backtrace::trace(|frame| {
                    let stack_map = stack_maps_registry.lookup_stack_map(frame.ip() as usize);
                    assert!(stack_map.is_none());
                    true
                });
            }
            externref_activations_table.reset();
            log::debug!("end GC");
            return;
        }
        Some(canary) => canary.as_ptr() as usize,
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

    // The SP of the previous frame we processed.
    let mut last_sp = None;

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

        if let Some(stack_map) = stack_maps_registry.lookup_stack_map(pc) {
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
                         have an entry in the VMExternRefActivationsTable"
                    );
                    if let Some(r) = NonNull::new(r) {
                        externref_activations_table.insert_precise_stack_root(r);
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

    // Only reset the table if we found the stack canary, and therefore know
    // that we discovered all the on-stack, inside-a-Wasm-frame roots. If we did
    // *not* find the stack canary, then `libunwind` failed to walk the whole
    // stack, and we might be missing roots. Reseting the table would free those
    // missing roots while they are still in use, leading to use-after-free.
    if found_canary {
        externref_activations_table.reset();
    } else {
        let mut roots = externref_activations_table.precise_stack_roots.borrow_mut();
        roots.clear();
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
        let s: &dyn Any = &s as _;
        let s: *const dyn Any = s as _;
        let s: *mut dyn Any = s as _;

        let extern_data = VMExternData {
            ref_count: UnsafeCell::new(0),
            value_ptr: NonNull::new(s).unwrap(),
        };

        let extern_data_ptr = &extern_data as *const _;
        let ref_count_ptr = &extern_data.ref_count as *const _;

        let actual_offset = (ref_count_ptr as usize) - (extern_data_ptr as usize);

        assert_eq!(
            wasmtime_environ::VMOffsets::vm_extern_data_ref_count(),
            actual_offset.try_into().unwrap(),
        );
    }

    #[test]
    fn table_next_is_at_correct_offset() {
        let table = VMExternRefActivationsTable::new();

        let table_ptr = &table as *const _;
        let next_ptr = &table.next as *const _;

        let actual_offset = (next_ptr as usize) - (table_ptr as usize);

        let offsets = wasmtime_environ::VMOffsets {
            pointer_size: 8,
            num_signature_ids: 0,
            num_imported_functions: 0,
            num_imported_tables: 0,
            num_imported_memories: 0,
            num_imported_globals: 0,
            num_defined_tables: 0,
            num_defined_memories: 0,
            num_defined_globals: 0,
        };
        assert_eq!(offsets.vm_extern_ref_activation_table_next(), actual_offset);
    }

    #[test]
    fn table_end_is_at_correct_offset() {
        let table = VMExternRefActivationsTable::new();

        let table_ptr = &table as *const _;
        let end_ptr = &table.end as *const _;

        let actual_offset = (end_ptr as usize) - (table_ptr as usize);

        let offsets = wasmtime_environ::VMOffsets {
            pointer_size: 8,
            num_signature_ids: 0,
            num_imported_functions: 0,
            num_imported_tables: 0,
            num_imported_memories: 0,
            num_imported_globals: 0,
            num_defined_tables: 0,
            num_defined_memories: 0,
            num_defined_globals: 0,
        };
        assert_eq!(offsets.vm_extern_ref_activation_table_end(), actual_offset);
    }

    fn assert_is_send<T: Send>() {}
    fn assert_is_sync<T: Send>() {}

    #[test]
    fn stack_map_registry_is_send_sync() {
        assert_is_send::<StackMapRegistry>();
        assert_is_sync::<StackMapRegistry>();
    }

    #[test]
    fn stack_map_registration_is_send_sync() {
        assert_is_send::<StackMapRegistration>();
        assert_is_sync::<StackMapRegistration>();
    }
}
