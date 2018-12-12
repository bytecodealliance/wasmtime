//! Runtime library support for Wasmtime.

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
extern crate cranelift_wasm;
extern crate errno;
extern crate region;
extern crate wasmtime_environ;
#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate memoffset;
extern crate cast;
extern crate failure;
#[macro_use]
extern crate failure_derive;

mod export;
mod imports;
mod instance;
mod memory;
mod mmap;
mod sig_registry;
mod signalhandlers;
mod table;
mod traphandlers;
mod vmcontext;

pub mod libcalls;

pub use export::Export;
pub use imports::Imports;
pub use instance::{Instance, InstantiationError};
pub use mmap::Mmap;
pub use signalhandlers::{wasmtime_init_eager, wasmtime_init_finish};
pub use traphandlers::{wasmtime_call, wasmtime_call_trampoline};
pub use vmcontext::{
    VMContext, VMFunctionBody, VMFunctionImport, VMGlobalDefinition, VMGlobalImport,
    VMMemoryDefinition, VMMemoryImport, VMTableDefinition, VMTableImport,
};

#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{string, vec};
    pub use core::*;
    pub use core::{i32, str, u32};
}
