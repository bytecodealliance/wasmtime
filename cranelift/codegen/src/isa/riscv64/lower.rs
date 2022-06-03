//! Lowering rules for Riscv64.
use super::lower_inst;
use crate::ir::Inst as IRInst;
use crate::isa::riscv64::inst::*;
use crate::isa::riscv64::Riscv64Backend;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::CodegenResult;
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

/// Checks for an instance of `op` feeding the given input.
pub(crate) fn maybe_input_insn<C: LowerCtx<I = Inst>>(
    c: &C,
    input: InsnInput,
    op: Opcode,
) -> Option<IRInst> {
    let inputs = c.get_input_as_source_or_const(input.insn, input.input);
    log::trace!(
        "maybe_input_insn: input {:?} has options {:?}; looking for op {:?}",
        input,
        inputs,
        op
    );
    if let Some((src_inst, _)) = inputs.inst.as_inst() {
        let data = c.data(src_inst);
        log::trace!(" -> input inst {:?}", data);
        if data.opcode() == op {
            return Some(src_inst);
        }
    }
    None
}

pub(crate) fn get_icmp_parameters<C: LowerCtx<I = Inst>>(
    c: &mut C,
    input: IRInst,
) -> (
    IntCC,
    ValueRegs<Reg>, /* x */
    ValueRegs<Reg>, /* y  */
    Type,
) {
    let condcode = c.data(input).cond_code().unwrap();
    let x = c.put_input_in_regs(input, 0);
    let y = c.put_input_in_regs(input, 1);
    let ty = c.input_ty(input, 0);
    (condcode, x, y, ty)
}

pub(crate) fn get_fcmp_parameters<C: LowerCtx<I = Inst>>(
    c: &mut C,
    input: IRInst,
) -> (FloatCC, Reg /* x */, Reg /* y  */, Type) {
    let condcode = c.data(input).fp_cond_code().unwrap();
    let x = c.put_input_in_regs(input, 0).only_reg().unwrap();
    let y = c.put_input_in_regs(input, 1).only_reg().unwrap();
    let ty = c.input_ty(input, 0);
    (condcode, x, y, ty)
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
