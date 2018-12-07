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

mod action;
mod code;
mod export;
mod get;
mod imports;
mod instance;
mod invoke;
mod libcalls;
mod link;
mod memory;
mod mmap;
mod sig_registry;
mod signalhandlers;
mod table;
mod traphandlers;
mod vmcontext;
mod world;

pub use action::{ActionError, ActionOutcome, RuntimeValue};
pub use code::Code;
pub use export::{ExportValue, NullResolver, Resolver};
pub use get::{get, get_by_index};
pub use instance::Instance;
pub use invoke::{invoke, invoke_by_index, invoke_start_function};
pub use link::link_module;
pub use traphandlers::{call_wasm, LookupCodeSegment, RecordTrap, Unwind};
pub use vmcontext::{VMContext, VMGlobal, VMMemory, VMTable};
pub use world::InstanceWorld;

#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{string, vec};
    pub use core::*;
    pub use core::{i32, str, u32};
}
