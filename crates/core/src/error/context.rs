use crate::error::{
    Error, ErrorExt, OutOfMemory, Result,
    boxed::try_new_uninit_box,
    error::{OomOrDynError, OomOrDynErrorMut, OomOrDynErrorRef},
};
use core::any::TypeId;
use core::fmt;
use core::ptr::NonNull;
use std_alloc::boxed::Box;

mod sealed {
    use super::*;
    pub trait Sealed {}
    impl<T, E> Sealed for Result<T, E> {}
    impl<T> Sealed for Option<T> {}
}

/// Extension trait to add error context to results.
///
/// This extension trait, and its methods, are the primary way to create error
/// chains. An error's debug output will include the full chain of
/// errors. Errors in these chains are accessible via the
/// [`Error::chain`] and [`Error::root_cause`] methods.
///
/// After applying error context of type `C`, calling
/// [`error.is::<C>()`](Error::is) will return `true` for the new error
/// (unless there was a memory allocation failure) in addition to any other
/// types `T` for which it was already the case that `error.is::<T>()`.
///
/// This boxes the inner `C` type, but if that box allocation fails, then this
/// trait's functions return an `Error` where
/// [`error.is::<OutOfMemory>()`](OutOfMemory) is true.
///
/// # Example
///
/// ```
/// # use wasmtime_internal_core::error as wasmtime;
/// use wasmtime::{Context as _, Result};
/// # #[cfg(feature = "backtrace")]
/// # wasmtime_internal_core::error::disable_backtrace();
///
/// fn u32_to_u8(x: u32) -> Result<u8> {
///     let y = u8::try_from(x).with_context(|| {
///         format!("failed to convert `{x}` into a `u8` (max = `{}`)", u8::MAX)
///     })?;
///     Ok(y)
/// }
///
/// let x = u32_to_u8(42).unwrap();
/// assert_eq!(x, 42);
///
/// let error = u32_to_u8(999).unwrap_err();
///
/// // The error is a `String` because of our added context.
/// assert!(error.is::<String>());
/// assert_eq!(
///     error.to_string(),
///     "failed to convert `999` into a `u8` (max = `255`)",
/// );
///
/// // But it is also a `TryFromIntError` because of the inner error.
/// assert!(error.is::<std::num::TryFromIntError>());
/// assert_eq!(
///     error.root_cause().to_string(),
///     "out of range integral type conversion attempted",
/// );
///
/// // The debug output of the error contains the full error chain.
/// assert_eq!(
///     format!("{error:?}").trim(),
///     r#"
/// failed to convert `999` into a `u8` (max = `255`)
///
/// Caused by:
///     out of range integral type conversion attempted
///     "#.trim(),
/// );
/// ```
///
/// # Example with `Option<T>`
///
/// You can also use this trait to create the initial, root-cause `Error` when a
/// fallible function returns an `Option`:
///
/// ```
/// # use wasmtime_internal_core as wasmtime;
/// use wasmtime::error::{Context as _, Result};
///
/// fn try_get<T>(slice: &[T], i: usize) -> Result<&T> {
///     let elem: Option<&T> = slice.get(i);
///     elem.with_context(|| {
///         format!("out of bounds access: index is {i} but length is {}", slice.len())
///     })
/// }
///
/// let arr = [921, 36, 123, 42, 785];
///
/// let x = try_get(&arr, 2).unwrap();
/// assert_eq!(*x, 123);
///
/// let error = try_get(&arr, 9999).unwrap_err();
/// assert_eq!(
///     error.to_string(),
///     "out of bounds access: index is 9999 but length is 5",
/// );
/// ```
pub trait Context<T, E>: sealed::Sealed {
    /// Add additional, already-computed error context to this result.
    ///
    /// Because this method requires that the error context is already computed,
    /// it should only be used when the `context` is already available or is
    /// effectively a constant. Otherwise, it effectively forces computation of
    /// the context, even when we aren't on an error path. The
    /// [`Context::with_context`](Context::with_context) method is
    /// preferred in these scenarios, as it lazily computes the error context,
    /// only doing so when we are actually on an error path.
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static;

    /// Add additional, lazily-computed error context to this result.
    ///
    /// Only invokes `f` to compute the error context when we are actually on an
    /// error path. Does not invoke `f` if we are not on an error path.
    fn with_context<C, F>(self, f: F) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T, E> Context<T, E> for Result<T, E>
where
    E: core::error::Error + Send + Sync + 'static,
{
    #[inline]
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(Error::new(e).context(context)),
        }
    }

    #[inline]
    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(Error::new(e).context(f())),
        }
    }
}

impl<T> Context<T, Error> for Result<T> {
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(e.context(context)),
        }
    }

    fn with_context<C, F>(self, f: F) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(e.context(f())),
        }
    }
}

impl<T> Context<T, core::convert::Infallible> for Option<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        match self {
            Some(x) => Ok(x),
            None => Err(Error::from_error_ext(ContextError {
                context,
                error: None,
            })),
        }
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        match self {
            Some(x) => Ok(x),
            None => Err(Error::from_error_ext(ContextError {
                context: f(),
                error: None,
            })),
        }
    }
}

// NB: The `repr(C)` is required for safety of the `ErrorExt::ext_is`
// implementation and the casts that are performed using that method's
// return value.
#[repr(C)]
pub(crate) struct ContextError<C> {
    // NB: This must be the first field for safety of the `ErrorExt::ext_is`
    // implementation and the casts that are performed using that method's
    // return value.
    pub(crate) context: C,

    pub(crate) error: Option<Error>,
}

impl<C> fmt::Debug for ContextError<C>
where
    C: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<C> fmt::Display for ContextError<C>
where
    C: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.context.fmt(f)
    }
}

impl<C> core::error::Error for ContextError<C>
where
    C: fmt::Display + Send + Sync + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        let source = self.ext_source()?;
        Some(source.as_dyn_core_error())
    }
}

unsafe impl<C> ErrorExt for ContextError<C>
where
    C: fmt::Display + Send + Sync + 'static,
{
    fn ext_as_dyn_core_error(&self) -> &(dyn core::error::Error + Send + Sync + 'static) {
        self
    }

    fn ext_into_boxed_dyn_core_error(
        self,
    ) -> Result<Box<dyn core::error::Error + Send + Sync + 'static>, OutOfMemory> {
        let boxed = try_new_uninit_box()?;
        Ok(Box::write(boxed, self) as _)
    }

    fn ext_source(&self) -> Option<OomOrDynErrorRef<'_>> {
        let error = self.error.as_ref()?;
        Some(error.inner.unpack())
    }

    fn ext_source_mut(&mut self) -> Option<OomOrDynErrorMut<'_>> {
        let error = self.error.as_mut()?;
        Some(error.inner.unpack_mut())
    }

    fn ext_take_source(&mut self) -> Option<OomOrDynError> {
        let error = self.error.take()?;
        Some(error.inner)
    }

    fn ext_is(&self, type_id: TypeId) -> bool {
        // NB: need to check type id of `C`, not `Self` aka
        // `ContextError<C>`.
        type_id == TypeId::of::<C>()
    }

    unsafe fn ext_move(self, to: NonNull<u8>) {
        // Safety: implied by this trait method's contract.
        unsafe {
            to.cast::<C>().write(self.context);
        }
    }

    #[cfg(feature = "backtrace")]
    fn take_backtrace(&mut self) -> Option<std::backtrace::Backtrace> {
        let error = self.error.as_mut()?;
        match error.inner.unpack_mut() {
            OomOrDynErrorMut::Oom(_) => None,
            OomOrDynErrorMut::DynError(mut e) => {
                let r = unsafe { e.as_mut() };
                r.backtrace.take()
            }
        }
    }
}
