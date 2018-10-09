//! JIT-style runtime for WebAssembly using Cranelift.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
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

extern crate cranelift_codegen;
extern crate cranelift_entity;
extern crate cranelift_wasm;
extern crate memmap;
extern crate region;
extern crate wasmtime_environ;

mod execute;
mod instance;
mod memory;

pub use execute::{compile_and_link_module, execute};
pub use instance::Instance;
