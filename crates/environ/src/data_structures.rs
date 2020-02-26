#![doc(hidden)]

pub mod ir {
    pub use cranelift_codegen::ir::{
        types, AbiParam, ArgumentPurpose, Signature, SourceLoc, StackSlots, TrapCode, Type,
        ValueLabel, ValueLoc,
    };
    pub use cranelift_codegen::ValueLabelsRanges;
}

pub mod settings {
    pub use cranelift_codegen::settings::{builder, Builder, Configurable, Flags};
}

pub mod isa {
    pub use cranelift_codegen::isa::{CallConv, RegUnit, TargetFrontendConfig, TargetIsa};
}

pub mod entity {
    pub use cranelift_entity::{packed_option, BoxedSlice, EntityRef, PrimaryMap};
}

pub mod wasm {
    pub use cranelift_wasm::{
        get_vmctx_value_label, DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex,
        DefinedTableIndex, FuncIndex, Global, GlobalIndex, GlobalInit, Memory, MemoryIndex,
        PassiveElemIndex, SignatureIndex, Table, TableElementType, TableIndex,
    };
}
