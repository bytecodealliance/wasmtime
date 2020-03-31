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
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

mod code_memory;
mod compiler;
mod imports;
mod instantiate;
mod link;
mod resolver;
mod unwind;

pub mod native;
pub mod trampoline;

pub use crate::code_memory::CodeMemory;
pub use crate::compiler::{make_trampoline, Compilation, CompilationStrategy, Compiler};
pub use crate::instantiate::{instantiate, CompiledModule, SetupError};
pub use crate::link::link_module;
pub use crate::resolver::{NullResolver, Resolver};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
