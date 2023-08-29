//! Top-level lib.rs for `cranelift_jit`.
//!
//! There is an [example project](https://github.com/bytecodealliance/cranelift-jit-demo/)
//! which shows how to use some of the features of `cranelift_jit`.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features,
    unreachable_pub
)]
#![warn(unused_import_braces)]

mod backend;
mod compiled_blob;
mod memory;

pub use crate::backend::{JITBuilder, JITModule};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
