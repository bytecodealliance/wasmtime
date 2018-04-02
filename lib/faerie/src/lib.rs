//! Top-level lib.rs for `cretonne_faerie`.
//!
//! Users of this module should not have to depend on faerie directly.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces, unstable_features)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy",
            allow(new_without_default, new_without_default_derive))]
#![cfg_attr(feature="cargo-clippy", warn(
                float_arithmetic,
                mut_mut,
                nonminimal_bool,
                option_map_unwrap_or,
                option_map_unwrap_or_else,
                print_stdout,
                unicode_not_nfc,
                use_self,
                ))]

extern crate cretonne_codegen;
extern crate cretonne_module;
extern crate faerie;
#[macro_use]
extern crate failure;
extern crate goblin;

mod backend;
mod container;
mod target;

pub use backend::FaerieBackend;
pub use container::Format;
