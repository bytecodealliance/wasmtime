pub(crate) mod ir {
    pub(crate) use cranelift_codegen::ir::{types, AbiParam, ArgumentPurpose, Signature, Type};
}

pub(crate) mod settings {
    pub(crate) use cranelift_codegen::settings::{builder, Flags};
}

pub(crate) use cranelift_codegen::isa::CallConv;
pub(crate) use cranelift_entity::{EntityRef, PrimaryMap};

pub(crate) mod wasm {
    pub(crate) use cranelift_wasm::{
        DefinedFuncIndex, DefinedTableIndex, FuncIndex, Global, GlobalInit, Memory, Table,
        TableElementType,
    };
}

pub(crate) fn native_isa_builder() -> cranelift_codegen::isa::Builder {
    cranelift_native::builder().expect("host machine is not a supported target")
}

pub(crate) fn native_isa_call_conv() -> CallConv {
    use target_lexicon::HOST;
    CallConv::triple_default(&HOST)
}
