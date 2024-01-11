//! JIT-style runtime for WebAssembly using Cranelift.

#![deny(missing_docs)]

mod code_memory;
#[cfg(feature = "debug-builtins")]
mod debug;
mod instantiate;
pub mod profiling;

pub use crate::code_memory::CodeMemory;
pub use crate::instantiate::{finish_object, CompiledModule};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
