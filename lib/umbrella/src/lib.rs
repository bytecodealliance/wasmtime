//! Cretonne umbrella crate, providing a convenient one-line dependency.

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

/// Provide these crates, renamed to reduce stutter.
pub extern crate cretonne_codegen as codegen;
pub extern crate cretonne_frontend as frontend;

/// A prelude providing convenient access to commonly-used cretonne features. Use
/// as `use cretonne::prelude::*`.
pub mod prelude {
    pub use codegen;
    pub use codegen::entity::EntityRef;
    pub use codegen::ir::{AbiParam, InstBuilder, Value, Ebb, Signature, CallConv, Type,
                          JumpTableData, MemFlags};
    pub use codegen::ir::types;
    pub use codegen::ir::condcodes::{IntCC, FloatCC};
    pub use codegen::ir::immediates::{Ieee32, Ieee64};

    pub use frontend::{FunctionBuilderContext, FunctionBuilder, Variable};
}
