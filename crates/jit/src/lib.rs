//! JIT-style runtime for WebAssembly using Cranelift.

#![deny(missing_docs)]

mod code_memory;
#[cfg(feature = "debug-builtins")]
mod debug;
mod demangling;
mod instantiate;
pub mod profiling;

pub use crate::code_memory::CodeMemory;
#[cfg(feature = "addr2line")]
pub use crate::instantiate::SymbolizeContext;
pub use crate::instantiate::{
    subslice_range, CompiledFunctionInfo, CompiledModule, CompiledModuleInfo, ObjectBuilder,
};
pub use demangling::*;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
