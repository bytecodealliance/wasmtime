use crate::{GuestPtr, GuestPtrMut, MemoryError};
use thiserror::Error;

pub trait GuestType: Sized {
    fn size() -> u32;
    fn name() -> &'static str;
}

#[derive(Debug, Error)]
pub enum GuestValueError {
    #[error("Invalid enum {0}")]
    InvalidEnum(&'static str),
}

pub trait GuestTypeCopy: GuestType + Copy {
    fn read_val(src: GuestPtr<Self>) -> Result<Self, GuestValueError>;
    fn write_val(val: Self, dest: GuestPtrMut<Self>);
}

pub trait GuestTypeClone: GuestType + Clone {
    fn read_ref(src: GuestPtr<Self>, dest: &mut Self) -> Result<(), GuestValueError>;
    fn write_ref(val: &Self, dest: GuestPtrMut<Self>);
}

impl<T> GuestTypeClone for T
where
    T: GuestTypeCopy,
{
    fn read_ref(src: GuestPtr<T>, dest: &mut T) -> Result<(), GuestValueError> {
        let val = GuestTypeCopy::read_val(src)?;
        *dest = val;
        Ok(())
    }
    fn write_ref(val: &T, dest: GuestPtrMut<T>) {
        GuestTypeCopy::write_val(*val, dest)
    }
}

macro_rules! builtin_copy {
    ( $( $t:ident ), * ) => {
        $(
        impl GuestType for $t {
            fn size() -> u32 {
                ::std::mem::size_of::<$t>() as u32
            }
            fn name() -> &'static str {
                ::std::stringify!($t)
            }
        }

        impl GuestTypeCopy for $t {
            fn read_val(src: GuestPtr<$t>) -> Result<$t, GuestValueError> {
                Ok(unsafe {
                    ::std::ptr::read_unaligned(src.ptr() as *const $t)
                })
            }
            fn write_val(val: $t, dest: GuestPtrMut<$t>) {
                unsafe {
                    ::std::ptr::write_unaligned(dest.ptr_mut() as *mut $t, val)
                }
            }
        }
        )*
    };
}

// These definitions correspond to all the witx BuiltinType variants that are Copy:
builtin_copy!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64, usize, char);

pub trait GuestError {
    type Context;
    fn is_success(&self) -> bool;
    fn from_memory_error(memory_error: MemoryError, ctx: &mut Self::Context) -> Self;
    fn from_value_error(value_error: GuestValueError, ctx: &mut Self::Context) -> Self;
}
