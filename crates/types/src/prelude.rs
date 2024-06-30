//! Rust module prelude for Wasmtime crates.
//!
//! Wasmtime crates that use `no_std` use `core::prelude::*` by default which
//! does not include `alloc`-related functionality such as `String` and `Vec`.
//! To have similar ergonomics to `std` and additionally group up some common
//! functionality this module is intended to be imported at the top of all
//! modules with:
//!
//! ```rust,ignore
//! use crate::*;
//! ```
//!
//! Externally for crates that depend on `wasmtime-types` they should have this
//! in the root of the crate:
//!
//! ```rust,ignore
//! use wasmtime_types::prelude;
//! ```
//!
//! and then `use crate::*` works as usual.

pub use alloc::borrow::ToOwned;
pub use alloc::boxed::Box;
pub use alloc::format;
pub use alloc::string::{String, ToString};
pub use alloc::vec;
pub use alloc::vec::Vec;
pub use wasmparser::collections::{IndexMap, IndexSet};

/// Convenience trait for converting `Result<T, E>` into `anyhow::Result<T>`
///
/// Typically this is automatically done with the `?` operator in Rust and
/// by default this trait isn't necessary. With the `anyhow` crate's `std`
/// feature disabled, however, the `?` operator won't work because the `Error`
/// trait is not defined. This trait helps to bridge this gap.
///
/// This does the same thing as `?` when the `std` feature is enabled, and when
/// `std` is disabled it'll use different trait bounds to create an
/// `anyhow::Error`.
///
/// This trait is not suitable as a public interface because features change
/// what implements the trait. It's good enough for a wasmtime internal
/// implementation detail, however.
pub trait Err2Anyhow<T> {
    /// Convert `self` to `anyhow::Result<T>`.
    fn err2anyhow(self) -> anyhow::Result<T>;
}

impl<T, E: IntoAnyhow> Err2Anyhow<T> for Result<T, E> {
    fn err2anyhow(self) -> anyhow::Result<T> {
        match self {
            Ok(e) => Ok(e),
            Err(e) => Err(e.into_anyhow()),
        }
    }
}

/// Convenience trait to convert a value into `anyhow::Error`
///
/// This trait is not a suitable public interface of Wasmtime so it's just an
/// internal implementation detail for now. This trait is conditionally
/// implemented on the `std` feature with different bounds.
pub trait IntoAnyhow {
    /// Converts `self` into an `anyhow::Error`.
    fn into_anyhow(self) -> anyhow::Error;
}

#[cfg(feature = "std")]
impl<T> IntoAnyhow for T
where
    T: Into<anyhow::Error>,
{
    fn into_anyhow(self) -> anyhow::Error {
        self.into()
    }
}

#[cfg(not(feature = "std"))]
impl<T> IntoAnyhow for T
where
    T: core::fmt::Display + core::fmt::Debug + Send + Sync + 'static,
{
    fn into_anyhow(self) -> anyhow::Error {
        anyhow::Error::msg(self)
    }
}
