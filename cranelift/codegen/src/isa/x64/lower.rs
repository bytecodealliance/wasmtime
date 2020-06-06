//! Lowering rules for X64.

#![allow(dead_code)]
#![allow(non_snake_case)]

use log::trace;
use regalloc::{Reg, RegClass, Writable};

use crate::ir::types;
use crate::ir::types::*;
use crate::ir::Inst as IRInst;
use crate::ir::{condcodes::IntCC, InstructionData, Opcode, Type};

use crate::machinst::lower::*;
use crate::machinst::*;
use crate::result::CodegenResult;

use crate::isa::x64::inst::args::*;
use crate::isa::x64::inst::*;
use crate::isa::x64::X64Backend;

/// Context passed to all lowering functions.
type Ctx<'a> = &'a mut dyn LowerCtx<I = Inst>;

//=============================================================================
// Helpers for instruction lowering.

fn is_int_ty(ty: Type) -> bool {
    match ty {
        types::I8 | types::I16 | types::I32 | types::I64 => true,
        _ => false,
    }
}

fn int_ty_is_64(ty: Type) -> bool {
    match ty {
        types::I8 | types::I16 | types::I32 => false,
        types::I64 => true,
        _ => panic!("type {} is none of I8, I16, I32 or I64", ty),
    }
}

fn flt_ty_is_64(ty: Type) -> bool {
    match ty {
        types::F32 => false,
        types::F64 => true,
        _ => panic!("type {} is none of F32, F64", ty),
    }
}

fn int_ty_to_sizeB(ty: Type) -> u8 {
    match ty {
        types::I8 => 1,
        types::I16 => 2,
        types::I32 => 4,
        types::I64 => 8,
        _ => panic!("ity_to_sizeB"),
    }
}

fn iri_to_u64_immediate<'a>(ctx: Ctx<'a>, iri: IRInst) -> Option<u64> {
    let inst_data = ctx.data(iri);
    if inst_data.opcode() == Opcode::Null {
        Some(0)
    } else {
        match inst_data {
            &InstructionData::UnaryImm { opcode: _, imm } => {
                // Only has Into for i64; we use u64 elsewhere, so we cast.
                let imm: i64 = imm.into();
                Some(imm as u64)
            }
            _ => None,
        }
    }
}

fn inst_condcode(data: &InstructionData) -> IntCC {
    match data {
        &InstructionData::IntCond { cond, .. }
        | &InstructionData::BranchIcmp { cond, .. }
        | &InstructionData::IntCompare { cond, .. }
        | &InstructionData::IntCondTrap { cond, .. }
        | &InstructionData::BranchInt { cond, .. }
        | &InstructionData::IntSelect { cond, .. }
        | &InstructionData::IntCompareImm { cond, .. } => cond,
        _ => panic!("inst_condcode(x64): unhandled: {:?}", data),
    }
}

fn input_to_reg<'a>(ctx: Ctx<'a>, iri: IRInst, input: usize) -> Reg {
    let inputs = ctx.get_input(iri, input);
    ctx.use_input_reg(inputs);
    inputs.reg
}

fn output_to_reg<'a>(ctx: Ctx<'a>, iri: IRInst, output: usize) -> Writable<Reg> {
    ctx.get_output(iri, output)
}

//=============================================================================
// Top-level instruction lowering entry point, for one instruction.

/// Actually codegen an instruction's results into registers.
fn lower_insn_to_regs<'a>(ctx: Ctx<'a>, inst: IRInst) {
    let op = ctx.data(inst).opcode();
    let ty = if ctx.num_outputs(inst) == 1 {
        Some(ctx.output_ty(inst, 0))
    } else {
        None
    };

    // This is all outstandingly feeble.  TODO: much better!
    match op {
        Opcode::Iconst => {
            if let Some(w64) = iri_to_u64_immediate(ctx, inst) {
                // Get exactly the bit pattern in 'w64' into the dest.  No
                // monkeying with sign extension etc.
                let dst_is_64 = w64 > 0xFFFF_FFFF;
                let dst = output_to_reg(ctx, inst, 0);
                ctx.emit(Inst::imm_r(dst_is_64, w64, dst));
            } else {
                unimplemented!();
            }
        }

        Opcode::Iadd | Opcode::Isub => {
            let dst = output_to_reg(ctx, inst, 0);
            let lhs = input_to_reg(ctx, inst, 0);
            let rhs = input_to_reg(ctx, inst, 1);
            let is_64 = int_ty_is_64(ty.unwrap());
            let alu_op = if op == Opcode::Iadd {
                AluRmiROpcode::Add
            } else {
                AluRmiROpcode::Sub
            };
            ctx.emit(Inst::mov_r_r(true, lhs, dst));
            ctx.emit(Inst::alu_rmi_r(is_64, alu_op, RegMemImm::reg(rhs), dst));
        }

        Opcode::Ishl | Opcode::Ushr | Opcode::Sshr => {
            // TODO: implement imm shift value into insn
            let dst_ty = ctx.output_ty(inst, 0);
            assert_eq!(ctx.input_ty(inst, 0), dst_ty);
            assert!(dst_ty == types::I32 || dst_ty == types::I64);

            let lhs = input_to_reg(ctx, inst, 0);
            let rhs = input_to_reg(ctx, inst, 1);
            let dst = output_to_reg(ctx, inst, 0);

            let shift_kind = match op {
                Opcode::Ishl => ShiftKind::Left,
                Opcode::Ushr => ShiftKind::RightZ,
                Opcode::Sshr => ShiftKind::RightS,
                _ => unreachable!(),
            };

            let is_64 = dst_ty == types::I64;
            let w_rcx = Writable::from_reg(regs::rcx());
            ctx.emit(Inst::mov_r_r(true, lhs, dst));
            ctx.emit(Inst::mov_r_r(true, rhs, w_rcx));
            ctx.emit(Inst::shift_r(is_64, shift_kind, None /*%cl*/, dst));
        }

        Opcode::Uextend | Opcode::Sextend => {
            // TODO: this is all extremely lame, all because Mov{ZX,SX}_M_R
            // don't accept a register source operand.  They should be changed
            // so as to have _RM_R form.
            // TODO2: if the source operand is a load, incorporate that.
            let zero_extend = op == Opcode::Uextend;
            let src_ty = ctx.input_ty(inst, 0);
            let dst_ty = ctx.output_ty(inst, 0);
            let src = input_to_reg(ctx, inst, 0);
            let dst = output_to_reg(ctx, inst, 0);

            ctx.emit(Inst::mov_r_r(true, src, dst));
            match (src_ty, dst_ty, zero_extend) {
                (types::I8, types::I64, false) => {
                    ctx.emit(Inst::shift_r(true, ShiftKind::Left, Some(56), dst));
                    ctx.emit(Inst::shift_r(true, ShiftKind::RightS, Some(56), dst));
                }
                _ => unimplemented!(),
            }
        }

        Opcode::FallthroughReturn | Opcode::Return => {
            for i in 0..ctx.num_inputs(inst) {
                let src_reg = input_to_reg(ctx, inst, i);
                let retval_reg = ctx.retval(i);
                if src_reg.get_class() == RegClass::I64 {
                    ctx.emit(Inst::mov_r_r(true, src_reg, retval_reg));
                } else if src_reg.get_class() == RegClass::V128 {
                    ctx.emit(Inst::xmm_r_r(SseOpcode::Movsd, src_reg, retval_reg));
                }
            }
            // N.B.: the Ret itself is generated by the ABI.
        }

        Opcode::Fadd | Opcode::Fsub | Opcode::Fmul | Opcode::Fdiv => {
            let dst = output_to_reg(ctx, inst, 0);
            let lhs = input_to_reg(ctx, inst, 0);
            let rhs = input_to_reg(ctx, inst, 1);
            let is_64 = flt_ty_is_64(ty.unwrap());
            if !is_64 {
                let sse_op = match op {
                    Opcode::Fadd => SseOpcode::Addss,
                    Opcode::Fsub => SseOpcode::Subss,
                    Opcode::Fmul => SseOpcode::Mulss,
                    Opcode::Fdiv => SseOpcode::Divss,
                    // TODO Fmax, Fmin.
                    _ => unimplemented!(),
                };
                ctx.emit(Inst::xmm_r_r(SseOpcode::Movss, lhs, dst));
                ctx.emit(Inst::xmm_rm_r(sse_op, RegMem::reg(rhs), dst));
            } else {
                unimplemented!("unimplemented lowering for opcode {:?}", op);
            }
        }
        Opcode::Fcopysign => {
            let dst = output_to_reg(ctx, inst, 0);
            let lhs = input_to_reg(ctx, inst, 0);
            let rhs = input_to_reg(ctx, inst, 1);
            if !flt_ty_is_64(ty.unwrap()) {
                // movabs   0x8000_0000, tmp_gpr1
                // movd     tmp_gpr1, tmp_xmm1
                // movaps   tmp_xmm1, dst
                // andnps   src_1, dst
                // movss    src_2, tmp_xmm2
                // andps    tmp_xmm1, tmp_xmm2
                // orps     tmp_xmm2, dst
                let tmp_gpr1 = ctx.alloc_tmp(RegClass::I64, I32);
                let tmp_xmm1 = ctx.alloc_tmp(RegClass::V128, F32);
                let tmp_xmm2 = ctx.alloc_tmp(RegClass::V128, F32);
                ctx.emit(Inst::imm_r(true, 0x8000_0000, tmp_gpr1));
                ctx.emit(Inst::xmm_mov_rm_r(
                    SseOpcode::Movd,
                    RegMem::reg(tmp_gpr1.to_reg()),
                    tmp_xmm1,
                ));
                ctx.emit(Inst::xmm_mov_rm_r(
                    SseOpcode::Movaps,
                    RegMem::reg(tmp_xmm1.to_reg()),
                    dst,
                ));
                ctx.emit(Inst::xmm_rm_r(SseOpcode::Andnps, RegMem::reg(lhs), dst));
                ctx.emit(Inst::xmm_mov_rm_r(
                    SseOpcode::Movss,
                    RegMem::reg(rhs),
                    tmp_xmm2,
                ));
                ctx.emit(Inst::xmm_rm_r(
                    SseOpcode::Andps,
                    RegMem::reg(tmp_xmm1.to_reg()),
                    tmp_xmm2,
                ));
                ctx.emit(Inst::xmm_rm_r(
                    SseOpcode::Orps,
                    RegMem::reg(tmp_xmm2.to_reg()),
                    dst,
                ));
            } else {
                unimplemented!("{:?} for non 32-bit destination is not supported", op);
            }
        }
        Opcode::IaddImm
        | Opcode::ImulImm
        | Opcode::UdivImm
        | Opcode::SdivImm
        | Opcode::UremImm
        | Opcode::SremImm
        | Opcode::IrsubImm
        | Opcode::IaddCin
        | Opcode::IaddIfcin
        | Opcode::IaddCout
        | Opcode::IaddIfcout
        | Opcode::IaddCarry
        | Opcode::IaddIfcarry
        | Opcode::IsubBin
        | Opcode::IsubIfbin
        | Opcode::IsubBout
        | Opcode::IsubIfbout
        | Opcode::IsubBorrow
        | Opcode::IsubIfborrow
        | Opcode::BandImm
        | Opcode::BorImm
        | Opcode::BxorImm
        | Opcode::RotlImm
        | Opcode::RotrImm
        | Opcode::IshlImm
        | Opcode::UshrImm
        | Opcode::SshrImm => {
            panic!("ALU+imm and ALU+carry ops should not appear here!");
        }
        _ => unimplemented!("unimplemented lowering for opcode {:?}", op),
    }
}

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for X64Backend {
    type MInst = Inst;

    fn lower<C: LowerCtx<I = Inst>>(&self, ctx: &mut C, ir_inst: IRInst) -> CodegenResult<()> {
        lower_insn_to_regs(ctx, ir_inst);
        Ok(())
    }

    fn lower_branch_group<C: LowerCtx<I = Inst>>(
        &self,
        ctx: &mut C,
        branches: &[IRInst],
        targets: &[MachLabel],
        fallthrough: Option<MachLabel>,
    ) -> CodegenResult<()> {
        // A block should end with at most two branches. The first may be a
        // conditional branch; a conditional branch can be followed only by an
        // unconditional branch or fallthrough. Otherwise, if only one branch,
        // it may be an unconditional branch, a fallthrough, a return, or a
        // trap. These conditions are verified by `is_ebb_basic()` during the
        // verifier pass.
        assert!(branches.len() <= 2);

        if branches.len() == 2 {
            // Must be a conditional branch followed by an unconditional branch.
            let op0 = ctx.data(branches[0]).opcode();
            let op1 = ctx.data(branches[1]).opcode();

            trace!(
                "lowering two-branch group: opcodes are {:?} and {:?}",
                op0,
                op1
            );
            assert!(op1 == Opcode::Jump || op1 == Opcode::Fallthrough);

            let taken = BranchTarget::Label(targets[0]);
            let not_taken = match op1 {
                Opcode::Jump => BranchTarget::Label(targets[1]),
                Opcode::Fallthrough => BranchTarget::Label(fallthrough.unwrap()),
                _ => unreachable!(), // assert above.
            };

            match op0 {
                Opcode::Brz | Opcode::Brnz => {
                    let src_ty = ctx.input_ty(branches[0], 0);
                    if is_int_ty(src_ty) {
                        let src = input_to_reg(ctx, branches[0], 0);
                        let cc = match op0 {
                            Opcode::Brz => CC::Z,
                            Opcode::Brnz => CC::NZ,
                            _ => unreachable!(),
                        };
                        let sizeB = int_ty_to_sizeB(src_ty);
                        ctx.emit(Inst::cmp_rmi_r(sizeB, RegMemImm::imm(0), src));
                        ctx.emit(Inst::jmp_cond_symm(cc, taken, not_taken));
                    } else {
                        unimplemented!("brz/brnz with non-int type");
                    }
                }

                Opcode::BrIcmp => {
                    let src_ty = ctx.input_ty(branches[0], 0);
                    if is_int_ty(src_ty) {
                        let lhs = input_to_reg(ctx, branches[0], 0);
                        let rhs = input_to_reg(ctx, branches[0], 1);
                        let cc = CC::from_intcc(inst_condcode(ctx.data(branches[0])));
                        let byte_size = int_ty_to_sizeB(src_ty);
                        // FIXME verify rSR vs rSL ordering
                        ctx.emit(Inst::cmp_rmi_r(byte_size, RegMemImm::reg(rhs), lhs));
                        ctx.emit(Inst::jmp_cond_symm(cc, taken, not_taken));
                    } else {
                        unimplemented!("bricmp with non-int type");
                    }
                }

                // TODO: Brif/icmp, Brff/icmp, jump tables
                _ => unimplemented!("branch opcode"),
            }
        } else {
            assert!(branches.len() == 1);

            // Must be an unconditional branch or trap.
            let op = ctx.data(branches[0]).opcode();
            match op {
                Opcode::Jump => {
                    ctx.emit(Inst::jmp_known(BranchTarget::Label(targets[0])));
                }
                Opcode::Fallthrough => {
                    ctx.emit(Inst::jmp_known(BranchTarget::Label(targets[0])));
                }
                Opcode::Trap => {
                    unimplemented!("trap");
                }
                _ => panic!("Unknown branch type!"),
            }
        }

        Ok(())
    }
}
