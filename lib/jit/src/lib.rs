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

#[macro_use]
extern crate cranelift_entity;

use region;

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;
use failure;

#[macro_use]
extern crate failure_derive;

mod action;
mod code_memory;
mod compiler;
mod instantiate;
mod link;
mod namespace;
mod resolver;
mod target_tunables;

pub use crate::action::{ActionError, ActionOutcome, RuntimeValue};
pub use crate::compiler::Compiler;
pub use crate::instantiate::{instantiate, CompiledModule, SetupError};
pub use crate::link::link_module;
pub use crate::namespace::{InstanceIndex, Namespace};
pub use crate::resolver::{NullResolver, Resolver};
pub use crate::target_tunables::target_tunables;

// Re-export `Instance` so that users won't need to separately depend on
// wasmtime-runtime in common cases.
pub use wasmtime_runtime::{Instance, InstantiationError};

#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{boxed, rc, string, vec};
    pub use core::*;
    pub use core::{i32, str, u32};
}
