//! Intel Instruction Set Architectures.

pub mod settings;
mod abi;
mod binemit;
mod enc_tables;
mod registers;

use binemit::{CodeSink, MemoryCodeSink, emit_function};
use super::super::settings as shared_settings;
use isa::enc_tables::{self as shared_enc_tables, lookup_enclist, Encodings};
use isa::Builder as IsaBuilder;
use isa::{TargetIsa, RegInfo, RegClass, EncInfo, RegUnit};
use self::registers::RU;
use ir;
use regalloc;
use result;
use ir::InstBuilder;
use legalizer;


#[allow(dead_code)]
struct Isa {
    shared_flags: shared_settings::Flags,
    isa_flags: settings::Flags,
    cpumode: &'static [shared_enc_tables::Level1Entry<u16>],
}

/// Get an ISA builder for creating Intel targets.
pub fn isa_builder() -> IsaBuilder {
    IsaBuilder {
        setup: settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(
    shared_flags: shared_settings::Flags,
    builder: &shared_settings::Builder,
) -> Box<TargetIsa> {
    let level1 = if shared_flags.is_64bit() {
        &enc_tables::LEVEL1_I64[..]
    } else {
        &enc_tables::LEVEL1_I32[..]
    };
    Box::new(Isa {
        isa_flags: settings::Flags::new(&shared_flags, builder),
        shared_flags,
        cpumode: level1,
    })
}

impl TargetIsa for Isa {
    fn name(&self) -> &'static str {
        "intel"
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
        dfg: &'a ir::DataFlowGraph,
        inst: &'a ir::InstructionData,
        ctrl_typevar: ir::Type,
    ) -> Encodings<'a> {
        lookup_enclist(
            ctrl_typevar,
            inst,
            dfg,
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

    fn allocatable_registers(&self, func: &ir::Function) -> regalloc::AllocatableSet {
        abi::allocatable_registers(func, &self.shared_flags)
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

    fn reloc_names(&self) -> &'static [&'static str] {
        &binemit::RELOC_NAMES
    }

    fn prologue_epilogue(&self, func: &mut ir::Function) -> result::CtonResult {
        use stack_layout::layout_stack;
        use cursor::{Cursor, EncCursor};

        let word_size = if self.flags().is_64bit() { 8 } else { 4 };
        let stack_size = layout_stack(&mut func.stack_slots, word_size)?;

        // Append frame pointer to function signature
        let rbp_arg = ir::AbiParam::special_reg(
            ir::types::I64,
            ir::ArgumentPurpose::FramePointer,
            RU::rbp as RegUnit,
        );
        func.signature.params.push(rbp_arg);

        // Append param to entry EBB
        let entry_ebb = func.layout.entry_block().expect("missing entry block");
        func.dfg.append_ebb_param(entry_ebb, ir::types::I64);

        // Find our frame pointer parameter Value
        let fp = func.special_param(ir::ArgumentPurpose::FramePointer)
            .expect("missing frame pointer");

        // Assign it a location
        func.locations[fp] = ir::ValueLoc::Reg(RU::rbp as RegUnit);

        // Insert prologue
        let mut pos = EncCursor::new(func, self).at_first_insertion_point(entry_ebb);
        pos.ins().x86_push(fp);
        pos.ins().copy_special(
            RU::rbp as RegUnit,
            RU::rsp as RegUnit,
        );
        pos.ins().adjust_sp_imm(-(stack_size as i32));

        //legalizer::legalize_function(func, &mut func.cfg, self);

        Ok(())
    }
}
