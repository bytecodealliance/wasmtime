//! Top-level lib.rs for `cranelift_object`.
//!
//! This re-exports `object` so you don't have to explicitly keep the versions in sync.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::map_unwrap_or,
        clippy::clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

mod backend;

pub use crate::backend::{ObjectBuilder, ObjectModule, ObjectProduct};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use object;
