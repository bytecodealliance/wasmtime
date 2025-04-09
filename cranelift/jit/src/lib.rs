//! Top-level lib.rs for `cranelift_jit`.
//!
//! There is an [example project](https://github.com/bytecodealliance/cranelift-jit-demo/)
//! which shows how to use some of the features of `cranelift_jit`.

#![deny(missing_docs, unreachable_pub)]
#![expect(unsafe_op_in_unsafe_fn, reason = "crate isn't migrated yet")]

mod backend;
mod compiled_blob;
mod memory;

pub use crate::backend::{JITBuilder, JITModule};
pub use crate::memory::{
    ArenaMemoryProvider, BranchProtection, JITMemoryProvider, SystemMemoryProvider,
};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
