//! Top-level lib.rs for `cranelift_faerie`.
//!
//! Users of this module should not have to depend on faerie directly.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(new_without_default, new_without_default_derive)
)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic,
        mut_mut,
        nonminimal_bool,
        option_map_unwrap_or,
        option_map_unwrap_or_else,
        print_stdout,
        unicode_not_nfc,
        use_self
    )
)]

extern crate cranelift_codegen;
extern crate cranelift_module;
extern crate faerie;
extern crate failure;
extern crate goblin;
extern crate target_lexicon;

mod backend;
mod container;
pub mod traps;

pub use backend::{FaerieBackend, FaerieBuilder, FaerieProduct, FaerieTrapCollection};
pub use container::Format;
