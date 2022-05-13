//! Lowering rules for AArch64.
//!
//! TODO: opportunities for better code generation:
//!
//! - Smarter use of addressing modes. Recognize a+SCALE*b patterns. Recognize
//!   pre/post-index opportunities.
//!
//! - Floating-point immediates (FIMM instruction).

use crate::ir::Inst as IRInst;
use crate::ir::Type;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::CodegenResult;

use crate::isa::riscv64::inst::*;
use crate::isa::riscv64::Riscv64Backend;

use super::lower_inst;

pub mod isle;

/// Put the given input into possibly multiple registers, and mark it as used (side-effect).
pub(crate) fn put_input_in_regs<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    spec: InsnInput,
) -> ValueRegs<Reg> {
    ctx.put_input_in_regs(spec.insn, spec.input)
}

/// Put the given input into a register, and mark it as used (side-effect).
pub(crate) fn put_input_in_reg<C: LowerCtx<I = Inst>>(ctx: &mut C, spec: InsnInput) -> Reg {
    put_input_in_regs(ctx, spec)
        .only_reg()
        .expect("Multi-register value not expected")
}

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for Riscv64Backend {
    type MInst = Inst;

    fn lower<C: LowerCtx<I = Inst>>(&self, ctx: &mut C, ir_inst: IRInst) -> CodegenResult<()> {
        lower_inst::lower_insn_to_regs(ctx, ir_inst, &self.flags, &self.isa_flags)
    }

    fn lower_branch_group<C: LowerCtx<I = Inst>>(
        &self,
        ctx: &mut C,
        branches: &[IRInst],
        targets: &[MachLabel],
    ) -> CodegenResult<()> {
        lower_inst::lower_branch(ctx, branches, targets)
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        //todo what is this??
        None
    }
}
