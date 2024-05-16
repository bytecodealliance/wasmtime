//! Cranelift umbrella crate, providing a convenient one-line dependency.

#![deny(missing_docs)]
#![no_std]

/// Provide these crates, renamed to reduce stutter.
pub use cranelift_codegen as codegen;
#[cfg(feature = "frontend")]
pub use cranelift_frontend as frontend;
#[cfg(feature = "interpreter")]
pub use cranelift_interpreter as interpreter;
#[cfg(feature = "jit")]
pub use cranelift_jit as jit;
#[cfg(feature = "module")]
pub use cranelift_module as module;
#[cfg(feature = "native")]
pub use cranelift_native as native;
#[cfg(feature = "object")]
pub use cranelift_object as object;

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

    #[cfg(feature = "frontend")]
    pub use crate::frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
}

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
