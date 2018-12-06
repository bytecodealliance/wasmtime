//! Top-level lib.rs for `cranelift_simplejit`.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(new_without_default, new_without_default_derive)
)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

extern crate cranelift_codegen;
extern crate cranelift_module;
extern crate cranelift_native;
extern crate errno;
extern crate libc;
extern crate region;
extern crate target_lexicon;

#[cfg(target_os = "windows")]
extern crate winapi;

mod backend;
mod memory;

pub use backend::{SimpleJITBackend, SimpleJITBuilder};
