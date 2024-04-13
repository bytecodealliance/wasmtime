use anyhow::{bail, Result};
use std::cell::UnsafeCell;
use std::fmt;
use std::mem;
use std::slice;
use std::str;
use std::sync::Arc;

pub use wiggle_macro::{async_trait, from_witx};

pub use anyhow;
pub use wiggle_macro::wasmtime_integration;

pub use bitflags;

#[cfg(feature = "wiggle_metadata")]
pub use witx;

pub mod borrow;
mod error;
mod guest_type;
mod region;

pub use tracing;

pub use error::GuestError;
pub use guest_type::{GuestErrorType, GuestType, GuestTypeTransparent};
pub use region::Region;

pub mod async_trait_crate {
    pub use async_trait::*;
}

pub mod wasmtime;
#[cfg(feature = "wasmtime")]
pub mod wasmtime_crate {
    pub use wasmtime::*;
}

/// A trait which abstracts how to get at the region of host memory that
/// contains guest memory.
///
/// All `GuestPtr` types will contain a handle to this trait, signifying where
/// the pointer is actually pointing into. This type will need to be implemented
/// for the host's memory storage object.
///
/// # Safety
///
/// Safety around this type is tricky, and the trait is `unsafe` since there are
/// a few contracts you need to uphold to implement this type correctly and have
/// everything else in this crate work out safely.
///
/// The most important method of this trait is the `base` method. This returns,
/// in host memory, a pointer and a length. The pointer should point to valid
/// memory for the guest to read/write for the length contiguous bytes
/// afterwards.
///
/// The region returned by `base` must not only be valid, however, but it must
/// be valid for "a period of time before the guest is reentered". This isn't
/// exactly well defined but the general idea is that `GuestMemory` is allowed
/// to change under our feet to accommodate instructions like `memory.grow` or
/// other guest modifications. Memory, however, cannot be changed if the guest
/// is not reentered or if no explicitly action is taken to modify the guest
/// memory.
///
/// This provides the guarantee that host pointers based on the return value of
/// `base` have a dynamic period for which they are valid. This time duration
/// must be "somehow nonzero in length" to allow users of `GuestMemory` and
/// `GuestPtr` to safely read and write interior data.
///
/// This type also provides methods for run-time borrow checking of references
/// into the memory. The safety of this mechanism depends on there being exactly
/// one associated tracking of borrows for a given WebAssembly memory. There
/// must be no other reads or writes of WebAssembly the memory by either Rust or
/// WebAssembly code while there are any outstanding borrows.
///
/// # Using References
///
/// The [`GuestPtr::as_slice`] or [`GuestPtr::as_str`] will return smart
/// pointers [`GuestSlice`] and [`GuestStr`]. These types, which implement
/// [`std::ops::Deref`] and [`std::ops::DerefMut`], provide mutable references
/// into the memory region given by a `GuestMemory`.
///
/// These smart pointers are dynamically borrow-checked by the borrow checker
/// methods on this trait. While a `GuestSlice` or a `GuestStr` are live,
/// WebAssembly cannot be reentered because the store's borrow is connected to
/// the relevant `'a` lifetime on the guest pointer.
pub unsafe trait GuestMemory: Send + Sync {
    /// Returns the base allocation of this guest memory, located in host
    /// memory.
    ///
    /// A pointer/length pair are returned to signify where the guest memory
    /// lives in the host, and how many contiguous bytes the memory is valid for
    /// after the returned pointer.
    ///
    /// Note that there are safety guarantees about this method that
    /// implementations must uphold, and for more details see the
    /// [`GuestMemory`] documentation.
    fn base(&self) -> &[UnsafeCell<u8>];

    /// Convenience method for creating a `GuestPtr` at a particular offset.
    ///
    /// Note that `T` can be almost any type, and typically `offset` is a `u32`.
    /// The exception is slices and strings, in which case `offset` is a `(u32,
    /// u32)` of `(offset, length)`.
    fn ptr<'a, T>(&'a self, offset: T::Pointer) -> GuestPtr<'a, T>
    where
        Self: Sized,
        T: ?Sized + Pointee,
    {
        GuestPtr::new(self, offset)
    }

    /// Check if a region of memory can be read.
    ///
    /// This will only return `true` if there are no active mutable borrows.
    fn can_read(&self, r: Region) -> bool;

    /// Check if a region of memory can be written.
    ///
    /// This will only return `true` if there are no active borrows.
    fn can_write(&self, r: Region) -> bool;

    /// Acquires a mutable borrow on a region of memory.
    ///
    /// Only succeeds if there are no active shared or mutable borrows and this
    /// is not a `shared` WebAssembly memory.
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError>;

    /// Acquires a shared borrow on a region of memory.
    ///
    /// Only succeeds if there are no active mutable borrows and this is not a
    /// `shared` WebAssembly memory.
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError>;

    /// Undoes a borrow by `mut_borrow`.
    fn mut_unborrow(&self, h: BorrowHandle);

    /// Undoes a borrow by `shared_borrow`.
    fn shared_unborrow(&self, h: BorrowHandle);

    /// Check if the underlying memory is shared across multiple threads; e.g.,
    /// with a WebAssembly shared memory.
    fn is_shared_memory(&self) -> bool {
        false
    }
}

/// Validates a guest-relative pointer given various attributes, and returns
/// the corresponding host pointer.
///
/// * `mem` - this is the guest memory being accessed.
/// * `offset` - this is the guest-relative pointer, an offset from the
///   base.
/// * `len` - this is the number of length, in units of `T`, to return
///   in the resulting slice.
///
/// If the parameters are valid then this function will return a slice into
/// `mem` for units of `T`, assuming everything is in-bounds and properly
/// aligned. Additionally the byte-based `Region` is returned, used for borrows
/// later on.
fn validate_size_align<'a, T: GuestTypeTransparent<'a>>(
    mem: &'a dyn GuestMemory,
    offset: u32,
    len: u32,
) -> Result<(&[UnsafeCell<T>], Region), GuestError> {
    let base = mem.base();
    let byte_len = len
        .checked_mul(T::guest_size())
        .ok_or(GuestError::PtrOverflow)?;
    let region = Region {
        start: offset,
        len: byte_len,
    };
    let offset = usize::try_from(offset)?;
    let byte_len = usize::try_from(byte_len)?;

    // Slice the input region to the byte range that we're interested in.
    let bytes = base
        .get(offset..)
        .and_then(|s| s.get(..byte_len))
        .ok_or(GuestError::PtrOutOfBounds(region))?;

    // ... and then align it to `T`, failing if either the head or tail slices
    // are nonzero in length. This `unsafe` here is from the standard library
    // and should be ok since the input slice is `UnsafeCell<u8>` and the output
    // slice is `UnsafeCell<T>`, meaning the only guarantee of the output is
    // that it's valid addressable memory, still unsafe to actually access.
    assert!(mem::align_of::<T>() <= T::guest_align());
    let (start, mid, end) = unsafe { bytes.align_to() };
    if start.len() > 0 || end.len() > 0 {
        return Err(GuestError::PtrNotAligned(region, T::guest_align() as u32));
    }
    Ok((mid, region))
}

/// A handle to a borrow on linear memory. It is produced by `{mut, shared}_borrow` and
/// consumed by `{mut, shared}_unborrow`. Only the `GuestMemory` impl should ever construct
/// a `BorrowHandle` or inspect its contents.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BorrowHandle {
    _priv: (),
}

// Forwarding trait implementations to the original type
unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a T {
    fn base(&self) -> &[UnsafeCell<u8>] {
        T::base(self)
    }
    fn can_read(&self, r: Region) -> bool {
        T::can_read(self, r)
    }
    fn can_write(&self, r: Region) -> bool {
        T::can_write(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a mut T {
    fn base(&self) -> &[UnsafeCell<u8>] {
        T::base(self)
    }
    fn can_read(&self, r: Region) -> bool {
        T::can_read(self, r)
    }
    fn can_write(&self, r: Region) -> bool {
        T::can_write(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Box<T> {
    fn base(&self) -> &[UnsafeCell<u8>] {
        T::base(self)
    }
    fn can_read(&self, r: Region) -> bool {
        T::can_read(self, r)
    }
    fn can_write(&self, r: Region) -> bool {
        T::can_write(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Arc<T> {
    fn base(&self) -> &[UnsafeCell<u8>] {
        T::base(self)
    }
    fn can_read(&self, r: Region) -> bool {
        T::can_read(self, r)
    }
    fn can_write(&self, r: Region) -> bool {
        T::can_write(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

/// A *guest* pointer into host memory.
///
/// This type represents a pointer from the guest that points into host memory.
/// Internally a `GuestPtr` contains a handle to its original [`GuestMemory`] as
/// well as the offset into the memory that the pointer is pointing at.
///
/// Presence of a [`GuestPtr`] does not imply any form of validity. Pointers can
/// be out-of-bounds, misaligned, etc. It is safe to construct a `GuestPtr` with
/// any offset at any time. Consider a `GuestPtr<T>` roughly equivalent to `*mut
/// T`, although there are a few more safety guarantees around this type.
///
/// ## Slices and Strings
///
/// Note that the type parameter does not need to implement the `Sized` trait,
/// so you can implement types such as this:
///
/// * `GuestPtr<'_, str>` - a pointer to a guest string. Has the methods
///   [`GuestPtr::as_str_mut`], which gives a dynamically borrow-checked
///   `GuestStrMut<'_>`, which `DerefMut`s to a `&mut str`, and
///   [`GuestPtr::as_str`], which is the shareable version of same.
/// * `GuestPtr<'_, [T]>` - a pointer to a guest array. Has methods
///   [`GuestPtr::as_slice_mut`], which gives a dynamically borrow-checked
///   `GuestSliceMut<'_, T>`, which `DerefMut`s to a `&mut [T]` and
///   [`GuestPtr::as_slice`], which is the shareable version of same.
///
/// Unsized types such as this may have extra methods and won't have methods
/// like [`GuestPtr::read`] or [`GuestPtr::write`].
///
/// ## Type parameter and pointee
///
/// The `T` type parameter is largely intended for more static safety in Rust as
/// well as having a better handle on what we're pointing to. A `GuestPtr<T>`,
/// however, does not necessarily literally imply a guest pointer pointing to
/// type `T`. Instead the [`GuestType`] trait is a layer of abstraction where
/// `GuestPtr<T>` may actually be a pointer to `U` in guest memory, but you can
/// construct a `T` from a `U`.
///
/// For example `GuestPtr<GuestPtr<T>>` is a valid type, but this is actually
/// more equivalent to `GuestPtr<u32>` because guest pointers are always
/// 32-bits. That being said you can create a `GuestPtr<T>` from a `u32`.
///
/// Additionally `GuestPtr<MyEnum>` will actually delegate, typically, to and
/// implementation which loads the underlying data as `GuestPtr<u8>` (or
/// similar) and then the bytes loaded are validated to fit within the
/// definition of `MyEnum` before `MyEnum` is returned.
///
/// For more information see the [`GuestPtr::read`] and [`GuestPtr::write`]
/// methods. In general though be extremely careful about writing `unsafe` code
/// when working with a `GuestPtr` if you're not using one of the
/// already-attached helper methods.
pub struct GuestPtr<'a, T: ?Sized + Pointee> {
    mem: &'a (dyn GuestMemory + 'a),
    pointer: T::Pointer,
}

impl<'a, T: ?Sized + Pointee> GuestPtr<'a, T> {
    /// Creates a new `GuestPtr` from the given `mem` and `pointer` values.
    ///
    /// Note that for sized types like `u32`, `GuestPtr<T>`, etc, the `pointer`
    /// value is a `u32` offset into guest memory. For slices and strings,
    /// `pointer` is a `(u32, u32)` offset/length pair.
    pub fn new(mem: &'a (dyn GuestMemory + 'a), pointer: T::Pointer) -> GuestPtr<'a, T> {
        GuestPtr { mem, pointer }
    }

    /// Returns the offset of this pointer in guest memory.
    ///
    /// Note that for sized types this returns a `u32`, but for slices and
    /// strings it returns a `(u32, u32)` pointer/length pair.
    pub fn offset(&self) -> T::Pointer {
        self.pointer
    }

    /// Returns the guest memory that this pointer is coming from.
    pub fn mem(&self) -> &'a (dyn GuestMemory + 'a) {
        self.mem
    }

    /// Casts this `GuestPtr` type to a different type.
    ///
    /// This is a safe method which is useful for simply reinterpreting the type
    /// parameter on this `GuestPtr`. Note that this is a safe method, where
    /// again there's no guarantees about alignment, validity, in-bounds-ness,
    /// etc of the returned pointer.
    pub fn cast<U>(&self) -> GuestPtr<'a, U>
    where
        U: Pointee<Pointer = T::Pointer> + ?Sized,
    {
        GuestPtr::new(self.mem, self.pointer)
    }

    /// Safely read a value from this pointer.
    ///
    /// This is a fun method, and is one of the lynchpins of this
    /// implementation. The highlight here is that this is a *safe* operation,
    /// not an unsafe one like `*mut T`. This works for a few reasons:
    ///
    /// * The `unsafe` contract of the `GuestMemory` trait means that there's
    ///   always at least some backing memory for this `GuestPtr<T>`.
    ///
    /// * This does not use Rust-intrinsics to read the type `T`, but rather it
    ///   delegates to `T`'s implementation of [`GuestType`] to actually read
    ///   the underlying data. This again is a safe method, so any unsafety, if
    ///   any, must be internally documented.
    ///
    /// * Eventually what typically happens it that this bottoms out in the read
    ///   implementations for primitives types (like `i32`) which can safely be
    ///   read at any time, and then it's up to the runtime to determine what to
    ///   do with the bytes it read in a safe manner.
    ///
    /// Naturally lots of things can still go wrong, such as out-of-bounds
    /// checks, alignment checks, validity checks (e.g. for enums), etc. All of
    /// these check failures, however, are returned as a [`GuestError`] in the
    /// `Result` here, and `Ok` is only returned if all the checks passed.
    pub fn read(&self) -> Result<T, GuestError>
    where
        T: GuestType<'a>,
    {
        T::read(self)
    }

    /// Safely write a value to this pointer.
    ///
    /// This method, like [`GuestPtr::read`], is pretty crucial for the safe
    /// operation of this crate. All the same reasons apply though for why this
    /// method is safe, even eventually bottoming out in primitives like writing
    /// an `i32` which is safe to write bit patterns into memory at any time due
    /// to the guarantees of [`GuestMemory`].
    ///
    /// Like `read`, `write` can fail due to any manner of pointer checks, but
    /// any failure is returned as a [`GuestError`].
    pub fn write(&self, val: T) -> Result<(), GuestError>
    where
        T: GuestType<'a>,
    {
        T::write(self, val)
    }

    /// Performs pointer arithmetic on this pointer, moving the pointer forward
    /// `amt` slots.
    ///
    /// This will either return the resulting pointer or `Err` if the pointer
    /// arithmetic calculation would overflow around the end of the address
    /// space.
    pub fn add(&self, amt: u32) -> Result<GuestPtr<'a, T>, GuestError>
    where
        T: GuestType<'a> + Pointee<Pointer = u32>,
    {
        let offset = amt
            .checked_mul(T::guest_size())
            .and_then(|o| self.pointer.checked_add(o));
        let offset = match offset {
            Some(o) => o,
            None => return Err(GuestError::PtrOverflow),
        };
        Ok(GuestPtr::new(self.mem, offset))
    }

    /// Returns a `GuestPtr` for an array of `T`s using this pointer as the
    /// base.
    pub fn as_array(&self, elems: u32) -> GuestPtr<'a, [T]>
    where
        T: GuestType<'a> + Pointee<Pointer = u32>,
    {
        GuestPtr::new(self.mem, (self.pointer, elems))
    }

    /// Check if this pointer references WebAssembly shared memory.
    pub fn is_shared_memory(&self) -> bool {
        self.mem.is_shared_memory()
    }
}

impl<'a, T> GuestPtr<'a, [T]> {
    /// For slices, specifically returns the relative pointer to the base of the
    /// array.
    ///
    /// This is similar to `<[T]>::as_ptr()`
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    /// For slices, returns the length of the slice, in elements.
    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    /// Returns an iterator over interior pointers.
    ///
    /// Each item is a `Result` indicating whether it overflowed past the end of
    /// the address space or not.
    pub fn iter<'b>(
        &'b self,
    ) -> impl ExactSizeIterator<Item = Result<GuestPtr<'a, T>, GuestError>> + 'b
    where
        T: GuestType<'a>,
    {
        let base = self.as_ptr();
        (0..self.len()).map(move |i| base.add(i))
    }

    /// Attempts to create a [`GuestCow<'_, T>`] from this pointer, performing
    /// bounds checks and type validation. Whereas [`GuestPtr::as_slice`] will
    /// fail with `None` if attempting to access Wasm shared memory, this call
    /// will succeed: if used on shared memory, this function will copy the
    /// slice into [`GuestCow::Copied`]. If the memory is non-shared, this
    /// returns a [`GuestCow::Borrowed`] (a thin wrapper over [`GuestSlice<'_,
    /// T>]`).
    pub fn as_cow(&self) -> Result<GuestCow<'a, T>, GuestError>
    where
        T: GuestTypeTransparent<'a> + Copy + 'a,
    {
        match self.as_unsafe_slice_mut()?.shared_borrow() {
            UnsafeBorrowResult::Ok(slice) => Ok(GuestCow::Borrowed(slice)),
            UnsafeBorrowResult::Shared(_) => Ok(GuestCow::Copied(self.to_vec()?)),
            UnsafeBorrowResult::Err(e) => Err(e),
        }
    }

    /// Attempts to create a [`GuestSlice<'_, T>`] from this pointer, performing
    /// bounds checks and type validation. The `GuestSlice` is a smart pointer
    /// that can be used as a `&[T]` via the `Deref` trait.
    ///
    /// This method will flag the entire linear memory as marked with a shared
    /// borrow. This means that any writes to memory are disallowed until
    /// the returned `GuestSlice` is dropped.
    ///
    /// This function will return a `GuestSlice` into host memory if all checks
    /// succeed (valid utf-8, valid pointers, memory is not borrowed, etc.). If
    /// any checks fail then `GuestError` will be returned.
    ///
    /// Additionally, because it is `unsafe` to have a `GuestSlice` of shared
    /// memory, this function will return `None` in this case (see
    /// [`GuestPtr::as_cow`]).
    pub fn as_slice(&self) -> Result<Option<GuestSlice<'a, T>>, GuestError>
    where
        T: GuestTypeTransparent<'a>,
    {
        match self.as_unsafe_slice_mut()?.shared_borrow() {
            UnsafeBorrowResult::Ok(slice) => Ok(Some(slice)),
            UnsafeBorrowResult::Shared(_) => Ok(None),
            UnsafeBorrowResult::Err(e) => Err(e),
        }
    }

    /// Attempts to create a [`GuestSliceMut<'_, T>`] from this pointer,
    /// performing bounds checks and type validation. The `GuestSliceMut` is a
    /// smart pointer that can be used as a `&[T]` or a `&mut [T]` via the
    /// `Deref` and `DerefMut` traits.
    ///
    /// This method will flag the entire linear memory as marked with a mutable
    /// borrow. This means that all reads/writes to memory are disallowed until
    /// the returned `GuestSliceMut` type is dropped.
    ///
    /// This function will return a `GuestSliceMut` into host memory if all
    /// checks succeed (valid utf-8, valid pointers, memory is not borrowed,
    /// etc). If any checks fail then `GuestError` will be returned.
    ///
    /// Additionally, because it is `unsafe` to have a `GuestSliceMut` of shared
    /// memory, this function will return `None` in this case.
    pub fn as_slice_mut(&self) -> Result<Option<GuestSliceMut<'a, T>>, GuestError>
    where
        T: GuestTypeTransparent<'a>,
    {
        self.as_unsafe_slice_mut()?.as_slice_mut()
    }

    /// Similar to `as_slice_mut`, this function will attempt to create a smart
    /// pointer to the WebAssembly linear memory. All validation and Wiggle
    /// borrow checking is the same, but unlike `as_slice_mut`, the returned
    /// `&mut` slice can point to WebAssembly shared memory. Though the Wiggle
    /// borrow checker can guarantee no other Wiggle calls will access this
    /// slice, it cannot guarantee that another thread is not modifying the
    /// `&mut` slice in some other way. Thus, access to that slice is marked
    /// `unsafe`.
    pub fn as_unsafe_slice_mut(&self) -> Result<UnsafeGuestSlice<'a, T>, GuestError>
    where
        T: GuestTypeTransparent<'a>,
    {
        let (ptr, region) = validate_size_align(self.mem, self.pointer.0, self.pointer.1)?;

        Ok(UnsafeGuestSlice {
            ptr,
            region,
            mem: self.mem,
        })
    }

    /// Copies the data in the guest region into a [`Vec`].
    ///
    /// This is useful when one cannot use [`GuestPtr::as_slice`], e.g., when
    /// pointing to a region of WebAssembly shared memory.
    pub fn to_vec(&self) -> Result<Vec<T>, GuestError>
    where
        T: GuestTypeTransparent<'a> + Copy + 'a,
    {
        let guest_slice = self.as_unsafe_slice_mut()?;
        let len = guest_slice.ptr.len();
        let mut vec = Vec::with_capacity(len);

        // SAFETY: The `guest_slice` variable is already a valid pointer into
        // the guest's memory, and it may or may not be a pointer into shared
        // memory. We can't naively use `.to_vec(..)` which could introduce data
        // races but all that needs to happen is to copy data into our local
        // `vec` as all the data is `Copy` and transparent anyway. For this
        // purpose the `ptr::copy` function should be sufficient for copying
        // over all the data.
        //
        // TODO: audit that this use of `std::ptr::copy` is safe with shared
        // memory (https://github.com/bytecodealliance/wasmtime/issues/4203)
        unsafe {
            std::ptr::copy(guest_slice.ptr.as_ptr().cast::<T>(), vec.as_mut_ptr(), len);
            vec.set_len(len);
        }
        Ok(vec)
    }

    /// Copies the data pointed to by `slice` into this guest region.
    ///
    /// This method is a *safe* method to copy data from the host to the guest.
    /// This requires that `self` and `slice` have the same length. The pointee
    /// type `T` requires the [`GuestTypeTransparent`] trait which is an
    /// assertion that the representation on the host and on the guest is the
    /// same.
    ///
    /// # Errors
    ///
    /// Returns an error if this guest pointer is out of bounds or if the length
    /// of this guest pointer is not equal to the length of the slice provided.
    pub fn copy_from_slice(&self, slice: &[T]) -> Result<(), GuestError>
    where
        T: GuestTypeTransparent<'a> + Copy + 'a,
    {
        self.as_unsafe_slice_mut()?.copy_from_slice(slice)
    }

    /// Returns a `GuestPtr` pointing to the base of the array for the interior
    /// type `T`.
    pub fn as_ptr(&self) -> GuestPtr<'a, T> {
        GuestPtr::new(self.mem, self.offset_base())
    }

    pub fn get(&self, index: u32) -> Option<GuestPtr<'a, T>>
    where
        T: GuestType<'a>,
    {
        if index < self.len() {
            Some(
                self.as_ptr()
                    .add(index)
                    .expect("just performed bounds check"),
            )
        } else {
            None
        }
    }

    pub fn get_range(&self, r: std::ops::Range<u32>) -> Option<GuestPtr<'a, [T]>>
    where
        T: GuestType<'a>,
    {
        if r.end < r.start {
            return None;
        }
        let range_length = r.end - r.start;
        if r.start <= self.len() && r.end <= self.len() {
            Some(
                self.as_ptr()
                    .add(r.start)
                    .expect("just performed bounds check")
                    .as_array(range_length),
            )
        } else {
            None
        }
    }
}

impl<'a> GuestPtr<'a, str> {
    /// For strings, returns the relative pointer to the base of the string
    /// allocation.
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    /// Returns the length, in bytes, of the string.
    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    /// Returns a raw pointer for the underlying slice of bytes that this
    /// pointer points to.
    pub fn as_bytes(&self) -> GuestPtr<'a, [u8]> {
        GuestPtr::new(self.mem, self.pointer)
    }

    /// Attempts to create a [`GuestStr<'_>`] from this pointer, performing
    /// bounds checks and utf-8 checks. The resulting `GuestStr` can be used as
    /// a `&str` via the `Deref` trait. The region of memory backing the `str`
    /// will be marked as shareably borrowed by the [`GuestMemory`] until the
    /// `GuestStr` is dropped.
    ///
    /// This function will return `GuestStr` into host memory if all checks
    /// succeed (valid utf-8, valid pointers, etc). If any checks fail then
    /// `GuestError` will be returned.
    ///
    /// Additionally, because it is `unsafe` to have a `GuestStr` of shared
    /// memory, this function will return `None` in this case (see
    /// [`GuestPtr<'_, str>::as_cow`]).
    pub fn as_str(&self) -> Result<Option<GuestStr<'a>>, GuestError> {
        match self.as_bytes().as_unsafe_slice_mut()?.shared_borrow() {
            UnsafeBorrowResult::Ok(s) => Ok(Some(s.try_into()?)),
            UnsafeBorrowResult::Shared(_) => Ok(None),
            UnsafeBorrowResult::Err(e) => Err(e),
        }
    }

    /// Attempts to create a [`GuestStrMut<'_>`] from this pointer, performing
    /// bounds checks and utf-8 checks. The resulting `GuestStrMut` can be used
    /// as a `&str` or `&mut str` via the `Deref` and `DerefMut` traits. The
    /// region of memory backing the `str` will be marked as borrowed by the
    /// [`GuestMemory`] until the `GuestStrMut` is dropped.
    ///
    /// This function will return `GuestStrMut` into host memory if all checks
    /// succeed (valid utf-8, valid pointers, etc). If any checks fail then
    /// `GuestError` will be returned.
    ///
    /// Additionally, because it is `unsafe` to have a `GuestStrMut` of shared
    /// memory, this function will return `None` in this case.
    pub fn as_str_mut(&self) -> Result<Option<GuestStrMut<'a>>, GuestError> {
        match self.as_bytes().as_unsafe_slice_mut()?.mut_borrow() {
            UnsafeBorrowResult::Ok(s) => Ok(Some(s.try_into()?)),
            UnsafeBorrowResult::Shared(_) => Ok(None),
            UnsafeBorrowResult::Err(e) => Err(e),
        }
    }

    /// Attempts to create a [`GuestStrCow<'_>`] from this pointer, performing
    /// bounds checks and utf-8 checks. Whereas [`GuestPtr::as_str`] will fail
    /// with `None` if attempting to access Wasm shared memory, this call will
    /// succeed: if used on shared memory, this function will copy the string
    /// into [`GuestStrCow::Copied`]. If the memory is non-shared, this returns
    /// a [`GuestStrCow::Borrowed`] (a thin wrapper over [`GuestStr<'_, T>]`).
    pub fn as_cow(&self) -> Result<GuestStrCow<'a>, GuestError> {
        match self.as_bytes().as_unsafe_slice_mut()?.shared_borrow() {
            UnsafeBorrowResult::Ok(s) => Ok(GuestStrCow::Borrowed(s.try_into()?)),
            UnsafeBorrowResult::Shared(_) => {
                let copied = self.as_bytes().to_vec()?;
                let utf8_string = String::from_utf8(copied).map_err(|e| e.utf8_error())?;
                Ok(GuestStrCow::Copied(utf8_string))
            }
            UnsafeBorrowResult::Err(e) => Err(e),
        }
    }
}

impl<'a> GuestPtr<'a, [u8]> {
    /// Returns a pointer to the string represented by a `[u8]` without
    /// validating whether each u8 is a utf-8 codepoint.
    pub fn as_str_ptr(&self) -> GuestPtr<'a, str> {
        GuestPtr::new(self.mem, self.pointer)
    }
}

impl<T: ?Sized + Pointee> Clone for GuestPtr<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Pointee> Copy for GuestPtr<'_, T> {}

impl<T: ?Sized + Pointee> fmt::Debug for GuestPtr<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::debug(self.pointer, f)
    }
}

/// A smart pointer to an shareable slice in guest memory.
///
/// Usable as a `&'a [T]` via [`std::ops::Deref`].
pub struct GuestSlice<'a, T> {
    ptr: &'a [UnsafeCell<T>],
    mem: &'a dyn GuestMemory,
    borrow: BorrowHandle,
}

// This is a wrapper around `&[T]` and must mirror send/sync impls due to the
// interior usage of `&[UnsafeCell<T>]`.
unsafe impl<T: Send> Send for GuestSlice<'_, T> {}
unsafe impl<T: Sync> Sync for GuestSlice<'_, T> {}

impl<'a, T> std::ops::Deref for GuestSlice<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // SAFETY: The presence of `GuestSlice` indicates that this is an
        // unshared memory meaning concurrent accesses will not happen.
        // Furthermore the validity of the slice has already been established
        // and a runtime borrow has been recorded to prevent conflicting views.
        // This all adds up to the ability to return a safe slice from this
        // method whose lifetime is connected to `self`.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr().cast(), self.ptr.len()) }
    }
}

impl<'a, T> Drop for GuestSlice<'a, T> {
    fn drop(&mut self) {
        self.mem.shared_unborrow(self.borrow)
    }
}

/// A smart pointer to a mutable slice in guest memory.
///
/// Usable as a `&'a [T]` via [`std::ops::Deref`] and as a `&'a mut [T]` via
/// [`std::ops::DerefMut`].
pub struct GuestSliceMut<'a, T> {
    ptr: &'a [UnsafeCell<T>],
    mem: &'a dyn GuestMemory,
    borrow: BorrowHandle,
}

// See docs in these impls for `GuestSlice` above.
unsafe impl<T: Send> Send for GuestSliceMut<'_, T> {}
unsafe impl<T: Sync> Sync for GuestSliceMut<'_, T> {}

impl<'a, T> std::ops::Deref for GuestSliceMut<'a, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        // SAFETY: See docs in `Deref for GuestSlice`
        unsafe { slice::from_raw_parts(self.ptr.as_ptr().cast(), self.ptr.len()) }
    }
}

impl<'a, T> std::ops::DerefMut for GuestSliceMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: See docs in `Deref for GuestSlice`
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr() as *mut T, self.ptr.len()) }
    }
}

impl<'a, T> Drop for GuestSliceMut<'a, T> {
    fn drop(&mut self) {
        self.mem.mut_unborrow(self.borrow)
    }
}

/// A smart pointer for distinguishing between different kinds of Wasm memory:
/// shared and non-shared.
///
/// As with `GuestSlice`, this is usable as a `&'a [T]` via [`std::ops::Deref`].
/// The major difference is that, for shared memories, the memory will be copied
/// out of Wasm linear memory to avoid the possibility of concurrent mutation by
/// another thread. This extra copy exists solely to maintain the Rust
/// guarantees regarding `&[T]`.
pub enum GuestCow<'a, T> {
    Borrowed(GuestSlice<'a, T>),
    Copied(Vec<T>),
}

impl<'a, T> std::ops::Deref for GuestCow<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            GuestCow::Borrowed(s) => s,
            GuestCow::Copied(s) => s,
        }
    }
}

/// A smart pointer to an `unsafe` slice in guest memory.
///
/// Accessing guest memory (e.g., WebAssembly linear memory) is inherently
/// `unsafe`. Even though this structure expects that we will have validated the
/// addresses, lengths, and alignment, we must be extra careful to maintain the
/// Rust borrowing guarantees if we hand out slices to the underlying memory.
/// This is done in two ways:
///
/// - with shared memory (i.e., memory that may be accessed concurrently by
///   multiple threads), we have no guarantee that the underlying data will not
///   be changed; thus, we can only hand out slices `unsafe`-ly (TODO:
///   eventually with `UnsafeGuestSlice::as_slice`,
///   `UnsafeGuestSlice::as_slice_mut`)
/// - with non-shared memory, we _can_ maintain the Rust slice guarantees, but
///   only by manually performing borrow-checking of the underlying regions that
///   are accessed; this kind of borrowing is wrapped up in the [`GuestSlice`]
///   and [`GuestSliceMut`] smart pointers (see
///   `UnsafeGuestSlice::shared_borrow`, `UnsafeGuestSlice::mut_borrow`).
pub struct UnsafeGuestSlice<'a, T> {
    /// A raw pointer to the bytes in memory.
    ptr: &'a [UnsafeCell<T>],
    /// The (validated) address bounds of the slice in memory.
    region: Region,
    /// The original memory.
    mem: &'a dyn GuestMemory,
}

// SAFETY: `UnsafeGuestSlice` can be used across an `await` and therefore must
// be `Send` and `Sync`. As with `GuestSlice` and friends, we mirror the
// `Send`/`Sync` impls due to the interior usage of `&[UnsafeCell<T>]`.
unsafe impl<T: Sync> Sync for UnsafeGuestSlice<'_, T> {}
unsafe impl<T: Send> Send for UnsafeGuestSlice<'_, T> {}

impl<'a, T> UnsafeGuestSlice<'a, T> {
    /// See `GuestPtr::copy_from_slice`.
    pub fn copy_from_slice(self, slice: &[T]) -> Result<(), GuestError>
    where
        T: GuestTypeTransparent<'a> + Copy + 'a,
    {
        // Check the length...
        if self.ptr.len() != slice.len() {
            return Err(GuestError::SliceLengthsDiffer);
        }
        if slice.len() == 0 {
            return Ok(());
        }

        // ... and copy the bytes.
        match self.mut_borrow() {
            UnsafeBorrowResult::Ok(mut dst) => dst.copy_from_slice(slice),
            UnsafeBorrowResult::Shared(guest_slice) => {
                // SAFETY: in the shared memory case, we copy and accept that
                // the guest data may be concurrently modified. TODO: audit that
                // this use of `std::ptr::copy` is safe with shared memory
                // (https://github.com/bytecodealliance/wasmtime/issues/4203)
                //
                // Also note that the validity of `guest_slice` has already been
                // determined by the `as_unsafe_slice_mut` call above.
                unsafe {
                    std::ptr::copy(
                        slice.as_ptr(),
                        guest_slice.ptr[0].get(),
                        guest_slice.ptr.len(),
                    )
                };
            }
            UnsafeBorrowResult::Err(e) => return Err(e),
        }
        Ok(())
    }

    /// Return the number of items in this slice.
    pub fn len(&self) -> usize {
        self.ptr.len()
    }

    /// Check if this slice comes from WebAssembly shared memory.
    pub fn is_shared_memory(&self) -> bool {
        self.mem.is_shared_memory()
    }

    /// See `GuestPtr::as_slice_mut`.
    pub fn as_slice_mut(self) -> Result<Option<GuestSliceMut<'a, T>>, GuestError>
    where
        T: GuestTypeTransparent<'a>,
    {
        match self.mut_borrow() {
            UnsafeBorrowResult::Ok(slice) => Ok(Some(slice)),
            UnsafeBorrowResult::Shared(_) => Ok(None),
            UnsafeBorrowResult::Err(e) => Err(e),
        }
    }

    /// Transform an `unsafe` guest slice to a [`GuestSliceMut`].
    ///
    /// # Safety
    ///
    /// This function is safe if and only if:
    /// - the memory is not shared (it will return `None` in this case) and
    /// - there are no overlapping mutable borrows for this region.
    fn shared_borrow(self) -> UnsafeBorrowResult<GuestSlice<'a, T>, Self> {
        if self.mem.is_shared_memory() {
            UnsafeBorrowResult::Shared(self)
        } else {
            match self.mem.shared_borrow(self.region) {
                Ok(borrow) => UnsafeBorrowResult::Ok(GuestSlice {
                    ptr: self.ptr,
                    mem: self.mem,
                    borrow,
                }),
                Err(e) => UnsafeBorrowResult::Err(e),
            }
        }
    }

    /// Transform an `unsafe` guest slice to a [`GuestSliceMut`].
    ///
    /// # Safety
    ///
    /// This function is safe if and only if:
    /// - the memory is not shared (it will return `None` in this case) and
    /// - there are no overlapping borrows of any kind (shared or mutable) for
    ///   this region.
    fn mut_borrow(self) -> UnsafeBorrowResult<GuestSliceMut<'a, T>, Self> {
        if self.mem.is_shared_memory() {
            UnsafeBorrowResult::Shared(self)
        } else {
            match self.mem.mut_borrow(self.region) {
                Ok(borrow) => UnsafeBorrowResult::Ok(GuestSliceMut {
                    ptr: self.ptr,
                    mem: self.mem,
                    borrow,
                }),
                Err(e) => UnsafeBorrowResult::Err(e),
            }
        }
    }
}

/// A three-way result type for expressing that borrowing from an
/// [`UnsafeGuestSlice`] could fail in multiple ways. Retaining the
/// [`UnsafeGuestSlice`] in the `Shared` case allows us to reuse it.
enum UnsafeBorrowResult<T, S> {
    /// The borrow succeeded.
    Ok(T),
    /// The borrow failed because the underlying memory was shared--we cannot
    /// safely borrow in this case and return the original unsafe slice.
    Shared(S),
    /// The borrow failed for some other reason, e.g., the region was already
    /// borrowed.
    Err(GuestError),
}

impl<T, S> From<GuestError> for UnsafeBorrowResult<T, S> {
    fn from(e: GuestError) -> Self {
        UnsafeBorrowResult::Err(e)
    }
}

/// A smart pointer to an shareable `str` in guest memory.
/// Usable as a `&'a str` via [`std::ops::Deref`].
pub struct GuestStr<'a>(GuestSlice<'a, u8>);

impl<'a> std::convert::TryFrom<GuestSlice<'a, u8>> for GuestStr<'a> {
    type Error = GuestError;
    fn try_from(slice: GuestSlice<'a, u8>) -> Result<Self, Self::Error> {
        match str::from_utf8(&slice) {
            Ok(_) => Ok(Self(slice)),
            Err(e) => Err(GuestError::InvalidUtf8(e)),
        }
    }
}

impl<'a> std::ops::Deref for GuestStr<'a> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        // SAFETY: every slice in a `GuestStr` has already been checked for
        // UTF-8 validity during construction (i.e., `TryFrom`).
        unsafe { str::from_utf8_unchecked(&self.0) }
    }
}

/// A smart pointer to a mutable `str` in guest memory.
/// Usable as a `&'a str` via [`std::ops::Deref`] and as a `&'a mut str` via
/// [`std::ops::DerefMut`].
pub struct GuestStrMut<'a>(GuestSliceMut<'a, u8>);

impl<'a> std::convert::TryFrom<GuestSliceMut<'a, u8>> for GuestStrMut<'a> {
    type Error = GuestError;
    fn try_from(slice: GuestSliceMut<'a, u8>) -> Result<Self, Self::Error> {
        match str::from_utf8(&slice) {
            Ok(_) => Ok(Self(slice)),
            Err(e) => Err(GuestError::InvalidUtf8(e)),
        }
    }
}

impl<'a> std::ops::Deref for GuestStrMut<'a> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        // SAFETY: every slice in a `GuestStrMut` has already been checked for
        // UTF-8 validity during construction (i.e., `TryFrom`).
        unsafe { str::from_utf8_unchecked(&self.0) }
    }
}

impl<'a> std::ops::DerefMut for GuestStrMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: every slice in a `GuestStrMut` has already been checked for
        // UTF-8 validity during construction (i.e., `TryFrom`).
        unsafe { str::from_utf8_unchecked_mut(&mut self.0) }
    }
}

/// A smart pointer to a `str` for distinguishing between different kinds of
/// Wasm memory: shared and non-shared.
///
/// As with `GuestStr`, this is usable as a `&'a str` via [`std::ops::Deref`].
/// The major difference is that, for shared memories, the string will be copied
/// out of Wasm linear memory to avoid the possibility of concurrent mutation by
/// another thread. This extra copy exists solely to maintain the Rust
/// guarantees regarding `&str`.
pub enum GuestStrCow<'a> {
    Borrowed(GuestStr<'a>),
    Copied(String),
}

impl<'a> std::ops::Deref for GuestStrCow<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            GuestStrCow::Borrowed(s) => s,
            GuestStrCow::Copied(s) => s,
        }
    }
}

mod private {
    pub trait Sealed {}
    impl<T> Sealed for T {}
    impl<T> Sealed for [T] {}
    impl Sealed for str {}
}

/// Types that can be pointed to by `GuestPtr<T>`.
///
/// In essence everything can, and the only special-case is unsized types like
/// `str` and `[T]` which have special implementations.
pub trait Pointee: private::Sealed {
    #[doc(hidden)]
    type Pointer: Copy;
    #[doc(hidden)]
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result;
}

impl<T> Pointee for T {
    type Pointer = u32;
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}", pointer)
    }
}

impl<T> Pointee for [T] {
    type Pointer = (u32, u32);
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}/{}", pointer.0, pointer.1)
    }
}

impl Pointee for str {
    type Pointer = (u32, u32);
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        <[u8]>::debug(pointer, f)
    }
}

pub fn run_in_dummy_executor<F: std::future::Future>(future: F) -> Result<F::Output> {
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    let mut f = Pin::from(Box::new(future));
    let waker = dummy_waker();
    let mut cx = Context::from_waker(&waker);
    match f.as_mut().poll(&mut cx) {
        Poll::Ready(val) => return Ok(val),
        Poll::Pending =>
            bail!("Cannot wait on pending future: must enable wiggle \"async\" future and execute on an async Store"),
    }

    fn dummy_waker() -> Waker {
        return unsafe { Waker::from_raw(clone(5 as *const _)) };

        unsafe fn clone(ptr: *const ()) -> RawWaker {
            assert_eq!(ptr as usize, 5);
            const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
            RawWaker::new(ptr, &VTABLE)
        }

        unsafe fn wake(ptr: *const ()) {
            assert_eq!(ptr as usize, 5);
        }

        unsafe fn wake_by_ref(ptr: *const ()) {
            assert_eq!(ptr as usize, 5);
        }

        unsafe fn drop(ptr: *const ()) {
            assert_eq!(ptr as usize, 5);
        }
    }
}
