//! JIT-style runtime for WebAssembly using Cranelift.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]

mod code_memory;
mod debug;
mod instantiate;
pub mod profiling;
mod unwind;

pub use crate::code_memory::CodeMemory;
pub use crate::instantiate::{finish_object, CompiledModule};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
