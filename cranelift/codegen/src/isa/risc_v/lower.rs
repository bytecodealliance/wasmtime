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

use crate::isa::risc_v::inst::*;
use crate::isa::risc_v::Riscv64Backend;

use super::lower_inst;

use regalloc::Reg;

pub mod isle;

/// Emits instruction(s) to generate the given 64-bit constant value into a newly-allocated
/// temporary register, returning that register.
fn generate_constant<C: LowerCtx<I = Inst>>(ctx: &mut C, ty: Type, c: u64) -> ValueRegs<Reg> {
    let from_bits = ty_bits(ty);
    let masked = if from_bits < 64 {
        c & ((1u64 << from_bits) - 1)
    } else {
        c
    };
    let cst_copy = ctx.alloc_tmp(ty);
    for inst in Inst::gen_constant(cst_copy, masked as u128, ty, |ty| {
        ctx.alloc_tmp(ty).only_reg().unwrap()
    })
    .into_iter()
    {
        ctx.emit(inst);
    }
    non_writable_value_regs(cst_copy)
}

/// Put the given input into possibly multiple registers, and mark it as used (side-effect).
pub(crate) fn put_input_in_regs<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    spec: InsnInput,
) -> ValueRegs<Reg> {
    let ty = ctx.input_ty(spec.insn, spec.input);
    let input = ctx.get_input_as_source_or_const(spec.insn, spec.input);

    if let Some(c) = input.constant {
        // Generate constants fresh at each use to minimize long-range register pressure.
        generate_constant(ctx, ty, c)
    } else {
        ctx.put_input_in_regs(spec.insn, spec.input)
    }
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
