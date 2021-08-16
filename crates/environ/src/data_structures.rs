#![doc(hidden)]

pub mod ir {
    pub use cranelift_codegen::binemit::StackMap;
    pub use cranelift_codegen::ir::{types, SourceLoc, TrapCode, Type};
}

pub mod entity {
    pub use cranelift_entity::{packed_option, BoxedSlice, EntityRef, EntitySet, PrimaryMap};
}

pub mod wasm {
    pub use cranelift_wasm::*;
}
