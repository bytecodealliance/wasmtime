use crate::error::OutOfMemory;
use core::mem;
use std_alloc::sync::Arc;

/// A trait for values that can be cloned, but contain owned, heap-allocated
/// values whose allocations may fail during cloning.
pub trait TryClone: Sized {
    /// Attempt to clone `self`, returning an error if any allocation fails
    /// during cloning.
    fn try_clone(&self) -> Result<Self, OutOfMemory>;
}

impl<T> TryClone for *mut T
where
    T: ?Sized,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(*self)
    }
}

impl<T> TryClone for core::ptr::NonNull<T>
where
    T: ?Sized,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(*self)
    }
}

impl<'a, T> TryClone for &'a T
where
    T: ?Sized,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(*self)
    }
}

impl<T, E> TryClone for Result<T, E>
where
    T: TryClone,
    E: TryClone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        match self {
            Ok(x) => Ok(Ok(x.try_clone()?)),
            Err(e) => Ok(Err(e.try_clone()?)),
        }
    }
}

impl<T> TryClone for Option<T>
where
    T: TryClone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        match self {
            Some(x) => Ok(Some(x.try_clone()?)),
            None => Ok(None),
        }
    }
}

impl<T> TryClone for Arc<T> {
    #[inline]
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(self.clone())
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

impl<T> TryClone for mem::ManuallyDrop<T>
where
    T: TryClone,
{
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(mem::ManuallyDrop::new((**self).try_clone()?))
    }
}

#[cfg(feature = "std")]
impl_try_clone_via_clone! {
    std::hash::RandomState
}

impl_try_clone_via_clone! {
    hashbrown::DefaultHashBuilder
}
