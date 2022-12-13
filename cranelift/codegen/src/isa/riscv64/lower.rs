//! Lowering rules for Riscv64.
use crate::ir::Inst as IRInst;
use crate::isa::riscv64::inst::*;
use crate::isa::riscv64::Riscv64Backend;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::CodegenResult;
pub mod isle;

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for Riscv64Backend {
    type MInst = Inst;

    fn lower(&self, ctx: &mut Lower<Inst>, ir_inst: IRInst) -> CodegenResult<InstOutput> {
        if let Some(temp_regs) = super::lower::isle::lower(ctx, self, ir_inst) {
            return Ok(temp_regs);
        }

        let ty = if ctx.num_outputs(ir_inst) > 0 {
            Some(ctx.output_ty(ir_inst, 0))
        } else {
            None
        };

        unreachable!(
            "not implemented in ISLE: inst = `{}`, type = `{:?}`",
            ctx.dfg().display_inst(ir_inst),
            ty
        );
    }

    fn lower_branch_group(
        &self,
        ctx: &mut Lower<Inst>,
        branches: &[IRInst],
        targets: &[MachLabel],
    ) -> CodegenResult<()> {
        // A block should end with at most two branches. The first may be a
        // conditional branch; a conditional branch can be followed only by an
        // unconditional branch or fallthrough. Otherwise, if only one branch,
        // it may be an unconditional branch, a fallthrough, a return, or a
        // trap. These conditions are verified by `is_ebb_basic()` during the
        // verifier pass.
        assert!(branches.len() <= 2);
        if branches.len() == 2 {
            let op1 = ctx.data(branches[1]).opcode();
            assert!(op1 == Opcode::Jump);
        }

        // Lower the first branch in ISLE.  This will automatically handle
        // the second branch (if any) by emitting a two-way conditional branch.
        if let Some(temp_regs) = super::lower::isle::lower_branch(ctx, self, branches[0], targets) {
            assert!(temp_regs.len() == 0);
            return Ok(());
        }

        unreachable!(
            "not implemented in ISLE: branch = `{}`",
            ctx.dfg().display_inst(branches[0]),
        );
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        // pinned register is a register that you want put anything in it.
        // right now riscv64 not support this feature.
        None
    }
}
