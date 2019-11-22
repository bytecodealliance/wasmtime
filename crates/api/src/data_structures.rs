pub(crate) mod ir {
    pub(crate) use cranelift_codegen::ir::{
        types, AbiParam, ArgumentPurpose, ExternalName, Function, InstBuilder, MemFlags, Signature,
        StackSlotData, StackSlotKind, TrapCode, Type,
    };
}

pub(crate) mod settings {
    pub(crate) use cranelift_codegen::settings::{builder, Flags};
}

pub(crate) use cranelift_codegen::isa::{CallConv, TargetIsa};
pub(crate) use cranelift_codegen::Context;
pub(crate) use cranelift_entity::{EntityRef, PrimaryMap};

pub(crate) mod wasm {
    pub(crate) use cranelift_wasm::{
        DefinedFuncIndex, DefinedTableIndex, FuncIndex, Global, GlobalInit, Memory, Table,
        TableElementType,
    };
}

pub(crate) mod binemit {
    pub(crate) use cranelift_codegen::binemit::{NullStackmapSink, NullTrapSink};

    pub use cranelift_codegen::{binemit, ir};

    /// We don't expect trampoline compilation to produce any relocations, so
    /// this `RelocSink` just asserts that it doesn't recieve any.
    pub(crate) struct TrampolineRelocSink {}

    impl binemit::RelocSink for TrampolineRelocSink {
        fn reloc_ebb(
            &mut self,
            _offset: binemit::CodeOffset,
            _reloc: binemit::Reloc,
            _ebb_offset: binemit::CodeOffset,
        ) {
            panic!("trampoline compilation should not produce ebb relocs");
        }
        fn reloc_external(
            &mut self,
            _offset: binemit::CodeOffset,
            _reloc: binemit::Reloc,
            _name: &ir::ExternalName,
            _addend: binemit::Addend,
        ) {
            panic!("trampoline compilation should not produce external symbol relocs");
        }
        fn reloc_constant(
            &mut self,
            _code_offset: binemit::CodeOffset,
            _reloc: binemit::Reloc,
            _constant_offset: ir::ConstantOffset,
        ) {
            panic!("trampoline compilation should not produce constant relocs");
        }
        fn reloc_jt(
            &mut self,
            _offset: binemit::CodeOffset,
            _reloc: binemit::Reloc,
            _jt: ir::JumpTable,
        ) {
            panic!("trampoline compilation should not produce jump table relocs");
        }
    }
}

pub(crate) use cranelift_codegen::print_errors::pretty_error;
pub(crate) use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

pub(crate) fn native_isa_builder() -> cranelift_codegen::isa::Builder {
    cranelift_native::builder().expect("host machine is not a supported target")
}
