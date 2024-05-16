use crate::{GuestError, GuestPtr};
use std::mem;
use std::sync::atomic::{
    AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicU16, AtomicU32, AtomicU64, AtomicU8, Ordering,
};

/// A trait for types which are used to report errors. Each type used in the
/// first result position of an interface function is used, by convention, to
/// indicate whether the function was successful and subsequent results are valid,
/// or whether an error occurred. This trait allows wiggle to return the correct
/// value when the interface function's idiomatic Rust method returns
/// `Ok(<rest of return values>)`.
pub trait GuestErrorType {
    fn success() -> Self;
}

/// A trait for types that are intended to be pointees in `GuestPtr<T>`.
///
/// This trait abstracts how to read/write information from the guest memory, as
/// well as how to offset elements in an array of guest memory. This layer of
/// abstraction allows the guest representation of a type to be different from
/// the host representation of a type, if necessary. It also allows for
/// validation when reading/writing.
pub trait GuestType<'a>: Sized {
    /// Returns the size, in bytes, of this type in the guest memory.
    fn guest_size() -> u32;

    /// Returns the required alignment of this type, in bytes, for both guest
    /// and host memory.
    fn guest_align() -> usize;

    /// Reads this value from the provided `ptr`.
    ///
    /// Must internally perform any safety checks necessary and is allowed to
    /// fail if the bytes pointed to are also invalid.
    ///
    /// Typically if you're implementing this by hand you'll want to delegate to
    /// other safe implementations of this trait (e.g. for primitive types like
    /// `u32`) rather than writing lots of raw code yourself.
    fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, GuestError>;

    /// Writes a value to `ptr` after verifying that `ptr` is indeed valid to
    /// store `val`.
    ///
    /// Similar to `read`, you'll probably want to implement this in terms of
    /// other primitives.
    fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError>;
}

/// A trait for `GuestType`s that have the same representation in guest memory
/// as in Rust. These types can be used with the `GuestPtr::as_slice` method to
/// view as a slice.
///
/// Unsafe trait because a correct `GuestTypeTransparent` implementation ensures
/// that the `GuestPtr::as_slice` methods are safe, notably that the
/// representation on the host matches the guest and all bit patterns are
/// valid. This trait should only ever be implemented by
/// wiggle_generate-produced code.
pub unsafe trait GuestTypeTransparent<'a>: GuestType<'a> {}

macro_rules! integer_primitives {
    ($([$ty:ident, $ty_atomic:ident],)*) => ($(
        impl<'a> GuestType<'a> for $ty {
            #[inline]
            fn guest_size() -> u32 { mem::size_of::<Self>() as u32 }
            #[inline]
            fn guest_align() -> usize { mem::align_of::<Self>() }

            #[inline]
            fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
                // Use `validate_size_align` to validate offset and alignment
                // internally. The `host_ptr` type will be `&UnsafeCell<Self>`
                // indicating that the memory is valid, and next safety checks
                // are required to access it.
                let offset = ptr.offset();
                let (host_ptr, region) = super::validate_size_align::<Self>(ptr.mem(), offset, 1)?;
                let host_ptr = &host_ptr[0];

                // If this memory is mutable borrowed then it cannot be read
                // here, so skip this operation.
                //
                // Note that shared memories don't allow borrows and other
                // shared borrows are ok to overlap with this.
                if !ptr.mem().can_read(region) {
                    return Err(GuestError::PtrBorrowed(region));
                }

                // If the accessed memory is shared, we need to load the bytes
                // with the correct memory consistency. We could check if the
                // memory is shared each time, but we expect little performance
                // difference between an additional branch and a relaxed memory
                // access and thus always do the relaxed access here.
                let atomic_value_ref: &$ty_atomic =
                    unsafe { &*(host_ptr.get().cast::<$ty_atomic>()) };
                let val = atomic_value_ref.load(Ordering::Relaxed);

                // And as a final operation convert from the little-endian wasm
                // value to a native-endian value for the host.
                Ok($ty::from_le(val))
            }

            #[inline]
            fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError> {
                // See `read` above for various checks here.
                let val = val.to_le();
                let offset = ptr.offset();
                let (host_ptr, region) = super::validate_size_align::<Self>(ptr.mem(), offset, 1)?;
                let host_ptr = &host_ptr[0];
                if !ptr.mem().can_write(region) {
                    return Err(GuestError::PtrBorrowed(region));
                }
                let atomic_value_ref: &$ty_atomic =
                    unsafe { &*(host_ptr.get().cast::<$ty_atomic>()) };
                atomic_value_ref.store(val, Ordering::Relaxed);
                Ok(())
            }
        }

        unsafe impl<'a> GuestTypeTransparent<'a> for $ty {}

    )*)
}

macro_rules! float_primitives {
    ($([$ty:ident, $ty_unsigned:ident, $ty_atomic:ident],)*) => ($(
        impl<'a> GuestType<'a> for $ty {
            #[inline]
            fn guest_size() -> u32 { mem::size_of::<Self>() as u32 }
            #[inline]
            fn guest_align() -> usize { mem::align_of::<Self>() }

            #[inline]
            fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
                // For more commentary see `read` for integers
                let offset = ptr.offset();
                let (host_ptr, region) = super::validate_size_align::<$ty_unsigned>(
                    ptr.mem(),
                    offset,
                    1,
                )?;
                let host_ptr = &host_ptr[0];
                if !ptr.mem().can_read(region) {
                    return Err(GuestError::PtrBorrowed(region));
                }
                let atomic_value_ref: &$ty_atomic =
                    unsafe { &*(host_ptr.get().cast::<$ty_atomic>()) };
                let value = $ty_unsigned::from_le(atomic_value_ref.load(Ordering::Relaxed));
                Ok($ty::from_bits(value))
            }

            #[inline]
            fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError> {
                // For more commentary see `read`/`write` for integers.
                let offset = ptr.offset();
                let (host_ptr, region) = super::validate_size_align::<$ty_unsigned>(
                    ptr.mem(),
                    offset,
                    1,
                )?;
                let host_ptr = &host_ptr[0];
                if !ptr.mem().can_write(region) {
                    return Err(GuestError::PtrBorrowed(region));
                }
                let atomic_value_ref: &$ty_atomic =
                    unsafe { &*(host_ptr.get().cast::<$ty_atomic>()) };
                let le_value = $ty_unsigned::to_le(val.to_bits());
                atomic_value_ref.store(le_value, Ordering::Relaxed);
                Ok(())
            }
        }

        unsafe impl<'a> GuestTypeTransparent<'a> for $ty {}

    )*)
}

integer_primitives! {
    // signed
    [i8, AtomicI8], [i16, AtomicI16], [i32, AtomicI32], [i64, AtomicI64],
    // unsigned
    [u8, AtomicU8], [u16, AtomicU16], [u32, AtomicU32], [u64, AtomicU64],
}

float_primitives! {
    [f32, u32, AtomicU32], [f64, u64, AtomicU64],
}

// Support pointers-to-pointers where pointers are always 32-bits in wasm land
impl<'a, T> GuestType<'a> for GuestPtr<'a, T> {
    #[inline]
    fn guest_size() -> u32 {
        u32::guest_size()
    }

    #[inline]
    fn guest_align() -> usize {
        u32::guest_align()
    }

    fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
        let offset = ptr.cast::<u32>().read()?;
        Ok(GuestPtr::new(ptr.mem(), offset))
    }

    fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError> {
        ptr.cast::<u32>().write(val.offset())
    }
}

// Support pointers-to-arrays where pointers are always 32-bits in wasm land
impl<'a, T> GuestType<'a> for GuestPtr<'a, [T]>
where
    T: GuestType<'a>,
{
    #[inline]
    fn guest_size() -> u32 {
        u32::guest_size() * 2
    }

    #[inline]
    fn guest_align() -> usize {
        u32::guest_align()
    }

    fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
        let offset = ptr.cast::<u32>().read()?;
        let len = ptr.cast::<u32>().add(1)?.read()?;
        Ok(GuestPtr::new(ptr.mem(), offset).as_array(len))
    }

    fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError> {
        let (offs, len) = val.offset();
        let len_ptr = ptr.cast::<u32>().add(1)?;
        ptr.cast::<u32>().write(offs)?;
        len_ptr.write(len)
    }
}
