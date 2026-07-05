//! This module defines arm32-specific machine instruction types.

use crate::alloc::{borrow::ToOwned, vec::Vec};
use crate::binemit::CodeOffset;
pub use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::types::{I8, I16, I32, I64, I128, Type};
use crate::isa::{FunctionAlignment, arm32};
use crate::machinst::{
    ArgPair, CallType, MachInst, MachInstLabelUse, MachTerminator, OperandVisitor,
    OperandVisitorImpl, Reg, RegClass, RetPair, Writable,
};
use crate::{CodegenError, CodegenResult};

pub mod emit;
pub mod regs;
pub mod unwind;

#[cfg(test)]
mod emit_tests;

// Re-export EmitInfo from emit module (not here, to avoid duplicates).
pub use self::emit::EmitInfo;

use arm32::abi::Arm32MachineDeps;

/// Re-export the ISLE-generated MInst as Inst, matching riscv64 pattern.
pub use arm32::lower::isle::generated_code::MInst as Inst;

impl Inst {
    pub fn function_alignment() -> FunctionAlignment {
        FunctionAlignment {
            minimum: 2,
            preferred: 4,
        }
    }
}

// ===========================================================================
// Operand collection for register allocation.
// ===========================================================================

/// Collect def/use operands from an instruction into the collector,
/// so regalloc sees correct operand information. Model on riscv64_get_operands.
fn arm32_get_operands(inst: &mut Inst, collector: &mut impl OperandVisitor) {
    match inst {
        // Ret — no operands (LR used as implicit return target).
        Inst::Ret => {}

        // Args — defines incoming argument vregs, each fixed to its arg preg.
        Inst::Args { args } => {
            for ArgPair { vreg, preg } in args {
                collector.reg_fixed_def(vreg, *preg);
            }
        }

        // Push {Rn, ...} — uses SP implicitly. No explicit register operands.
        Inst::Push { rs: _ } => {}

        // Pop {Rt, ...} — uses SP implicitly. No explicit register operands.
        Inst::Pop { rt: _ } => {}

        // Rets — constrains vregs to specific return registers.
        Inst::Rets { rets } => {
            for RetPair { vreg, preg } in rets {
                collector.reg_fixed_use(vreg, *preg);
            }
        }
    }
}

impl MachInst for Inst {
    type ABIMachineSpec = Arm32MachineDeps;
    type LabelUse = LabelUse;

    const TRAP_OPCODE: &'static [u8] = &[0x00, 0xbe];

    fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        arm32_get_operands(self, collector);
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        match self {
            _ => None,
        }
    }

    fn is_term(&self) -> MachTerminator {
        match self {
            Inst::Ret => MachTerminator::Ret,
            Inst::Rets { .. } => MachTerminator::Ret,
            _ => MachTerminator::None,
        }
    }

    fn is_trap(&self) -> bool {
        false
    }

    fn is_args(&self) -> bool {
        matches!(self, Inst::Args { .. })
    }

    fn call_type(&self) -> CallType {
        match self {
            Inst::Ret => CallType::Regular,
            _ => CallType::None,
        }
    }

    fn is_included_in_clobbers(&self) -> bool {
        !matches!(self, Inst::Ret)
    }

    fn is_mem_access(&self) -> bool {
        todo!()
    }

    fn gen_move(_to_reg: Writable<crate::Reg>, _from_reg: crate::Reg, _ty: Type) -> Self {
        todo!()
    }

    fn gen_dummy_use(_reg: crate::Reg) -> Self {
        todo!()
    }

    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> {
        match ty {
            I8 => Ok((&[RegClass::Int], &[I8])),
            I16 => Ok((&[RegClass::Int], &[I16])),
            I32 => Ok((&[RegClass::Int], &[I32])),
            I64 => Ok((&[RegClass::Int, RegClass::Int], &[I32, I32])),
            I128 => Err(CodegenError::Unsupported(
                "i128 is not supported on ARM32 (deferred)".to_owned(),
            )),
            _ if ty.is_vector() => Err(CodegenError::Unsupported(format!(
                "Vector types are not supported on ARM32: {ty}"
            ))),
            _ => Err(CodegenError::Unsupported(format!(
                "Unexpected SSA-value type: {ty}"
            ))),
        }
    }

    fn canonical_type_for_rc(_rc: RegClass) -> Type {
        todo!()
    }

    fn gen_jump(_target: crate::MachLabel) -> Self {
        todo!()
    }

    fn gen_nop(_preferred_size: usize) -> Self {
        todo!()
    }

    fn gen_nop_units() -> Vec<Vec<u8>> {
        vec![vec![0xc0, 0x46]] // ARM Thumb NOP placeholder
    }

    fn worst_case_size() -> CodeOffset {
        // movw + movt (MovImm) = two wide Thumb-2 instructions = 8 bytes.
        8
    }

    fn worst_case_island_growth() -> CodeOffset {
        // Same as worst_case_size since the largest single construct is a MovImm pair.
        8
    }

    fn ref_type_regclass(_flags: &crate::settings::Flags) -> RegClass {
        todo!()
    }

    fn is_safepoint(&self) -> bool {
        matches!(self, Inst::Ret)
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

    fn patch(self, _buffer: &mut [u8], _use_offset: CodeOffset, _label_offset: CodeOffset) {
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

    fn generate_veneer(self, _buffer: &mut [u8], _veneer_offset: CodeOffset) -> (CodeOffset, Self) {
        todo!()
    }

    fn from_reloc(_reloc: crate::binemit::Reloc, _addend: crate::binemit::Addend) -> Option<Self> {
        todo!()
    }
}
