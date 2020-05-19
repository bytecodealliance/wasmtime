use crate::{region::Region, GuestError, GuestPtr};
use std::mem;

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
/// as in Rust. These types can be used with the `GuestPtr::as_raw` method to
/// view as a slice.
///
/// Unsafe trait because a correct GuestTypeTransparent implemengation ensures that the
/// GuestPtr::as_raw methods are safe. This trait should only ever be implemented
/// by wiggle_generate-produced code.
pub unsafe trait GuestTypeTransparent<'a>: GuestType<'a> {
    /// Checks that the memory at `ptr` is a valid representation of `Self`.
    ///
    /// Assumes that memory safety checks have already been performed: `ptr`
    /// has been checked to be aligned correctly and reside in memory using
    /// `GuestMemory::validate_size_align`
    fn validate(ptr: *mut Self) -> Result<(), GuestError>;
}

macro_rules! primitives {
    ($($i:ident)*) => ($(
        impl<'a> GuestType<'a> for $i {
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
                let borrow_handle = ptr.borrow_checker().borrow( Region {
                    start: offset,
                    len: size,
                })?;
                let v = unsafe { *host_ptr.cast::<Self>() };
                ptr.borrow_checker().unborrow(borrow_handle);
                Ok(v)
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
                let borrow_handle = ptr.borrow_checker().borrow( Region {
                    start: offset,
                    len: size,
                })?;
                unsafe {
                    *host_ptr.cast::<Self>() = val;
                }
                ptr.borrow_checker().unborrow(borrow_handle);
                Ok(())
            }
        }

        unsafe impl<'a> GuestTypeTransparent<'a> for $i {
            #[inline]
            fn validate(_ptr: *mut $i) -> Result<(), GuestError> {
                // All bit patterns are safe, nothing to do here
                Ok(())
            }
        }

    )*)
}

primitives! {
    // signed
    i8 i16 i32 i64 i128
    // unsigned
    u8 u16 u32 u64 u128
    // floats
    f32 f64
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
        Ok(GuestPtr::new(ptr.mem(), ptr.borrow_checker(), offset))
    }

    fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError> {
        ptr.cast::<u32>().write(val.offset())
    }
}
