//! Top-level lib.rs for `cranelift_object`.
//!
//! Users of this module should not have to depend on `object` directly.

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
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

mod backend;
mod traps;

pub use crate::backend::{ObjectBackend, ObjectBuilder, ObjectProduct, ObjectTrapCollection};
pub use crate::traps::ObjectTrapSink;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
