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

use errno;
use region;

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate lazy_static;
use libc;
#[macro_use]
extern crate memoffset;
use cast;
use failure;
#[macro_use]
extern crate failure_derive;
#[cfg(target_os = "windows")]
extern crate winapi;

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

pub use crate::export::Export;
pub use crate::imports::Imports;
pub use crate::instance::{Instance, InstantiationError, LinkError};
pub use crate::mmap::Mmap;
pub use crate::sig_registry::SignatureRegistry;
pub use crate::signalhandlers::{wasmtime_init_eager, wasmtime_init_finish};
pub use crate::traphandlers::{wasmtime_call, wasmtime_call_trampoline};
pub use crate::vmcontext::{
    VMContext, VMFunctionBody, VMFunctionImport, VMGlobalDefinition, VMGlobalImport,
    VMMemoryDefinition, VMMemoryImport, VMSharedSignatureIndex, VMTableDefinition, VMTableImport,
};

#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{string, vec};
    pub use core::*;
    pub use core::{i32, str, u32};
}
