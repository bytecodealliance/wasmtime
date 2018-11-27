//! JIT-style runtime for WebAssembly using Cranelift.

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
extern crate memmap;
extern crate region;
extern crate wasmtime_environ;
#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

mod execute;
mod instance;
mod memory;

pub use execute::{compile_and_link_module, execute, finish_instantiation};
pub use instance::Instance;

#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{string, vec};
    pub use core::*;
    pub use core::{i32, str, u32};
}
