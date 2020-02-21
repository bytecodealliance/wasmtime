use crate::{GuestError, GuestPtr, GuestPtrMut};

pub trait GuestType: Sized {
    // These are morally the same as Rust ::std::mem::size_of / align_of, but they return
    // a u32 because the wasm memory space is 32 bits. They have a different names so they
    // don't collide with the std::mem methods.
    fn size() -> u32;
    fn align() -> u32;
    fn name() -> String;
    fn validate<'a>(location: &GuestPtr<'a, Self>) -> Result<(), GuestError>;
}

pub trait GuestTypeCopy: GuestType + Copy {}
pub trait GuestTypeClone<'a>: GuestType + Clone {
    fn read_from_guest(location: &GuestPtr<'a, Self>) -> Result<Self, GuestError>;
    fn write_to_guest(&self, location: &GuestPtrMut<'a, Self>);
}

macro_rules! builtin_type {
    ( $( $t:ident ), * ) => {
        $(
        impl GuestType for $t {
            fn size() -> u32 {
                ::std::mem::size_of::<$t>() as u32
            }
            fn align() -> u32 {
                ::std::mem::align_of::<$t>() as u32
            }
            fn name() -> String {
                ::std::stringify!($t).to_owned()
            }
            fn validate(_ptr: &GuestPtr<$t>) -> Result<(), GuestError> {
                Ok(())
            }
        }
        impl GuestTypeCopy for $t {}
        )*
    };
}

// These definitions correspond to all the witx BuiltinType variants that are Copy:
builtin_type!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64, usize);

// FIXME implement GuestType for char. needs to validate that its a code point. what is the sizeof a char?
// FIXME implement GuestType for String. how does validate work for array types?

pub trait GuestErrorType {
    type Context;
    fn success() -> Self;
    fn from_error(e: GuestError, ctx: &mut Self::Context) -> Self;
}
