//! This module defines arm32-specific machine instruction types.

use crate::binemit::{Addend, CodeOffset, Reloc};
pub use crate::ir::condcodes::FloatCC;
pub use crate::ir::condcodes::IntCC;
use crate::ir::types::{I8, I16, I32, I64, I128};
pub use crate::ir::{MemFlagsData, Type};
use crate::isa::FunctionAlignment;
use crate::machinst::*;
use crate::{CodegenError, CodegenResult, settings};
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use regalloc2::RegClass;

pub mod emit;
pub mod regs;
pub use self::regs::*;
pub mod unwind;

#[cfg(test)]
mod emit_tests;

// Re-export EmitInfo from emit module (not here, to avoid duplicates).
pub use self::emit::EmitInfo;

use crate::isa::arm32::abi::Arm32MachineDeps;

/// Re-export the ISLE-generated MInst as Inst, matching riscv64 pattern.
pub use crate::isa::arm32::lower::isle::generated_code::MInst as Inst;

// use crate::{
//     MachInst,
//     isa::{FunctionAlignment, arm32::abi::Arm32MachineDeps},
//     machinst::{MachInstLabelUse, MachTerminator, OperandVisitor},
// };

impl Inst {
    pub fn function_alignment() -> FunctionAlignment {
        FunctionAlignment {
            minimum: 2,
            preferred: 4,
        }
    }
}

impl MachInst for Inst {
    type ABIMachineSpec = Arm32MachineDeps;
    type LabelUse = LabelUse;

    const TRAP_OPCODE: &'static [u8] = &[0x00, 0xbe];

    fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        todo!()
    }

    fn is_move(&self) -> Option<(crate::Writable<crate::Reg>, crate::Reg)> {
        todo!()
    }

    fn is_term(&self) -> MachTerminator {
        todo!()
    }

    fn is_trap(&self) -> bool {
        todo!()
    }

    fn is_args(&self) -> bool {
        todo!()
    }

    fn call_type(&self) -> crate::machinst::CallType {
        todo!()
    }

    fn is_included_in_clobbers(&self) -> bool {
        todo!()
    }

    fn is_mem_access(&self) -> bool {
        todo!()
    }

    fn gen_move(
        to_reg: crate::Writable<crate::Reg>,
        from_reg: crate::Reg,
        ty: crate::ir::Type,
    ) -> Self {
        todo!()
    }

    fn gen_dummy_use(reg: crate::Reg) -> Self {
        todo!()
    }

    fn rc_for_type(
        ty: crate::ir::Type,
    ) -> crate::CodegenResult<(&'static [crate::RegClass], &'static [crate::ir::Type])> {
        todo!()
    }

    fn canonical_type_for_rc(rc: crate::RegClass) -> crate::ir::Type {
        todo!()
    }

    fn gen_jump(target: crate::MachLabel) -> Self {
        todo!()
    }

    fn gen_nop(preferred_size: usize) -> Self {
        todo!()
    }

    fn gen_nop_units() -> std::prelude::v1::Vec<std::prelude::v1::Vec<u8>> {
        todo!()
    }

    fn worst_case_size() -> CodeOffset {
        todo!()
    }

    fn worst_case_island_growth() -> CodeOffset {
        todo!()
    }

    fn ref_type_regclass(_flags: &crate::settings::Flags) -> crate::RegClass {
        todo!()
    }

    fn is_safepoint(&self) -> bool {
        todo!()
    }

    fn function_alignment() -> FunctionAlignment {
        FunctionAlignment {
            minimum: 2,
            preferred: 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {}

impl MachInstLabelUse for LabelUse {
    const ALIGN: CodeOffset = 4;

    fn max_pos_range(self) -> CodeOffset {
        todo!()
    }

    fn max_neg_range(self) -> CodeOffset {
        todo!()
    }

    fn patch_size(self) -> CodeOffset {
        todo!()
    }

    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        todo!()
    }

    fn supports_veneer(self) -> bool {
        todo!()
    }

    fn veneer_size(self) -> CodeOffset {
        todo!()
    }

    fn worst_case_veneer_size() -> CodeOffset {
        todo!()
    }

    fn generate_veneer(self, buffer: &mut [u8], veneer_offset: CodeOffset) -> (CodeOffset, Self) {
        todo!()
    }

    fn from_reloc(reloc: crate::binemit::Reloc, addend: crate::binemit::Addend) -> Option<Self> {
        todo!()
    }
}
