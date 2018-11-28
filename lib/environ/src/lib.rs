//! Standalone environment for WebAssembly using Cranelift. Provides functions to translate
//! `get_global`, `set_global`, `memory.size`, `memory.grow`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]
#![cfg_attr(
    feature = "clippy",
    plugin(clippy(conf_file = "../../clippy.toml"))
)]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(new_without_default, new_without_default_derive)
)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic,
        mut_mut,
        nonminimal_bool,
        option_map_unwrap_or,
        option_map_unwrap_or_else,
        print_stdout,
        unicode_not_nfc,
        use_self
    )
)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

extern crate cranelift_codegen;
extern crate cranelift_entity;
extern crate cranelift_wasm;
#[macro_use]
extern crate memoffset;
#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

mod compilation;
mod environ;
mod module;
mod vmcontext;

pub use compilation::{compile_module, Compilation, Relocation, RelocationTarget, Relocations};
pub use environ::{ModuleEnvironment, ModuleTranslation};
pub use module::{DataInitializer, Export, Module, TableElements};

#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{string, vec};
    pub use core::*;
    pub use core::{i32, str, u32};
}
