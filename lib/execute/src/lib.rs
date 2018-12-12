//! JIT-style runtime for WebAssembly using Cranelift.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::new_without_default, clippy::new_without_default_derive)
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
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

extern crate cranelift_codegen;
extern crate cranelift_entity;
extern crate cranelift_frontend;
extern crate cranelift_wasm;
extern crate region;
extern crate wasmtime_environ;
extern crate wasmtime_runtime;
#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;
extern crate failure;
#[macro_use]
extern crate failure_derive;

mod action;
mod instance_plus;
mod jit_code;
mod link;
mod resolver;
mod trampoline_park;

pub use action::{ActionError, ActionOutcome, RuntimeValue};
pub use instance_plus::InstancePlus;
pub use jit_code::JITCode;
pub use link::link_module;
pub use resolver::{NullResolver, Resolver};

#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{string, vec};
    pub use core::*;
    pub use core::{i32, str, u32};
}
