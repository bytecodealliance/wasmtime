//! ARM 32-bit Instruction Set Architecture.

mod abi;
mod binemit;
mod enc_tables;
mod registers;
pub mod settings;

use super::super::settings as shared_settings;
use binemit::{emit_function, CodeSink, MemoryCodeSink};
use ir;
use isa::Builder as IsaBuilder;
use isa::enc_tables::{self as shared_enc_tables, lookup_enclist, Encodings};
use isa::{EncInfo, RegClass, RegInfo, TargetIsa};
use regalloc;
use std::boxed::Box;
use std::fmt;

#[allow(dead_code)]
struct Isa {
    shared_flags: shared_settings::Flags,
    isa_flags: settings::Flags,
    cpumode: &'static [shared_enc_tables::Level1Entry<u16>],
}

/// Get an ISA builder for creating ARM32 targets.
pub fn isa_builder() -> IsaBuilder {
    IsaBuilder {
        setup: settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(
    shared_flags: shared_settings::Flags,
    builder: shared_settings::Builder,
) -> Box<TargetIsa> {
    let level1 = if shared_flags.is_compressed() {
        &enc_tables::LEVEL1_T32[..]
    } else {
        &enc_tables::LEVEL1_A32[..]
    };
    Box::new(Isa {
        isa_flags: settings::Flags::new(&shared_flags, builder),
        shared_flags,
        cpumode: level1,
    })
}

impl TargetIsa for Isa {
    fn name(&self) -> &'static str {
        "arm32"
    }

    fn flags(&self) -> &shared_settings::Flags {
        &self.shared_flags
    }

    fn register_info(&self) -> RegInfo {
        registers::INFO.clone()
    }

    fn encoding_info(&self) -> EncInfo {
        enc_tables::INFO.clone()
    }

    fn legal_encodings<'a>(
        &'a self,
        func: &'a ir::Function,
        inst: &'a ir::InstructionData,
        ctrl_typevar: ir::Type,
    ) -> Encodings<'a> {
        lookup_enclist(
            ctrl_typevar,
            inst,
            func,
            self.cpumode,
            &enc_tables::LEVEL2[..],
            &enc_tables::ENCLISTS[..],
            &enc_tables::LEGALIZE_ACTIONS[..],
            &enc_tables::RECIPE_PREDICATES[..],
            &enc_tables::INST_PREDICATES[..],
            self.isa_flags.predicate_view(),
        )
    }

    fn legalize_signature(&self, sig: &mut ir::Signature, current: bool) {
        abi::legalize_signature(sig, &self.shared_flags, current)
    }

    fn regclass_for_abi_type(&self, ty: ir::Type) -> RegClass {
        abi::regclass_for_abi_type(ty)
    }

    fn allocatable_registers(&self, func: &ir::Function) -> regalloc::RegisterSet {
        abi::allocatable_registers(func)
    }

    fn emit_inst(
        &self,
        func: &ir::Function,
        inst: ir::Inst,
        divert: &mut regalloc::RegDiversions,
        sink: &mut CodeSink,
    ) {
        binemit::emit_inst(func, inst, divert, sink)
    }

    fn emit_function(&self, func: &ir::Function, sink: &mut MemoryCodeSink) {
        emit_function(func, binemit::emit_inst, sink)
    }
}

impl fmt::Display for Isa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n{}", self.shared_flags, self.isa_flags)
    }
}
