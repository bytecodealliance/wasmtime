use super::boxed::try_box;
use super::context::ContextError;
use super::ptr::{MutPtr, OwnedPtr, SharedPtr};
use super::vtable::Vtable;
use crate::{OutOfMemory, Result};
use alloc::boxed::Box;
use core::{
    any::TypeId,
    fmt::{self, Debug},
    iter::FusedIterator,
    mem,
    ptr::NonNull,
};
#[cfg(feature = "backtrace")]
use std::backtrace::{Backtrace, BacktraceStatus};

/// Internal extension trait for errors.
///
/// # Safety
///
/// Implementations must correctly report their type (or a type `T` where `*mut
/// Self` can be cast to `*mut T` and safely accessed) in `ext_is`.
pub(crate) unsafe trait ErrorExt: core::error::Error + Send + Sync + 'static {
    /// Get a shared borrow of the next error in the chain.
    fn ext_source(&self) -> Option<OomOrDynErrorRef<'_>>;

    /// Get an exclusive borrow of the next error in the chain.
    fn ext_source_mut(&mut self) -> Option<OomOrDynErrorMut<'_>>;

    /// Take ownership of the next error in the chain.
    fn ext_take_source(&mut self) -> Option<OomOrDynError>;

    /// Is this error an instance of `T`, where `type_id == TypeId::of::<T>()`?
    ///
    /// # Safety
    ///
    /// Implementations must return `true` only when they are actually a `T`, a
    /// `#[repr(transparent)]` newtype wrapper around a `T`, or a `#[repr(C)]`
    /// struct with a `T` as their first field. Safety relies on this invariant.
    fn ext_is(&self, type_id: TypeId) -> bool;

    /// Move the inner `T` error into the storage referenced by `dest`.
    ///
    /// # Safety
    ///
    /// Callers must ensure that `dest` is valid for writing a `T` to.
    ///
    /// Implementations must ensure that the memory block pointed to by `dest`
    /// contains a valid, initialized `T` upon successful return.
    unsafe fn ext_move(self, dest: NonNull<u8>);

    /// Take the backtrace from this error, if any.
    #[cfg(feature = "backtrace")]
    fn take_backtrace(&mut self) -> Option<Backtrace>;
}

/// Morally a `dyn ErrorExt` trait object that holds its own vtable.
///
/// Must only ever be used via some kind of indirection (pointer, reference,
/// `Box`, etc...) that is punning a `ConcreteError<?>` and never directly as an
/// on-stack value, for example.
///
/// See the docs for `Vtable` for details about why we make our own trait
/// objects.
///
/// XXX: Must have a compatible layout with `ConcreteError<E>`. See the
/// assertions in `BoxedDynError::new` and the
/// `dyn_error_and_concrete_error_layouts_are_compatible` test below.
#[repr(C)]
pub(crate) struct DynError {
    // Safety: this vtable must always be associated with the `E` for the
    // `ConcreteError<E>` that this `DynError` is punning.
    pub(crate) vtable: &'static Vtable,
    #[cfg(feature = "backtrace")]
    pub(crate) backtrace: Option<Backtrace>,
    // error: <?>
}

/// A `dyn ErrorExt` trait object that we know the concrete type of.
///
/// XXX: Must have a compatible layout with `DynError`. See the
/// assertions in `BoxedDynError::new` and the
/// `dyn_error_and_concrete_error_layouts_are_compatible` test below.
#[repr(C)]
pub(crate) struct ConcreteError<E> {
    // Safety: this vtable must always be `E`'s vtable. This is ensured in
    // `BoxDynError::new`.
    pub(crate) vtable: &'static Vtable,
    #[cfg(feature = "backtrace")]
    pub(crate) backtrace: Option<Backtrace>,
    pub(crate) error: E,
}

pub(crate) struct BoxedDynError {
    inner: OwnedPtr<DynError>,
}

// Safety: `BoxedDynError::new` ensures that every concrete error type we make a
// `BoxedDynError` from is `Send`.
unsafe impl Send for BoxedDynError {}

// Safety: `BoxedDynError::new` ensures that every concrete error type we make a
// `BoxedDynError` from is `Sync`.
unsafe impl Sync for BoxedDynError {}

impl Drop for BoxedDynError {
    fn drop(&mut self) {
        let ptr = self.inner.raw_copy();
        // Safety: We own the pointer and it is valid for reading/writing
        // `DynError`s.
        let inner = unsafe { ptr.as_ref() };
        let vtable = inner.vtable;
        // Safety: The vtable is for this pointer's concrete type and the
        // pointer is valid to deallocate because we are passing ownership in.
        unsafe {
            (vtable.drop_and_deallocate)(ptr);
        }
    }
}

impl BoxedDynError {
    #[inline]
    fn new<E>(mut error: E) -> Result<Self, OutOfMemory>
    where
        // NB: This implies `Send + Sync`, which is necessary for safety.
        E: ErrorExt,
    {
        #[cfg(not(feature = "backtrace"))]
        let _ = &mut error;

        // Note: do not use `Option::or_else` here to avoid an extra frame
        // showing up in the backtrace, which would create extra noise for users
        // to mentally filter out.
        #[cfg(feature = "backtrace")]
        let backtrace = match error.take_backtrace() {
            Some(bt) => bt,
            None => crate::backtrace::capture(),
        };

        let error = try_box(ConcreteError {
            vtable: Vtable::of::<E>(),
            #[cfg(feature = "backtrace")]
            backtrace: Some(backtrace),
            error,
        })?;

        // We are going to pun the `ConcreteError<E>` pointer into a `DynError`
        // pointer. Debug assert that their layouts are compatible first.
        #[cfg(debug_assertions)]
        {
            let dyn_size = mem::size_of::<DynError>();
            let concrete_size = mem::size_of::<ConcreteError<E>>();
            assert!(
                dyn_size <= concrete_size,
                "assertion failed: {dyn_size} <= {concrete_size}"
            );

            let dyn_align = mem::align_of::<DynError>();
            let concrete_align = mem::align_of::<ConcreteError<E>>();
            assert!(
                dyn_align <= concrete_align,
                "assertion failed: {dyn_align} <= {concrete_align}"
            );

            let dyn_offset = mem::offset_of!(DynError, vtable);
            let concrete_offset = mem::offset_of!(ConcreteError<E>, vtable);
            assert_eq!(dyn_offset, concrete_offset);

            #[cfg(feature = "backtrace")]
            {
                let dyn_offset = mem::offset_of!(DynError, backtrace);
                let concrete_offset = mem::offset_of!(ConcreteError<E>, backtrace);
                assert_eq!(dyn_offset, concrete_offset);
            }
        }

        let ptr = Box::into_raw(error);
        let ptr = ptr.cast::<DynError>();
        // Safety: `Box::into_raw` always returns a non-null pointer.
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        let ptr = OwnedPtr::new(ptr);
        Ok(Self::from_owned_ptr(ptr))
    }

    fn into_owned_ptr(self) -> OwnedPtr<DynError> {
        let ptr = self.inner.raw_copy();
        mem::forget(self);
        ptr
    }

    fn from_owned_ptr(inner: OwnedPtr<DynError>) -> Self {
        BoxedDynError { inner }
    }
}

/// Wasmtime's universal error type.
///
/// 99% API-compatible with `anyhow::Error` but additionally allows recovery
/// from memory exhaustion (see the [`OutOfMemory`] error).
///
/// `Error` is similar to `Box<dyn core::error::Error + Send + Sync + 'static>`
/// but fits in one word instead of two. Additionally, `Result<(), Error>` also
/// fits in a single word.
///
/// When the `"backtrace"` cargo feature is enabled, `Error` contains a
/// backtrace.
///
/// # Creating an `Error`
///
/// Because `Error` implements `From<E>` for all types `E` that implement
/// `core::error::Error + Send + Sync + 'static`, you don't usually need to
/// explicitly construct an `Error`. When you use `?`-style error propagation,
/// it will automatically get constructed from the root cause error for you.
///
/// Most often when creating an `Error`, you just want to early-exit from the
/// function, returning `Err(...)`. The [`ensure!`][crate::ensure] macro
/// early-returns an error when a condition is not met (similar to how `assert!`
/// panics when a condition is not met) and the [`bail!`][crate::bail] macro
/// early-returns an error unconditionally.
///
/// ```
/// # use wasmtime_internal_error as wasmtime;
/// use wasmtime::{bail, ensure, Result};
///
/// fn my_fallible_function(x: u32) -> Result<()> {
///     // This `ensure!` macro invocation is equivalent to
///     //
///     //     if x % 2 != 0 {
///     //         return Err(...);
///     //     }
///     ensure!(x % 2 == 0, "{x} is not even!");
///
///     // This `bail!` macro invocation is equivalent to
///     //
///     //     return Err(...);
///     bail!("oops, another error! {x}")
/// }
/// ```
///
/// If you do not want to early-return, just to create the `Error`, then the
/// [`anyhow!`][crate::anyhow] macro is preferred:
///
/// ```
/// # use wasmtime_internal_error as wasmtime;
/// use wasmtime::{anyhow, Error};
///
/// let x = 42;
/// let my_error: Error = anyhow!("whoops! {x}");
/// ```
///
/// If, however, you happen to require a constructor function instead of a
/// macro, you can use either [`Error::new`] or [`Error::msg`]:
///
/// ```
/// # use wasmtime_internal_error as wasmtime;
/// use wasmtime::Error;
///
/// let messages = ["yikes", "uh oh", "ouch"];
/// let errors = messages
///     .into_iter()
///     .map(Error::msg)
///     .collect::<Vec<_>>();
/// ```
///
/// # Printing an `Error`
///
/// Different format strings will print an `Error` differently:
///
/// * `{}`: Prints the `Display` of just the first error, without any of the
///   other errors in the chain or the root cause.
///
/// * `{:#}`: Prints the `Display` of the first error, then (if there are more
///   errors in the chain) a colon, then the display of the second error in the
///   chain, etc...
///
/// * `{:?}`: Prints the `Display` of the first error, then (if there are more
///   errors in the chain) a newline-separated list of the rest of the errors in
///   the chain, and finally (if the `"backtrace"` cargo feature is enabled, the
///   `RUST_BACKTRACE` environment variable is set and non-zero, and the
///   platform is supported by Rust's standard library's `Backtrace` type) the
///   captured backtrace is printed.
///
///   This is the default formatting used when `fn main() ->
///   wasmtime::Result<()>` returns an error.
///
/// * `{:#?}`: Prints an internal, debugging representation of the `Error`. We
///   make no guarantees about its stability.
///
/// Here is an example showing the different formats for the same error:
///
/// ```
/// # fn _foo() {
/// #![cfg(all(feature = "backtrace", not(miri)))]
/// # let _ = unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
/// # use wasmtime_internal_error as wasmtime;
/// use wasmtime::{bail, Context as _, Result};
///
/// fn uno() -> Result<()> {
///     bail!("ouch")
/// }
///
/// fn dos() -> Result<()> {
///     uno().context("whoops")
/// }
///
/// fn tres() -> Result<()> {
///     dos().context("uh oh")
/// }
///
/// let error = tres().unwrap_err();
///
/// println!("{error}");
/// // Prints:
/// //
/// //     uh oh
///
/// println!("{error:#}");
/// // Prints:
/// //
/// //     uh oh: whoops: ouch
///
/// println!("{error:?}");
/// // Prints
/// //
/// //     uh oh
/// //
/// //     Caused by:
/// //         0: whoops
/// //         1: ouch
/// //
/// //     Stack backtrace:
/// //       <...>
/// //        7: example::uno
/// //        8: example::dos
/// //        9: example::tres
/// //       10: example::main
/// //       <...>
///
/// println!("{error:#?}");
/// // Prints
/// //
/// //     Error {
/// //         <...>
/// //     }
/// # }
/// ```
///
/// # Converting a `wasmtime::Error` into an `anyhow::Error`
///
/// When the `"anyhow"` feature is enabled, there is a `From<wasmtime::Error>
/// for anyhow::Error` implementation. You can always call that implementation
/// explicitly if needed, but `?`-propagation allows the conversion to happen
/// seamlessly from functions that return a `Result<T, wasmtime::Error>` to
/// those that return a `Result<U, anyhow::Error>`.
///
/// ```
/// # fn _foo() {
/// #![cfg(feature = "anyhow")]
/// # use wasmtime_internal_error as wasmtime;
///
/// fn foo() -> Result<(), wasmtime::Error> {
///     wasmtime::bail!("decontamination failure")
/// }
///
/// fn bar() -> Result<(), anyhow::Error> {
///     foo()?; // `?` is auto-converting here!
///     Ok(())
/// }
///
/// let error = bar().unwrap_err();
/// assert_eq!(error.to_string(), "decontamination failure");
/// # }
/// ```
///
/// # Converting an `anyhow::Error` into a `wasmtime::Error`
///
/// When the `"anyhow"` feature is enabled, there is an `Error::from_anyhow`
/// constructor that you may use to convert an `anyhow::Error` into a
/// `wasmtime::Error`. (Unfortunately trait coherence does not allow us a
/// `From<anyhow::Error> for wasmtime::Error` implementation.) This will
/// most-often be used in combination with `Result::map_err`:
///
/// ```
/// # fn _foo() {
/// #![cfg(feature = "anyhow")]
/// # use wasmtime_internal_error as wasmtime;
///
/// fn baz() -> Result<(), anyhow::Error> {
///     anyhow::bail!("oops I ate worms")
/// }
///
/// fn qux() -> Result<(), wasmtime::Error> {
///     baz().map_err(wasmtime::Error::from_anyhow)?;
///     Ok(())
/// }
///
/// let error = qux().unwrap_err();
/// assert_eq!(error.to_string(), "oops I ate worms");
/// # }
/// ```
pub struct Error {
    pub(crate) inner: OomOrDynError,
}

/// For performance, it is important that `Error` and `Result<()>` fit in a
/// single word so that they can be passed in registers by rustc/llvm, rather
/// than on the stack, when used as a function's return type.
const _ERROR_IS_ONE_WORD_LARGE: () = assert!(mem::size_of::<Error>() == mem::size_of::<usize>());
const _RESULT_OF_UNIT_IS_ONE_WORD_LARGE: () =
    assert!(mem::size_of::<Result<()>>() == mem::size_of::<usize>());

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return f
                .debug_struct("Error")
                .field("inner", &self.inner.unpack())
                .finish();
        }

        let inner = self.inner.unpack();
        inner.display(f)?;

        if let Some(source) = inner.source() {
            f.write_str("\n\nCaused by:\n")?;
            for (i, e) in Chain::new(source).enumerate() {
                writeln!(f, "\t{i}: {e}")?;
            }
        }

        #[cfg(feature = "backtrace")]
        {
            let backtrace = inner.backtrace();
            if let BacktraceStatus::Captured = backtrace.status() {
                f.write_str("\nStack backtrace:\n")?;
                fmt::Display::fmt(backtrace, f)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.unpack();
        inner.display(f)?;

        if f.alternate() {
            if let Some(e) = inner.source() {
                for e in Chain::new(e) {
                    write!(f, ": {e}")?;
                }
            }
        }

        Ok(())
    }
}

impl<E> From<E> for Error
where
    E: core::error::Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self::new(error)
    }
}

impl From<Error> for Box<dyn core::error::Error + Send + Sync + 'static> {
    #[inline]
    fn from(error: Error) -> Self {
        error.into_boxed_dyn_error()
    }
}

/// Convert a [`Error`] into an [`anyhow::Error`].
///
/// # Example
///
/// ```
/// # use wasmtime_internal_error as wasmtime;
/// let wasmtime_error = wasmtime::Error::msg("whoops");
/// let anyhow_error = anyhow::Error::from(wasmtime_error);
/// ```
//
// Unfortunately, we can't also implement `From<anyhow::Error> for Error`
// because of trait coherence. From Rust's trait system's point of view,
// `anyhow` could theoretically add an `core::error::Error for anyhow::Error`
// implementation, which would make our desired `From<anyhow::Error>`
// implementation conflict with our existing `From<E: core::error::Error>`
// implementation. They cannot in fact add that implementation, however, because
// they already have a `From<E: core::error::Error> for anyhow::Error`
// implementation and so adding `core::error::Error for anyhow::Error` would
// cause that impl to conflict with `From<T> for T` (which is the same reason we
// cannot implement `core::error::Error for Error`). Nonetheless, our hands are
// tied here.
#[cfg(feature = "anyhow")]
impl From<Error> for anyhow::Error {
    #[inline]
    fn from(e: Error) -> Self {
        anyhow::Error::from_boxed(e.into_boxed_dyn_error())
    }
}

impl Error {
    /// Construct a new `Error` from a type that implements
    /// `core::error::Error`.
    ///
    /// Calling [`error.is::<E>()`][Error::is] will return `true` for the new
    /// error (unless there was a memory allocation failure).
    ///
    /// This boxes the inner error, but if that box allocation fails, then this
    /// function returns an `Error` where
    /// [`error.is::<OutOfMemory>()`][crate::OutOfMemory] is true.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::Error;
    ///
    /// let error = Error::new(std::fmt::Error);
    /// ```
    pub fn new<E>(error: E) -> Self
    where
        E: core::error::Error + Send + Sync + 'static,
    {
        if TypeId::of::<E>() == TypeId::of::<OutOfMemory>() {
            return Error {
                inner: OutOfMemory::new().into(),
            };
        }

        Self::from_error_ext(ForeignError(error))
    }

    /// Construct a new `Error` from any type that implements `Debug` and
    /// `Display`.
    ///
    /// Calling [`error.is::<M>()`][Error::is] will return `true` for the new
    /// error (unless there was a memory allocation failure).
    ///
    /// This boxes the inner `M` type, but if that box allocation fails, then
    /// this function returns an `Error` where
    /// [`error.is::<OutOfMemory>()`][crate::OutOfMemory] is true.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::Error;
    ///
    /// let error = Error::msg("hello");
    /// ```
    pub fn msg<M>(message: M) -> Self
    where
        M: fmt::Debug + fmt::Display + Send + Sync + 'static,
    {
        Self::from_error_ext(MessageError(message))
    }

    /// Create an `Error` from a `Box<dyn core::error::Error>`.
    ///
    /// This is useful when converting errors from other universal-error
    /// libraries into this crate's `Error` type. Prefer [`Error::from_anyhow`]
    /// for converting `anyhow::Error`s into `Error`s, as that preserves
    /// `error.is::<anyhow::Error>()`.
    ///
    /// Calling [`error.is::<Box<dyn core::error::Error + Send + Sync +
    /// 'static>>()`][Error::is] will return `true` for the new error (unless
    /// there was a memory allocation failure).
    ///
    /// This reboxes the inner error, but if that box allocation fails, then
    /// this function returns an `Error` where
    /// [`error.is::<OutOfMemory>()`][crate::OutOfMemory] is true.
    ///
    /// # Example
    ///
    /// ```
    /// # fn _foo() {
    /// #![cfg(all(feature = "std", feature = "anyhow"))]
    /// # use wasmtime_internal_error as wasmtime;
    /// use std::error::Error;
    ///
    /// let anyhow_error = anyhow::Error::msg("whoops");
    /// let boxed_error: Box<dyn Error + Send + Sync + 'static> = anyhow_error.into_boxed_dyn_error();
    /// let wasmtime_error = wasmtime::Error::from_boxed(boxed_error);
    /// # }
    /// ```
    pub fn from_boxed(error: Box<dyn core::error::Error + Send + Sync + 'static>) -> Self {
        Self::from_error_ext(BoxedError(error))
    }

    /// Convert an `anyhow::Error` into an `Error`.
    ///
    /// Calling [`error.is::<anyhow::Error>()`][Error::is] will return `true`
    /// for the new error (unless there was a memory allocation failure).
    ///
    /// This reboxes the `anyhow::Error`, but if that box allocation fails, then
    /// this function returns an `Error` where
    /// [`error.is::<OutOfMemory>()`][crate::OutOfMemory] is true.
    ///
    /// # Example
    ///
    /// ```
    /// # fn _foo() {
    /// #![cfg(all(feature = "std", feature = "anyhow"))]
    /// # use wasmtime_internal_error as wasmtime;
    /// let anyhow_error = anyhow::Error::msg("failed to flim the flam");
    /// let wasmtime_error = wasmtime::Error::from_anyhow(anyhow_error);
    /// assert_eq!(
    ///     wasmtime_error.to_string(),
    ///     "failed to flim the flam",
    /// );
    /// # }
    /// ```
    #[cfg(feature = "anyhow")]
    #[inline]
    pub fn from_anyhow(error: anyhow::Error) -> Self {
        Self::from_error_ext(AnyhowError(error))
    }

    /// Add additional context to this error.
    ///
    /// The new context will show up first in the error chain, and the original
    /// error will come next.
    ///
    /// This is similar to the [`Context::context`] trait method, but because it
    /// is a method directly on [`Error`], there is no need for lazily-computing
    /// the error context (like `with_context` does).
    ///
    /// Calling [`error.is::<C>()`][Error::is] will return `true` for the new
    /// error (unless there was a memory allocation failure) in addition to any
    /// other types `T` for which it was already the case that
    /// `error.is::<T>()`.
    ///
    /// This boxes the inner `C` type, but if that box allocation fails, then
    /// this function returns an `Error` where
    /// [`error.is::<OutOfMemory>()`][crate::OutOfMemory] is true.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::Error;
    ///
    /// let error = Error::msg("root cause");
    /// let error = error.context("failed to bonkinate");
    /// let error = error.context("cannot frob the blobbins");
    ///
    /// assert!(
    ///     format!("{error:?}").contains(
    ///         r#"
    /// cannot frob the blobbins
    ///
    /// Caused by:
    /// 	0: failed to bonkinate
    /// 	1: root cause
    ///         "#.trim(),
    ///     ),
    /// );
    /// ```
    pub fn context<C>(self, context: C) -> Self
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        if self.inner.is_oom() {
            self
        } else {
            Self::from_error_ext(ContextError {
                context,
                error: Some(self),
            })
        }
    }

    #[inline]
    pub(crate) fn from_error_ext(error: impl ErrorExt) -> Self {
        match BoxedDynError::new(error) {
            Ok(boxed) => Error {
                inner: boxed.into(),
            },
            Err(oom) => out_of_line_slow_path!(Error { inner: oom.into() }),
        }
    }

    /// Get this error's backtrace.
    ///
    /// Backtraces will be automatically captured on initial `Error` creation
    /// when all of the following conditions are met:
    ///
    /// * This crate's `"backtrace"` cargo feature is enabled
    /// * Rust's `std::backtrace::Backtrace` supports the platform
    /// * The `RUST_BACKTRACE` or `RUST_LIB_BACKTRACE` environment variables
    ///   are set and non-zero
    ///
    /// See [the `std::backtrace::Backtrace`
    /// documentation](https://doc.rust-lang.org/stable/std/backtrace/struct.Backtrace.html)
    /// for more details on backtraces.
    ///
    /// Note that `std::backtrace::Backtrace` does not provide a
    /// fallible-capture mechanism that returns an error, rather than aborting
    /// the process, when it encounters memory exhaustion. If you require
    /// out-of-memory error handling, do not enable this crate's `"backtrace"`
    /// cargo feature.
    ///
    /// # Example
    ///
    /// ```
    /// # fn _foo() {
    /// #![cfg(feature = "backtrace")]
    /// # use wasmtime_internal_error as wasmtime;
    /// use std::backtrace::BacktraceStatus;
    /// use wasmtime::Error;
    ///
    /// let error = Error::msg("whoops");
    ///
    /// let backtrace = error.backtrace();
    /// if let BacktraceStatus::Captured = backtrace.status() {
    ///     println!("error backtrace is:\n{backtrace}");
    /// }
    /// # }
    /// ```
    #[inline]
    #[cfg(feature = "backtrace")]
    pub fn backtrace(&self) -> &Backtrace {
        self.inner.unpack().backtrace()
    }

    /// Iterate over this error's context chain.
    ///
    /// The iterator yields `&(dyn core::error::Error + 'static)` items.
    ///
    /// Iterates from the most recently added error context towards the root
    /// cause.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::Error;
    ///
    /// let error = Error::msg("root cause");
    /// let error = error.context("failed to reticulate splines");
    /// let error = error.context("aborting launch");
    ///
    /// let messages: Vec<_> = error.chain().map(|e| e.to_string()).collect();
    /// assert_eq!(
    ///     messages,
    ///     ["aborting launch", "failed to reticulate splines", "root cause"],
    /// );
    /// ```
    #[inline]
    pub fn chain(&self) -> Chain<'_> {
        Chain::new(self.inner.unpack())
    }

    /// Get the last error in the context chain.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::Error;
    ///
    /// let error = Error::msg("ghosts");
    /// let error = error.context("failed to reticulate splines");
    /// let error = error.context("aborting launch");
    ///
    /// assert_eq!(
    ///     error.root_cause().to_string(),
    ///     "ghosts",
    /// );
    /// ```
    #[inline]
    pub fn root_cause(&self) -> &(dyn core::error::Error + 'static) {
        self.chain().last().expect("chain is always non-empty")
    }

    /// Is this an `E` error?
    ///
    /// Returns true if any error in the context chain is an `E`.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::{Error, OutOfMemory};
    ///
    /// let oom = Error::from(OutOfMemory::new());
    /// assert!(oom.is::<OutOfMemory>());
    /// assert!(!oom.is::<std::num::TryFromIntError>());
    ///
    /// // Here is an example with additional error context.
    /// let error = Error::from(u8::try_from(u32::MAX).unwrap_err());
    /// let error = error.context(format!("cannot convert {} into a u8", u32::MAX));
    /// assert!(
    ///     error.is::<std::num::TryFromIntError>(),
    ///     "root cause is an int conversion failure",
    /// );
    /// assert!(
    ///     error.is::<String>(),
    ///     "additional context is a `String`",
    /// );
    /// assert!(
    ///     !error.is::<OutOfMemory>(),
    ///     "no error in the chain is an out-of-memory error",
    /// );
    /// ```
    pub fn is<E>(&self) -> bool
    where
        E: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        let mut error = Some(self.inner.unpack());
        while let Some(e) = error {
            if e.is::<E>() {
                return true;
            } else {
                error = e.source();
            }
        }
        false
    }

    /// Downcast this error into an `E`, taking ownership.
    ///
    /// If this error is an `E`, then `Ok(E)` is returned. Otherwise,
    /// `Err(self)` is returned.
    ///
    /// If there are multiple instances of `E` in this error's chain, then the
    /// first (as encountered by [`Error::chain`]'s iteration order) is
    /// returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::{Error, OutOfMemory};
    ///
    /// let error = Error::msg("whoops");
    ///
    /// // `error` is not an `OutOfMemory`.
    /// let downcasted = error.downcast::<OutOfMemory>();
    /// assert!(downcasted.is_err());
    ///
    /// // Get the original `error` back.
    /// let error = downcasted.unwrap_err();
    ///
    /// // `error` is an `&str`.
    /// let downcasted = error.downcast::<&str>();
    /// assert!(downcasted.is_ok());
    /// assert_eq!(downcasted.unwrap(), "whoops");
    ///
    /// // If there are multiple `E`s in the chain, the first in the chain is
    /// // returned.
    /// let error = Error::msg("root cause");
    /// let error = error.context("failed to recombobulate");
    /// assert_eq!(
    ///     error.downcast::<&str>().unwrap(),
    ///     "failed to recombobulate",
    /// );
    /// ```
    pub fn downcast<E>(self) -> Result<E, Self>
    where
        E: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        if !self.is::<E>() {
            return Err(self);
        }

        let mut value = mem::MaybeUninit::<E>::uninit();

        // Safety: this error is an `E` and the given pointer is valid to write
        // an `E` to.
        unsafe {
            self.inner
                .downcast(TypeId::of::<E>(), NonNull::from(&mut value).cast::<u8>());
        }

        // Safety: `OomOrDynError::downcast` guarantees that the given pointer's
        // data is initialized upon successful return.
        Ok(unsafe { value.assume_init() })
    }

    /// Downcast this error into a shared `&E` borrow.
    ///
    /// If this error is an `E`, then `Some(&E)` is returned. Otherwise, `None`
    /// is returned.
    ///
    /// If there are multiple instances of `E` in this error's chain, then the
    /// first (as encountered by [`Error::chain`]'s iteration order) is
    /// returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::{Error, OutOfMemory};
    ///
    /// let error = Error::msg("whoops");
    ///
    /// // `error` is not an `OutOfMemory`.
    /// assert!(error.downcast_ref::<OutOfMemory>().is_none());
    ///
    /// // `error` is an `&str`.
    /// assert!(error.downcast_ref::<&str>().is_some());
    /// assert_eq!(*error.downcast_ref::<&str>().unwrap(), "whoops");
    ///
    /// // If there are multiple `E`s in the chain, the first in the chain is
    /// // returned.
    /// let error = Error::msg("root cause");
    /// let error = error.context("failed to recombobulate");
    /// assert_eq!(
    ///     *error.downcast_ref::<&str>().unwrap(),
    ///     "failed to recombobulate",
    /// );
    /// ```
    pub fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        let mut error = Some(self.inner.unpack());
        while let Some(e) = error {
            if e.is::<E>() {
                return Some(match e {
                    OomOrDynErrorRef::DynError(ptr) => {
                        let ptr = ptr.cast::<ConcreteError<E>>();
                        // Safety: we own the pointer, it is valid for reading,
                        // and we checked that it is an `E`.
                        let r = unsafe { ptr.as_ref() };
                        &r.error
                    }
                    OomOrDynErrorRef::Oom(oom) => {
                        // Note: Even though we know that `E == OutOfMemory`
                        // here, we still have to do this dance to satisfy the
                        // type system.
                        debug_assert_eq!(TypeId::of::<E>(), TypeId::of::<OutOfMemory>());
                        let ptr = NonNull::from(oom);
                        let ptr = ptr.cast::<E>();
                        // Safety: the pointer points to `oom`, which is valid
                        // for creating a shared reference to.
                        unsafe { ptr.as_ref() }
                    }
                });
            } else {
                error = e.source();
            }
        }
        None
    }

    /// Downcast this error into an exclusive `&mut E` borrow.
    ///
    /// If this error is an `E`, then `Some(&mut E)` is returned. Otherwise,
    /// `None` is returned.
    ///
    /// If there are multiple instances of `E` in this error's chain, then the
    /// first (as encountered by [`Error::chain`]'s iteration order) is
    /// returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::{Error, OutOfMemory};
    ///
    /// let mut error = Error::msg("whoops");
    ///
    /// // `error` is not an `OutOfMemory`.
    /// assert!(error.downcast_mut::<OutOfMemory>().is_none());
    ///
    /// // `error` is an `&str`.
    /// assert!(error.downcast_mut::<&str>().is_some());
    /// assert_eq!(*error.downcast_mut::<&str>().unwrap(), "whoops");
    /// *error.downcast_mut::<&str>().unwrap() = "yikes";
    /// assert_eq!(*error.downcast_mut::<&str>().unwrap(), "yikes");
    ///
    /// // If there are multiple `E`s in the chain, the first in the chain is
    /// // returned.
    /// let error = Error::msg("root cause");
    /// let mut error = error.context("failed to recombobulate");
    /// assert_eq!(
    ///     *error.downcast_mut::<&str>().unwrap(),
    ///     "failed to recombobulate",
    /// );
    /// ```
    pub fn downcast_mut<E>(&mut self) -> Option<&mut E>
    where
        E: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        let mut error = Some(self.inner.unpack_mut());
        while let Some(mut e) = error.take() {
            if e.as_ref().is::<E>() {
                return Some(match e {
                    OomOrDynErrorMut::DynError(ptr) => {
                        let mut ptr = ptr.cast::<ConcreteError<E>>();
                        // Safety: we own the pointer, it is valid for reading
                        // and writing, and we checked that it is an `E`.
                        let r = unsafe { ptr.as_mut() };
                        &mut r.error
                    }
                    OomOrDynErrorMut::Oom(oom) => {
                        // Note: Even though we know that `E == OutOfMemory`
                        // here, we still have to do this dance to satisfy the
                        // type system.
                        debug_assert_eq!(TypeId::of::<E>(), TypeId::of::<OutOfMemory>());
                        let ptr = NonNull::from(oom);
                        let mut ptr = ptr.cast::<E>();
                        // Safety: the pointer points to `oom`, which is valid
                        // for creating an exclusive reference to.
                        unsafe { ptr.as_mut() }
                    }
                });
            } else {
                error = e.source_mut();
            }
        }
        None
    }

    /// Convert this error into a `Box<dyn core::error::Error>`.
    ///
    /// This is useful for integrating this crate's `Error`s into other
    /// universal-error libraries.
    ///
    /// This functionality is also available via a `From<Error> for Box<dyn
    /// core::error::Error + Send + Sync + 'static>>` implementation.
    ///
    /// # Example
    ///
    /// ```
    /// # fn _foo() {
    /// #![cfg(feature = "std")]
    /// use std::fmt;
    ///
    /// /// A stub representing some other error library.
    /// #[derive(Debug)]
    /// pub struct OtherError {
    ///     inner: Box<dyn std::error::Error + Send + Sync + 'static>,
    /// }
    ///
    /// impl fmt::Display for OtherError {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         fmt::Display::fmt(&self.inner, f)
    ///     }
    /// }
    ///
    /// impl std::error::Error for OtherError {
    ///     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    ///         self.inner.source()
    ///     }
    /// }
    ///
    /// impl OtherError {
    ///     /// Create an `OtherError` from another error.
    ///     pub fn new<E>(error: E) -> Self
    ///     where
    ///         E: std::error::Error + Send + Sync + 'static,
    ///     {
    ///         OtherError { inner: Box::new(error) }
    ///     }
    ///
    ///     /// Create an `OtherError` from another, already-boxed error.
    ///     pub fn from_boxed(error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
    ///         OtherError { inner: error }
    ///     }
    /// }
    ///
    /// # use wasmtime_internal_error as wasmtime;
    /// use wasmtime::Error;
    ///
    /// // Create an `Error`.
    /// let error = Error::msg("whoopsies");
    ///
    /// // Convert it into an `OtherError`.
    /// let error = OtherError::from_boxed(error.into_boxed_dyn_error());
    /// # }
    /// ```
    #[inline]
    pub fn into_boxed_dyn_error(self) -> Box<dyn core::error::Error + Send + Sync + 'static> {
        self.inner.into_boxed_dyn_core_error()
    }
}

/// `ErrorExt` wrapper for foreign `core::error::Error` implementations.
///
/// For `Error::new`'s use only.
///
/// NB: The `repr(transparent)` is required for safety of the `ErrorExt::ext_is`
/// implementation and the casts that are performed using that method's return
/// value.
#[repr(transparent)]
struct ForeignError<E>(E);

impl<E> fmt::Debug for ForeignError<E>
where
    E: core::error::Error + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<E> fmt::Display for ForeignError<E>
where
    E: core::error::Error + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<E> core::error::Error for ForeignError<E>
where
    E: core::error::Error + Send + Sync + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.0.source()
    }
}

// Safety: `ext_is` is correct, `ext_move` always writes to `dest`.
unsafe impl<E> ErrorExt for ForeignError<E>
where
    E: core::error::Error + Send + Sync + 'static,
{
    fn ext_source(&self) -> Option<OomOrDynErrorRef<'_>> {
        None
    }

    fn ext_source_mut(&mut self) -> Option<OomOrDynErrorMut<'_>> {
        None
    }

    fn ext_take_source(&mut self) -> Option<OomOrDynError> {
        None
    }

    unsafe fn ext_move(self, dest: NonNull<u8>) {
        // Safety: implied by this trait method's safety contract.
        unsafe {
            dest.cast::<E>().write(self.0);
        }
    }

    fn ext_is(&self, type_id: TypeId) -> bool {
        // NB: need to check type id of `E`, not `Self` aka
        // `ForeignError<E>`.
        type_id == TypeId::of::<E>()
    }

    #[cfg(feature = "backtrace")]
    fn take_backtrace(&mut self) -> Option<Backtrace> {
        None
    }
}

/// `ErrorExt` wrapper for types given to `Error::msg`.
///
/// For `Error::msg`'s use only.
///
/// NB: The `repr(transparent)` is required for safety of the `ErrorExt::ext_is`
/// implementation and the casts that are performed using that method's return
/// value.
#[repr(transparent)]
struct MessageError<M>(M);

impl<M> fmt::Debug for MessageError<M>
where
    M: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<M> fmt::Display for MessageError<M>
where
    M: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<M> core::error::Error for MessageError<M> where M: fmt::Debug + fmt::Display {}

// Safety: `ext_is` is implemented correctly and `ext_move` always
// writes to its pointer.
unsafe impl<M> ErrorExt for MessageError<M>
where
    M: fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    fn ext_source(&self) -> Option<OomOrDynErrorRef<'_>> {
        None
    }

    fn ext_source_mut(&mut self) -> Option<OomOrDynErrorMut<'_>> {
        None
    }

    fn ext_take_source(&mut self) -> Option<OomOrDynError> {
        None
    }

    fn ext_is(&self, type_id: TypeId) -> bool {
        // NB: need to check type id of `M`, not `Self` aka
        // `MessageError<M>`.
        type_id == TypeId::of::<M>()
    }

    unsafe fn ext_move(self, dest: NonNull<u8>) {
        // Safety: implied by this trait method's contract.
        unsafe {
            dest.cast::<M>().write(self.0);
        }
    }

    #[cfg(feature = "backtrace")]
    fn take_backtrace(&mut self) -> Option<Backtrace> {
        None
    }
}

/// `ErrorExt` wrapper for `Box<dyn core::error::Error>`.
///
/// For `Error::from_boxed`'s use only.
///
/// NB: The `repr(transparent)` is required for safety of the `ErrorExt::ext_is`
/// implementation and the casts that are performed using that method's return
/// value.
#[repr(transparent)]
struct BoxedError(Box<dyn core::error::Error + Send + Sync + 'static>);

impl fmt::Debug for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl core::error::Error for BoxedError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.0.source()
    }
}

// Safety: `ext_is` is implemented correctly and `ext_move` always
// writes to its pointer.
unsafe impl ErrorExt for BoxedError {
    fn ext_source(&self) -> Option<OomOrDynErrorRef<'_>> {
        None
    }

    fn ext_source_mut(&mut self) -> Option<OomOrDynErrorMut<'_>> {
        None
    }

    fn ext_take_source(&mut self) -> Option<OomOrDynError> {
        None
    }

    fn ext_is(&self, type_id: TypeId) -> bool {
        // NB: need to check type id of `BoxDynSendSyncError`, not
        // `BoxedError`.
        type_id == TypeId::of::<Box<dyn core::error::Error + Send + Sync + 'static>>()
    }

    unsafe fn ext_move(self, dest: NonNull<u8>) {
        // Safety: implied by this trait method's contract.
        unsafe {
            dest.cast::<Box<dyn core::error::Error + Send + Sync + 'static>>()
                .write(self.0);
        }
    }

    #[cfg(feature = "backtrace")]
    fn take_backtrace(&mut self) -> Option<Backtrace> {
        None
    }
}

/// `ErrorExt` wrapper for `anyhow::Error`.
///
/// For `Error::from_anyhow`'s use only.
///
/// NB: The `repr(transparent)` is required for safety of the `ErrorExt::ext_is`
/// implementation and the casts that are performed using that method's return
/// value.
#[repr(transparent)]
#[cfg(feature = "anyhow")]
struct AnyhowError(anyhow::Error);

#[cfg(feature = "anyhow")]
impl fmt::Debug for AnyhowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

#[cfg(feature = "anyhow")]
impl fmt::Display for AnyhowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "anyhow")]
impl core::error::Error for AnyhowError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.0.source()
    }
}

// Safety: `ext_is` is implemented correctly and `ext_move` always
// writes to its pointer.
#[cfg(feature = "anyhow")]
unsafe impl ErrorExt for AnyhowError {
    fn ext_source(&self) -> Option<OomOrDynErrorRef<'_>> {
        None
    }

    fn ext_source_mut(&mut self) -> Option<OomOrDynErrorMut<'_>> {
        None
    }

    fn ext_take_source(&mut self) -> Option<OomOrDynError> {
        None
    }

    fn ext_is(&self, type_id: TypeId) -> bool {
        // NB: need to check type id of `BoxDynSendSyncError`, not
        // `AnyhowError`.
        type_id == TypeId::of::<anyhow::Error>()
    }

    unsafe fn ext_move(self, dest: NonNull<u8>) {
        // Safety: implied by this trait method's contract.
        unsafe {
            dest.cast::<anyhow::Error>().write(self.0);
        }
    }

    #[cfg(feature = "backtrace")]
    fn take_backtrace(&mut self) -> Option<Backtrace> {
        None
    }
}

pub(crate) enum OomOrDynErrorRef<'a> {
    // Safety: this must always be a valid pointer to read a `DynError` from for
    // the `'a` lifetime.
    DynError(SharedPtr<'a, DynError>),

    Oom(&'a OutOfMemory),
}

impl<'a> Debug for OomOrDynErrorRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

impl<'a> OomOrDynErrorRef<'a> {
    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OomOrDynErrorRef::DynError(e) => {
                // Safety: invariant of this type.
                let vtable = unsafe { e.as_ref().vtable };
                // Safety: using the vtable associated with this pointer's
                // concrete type and the pointer is valid.
                unsafe { (vtable.display)(*e, f) }
            }
            OomOrDynErrorRef::Oom(oom) => fmt::Display::fmt(oom, f),
        }
    }

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            OomOrDynErrorRef::Oom(oom) => f.debug_tuple("Oom").field(oom).finish(),
            OomOrDynErrorRef::DynError(error) => {
                struct DebugError<'a>(SharedPtr<'a, DynError>);
                impl fmt::Debug for DebugError<'_> {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        // Safety: invariant of `OomOrDynError` that the pointer
                        // is valid.
                        let vtable = unsafe { self.0.as_ref().vtable };
                        // Safety: the pointer is valid and the vtable is
                        // associated with the pointer's concrete error type.
                        unsafe { (vtable.debug)(self.0, f) }
                    }
                }

                let mut f = f.debug_struct("DynError");
                f.field("error", &DebugError(error));
                if let Some(source) = self.source() {
                    f.field("source", &source);
                }
                f.finish()
            }
        }
    }

    fn source(&self) -> Option<OomOrDynErrorRef<'a>> {
        match self {
            OomOrDynErrorRef::DynError(e) => {
                // Safety: invariant of this type.
                let vtable = unsafe { e.as_ref().vtable };
                // Safety: using the vtable associated with this pointer's
                // concrete type and the pointer is valid.
                unsafe { (vtable.source)(*e) }
            }
            OomOrDynErrorRef::Oom(_) => None,
        }
    }

    fn is<E>(&self) -> bool
    where
        E: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            OomOrDynErrorRef::DynError(e) => {
                // Safety: invariant of this type.
                let vtable = unsafe { e.as_ref().vtable };
                // Safety: using the vtable associated with this pointer's
                // concrete type and the pointer is valid.
                unsafe { (vtable.is)(*e, TypeId::of::<E>()) }
            }
            OomOrDynErrorRef::Oom(_) => TypeId::of::<E>() == TypeId::of::<OutOfMemory>(),
        }
    }

    pub(crate) fn as_dyn_core_error(&self) -> &'a (dyn core::error::Error + Send + Sync + 'static) {
        match *self {
            OomOrDynErrorRef::DynError(e) => {
                // Safety: invariant of this type.
                let vtable = unsafe { e.as_ref().vtable };
                // Safety: using the vtable associated with this pointer's
                // concrete type and the pointer is valid.
                unsafe { (vtable.as_dyn_core_error)(e) }
            }
            OomOrDynErrorRef::Oom(oom) => oom as _,
        }
    }

    #[cfg(feature = "backtrace")]
    fn backtrace(&self) -> &'a Backtrace {
        match self {
            OomOrDynErrorRef::DynError(e) => {
                // Safety: invariant of this type.
                let r = unsafe { e.as_ref() };
                r.backtrace
                    .as_ref()
                    .expect("the first error in the chain always has the backtrace")
            }

            OomOrDynErrorRef::Oom(_) => {
                static DISABLED: Backtrace = Backtrace::disabled();
                &DISABLED
            }
        }
    }
}

pub(crate) enum OomOrDynErrorMut<'a> {
    // Safety: this must always be a valid pointer to read and write a
    // `DynError` from for the `'a` lifetime.
    DynError(MutPtr<'a, DynError>),

    Oom(&'a mut OutOfMemory),
}

impl<'a> OomOrDynErrorMut<'a> {
    fn as_ref(&self) -> OomOrDynErrorRef<'_> {
        match self {
            OomOrDynErrorMut::DynError(e) => OomOrDynErrorRef::DynError(e.as_shared_ptr()),
            OomOrDynErrorMut::Oom(oom) => OomOrDynErrorRef::Oom(oom),
        }
    }

    fn source_mut(&mut self) -> Option<OomOrDynErrorMut<'a>> {
        match self {
            OomOrDynErrorMut::DynError(e) => {
                // Safety: invariant of this type.
                let vtable = unsafe { e.as_ref().vtable };
                // Safety: using the vtable associated with this pointer's
                // concrete type and the pointer is valid.
                unsafe { (vtable.source_mut)(e.raw_copy()) }
            }
            OomOrDynErrorMut::Oom(_) => None,
        }
    }
}

/// Bit packed version of `enum { BoxedDynError, OutOfMemory }` that relies on
/// implicit pointer tagging and `OutOfMemory` being zero-sized.
pub(crate) struct OomOrDynError {
    // Safety: this must always be the casted-to-`u8` version of either (a)
    // `OutOfMemory`'s dangling pointer, or (b) a valid, owned `DynError`
    // pointer. (Note that these cases cannot overlap because the dangling OOM
    // pointer is not aligned for `DynError`.)
    inner: NonNull<u8>,
}

// impl fmt::Debug for OomOrDynError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         self.unpack().debug(f)
//     }
// }

// Safety: `OomOrDynError` is either an `OutOfMemory` or a `BoxedDynError` and
// both are `Send`.
unsafe impl Send for OomOrDynError {}

// Safety: `OomOrDynError` is either an `OutOfMemory` or a `BoxedDynError` and
// both are `Sync`.
unsafe impl Sync for OomOrDynError {}

const _OOM_OR_DYN_ERROR_SEND_SYNC_SAFETY: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OutOfMemory>();
    assert_send_sync::<BoxedDynError>();
};

impl Drop for OomOrDynError {
    fn drop(&mut self) {
        if self.is_boxed_dyn_error() {
            let inner = self.inner.cast::<DynError>();
            let inner = OwnedPtr::new(inner);
            let _ = BoxedDynError::from_owned_ptr(inner);
        } else {
            debug_assert!(self.is_oom());
        }
    }
}

impl From<BoxedDynError> for OomOrDynError {
    fn from(boxed: BoxedDynError) -> Self {
        let inner = boxed.into_owned_ptr().into_non_null().cast::<u8>();
        debug_assert_ne!(inner, Self::OOM.inner);
        OomOrDynError { inner }
    }
}

impl OomOrDynError {
    const _SIZE: () = assert!(mem::size_of::<OomOrDynError>() == mem::size_of::<usize>());

    // Our pointer tagging relies on this property.
    const _DYN_ERROR_HAS_GREATER_ALIGN_THAN_OOM: () =
        assert!(mem::align_of::<DynError>() > mem::align_of::<OutOfMemory>());

    const OOM_PTR: NonNull<u8> = NonNull::<OutOfMemory>::dangling().cast();

    pub(crate) const OOM: Self = OomOrDynError {
        inner: Self::OOM_PTR,
    };

    fn is_oom(&self) -> bool {
        self.inner == Self::OOM_PTR
    }

    fn is_boxed_dyn_error(&self) -> bool {
        !self.is_oom()
    }

    /// # Safety
    ///
    /// `self.is_oom()` must be true.
    unsafe fn unchecked_oom(&self) -> &OutOfMemory {
        debug_assert!(self.is_oom());
        let inner = self.inner.cast::<OutOfMemory>();
        // Safety: `inner` is OOM's dangling pointer and it is always valid to
        // turn `T`'s dangling pointer into an `&T` reference for unit types.
        unsafe { inner.as_ref() }
    }

    /// # Safety
    ///
    /// `self.is_oom()` must be true.
    unsafe fn unchecked_oom_mut(&mut self) -> &mut OutOfMemory {
        debug_assert!(self.is_oom());
        let mut inner = self.inner.cast::<OutOfMemory>();
        // Safety: `inner` is OOM's dangling pointer and it is always valid to
        // turn `T`'s dangling pointer into an `&T` reference for unit types.
        unsafe { inner.as_mut() }
    }

    /// # Safety
    ///
    /// `self.is_boxed_dyn_error()` must be true.
    unsafe fn unchecked_into_dyn_error(self) -> OwnedPtr<DynError> {
        debug_assert!(self.is_boxed_dyn_error());
        let inner = self.inner.cast::<DynError>();
        mem::forget(self);
        OwnedPtr::new(inner)
    }

    /// # Safety
    ///
    /// `self.is_boxed_dyn_error()` must be true.
    unsafe fn unchecked_dyn_error_ref(&self) -> SharedPtr<'_, DynError> {
        debug_assert!(self.is_boxed_dyn_error());
        SharedPtr::new(self.inner.cast::<DynError>())
    }

    /// # Safety
    ///
    /// `self.is_boxed_dyn_error()` must be true.
    unsafe fn unchecked_dyn_error_mut(&mut self) -> MutPtr<'_, DynError> {
        debug_assert!(self.is_boxed_dyn_error());
        MutPtr::new(self.inner.cast::<DynError>())
    }

    pub(crate) fn unpack(&self) -> OomOrDynErrorRef<'_> {
        if self.is_oom() {
            // Safety: is_oom() is true.
            OomOrDynErrorRef::Oom(unsafe { self.unchecked_oom() })
        } else {
            debug_assert!(self.is_boxed_dyn_error());
            // Safety: self.is_boxed_dyn_error() is true.
            OomOrDynErrorRef::DynError(unsafe { self.unchecked_dyn_error_ref() })
        }
    }

    pub(crate) fn unpack_mut(&mut self) -> OomOrDynErrorMut<'_> {
        if self.is_oom() {
            // Safety: self.is_oom() is true
            OomOrDynErrorMut::Oom(unsafe { self.unchecked_oom_mut() })
        } else {
            debug_assert!(self.is_boxed_dyn_error());
            // Safety: self.is_boxed_dyn_error() is true.
            OomOrDynErrorMut::DynError(unsafe { self.unchecked_dyn_error_mut() })
        }
    }

    pub(crate) fn into_boxed_dyn_core_error(
        self,
    ) -> Box<dyn core::error::Error + Send + Sync + 'static> {
        let box_dyn_error_of_oom = || {
            let ptr = NonNull::<OutOfMemory>::dangling().as_ptr();
            // Safety: it is always safe to call `Box::<T>::from_raw` on `T`'s
            // dangling pointer if `T` is a unit type.
            let boxed = unsafe { Box::from_raw(ptr) };
            boxed as _
        };

        if self.is_oom() {
            box_dyn_error_of_oom()
        } else {
            debug_assert!(self.is_boxed_dyn_error());
            // Safety: this is a boxed dyn error.
            let ptr = unsafe { self.unchecked_into_dyn_error() };
            // Safety: invariant of the type that the pointer is valid.
            let vtable = unsafe { ptr.as_ref().vtable };
            // Safety: the pointer is valid and the vtable is associated with
            // this pointer's concrete error type.
            match unsafe { (vtable.into_boxed_dyn_core_error)(ptr) } {
                Ok(e) => e,
                Err(_oom) => box_dyn_error_of_oom(),
            }
        }
    }

    /// Given that this is known to be an instance of the type associated with
    /// the given `TypeId`, do an owning-downcast to that type, writing the
    /// result through the given `ret_ptr`, and deallocating `self` along the
    /// way.
    ///
    /// The `ret_ptr`'s storage will contain an initialized instance of the
    /// associated type upon this method's successful return.
    ///
    /// # Safety
    ///
    /// This error (or another in its chain) must be of the type associated with
    /// `TypeId`.
    ///
    /// The given `ret_ptr` must point to a valid-but-uninitialized storage
    /// location for an instance of the type associated with the given `TypeId`.
    pub(crate) unsafe fn downcast(self, type_id: TypeId, ret_ptr: NonNull<u8>) {
        if self.is_oom() {
            debug_assert_eq!(type_id, TypeId::of::<OutOfMemory>());
            // Safety: this is an OOM error.
            let oom = *unsafe { self.unchecked_oom() };
            // Safety: implied by this method's safety contract.
            unsafe {
                ret_ptr.cast::<OutOfMemory>().write(oom);
            }
        } else {
            debug_assert!(self.is_boxed_dyn_error());
            // Safety: this is a boxed dyn error.
            let ptr = unsafe { self.unchecked_into_dyn_error() };
            // Safety: invariant of this type that the pointer is valid.
            let vtable = unsafe { ptr.as_ref().vtable };
            // Safety: the pointer is valid and the vtable is associated with
            // this pointer's concrete type.
            unsafe { (vtable.downcast)(ptr, type_id, ret_ptr) }
        }
    }
}

/// An iterator over each error in an [`Error`]'s context chain.
///
/// The iterator yields `&'a (dyn core::error::Error + 'static)` items.
///
/// Iterates from the most recently added error context towards the root cause.
///
/// Created by the [`Error::chain`] method. See that method's documentation for
/// more details.
pub struct Chain<'a> {
    state: ChainState<'a>,
}

enum ChainState<'a> {
    Ours(OomOrDynErrorRef<'a>),
    Core(Option<&'a (dyn core::error::Error + 'static)>),
}

impl<'a> Chain<'a> {
    fn new(error: OomOrDynErrorRef<'a>) -> Self {
        Self {
            state: ChainState::Ours(error),
        }
    }
}

impl<'a> Iterator for Chain<'a> {
    type Item = &'a (dyn core::error::Error + 'static);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            ChainState::Ours(e) => {
                let core = e.as_dyn_core_error();
                self.state = if let Some(e) = e.source() {
                    ChainState::Ours(e)
                } else {
                    ChainState::Core(core.source())
                };
                Some(core)
            }
            ChainState::Core(error) => {
                let e = error.take()?;
                self.state = ChainState::Core(e.source());
                Some(e)
            }
        }
    }
}

impl FusedIterator for Chain<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestError;

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(self, f)
        }
    }

    impl core::error::Error for TestError {}

    #[test]
    fn from_oom() {
        let mut error = Error::from(OutOfMemory::new());
        assert!(error.is::<OutOfMemory>());
        assert!(error.downcast_ref::<OutOfMemory>().is_some());
        assert!(error.downcast_mut::<OutOfMemory>().is_some());

        // NB: use this module's scope to check that the inner representation is
        // `OomOrDynError::Oom` and not a `Box<OutOfMemory> as Box<dyn
        // Error>`. This is why this test cannot be in `tests/tests.rs`.
        assert!(error.inner.is_oom());
    }

    #[test]
    fn dyn_error_and_concrete_error_layouts_are_compatible() {
        type Concrete = ConcreteError<TestError>;

        let dyn_size = mem::size_of::<DynError>();
        let concrete_size = mem::size_of::<Concrete>();
        assert!(
            dyn_size <= concrete_size,
            "assertion failed: {dyn_size} <= {concrete_size}"
        );

        let dyn_align = mem::align_of::<DynError>();
        let concrete_align = mem::align_of::<Concrete>();
        assert!(
            dyn_align <= concrete_align,
            "assertion failed: {dyn_align} <= {concrete_align}"
        );

        let dyn_offset = mem::offset_of!(DynError, vtable);
        let concrete_offset = mem::offset_of!(Concrete, vtable);
        assert_eq!(dyn_offset, concrete_offset);

        #[cfg(feature = "backtrace")]
        {
            let dyn_offset = mem::offset_of!(DynError, backtrace);
            let concrete_offset = mem::offset_of!(Concrete, backtrace);
            assert_eq!(dyn_offset, concrete_offset);
        }
    }
}
