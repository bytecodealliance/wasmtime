//! This library contains code that is common to both the `cranelift-codegen` and
//! `cranelift-codegen-meta` libraries.

#![deny(missing_docs)]
#![no_std]

extern crate alloc;

pub mod constant_hash;
pub mod constants;
pub mod join_display;

/// Version number of this crate.
pub const VERSION: &str = core::env!("CARGO_PKG_VERSION");
