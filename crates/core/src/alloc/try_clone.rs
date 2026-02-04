use crate::error::OutOfMemory;

/// A trait for values that can be cloned, but contain owned, heap-allocated
/// values whose allocations may fail during cloning.
pub trait TryClone: Sized {
    /// Attempt to clone `self`, returning an error if any allocation fails
    /// during cloning.
    fn try_clone(&self) -> Result<Self, OutOfMemory>;
}

impl<T> TryClone for *mut T {
    #[inline]
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(*self)
    }
}

impl<T> TryClone for core::ptr::NonNull<T> {
    #[inline]
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(*self)
    }
}

macro_rules! impl_try_clone_via_clone {
    ( $( $ty:ty ),* $(,)? ) => {
        $(
            impl TryClone for $ty {
                #[inline]
                fn try_clone(&self) -> Result<Self, OutOfMemory> {
                    Ok(self.clone())
                }
            }
        )*
    };
}

impl_try_clone_via_clone! {
    bool, char,
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
    f32, f64,
}

macro_rules! tuples {
    ( $( $( $t:ident ),* );*) => {
        $(
            impl<$($t),*> TryClone for ( $($t,)* )
            where
                $( $t: TryClone ),*
            {
                #[inline]
                fn try_clone(&self) -> Result<Self, OutOfMemory> {
                    #[allow(non_snake_case, reason = "macro code")]
                    let ( $($t,)* ) = self;
                    Ok(( $( $t.try_clone()?, )* ))
                }
            }
        )*
    };
}

tuples! {
    A;
    A, B;
    A, B, C;
    A, B, C, D;
    A, B, C, D, E;
    A, B, C, D, E, F;
    A, B, C, D, E, F, G;
    A, B, C, D, E, F, G, H;
    A, B, C, D, E, F, G, H, I;
    A, B, C, D, E, F, G, H, I, J;
}
