//! Top-level lib.rs for `cranelift_jit`.
//!
//! There is an [example project](https://github.com/bytecodealliance/cranelift-jit-demo/)
//! which shows how to use some of the features of `cranelift_jit`.

#![deny(missing_docs, unreachable_pub)]
#![expect(unsafe_op_in_unsafe_fn, reason = "crate isn't migrated yet")]
#![cfg_attr(not(test), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc as std;
#[cfg(feature = "std")]
#[macro_use]
extern crate std;

mod backend;
mod compiled_blob;
mod memory;

pub use crate::backend::{JITBuilder, JITModule};
#[cfg(feature = "std")]
pub use crate::memory::{ArenaMemoryProvider, SystemMemoryProvider};
pub use crate::memory::{BranchProtection, JITMemoryProvider, VecMemoryProvider};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
