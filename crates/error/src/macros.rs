//! Macro definitions and the private runtime functions used in their generated
//! code.

// Items used by macro-generated code.
pub use core::format_args;
pub use core::result::Result::Err;

use super::{Error, OutOfMemory};
use alloc::string::String;
use core::fmt::{self, write};

/// Construct an [`Error`](crate::Error) via string formatting or another error.
///
/// Like `anyhow::anyhow!` but for [`wasmtime::Error`][crate::Error].
///
/// # String Formatting
///
/// When a string literal is the first argument, it is interpreted as a format
/// string template and the rest of the arguments are format arguments:
///
/// ```
/// # use wasmtime_internal_error as wasmtime;
/// use wasmtime::{anyhow, Error};
///
/// let x = 42;
/// let error: Error = anyhow!("x is {x}");
/// assert_eq!(error.to_string(), "x is 42");
///
/// let error: Error = anyhow!("x / 2 is {}", x / 2);
/// assert_eq!(error.to_string(), "x / 2 is 21");
///
/// let error: Error = anyhow!("x + 1 is {y}", y = x + 1);
/// assert_eq!(error.to_string(), "x + 1 is 43");
/// ```
///
/// # From Another Error
///
/// When a string literal is not the first argument, then it is treated as a
/// foreign error and is converted into an [`Error`][crate::Error]. The argument
/// must be of a type that can be passed to either
/// [`Error::new`][crate::Error::new] or [`Error::msg`][crate::Error::msg]:
///
/// ```
/// # fn _foo() {
/// #![cfg(feature = "std")]
/// # use wasmtime_internal_error as wasmtime;
/// use std::fmt;
/// use wasmtime::{anyhow, Error};
///
/// #[derive(Debug)]
/// struct SomeOtherError(u32);
///
/// impl fmt::Display for SomeOtherError {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "some other error (code {})", self.0)
///     }
/// }
///
/// impl std::error::Error for SomeOtherError {}
///
/// let error: Error = anyhow!(SomeOtherError(36));
/// assert!(error.is::<SomeOtherError>());
/// assert_eq!(error.to_string(), "some other error (code 36)");
/// # }
/// ```
#[macro_export]
macro_rules! anyhow {
    // Format-style invocation without explicit format arguments.
    ( $message:literal $(,)? ) => {
        $crate::Error::from_format_args($crate::macros::format_args!($message))
    };

    // Format-style invocation with explicit format arguments.
    ( $message:literal , $( $args:tt )* ) => {
        $crate::Error::from_format_args($crate::macros::format_args!($message , $( $args )* ))
    };

    // Do either `Error::new($error)` or `Error::msg($error)` depending on
    // whether `$error` implements `core::error::Error` or not.
    ( $error:expr $(,)? ) => {{
        use $crate::macros::ctor_specialization::*;
        let error = $error;
        (&&error).wasmtime_error_choose_ctor().construct(error)
    }};
}

/// Identical to the [`anyhow!`][crate::anyhow] macro.
///
/// Provided for compatibility.
#[macro_export]
macro_rules! format_err {
    ( $( $args:tt )* ) => {
        anyhow!( $( $args )* )
    };
}

/// Early exit from the current function with an error.
///
/// This helper is equivalent to `return Err(anyhow!(...))`.
///
/// See the docs for the [`anyhow!`][crate::anyhow] macro for details on the
/// kinds of errors that can be constructed.
///
/// Like `anyhow::bail!` but for [`wasmtime::Error`][crate::Error].
///
/// # Example
///
/// ```
/// # use wasmtime_internal_error as wasmtime;
/// use wasmtime::{bail, Result};
///
/// fn error_on_none(option: Option<u32>) -> Result<u32> {
///     match option {
///         None => bail!("`error_on_none` got `None`!"),
///         Some(x) => Ok(x),
///     }
/// }
///
/// let x = error_on_none(Some(42)).unwrap();
/// assert_eq!(x, 42);
///
/// let error = error_on_none(None).unwrap_err();
/// assert_eq!(
///     error.to_string(),
///     "`error_on_none` got `None`!",
/// );
/// ```
#[macro_export]
macro_rules! bail {
    ( $($args:tt)* ) => {{
        return $crate::macros::Err($crate::anyhow!( $( $args )* ));
    }};
}

/// Ensure that a condition holds true, or else early exit from the current
/// function with an error.
///
/// `ensure!(condition, ...)` is equivalent to the following:
///
/// ```ignore
/// if !condition {
///     return Err(anyhow!(...));
/// }
/// ```
///
/// Like `anyhow::ensure!` but for [`wasmtime::Error`][crate::Error].
///
/// # Example
///
/// ```rust
/// # use wasmtime_internal_error as wasmtime;
/// use wasmtime::{ensure, Result};
///
/// fn checked_div(a: u32, b: u32) -> Result<u32> {
///     ensure!(b != 0, "cannot divide by zero: {a} / {b}");
///     Ok(a / b)
/// }
///
/// let x = checked_div(6, 2).unwrap();
/// assert_eq!(x, 3);
///
/// let error = checked_div(9, 0).unwrap_err();
/// assert_eq!(
///     error.to_string(),
///     "cannot divide by zero: 9 / 0",
/// );
/// ```
#[macro_export]
macro_rules! ensure {
    ( $condition:expr , $( $args:tt )* ) => {{
        if $crate::macros::ensure::not($condition) {
            $crate::bail!( $( $args )* );
        }
    }};
}

/// We don't have specialization in stable Rust, so do a poor-person's
/// equivalent by hacking Rust's method name resolution and auto-deref. Given
/// that we have `n` versions of the "same" method, we do the following:
///
/// * We define `n` different traits, which each define the same trait method
///   name. The method need not have the same type across traits, but each must
///   type-check when chosen by method resolution at a particular call site.
///
/// * We implement each trait for an `i`-deep borrow of the type(s) we want to
///   specialize the `i`th implementation on, for example:
///
///   ```ignore
///   impl Specialization1 for &MyType { ... }
///   impl Specialization2 for &&OtherType { ... }
///   impl Specialization3 for &&&AnotherType { ... }
///   ```
///
/// * Call sites must have all specialization traits in scope and must borrow
///   the receiver `n` times before calling the method. Rust's method name
///   resolution will choose the method with the least number of references that
///   is well-typed. Therefore, specialization implementations for lower numbers
///   of borrows are preferred over those with higher numbers of borrows when
///   specializations overlap. For example, if both `<&&&T as
///   Specialization3>::method` and `<&T as Specialization1>::method` are
///   well-typed at the trait method call site `(&&&&&t).method()`, then
///   `Specialization1` will be prioritized over `Specialization3`.
///
/// In our specific case here of choosing an `Error` constructor, we only have
/// two specializations:
///
/// 1. When the type implements `core::error::Error`, we want to use the
///    `Error::new` constructor, which will preserve
///    `core::error::Error::source` chains.
///
/// 2. Otherwise, we want to use the `Error::msg` constructor.
///
/// The `*CtorTrait`s are our `n` specialization traits. Their
/// `wasmtime_error_choose_ctor` methods will return different types, each of
/// which is a dispatcher to their associated constructor. Those dispatchers
/// each have a constructor signature that is syntactically identical, but only
/// guaranteed to be well-typed based on the specialization that we did by
/// getting the dispatcher in the first place.
pub mod ctor_specialization {
    use super::*;

    pub trait NewCtorTrait {
        #[inline]
        fn wasmtime_error_choose_ctor(&self) -> NewCtor {
            NewCtor
        }
    }

    impl<E: core::error::Error + Send + Sync + 'static> NewCtorTrait for &E {}

    pub struct NewCtor;

    impl NewCtor {
        #[inline]
        pub fn construct<E>(&self, error: E) -> Error
        where
            E: core::error::Error + Send + Sync + 'static,
        {
            Error::new(error)
        }
    }

    pub trait MsgCtorTrait {
        #[inline]
        fn wasmtime_error_choose_ctor(&self) -> MsgCtor {
            MsgCtor
        }
    }

    impl<M: fmt::Debug + fmt::Display + Send + Sync + 'static> MsgCtorTrait for &&M {}

    pub struct MsgCtor;

    impl MsgCtor {
        #[inline]
        pub fn construct<M>(&self, message: M) -> Error
        where
            M: fmt::Debug + fmt::Display + Send + Sync + 'static,
        {
            Error::msg(message)
        }
    }
}

/// Runtime code for creating an `Error` from format arguments, handling OOM in
/// the process.
pub mod formatting {
    use super::*;

    #[derive(Default)]
    struct Formatter {
        message: String,
        oom: Option<OutOfMemory>,
    }

    impl fmt::Write for Formatter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            match self.message.try_reserve(s.len()) {
                Ok(()) => {
                    self.message.push_str(s);
                    Ok(())
                }
                Err(_) => {
                    self.oom = Some(OutOfMemory::new());
                    Err(fmt::Error)
                }
            }
        }
    }

    impl Error {
        /// Construct an `Error` from format arguments.
        ///
        /// Only for use by the `anyhow!` macro.
        #[doc(hidden)]
        pub fn from_format_args(args: fmt::Arguments<'_>) -> Self {
            if let Some(s) = args.as_str() {
                return Self::msg(s);
            }

            let mut f = Formatter::default();
            match write(&mut f, args) {
                Ok(()) => {
                    debug_assert!(f.oom.is_none());
                    Error::msg(f.message)
                }
                Err(fmt_error) => match f.oom {
                    Some(oom) => Error::new(oom),
                    None => Error::new(fmt_error),
                },
            }
        }
    }
}

pub mod ensure {
    /// Convenience trait to enable `ensure!(cond, ...)` to work when `cond` is of
    /// type `&bool`, not just `bool`. Saves useless rewrite-to-`*cond` busywork and
    /// matches `anyhow`'s behavior.
    pub trait ToBool {
        fn to_bool(self) -> bool;
    }

    impl ToBool for bool {
        #[inline]
        fn to_bool(self) -> bool {
            self
        }
    }

    impl ToBool for &bool {
        #[inline]
        fn to_bool(self) -> bool {
            *self
        }
    }

    #[inline]
    pub fn not(b: impl ToBool) -> bool {
        !b.to_bool()
    }
}
