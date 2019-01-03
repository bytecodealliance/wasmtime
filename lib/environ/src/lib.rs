//! Standalone environment for WebAssembly using Cranelift. Provides functions to translate
//! `get_global`, `set_global`, `memory.size`, `memory.grow`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::new_without_default, clippy::new_without_default_derive)
)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]
#![no_std]
#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc as std;
#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(not(feature = "std"))]
use hashmap_core::HashMap;
#[cfg(feature = "std")]
use std::collections::HashMap;

use cast;
use cranelift_wasm;
use failure;
#[macro_use]
extern crate failure_derive;

mod compilation;
mod func_environ;
mod module;
mod module_environ;
mod tunables;
mod vmoffsets;

pub mod cranelift;

pub use crate::compilation::{
    Compilation, CompileError, Relocation, RelocationTarget, Relocations,
};
pub use crate::module::{
    Export, MemoryPlan, MemoryStyle, Module, TableElements, TablePlan, TableStyle,
};
pub use crate::module_environ::{
    translate_signature, DataInitializer, DataInitializerLocation, ModuleEnvironment,
    ModuleTranslation,
};
pub use crate::tunables::Tunables;
pub use crate::vmoffsets::VMOffsets;

/// WebAssembly page sizes are defined to be 64KiB.
pub const WASM_PAGE_SIZE: u32 = 0x10000;

/// The number of pages we can have before we run out of byte index space.
pub const WASM_MAX_PAGES: u32 = 0x10000;
