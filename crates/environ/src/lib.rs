//! Standalone environment for WebAssembly using Cranelift. Provides functions to translate
//! `get_global`, `set_global`, `memory.size`, `memory.grow`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs)]
#![warn(clippy::cast_sign_loss)]

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
