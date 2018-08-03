//! Standalone runtime for WebAssembly using Cranelift. Provides functions to translate
//! `get_global`, `set_global`, `current_memory`, `grow_memory`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs)]

extern crate cranelift_codegen;
extern crate cranelift_wasm;
extern crate target_lexicon;

mod compilation;
mod environ;
mod module;

pub use compilation::{compile_module, Compilation, Relocation, Relocations};
pub use environ::{ModuleEnvironment, ModuleTranslation};
pub use module::{DataInitializer, Module, TableElements};
