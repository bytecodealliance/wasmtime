use crate::{GuestError, GuestPtr};
use std::mem;

pub trait GuestErrorType {
    type Context;
    fn success() -> Self;
    fn from_error(e: GuestError, ctx: &Self::Context) -> Self;
}

pub trait GuestType<'a>: Sized {
    fn guest_size() -> u32;
    fn guest_align() -> usize;
    fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, GuestError>;
    fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError>;
}

macro_rules! primitives {
    ($($i:ident)*) => ($(
        impl<'a> GuestType<'a> for $i {
            fn guest_size() -> u32 { mem::size_of::<Self>() as u32 }
            fn guest_align() -> usize { mem::align_of::<Self>() }

            fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {

                // Any bit pattern for any primitive implemented with this
                // macro is safe, so our `as_raw` method will guarantee that if
                // we are given a pointer it's valid for the size of our type
                // as well as properly aligned. Consequently we should be able
                // to safely ready the pointer just after we validated it,
                // returning it along here.
                let host_ptr = ptr.mem().validate_size_align(
                    ptr.offset(),
                    Self::guest_align(),
                    Self::guest_size(),
                )?;
                Ok(unsafe { *host_ptr.cast::<Self>() })
            }

            fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError> {
                let host_ptr = ptr.mem().validate_size_align(
                    ptr.offset(),
                    Self::guest_align(),
                    Self::guest_size(),
                )?;
                // Similar to above `as_raw` will do a lot of validation, and
                // then afterwards we can safely write our value into the
                // memory location.
                unsafe {
                    *host_ptr.cast::<Self>() = val;
                }
                Ok(())
            }
        }
    )*)
}

primitives! {
    i8 i16 i32 i64 i128 isize
    u8 u16 u32 u64 u128 usize
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
        Ok(GuestPtr::new(ptr.mem(), offset))
    }

    fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), GuestError> {
        ptr.cast::<u32>().write(val.offset())
    }
}
