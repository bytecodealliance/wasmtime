//! Standalone runtime for WebAssembly using Cranelift. Provides functions to translate
//! `get_global`, `set_global`, `current_memory`, `grow_memory`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates, unstable_features)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default, new_without_default_derive))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        print_stdout, unicode_not_nfc, use_self
    )
)]

extern crate cranelift_codegen;
extern crate cranelift_wasm;
extern crate target_lexicon;

mod compilation;
mod environ;
mod module;

pub use compilation::{compile_module, Compilation, Relocation, Relocations};
pub use environ::{ModuleEnvironment, ModuleTranslation};
pub use module::{DataInitializer, Module, TableElements};
