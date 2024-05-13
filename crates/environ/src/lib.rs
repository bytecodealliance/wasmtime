//! Standalone environment for WebAssembly using Cranelift. Provides functions to translate
//! `get_global`, `set_global`, `memory.size`, `memory.grow`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs)]
#![warn(clippy::cast_sign_loss)]
#![no_std]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;
extern crate alloc;

/// Rust module prelude for Wasmtime crates.
///
/// Wasmtime crates that use `no_std` use `core::prelude::*` by default which
/// does not include `alloc`-related functionality such as `String` and `Vec`.
/// To have similar ergonomics to `std` and additionally group up some common
/// functionality this module is intended to be imported at the top of all
/// modules with:
///
/// ```rust,ignore
/// use crate::*;
/// ```
///
/// Externally for crates that depend on `wasmtime-environ` they should have
/// this in the root of the crate:
///
/// ```rust,ignore
/// use wasmtime_environ::prelude;
/// ```
///
/// and then `use crate::*` works as usual.
pub mod prelude {
    pub use crate::{Err2Anyhow, IntoAnyhow};
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;
    pub use wasmparser::map::{IndexMap, IndexSet};
}

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

mod address_map;
mod builtin;
mod demangling;
mod gc;
mod module;
mod module_artifacts;
mod module_types;
pub mod obj;
mod ref_bits;
mod scopevec;
mod stack_map;
mod trap_encoding;
mod tunables;
mod vmoffsets;

pub use crate::address_map::*;
pub use crate::builtin::*;
pub use crate::demangling::*;
pub use crate::gc::*;
pub use crate::module::*;
pub use crate::module_artifacts::*;
pub use crate::module_types::*;
pub use crate::ref_bits::*;
pub use crate::scopevec::ScopeVec;
pub use crate::stack_map::StackMap;
pub use crate::trap_encoding::*;
pub use crate::tunables::*;
pub use crate::vmoffsets::*;
pub use object;

#[cfg(feature = "compile")]
mod compile;
#[cfg(feature = "compile")]
pub use crate::compile::*;

#[cfg(feature = "component-model")]
pub mod component;
#[cfg(all(feature = "component-model", feature = "compile"))]
pub mod fact;

// Reexport all of these type-level since they're quite commonly used and it's
// much easier to refer to everything through one crate rather than importing
// one of three and making sure you're using the right one.
pub use cranelift_entity::*;
pub use wasmtime_types::*;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
