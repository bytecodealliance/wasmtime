//! Standalone environment for WebAssembly using Cranelift. Provides functions to translate
//! `get_global`, `set_global`, `memory.size`, `memory.grow`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::new_without_default, clippy::new_without_default)
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

mod address_map;
mod compilation;
mod data_structures;
mod func_environ;
mod module;
mod module_environ;
mod tunables;
mod vmoffsets;

mod cache;

pub mod cranelift;
#[cfg(feature = "lightbeam")]
pub mod lightbeam;

pub use crate::address_map::{
    FunctionAddressMap, InstructionAddressMap, ModuleAddressMap, ModuleVmctxInfo, ValueLabelsRanges,
};
pub use crate::cache::{create_new_config as cache_create_new_config, init as cache_init};
pub use crate::compilation::{
    Compilation, CompileError, CompiledFunction, Compiler, Relocation, RelocationTarget,
    Relocations, TrapInformation, Traps,
};
pub use crate::cranelift::Cranelift;
pub use crate::data_structures::*;
pub use crate::func_environ::BuiltinFunctionIndex;
#[cfg(feature = "lightbeam")]
pub use crate::lightbeam::Lightbeam;
pub use crate::module::{
    Export, MemoryPlan, MemoryStyle, Module, TableElements, TablePlan, TableStyle,
};
pub use crate::module_environ::{
    translate_signature, DataInitializer, DataInitializerLocation, FunctionBodyData,
    ModuleEnvironment, ModuleTranslation,
};
pub use crate::tunables::Tunables;
pub use crate::vmoffsets::{TargetSharedSignatureIndex, VMOffsets};

/// WebAssembly page sizes are defined to be 64KiB.
pub const WASM_PAGE_SIZE: u32 = 0x10000;

/// The number of pages we can have before we run out of byte index space.
pub const WASM_MAX_PAGES: u32 = 0x10000;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
