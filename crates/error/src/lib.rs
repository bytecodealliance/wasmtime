//! Wasmtime's universal error handling crate.
//!
//! 99% API-compatible with `anyhow`, but additionally handles out-of-memory
//! errors, instead of aborting the process.
//!
//! See the [`Error`] documentation for more details.

#![no_std]
#![deny(missing_docs)]
#![doc(test(attr(deny(warnings))))]
#![doc(test(attr(allow(dead_code, unused_variables, unused_mut))))]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

/// Internal macro to mark a block as a slow path, pulling it out into its own
/// cold function that is never inlined.
///
/// This should be applied to the whole consequent/alternative block for a
/// conditional, never to a single expression within a larger block.
///
/// # Example
///
/// ```ignore
/// fn hot_function(x: u32) -> Result<()> {
///     if very_rare_condition(x) {
///         return out_of_line_slow_path!({
///             // Handle the rare case...
///             //
///             // This pulls the handling of the rare condition out into
///             // its own, separate function, which keeps the generated code
///             // tight, handling only the common cases inline.
///             Ok(())
///         });
///     }
///
///     // Handle the common case inline...
///     Ok(())
/// }
/// ```
macro_rules! out_of_line_slow_path {
    ( $e:expr ) => {{
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn out_of_line_slow_path<T>(f: impl FnOnce() -> T) -> T {
            f()
        }

        out_of_line_slow_path(|| $e)
    }};
}

#[cfg(feature = "backtrace")]
mod backtrace;
mod boxed;
mod context;
mod error;
mod oom;
mod ptr;
mod vtable;

#[doc(hidden)]
pub mod macros;

#[cfg(feature = "backtrace")]
pub use backtrace::disable_backtrace;
pub use context::Context;
pub use error::*;
pub use oom::OutOfMemory;

/// A result of either `Ok(T)` or an [`Err(Error)`][Error].
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Return `core::result::Result::<T, wasmtime::Error>::Ok(value)`.
///
/// Useful in situations where Rust's type inference cannot figure out that the
/// `Result`'s error type is [`Error`].
#[allow(non_snake_case)]
pub fn Ok<T>(value: T) -> Result<T> {
    core::result::Result::Ok(value)
}
