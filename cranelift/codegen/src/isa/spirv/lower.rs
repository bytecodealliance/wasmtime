use crate::result::{CodegenResult, CodegenError};
use crate::isa::spirv::SpirvBackend;
use crate::machinst::lower::*;
use crate::machinst::buffer::MachLabel;
use crate::ir::Inst as IRInst;
use super::inst::Inst;

impl LowerBackend for SpirvBackend {
    type MInst = Inst;

    fn lower<C: LowerCtx<I = Self::MInst>>(&self, ctx: &mut C, inst: IRInst) -> CodegenResult<()> {
        dbg!(inst);
        let op = ctx.data(inst).opcode();
        dbg!(op);
        Ok(())
        //Err(CodegenError::Unsupported(format!("lower")))
    }

    fn lower_branch_group<C: LowerCtx<I = Self::MInst>>(
        &self,
        ctx: &mut C,
        insts: &[IRInst],
        targets: &[MachLabel],
        fallthrough: Option<MachLabel>,
    ) -> CodegenResult<()> {
        Err(CodegenError::Unsupported(format!("lower_branch_group")))
    }
}