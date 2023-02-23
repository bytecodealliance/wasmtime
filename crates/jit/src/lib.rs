//! JIT-style runtime for WebAssembly using Cranelift.

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
        clippy::map_unwrap_or,
        clippy::clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

mod code_memory;
mod debug;
mod demangling;
mod instantiate;
mod profiling;
mod unwind;

pub use crate::code_memory::CodeMemory;
pub use crate::instantiate::{
    subslice_range, CompiledModule, CompiledModuleInfo, ObjectBuilder, SymbolizeContext,
};
pub use demangling::*;
pub use profiling::*;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
