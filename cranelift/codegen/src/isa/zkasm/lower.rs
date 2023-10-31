//! Lowering rules for ZkASM.
use crate::ir::Inst as IRInst;
use crate::isa::zkasm::inst::*;
use crate::isa::zkasm::ZkAsmBackend;
use crate::machinst::lower::*;
use crate::machinst::*;
pub mod isle;

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for ZkAsmBackend {
    type MInst = Inst;
    type FactFlowState = ();

    fn lower(&self, ctx: &mut Lower<Inst>, ir_inst: IRInst) -> Option<InstOutput> {
        isle::lower(ctx, self, ir_inst)
    }

    fn lower_branch(
        &self,
        ctx: &mut Lower<Inst>,
        ir_inst: IRInst,
        targets: &[MachLabel],
    ) -> Option<()> {
        isle::lower_branch(ctx, self, ir_inst, targets)
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        // pinned register is a register that you want put anything in it.
        // right now zkasm not support this feature.
        None
    }
}
