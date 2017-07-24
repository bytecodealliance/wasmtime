//! ARM 64-bit Instruction Set Architecture.

pub mod settings;
mod abi;
mod binemit;
mod enc_tables;
mod registers;

use binemit::{CodeSink, MemoryCodeSink, emit_function};
use super::super::settings as shared_settings;
use isa::enc_tables::{lookup_enclist, Encodings};
use isa::Builder as IsaBuilder;
use isa::{TargetIsa, RegInfo, RegClass, EncInfo, Legalize};
use ir;
use regalloc;

#[allow(dead_code)]
struct Isa {
    shared_flags: shared_settings::Flags,
    isa_flags: settings::Flags,
}

/// Get an ISA builder for creating ARM64 targets.
pub fn isa_builder() -> IsaBuilder {
    IsaBuilder {
        setup: settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(shared_flags: shared_settings::Flags,
                   builder: &shared_settings::Builder)
                   -> Box<TargetIsa> {
    Box::new(Isa {
                 isa_flags: settings::Flags::new(&shared_flags, builder),
                 shared_flags,
             })
}

impl TargetIsa for Isa {
    fn name(&self) -> &'static str {
        "arm64"
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

    fn legal_encodings<'a>(&'a self,
                           _dfg: &'a ir::DataFlowGraph,
                           inst: &'a ir::InstructionData,
                           ctrl_typevar: ir::Type)
                           -> Result<Encodings<'a>, Legalize> {
        lookup_enclist(ctrl_typevar,
                       inst.opcode(),
                       &enc_tables::LEVEL1_A64[..],
                       &enc_tables::LEVEL2[..])
                .and_then(|enclist_offset| {
                              Ok(Encodings::new(enclist_offset,
                                                &enc_tables::ENCLISTS[..],
                                                &enc_tables::RECIPE_PREDICATES[..],
                                                &enc_tables::INST_PREDICATES[..],
                                                inst,
                                                self.isa_flags.predicate_view()))
                          })
    }

    fn legalize_signature(&self, sig: &mut ir::Signature, current: bool) {
        abi::legalize_signature(sig, &self.shared_flags, current)
    }

    fn regclass_for_abi_type(&self, ty: ir::Type) -> RegClass {
        abi::regclass_for_abi_type(ty)
    }

    fn allocatable_registers(&self, func: &ir::Function) -> regalloc::AllocatableSet {
        abi::allocatable_registers(func)
    }

    fn emit_inst(&self,
                 func: &ir::Function,
                 inst: ir::Inst,
                 divert: &mut regalloc::RegDiversions,
                 sink: &mut CodeSink) {
        binemit::emit_inst(func, inst, divert, sink)
    }

    fn emit_function(&self, func: &ir::Function, sink: &mut MemoryCodeSink) {
        emit_function(func, binemit::emit_inst, sink)
    }

    fn reloc_names(&self) -> &'static [&'static str] {
        &binemit::RELOC_NAMES
    }
}
