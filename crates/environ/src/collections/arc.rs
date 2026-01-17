use crate::error::OutOfMemory;
use alloc::sync::Arc;
use core::ops::{Deref, DerefMut};

/// Wrapper around `alloc::sync::Arc` that provides fallible allocation.
#[derive(Debug)]
#[repr(transparent)]
pub struct OomArc<T: ?Sized> {
    inner: Arc<T>,
}

impl<T: ?Sized> Clone for OomArc<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: ?Sized> Deref for OomArc<T> {
    type Target = Arc<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: ?Sized> DerefMut for OomArc<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: ?Sized> AsRef<Arc<T>> for OomArc<T> {
    #[inline]
    fn as_ref(&self) -> &Arc<T> {
        &self.inner
    }
}

impl<T: ?Sized> AsMut<Arc<T>> for OomArc<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut Arc<T> {
        &mut self.inner
    }
}

impl<T: ?Sized> AsRef<T> for OomArc<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T: ?Sized> From<Arc<T>> for OomArc<T> {
    #[inline]
    fn from(value: Arc<T>) -> Self {
        OomArc { inner: value }
    }
}

// NB: Can't implement `From<Arc<T>> for Arc<T>` due to trait coherence
// rules.

impl<T> OomArc<T> {
    /// Allocate a new `OomArc<T>`, returning `Err(OutOfMemory)` on allocation
    /// failure.
    ///
    /// Note that stable Rust doesn't actually give us any method to build
    /// fallible allocation for `Arc<T>`, so this is only actually fallible when
    /// using nightly Rust and setting `RUSTFLAGS="--cfg arc_try_new"`.
    #[inline]
    pub fn new(value: T) -> Result<Self, OutOfMemory> {
        #[cfg(arc_try_new)]
        return Arc::try_new(value)
            .map(|inner| OomArc { inner })
            .map_err(|_| {
                // We don't have access to the exact size of the inner `Arc`
                // allocation, but (at least at one point) it was made up of a
                // strong ref count, a weak ref count, and the inner value.
                let bytes = core::mem::size_of::<(usize, usize, T)>();
                OutOfMemory::new(bytes)
            });

        #[cfg(not(arc_try_new))]
        return Ok(OomArc {
            inner: Arc::new(value),
        });
    }
}

impl<T: ?Sized> OomArc<T> {
    /// Create a `OomArc<T>` from an existing `std::sync::Arc<T>`.
    #[inline]
    pub fn from_std_arc(std: Arc<T>) -> Self {
        OomArc { inner: std }
    }

    /// Turn this `OomArc<T>` into a `std::sync::Arc<T>`.
    #[inline]
    pub fn into_std_arc(boxed: Self) -> Arc<T> {
        boxed.inner
    }
}

/// Convert a `OomArc<T: SomeTrait>` into a `OomArc<dyn SomeTrait>`.
///
/// This operation does not allocate.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "std")]
/// # fn _foo() -> wasmtime_environ::error::Result<()> {
/// use std::io;
/// use wasmtime_environ::collections::OomArc;
///
/// // We have an `OomArc` of some concrete type that implements `io::Write`.
/// let output: OomArc<io::Stdout> = Arc::new(io::stdout())?;
///
/// // We can type erase it into an `OomArc<dyn io::Write>` trait object with
/// // `to_dyn_arc!`.
/// let output: OomArc<dyn io::Write> = to_dyn_arc!(output);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! to_dyn_arc {
    ( $boxed:expr ) => {{
        let boxed: $crate::collections::OomArc<_> = $boxed;
        let boxed = $crate::collections::OomArc::into_std_arc(boxed);
        $crate::collections::OomArc::from_std_arc(boxed as _)
    }};
}
