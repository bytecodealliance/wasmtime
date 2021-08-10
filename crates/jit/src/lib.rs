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
mod compiler;
mod instantiate;
mod link;
mod object;
mod unwind;

pub mod native;
pub mod trampoline;

pub use crate::code_memory::CodeMemory;
pub use crate::compiler::{Compilation, CompilationStrategy, Compiler};
pub use crate::instantiate::{
    CompilationArtifacts, CompiledModule, ModuleCode, SetupError, SymbolizeContext, TypeTables,
};
pub use crate::link::link_module;
pub use wasmtime_cranelift::{blank_sig, wasmtime_call_conv};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
