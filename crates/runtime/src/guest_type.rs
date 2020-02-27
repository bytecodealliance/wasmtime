use crate::{GuestError, GuestPtr, GuestPtrMut};

pub trait GuestType<'a>: Sized + Clone {
    // These are morally the same as Rust ::std::mem::size_of / align_of, but they return
    // a u32 because the wasm memory space is 32 bits. They have a different names so they
    // don't collide with the std::mem methods.
    fn size() -> u32;
    fn align() -> u32;
    fn name() -> String;
    fn validate(location: &GuestPtr<'a, Self>) -> Result<(), GuestError>;
    fn read(location: &GuestPtr<'a, Self>) -> Result<Self, GuestError>;
    fn write(&self, location: &GuestPtrMut<'a, Self>);
}

/// Represents any guest type which can transparently be represented
/// as a host type.
pub trait GuestTypeTransparent<'a>: GuestType<'a> + Copy {}

macro_rules! builtin_type {
    ( $( $t:ident ), * ) => {
        $(
        impl<'a> GuestType<'a> for $t {
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
            fn read(location: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
                Ok(*location.as_ref()?)
            }
            fn write(&self, location: &GuestPtrMut<'a, Self>) {
                unsafe { (location.as_raw() as *mut $t).write(*self) };
            }
        }
        impl<'a> GuestTypeTransparent<'a> for $t {}
        )*
    };
}

// These definitions correspond to all the witx BuiltinType variants that are Copy:
builtin_type!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64, usize);

pub trait GuestErrorType {
    type Context;
    fn success() -> Self;
    fn from_error(e: GuestError, ctx: &mut Self::Context) -> Self;
}
