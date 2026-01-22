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

#[cfg(feature = "backtrace")]
mod backtrace;
mod boxed;
mod context;
mod error;
mod oom;
mod ptr;
#[cfg(feature = "anyhow")]
mod to_wasmtime_result;
mod vtable;

#[doc(hidden)]
pub mod macros;

#[cfg(feature = "backtrace")]
pub use backtrace::disable_backtrace;
pub use context::Context;
pub use error::*;
pub use oom::OutOfMemory;
#[cfg(feature = "anyhow")]
pub use to_wasmtime_result::ToWasmtimeResult;

/// A result of either `Ok(T)` or an [`Err(Error)`][Error].
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Return `core::result::Result::<T, wasmtime::Error>::Ok(value)`.
///
/// Useful in situations where Rust's type inference cannot figure out that the
/// `Result`'s error type is [`Error`].
#[allow(non_snake_case, reason = "matching anyhow API")]
pub fn Ok<T>(value: T) -> Result<T> {
    core::result::Result::Ok(value)
}
