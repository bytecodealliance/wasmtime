//! Cranelift umbrella crate, providing a convenient one-line dependency.

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
#![no_std]

/// Provide these crates, renamed to reduce stutter.
pub use cranelift_codegen as codegen;
pub use cranelift_frontend as frontend;

/// A prelude providing convenient access to commonly-used cranelift features. Use
/// as `use cranelift::prelude::*`.
pub mod prelude {
    pub use crate::codegen;
    pub use crate::codegen::entity::EntityRef;
    pub use crate::codegen::ir::condcodes::{FloatCC, IntCC};
    pub use crate::codegen::ir::immediates::{Ieee32, Ieee64, Imm64, Uimm64};
    pub use crate::codegen::ir::types;
    pub use crate::codegen::ir::{
        AbiParam, Block, ExtFuncData, ExternalName, GlobalValueData, InstBuilder, JumpTableData,
        MemFlags, Signature, StackSlotData, StackSlotKind, TrapCode, Type, Value,
    };
    pub use crate::codegen::isa;
    pub use crate::codegen::settings::{self, Configurable};

    pub use crate::frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
}

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
