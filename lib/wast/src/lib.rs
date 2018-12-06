//! JIT-style runtime for WebAssembly using Cranelift.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![deny(unstable_features)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
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
extern crate cranelift_wasm;
extern crate target_lexicon;
extern crate wabt;
extern crate wasmtime_environ;
extern crate wasmtime_execute;

mod spectest;
mod wast;

pub use wast::{wast_buffer, wast_file};
