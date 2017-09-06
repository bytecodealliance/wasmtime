//! Standalone JIT-style runtime for WebAssembly using Cretonne. Provides functions to translate
//! `get_global`, `set_global`, `current_memory`, `grow_memory`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

extern crate cretonne;
extern crate cton_frontend;
extern crate cton_wasm;
extern crate region;

mod execution;
mod standalone;

pub use execution::{compile_module, execute, ExecutableCode};
pub use standalone::StandaloneRuntime;
