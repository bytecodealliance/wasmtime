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

mod code;
mod execute;
mod instance;
mod invoke;
mod libcalls;
mod memory;
mod mmap;
mod signalhandlers;
mod table;
mod traphandlers;
mod vmcontext;
mod world;

pub use code::Code;
pub use execute::{compile_and_link_module, finish_instantiation};
pub use instance::Instance;
pub use invoke::{invoke, InvokeOutcome, Value};
pub use traphandlers::{call_wasm, LookupCodeSegment, RecordTrap, Unwind};
pub use vmcontext::VMContext;
pub use world::InstanceWorld;

#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{string, vec};
    pub use core::*;
    pub use core::{i32, str, u32};
}
