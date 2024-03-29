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

/// TODO
pub mod prelude {
    pub use crate::{Err2Anyhow, IntoAnyhow};
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;

    /// TODO
    pub type IndexMap<K, V, H = hashbrown::hash_map::DefaultHashBuilder> =
        indexmap::IndexMap<K, V, H>;
    /// TODO
    pub type IndexSet<K, H = hashbrown::hash_map::DefaultHashBuilder> = indexmap::IndexSet<K, H>;
}

/// TODO
pub trait IntoAnyhow {
    /// TODO
    fn into_anyhow(self) -> anyhow::Error;
}

/// TODO
pub trait Err2Anyhow<T> {
    /// TODO
    fn err2anyhow(self) -> anyhow::Result<T>;
}

impl<T, E: IntoAnyhow> Err2Anyhow<T> for Result<T, E> {
    fn err2anyhow(self) -> anyhow::Result<T> {
        self.map_err(|e| e.into_anyhow())
    }
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
mod module_environ;
#[cfg(feature = "compile")]
pub use crate::compile::*;
#[cfg(feature = "compile")]
pub use crate::module_environ::*;

#[cfg(feature = "component-model")]
pub mod component;
#[cfg(all(feature = "component-model", feature = "compile"))]
pub mod fact;

// Reexport all of these type-level since they're quite commonly used and it's
// much easier to refer to everything through one crate rather than importing
// one of three and making sure you're using the right one.
pub use cranelift_entity::*;
pub use wasmtime_types::*;

/// WebAssembly page sizes are defined to be 64KiB.
pub const WASM_PAGE_SIZE: u32 = 0x10000;

/// The number of pages (for 32-bit modules) we can have before we run out of
/// byte index space.
pub const WASM32_MAX_PAGES: u64 = 1 << 16;
/// The number of pages (for 64-bit modules) we can have before we run out of
/// byte index space.
pub const WASM64_MAX_PAGES: u64 = 1 << 48;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
