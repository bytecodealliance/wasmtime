//! ARM32-specific lowering logic.
use crate::ir::Inst as IRInst;
use crate::isa::arm32::Arm32Backend;
// Re-export types from inst/ so ISLE generated code can find them via `use super::*`.
pub use crate::isa::arm32::inst::{Inst, regs};
use crate::machinst::{Lower, LowerBackend, Reg};

/// Re-export ISLE module for use from mod.rs.
pub(crate) mod isle;

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for Arm32Backend {
    type MInst = Inst;

    fn lower(&self, ctx: &mut Lower<Inst>, ir_inst: IRInst) -> Option<crate::machinst::InstOutput> {
        isle::lower(ctx, self, ir_inst)
    }

    fn lower_branch(
        &self,
        ctx: &mut Lower<Inst>,
        ir_inst: IRInst,
        targets: &[crate::machinst::MachLabel],
    ) -> Option<()> {
        isle::lower_branch(ctx, self, ir_inst, targets)
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        // not yet supported on arm32
        None
    }
}
