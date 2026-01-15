use super::try_alloc;
use crate::error::OutOfMemory;
use crate::prelude::*;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

/// A wrapper around `std::boxed::Box<T>` with fallible allocation.
#[repr(transparent)]
pub struct OomBox<T: ?Sized> {
    inner: Box<T>,
}

impl<T: ?Sized> Deref for OomBox<T> {
    type Target = Box<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: ?Sized> DerefMut for OomBox<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: ?Sized> AsRef<Box<T>> for OomBox<T> {
    #[inline]
    fn as_ref(&self) -> &Box<T> {
        &self.inner
    }
}

impl<T: ?Sized> AsMut<Box<T>> for OomBox<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut Box<T> {
        &mut self.inner
    }
}

impl<T: ?Sized> AsRef<T> for OomBox<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T: ?Sized> AsMut<T> for OomBox<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: ?Sized> From<Box<T>> for OomBox<T> {
    #[inline]
    fn from(value: Box<T>) -> Self {
        OomBox { inner: value }
    }
}

// NB: Can't implement `From<OomBox<T>> for Box<T>` due to trait coherence
// rules.

impl<T> OomBox<T> {
    /// Allocate a new `OomBox<T>`, returning `Err(OutOfMemory)` on allocation
    /// failure.
    #[inline]
    pub fn new(value: T) -> Result<Self, OutOfMemory> {
        let boxed = Self::new_uninit()?;
        Ok(Self::write(boxed, value))
    }

    /// Allocate an `OomBox` with uninitialized contents, returning
    /// `Err(OutOfMemory)` on allocation failure.
    ///
    /// You can initialize the resulting box's value via [`OomBox::write`].
    #[inline]
    pub fn new_uninit() -> Result<OomBox<MaybeUninit<T>>, OutOfMemory> {
        let layout = alloc::alloc::Layout::new::<MaybeUninit<T>>();

        if layout.size() == 0 {
            // NB: no actual allocation takes place when boxing zero-sized
            // types.
            return Ok(OomBox {
                inner: Box::new(MaybeUninit::uninit()),
            });
        }

        // Safety: layout size is non-zero.
        let ptr = unsafe { try_alloc(layout)? };

        let ptr = ptr.cast::<MaybeUninit<T>>();

        // Safety: The pointer's memory block was allocated by the global allocator.
        let inner = unsafe { Box::from_raw(ptr.as_ptr()) };

        Ok(OomBox { inner })
    }

    /// Initialize an uninitialized box.
    #[inline]
    pub fn write(boxed: OomBox<MaybeUninit<T>>, value: T) -> Self {
        Self {
            inner: Box::write(boxed.inner, value),
        }
    }
}

impl<T: ?Sized> OomBox<T> {
    /// Create a `OomBox<T>` from an existing `std::boxed::Box<T>`.
    #[inline]
    pub fn from_std_box(std: Box<T>) -> Self {
        OomBox { inner: std }
    }

    /// Convert this `OomBox<T>` into a `std::boxed::Box<T>`.
    #[inline]
    pub fn into_std_box(boxed: Self) -> Box<T> {
        boxed.inner
    }
}

/// Convert a `OomBox<T: SomeTrait>` into a `OomBox<dyn SomeTrait>`.
///
/// This operation does not allocate.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "std")]
/// # fn _foo() -> wasmtime_environ::error::Result<()> {
/// use std::io;
/// use wasmtime_environ::collections::OomBox;
///
/// // We have an `Box` of some concrete type that implements `io::Write`.
/// let output: Box<io::Stdout> = Box::new(io::stdout())?;
///
/// // We can type erase it into an `Box<dyn io::Write>` trait object with
/// // `to_dyn_box!`.
/// let output: Box<dyn io::Write> = to_dyn_box!(output);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! to_dyn_box {
    ( $boxed:expr ) => {{
        let boxed: $crate::collections::OomBox<_> = $boxed;
        let boxed = $crate::collections::OomBox::into_std_box(boxed);
        $crate::collections::OomBox::from_std_box(boxed as _)
    }};
}
