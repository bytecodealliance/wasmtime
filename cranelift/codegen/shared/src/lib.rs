//! This library contains code that is common to both the `cranelift-codegen` and
//! `cranelift-codegen-meta` libraries.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]

pub mod constant_hash;
pub mod constants;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
