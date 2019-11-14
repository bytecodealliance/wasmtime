//! This library contains code that is common to both the `cranelift-codegen` and
//! `cranelift-codegen-meta` libraries.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]
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

use packed_struct;
#[macro_use]
extern crate packed_struct_codegen;

pub mod condcodes;
pub mod constant_hash;
pub mod constants;
pub mod isa;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
