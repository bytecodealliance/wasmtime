//! Top-level lib.rs for `cranelift_object`.
//!
//! This re-exports `object` so you don't have to explicitly keep the versions in sync.

#![deny(missing_docs)]

mod backend;

pub use crate::backend::{ObjectBuilder, ObjectModule, ObjectProduct};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use object;
