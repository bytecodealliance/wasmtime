//! Top-level lib.rs for `cretonne_module`.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces, unstable_features)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default, new_without_default_derive))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        print_stdout, unicode_not_nfc, use_self
    )
)]

#[macro_use]
extern crate cretonne_codegen;
#[macro_use]
extern crate cretonne_entity;
#[macro_use]
extern crate failure;

mod backend;
mod data_context;
mod module;

pub use backend::Backend;
pub use data_context::{DataContext, DataDescription, Init, Writability};
pub use module::{DataId, FuncId, FuncOrDataId, Linkage, Module, ModuleError, ModuleNamespace};
