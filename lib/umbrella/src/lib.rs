//! Cretonne umbrella crate, providing a convenient one-line dependency.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]

pub extern crate cretonne_codegen;
pub extern crate cretonne_frontend;

/// A prelude providing convenient access to commonly-used cretonne features. Use
/// as `use cretonne::prelude::*`.
pub mod prelude {
    pub use cretonne_codegen;
    pub use cretonne_codegen::entity::EntityRef;
    pub use cretonne_codegen::ir::{AbiParam, InstBuilder, Value, Ebb, Signature, CallConv};
    pub use cretonne_codegen::ir::types;
    pub use cretonne_codegen::ir::condcodes::IntCC;

    pub use cretonne_frontend::{FunctionBuilderContext, FunctionBuilder, Variable};
}
