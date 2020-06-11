#![allow(missing_docs)]

pub mod ir {
    pub use cranelift_codegen::ir::{
        ExternalName, Function, InstBuilder, MemFlags, StackSlotData, StackSlotKind,
    };
}
pub use cranelift_codegen::print_errors::pretty_error;
pub use cranelift_codegen::Context;
pub use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

pub mod binemit {
    pub use crate::compiler::RelocSink as TrampolineRelocSink;
    pub use cranelift_codegen::binemit::NullTrapSink;
    pub use cranelift_codegen::binemit::{CodeOffset, NullStackmapSink, TrapSink};
}
