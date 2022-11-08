use crate::{region::Region, GuestError, GuestPtr};
use std::mem;
use std::sync::atomic::{
    AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicU16, AtomicU32, AtomicU64, AtomicU8, Ordering,
};

/// A trait for types which are used to report errors. Each type used in the
/// first result position of an interface function is used, by convention, to
/// indicate whether the function was successful and subsequent results are valid,
/// or whether an error occured. This trait allows wiggle to return the correct
/// value when the interface function's idiomatic Rust method returns
/// Ok(<rest of return values>).
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
/// Unsafe trait because a correct GuestTypeTransparent implemengation ensures that the
/// GuestPtr::as_slice methods are safe. This trait should only ever be implemented
/// by wiggle_generate-produced code.
pub unsafe trait GuestTypeTransparent<'a>: GuestType<'a> {
    /// Checks that the memory at `ptr` is a valid representation of `Self`.
    ///
    /// Assumes that memory safety checks have already been performed: `ptr`
    /// has been checked to be aligned correctly and reside in memory using
    /// `GuestMemory::validate_size_align`
    fn validate(ptr: *mut Self) -> Result<(), GuestError>;
}

macro_rules! integer_primitives {
    ($([$ty:ident, $ty_atomic:ident],)*) => ($(
        impl<'a> GuestType<'a> for $ty {
            fn guest_size() -> u32 { mem::size_of::<Self>() as u32 }
            fn guest_align() -> usize { mem::align_of::<Self>() }

            #[inline]
            fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
                // Any bit pattern for any primitive implemented with this
                // macro is safe, so our `validate_size_align` method will
                // guarantee that if we are given a pointer it's valid for the
                // size of our type as well as properly aligned. Consequently we
                // should be able to safely ready the pointer just after we
                // validated it, returning it along here.
                let offset = ptr.offset();
                let size = Self::guest_size();
                let host_ptr = ptr.mem().validate_size_align(
                    offset,
                    Self::guest_align(),
                    size,
                )?;
                let region = Region {
                    start: offset,
                    len: size,
                };
                if ptr.mem().is_mut_borrowed(region) {
                    return Err(GuestError::PtrBorrowed(region));
                }
                // If the accessed memory is shared, we need to load the bytes
                // with the correct memory consistency. We could check if the
                // memory is shared each time, but we expect little performance
                // difference between an additional branch and a relaxed memory
                // access and thus always do the relaxed access here.
                let atomic_value_ref: &$ty_atomic =
                    unsafe { &*(host_ptr.cast::<$ty_atomic>()) };
                Ok($ty::from_le(atomic_value_ref.load(Ordering::Relaxed)))
            }

            #[inline]
            fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError> {
                let offset = ptr.offset();
                let size = Self::guest_size();
                let host_ptr = ptr.mem().validate_size_align(
                    offset,
                    Self::guest_align(),
                    size,
                )?;
                let region = Region {
                    start: offset,
                    len: size,
                };
                if ptr.mem().is_shared_borrowed(region) || ptr.mem().is_mut_borrowed(region) {
                    return Err(GuestError::PtrBorrowed(region));
                }
                // If the accessed memory is shared, we need to load the bytes
                // with the correct memory consistency. We could check if the
                // memory is shared each time, but we expect little performance
                // difference between an additional branch and a relaxed memory
                // access and thus always do the relaxed access here.
                let atomic_value_ref: &$ty_atomic =
                    unsafe { &*(host_ptr.cast::<$ty_atomic>()) };
                atomic_value_ref.store(val.to_le(), Ordering::Relaxed);
                Ok(())
            }
        }

        unsafe impl<'a> GuestTypeTransparent<'a> for $ty {
            #[inline]
            fn validate(_ptr: *mut $ty) -> Result<(), GuestError> {
                // All bit patterns are safe, nothing to do here
                Ok(())
            }
        }

    )*)
}

macro_rules! float_primitives {
    ($([$ty:ident, $ty_unsigned:ident, $ty_atomic:ident],)*) => ($(
        impl<'a> GuestType<'a> for $ty {
            fn guest_size() -> u32 { mem::size_of::<Self>() as u32 }
            fn guest_align() -> usize { mem::align_of::<Self>() }

            #[inline]
            fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
                // Any bit pattern for any primitive implemented with this
                // macro is safe, so our `validate_size_align` method will
                // guarantee that if we are given a pointer it's valid for the
                // size of our type as well as properly aligned. Consequently we
                // should be able to safely ready the pointer just after we
                // validated it, returning it along here.
                let offset = ptr.offset();
                let size = Self::guest_size();
                let host_ptr = ptr.mem().validate_size_align(
                    offset,
                    Self::guest_align(),
                    size,
                )?;
                let region = Region {
                    start: offset,
                    len: size,
                };
                if ptr.mem().is_mut_borrowed(region) {
                    return Err(GuestError::PtrBorrowed(region));
                }
                // If the accessed memory is shared, we need to load the bytes
                // with the correct memory consistency. We could check if the
                // memory is shared each time, but we expect little performance
                // difference between an additional branch and a relaxed memory
                // access and thus always do the relaxed access here.
                let atomic_value_ref: &$ty_atomic =
                    unsafe { &*(host_ptr.cast::<$ty_atomic>()) };
                let value = $ty_unsigned::from_le(atomic_value_ref.load(Ordering::Relaxed));
                Ok($ty::from_bits(value))
            }

            #[inline]
            fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError> {
                let offset = ptr.offset();
                let size = Self::guest_size();
                let host_ptr = ptr.mem().validate_size_align(
                    offset,
                    Self::guest_align(),
                    size,
                )?;
                let region = Region {
                    start: offset,
                    len: size,
                };
                if ptr.mem().is_shared_borrowed(region) || ptr.mem().is_mut_borrowed(region) {
                    return Err(GuestError::PtrBorrowed(region));
                }
                // If the accessed memory is shared, we need to load the bytes
                // with the correct memory consistency. We could check if the
                // memory is shared each time, but we expect little performance
                // difference between an additional branch and a relaxed memory
                // access and thus always do the relaxed access here.
                let atomic_value_ref: &$ty_atomic =
                    unsafe { &*(host_ptr.cast::<$ty_atomic>()) };
                let le_value = $ty_unsigned::to_le(val.to_bits());
                atomic_value_ref.store(le_value, Ordering::Relaxed);
                Ok(())
            }
        }

        unsafe impl<'a> GuestTypeTransparent<'a> for $ty {
            #[inline]
            fn validate(_ptr: *mut $ty) -> Result<(), GuestError> {
                // All bit patterns are safe, nothing to do here
                Ok(())
            }
        }

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
    fn guest_size() -> u32 {
        u32::guest_size()
    }

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
    fn guest_size() -> u32 {
        u32::guest_size() * 2
    }

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
