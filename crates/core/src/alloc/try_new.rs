use wasmtime_error::OutOfMemory;

/// Helper function to invoke `<T as TryNew>::try_new`.
///
/// # Example
///
/// ```
/// # use wasmtime_internal_core::alloc::*;
/// # use wasmtime_error::Result;
/// # fn _foo() -> Result<()> {
/// let boxed = try_new::<Box<u32>>(36)?;
/// assert_eq!(*boxed, 36);
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn try_new<T>(value: T::Value) -> Result<T, OutOfMemory>
where
    T: TryNew,
{
    TryNew::try_new(value)
}

/// Extension trait providing fallible allocation for types like `Arc<T>` and
/// `Box<T>.
pub trait TryNew {
    /// The inner `T` type that is getting wrapped into an `Arc<T>` or `Box<T>`.
    type Value;

    /// Allocate a new `Self`, returning `Err(OutOfMemory)` on allocation
    /// failure.
    fn try_new(value: Self::Value) -> Result<Self, OutOfMemory>
    where
        Self: Sized;
}
