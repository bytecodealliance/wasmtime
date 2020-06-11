//! Lowering rules for X64.

#![allow(dead_code)]
#![allow(non_snake_case)]

use regalloc::{Reg, RegClass, Writable};

use crate::ir::condcodes::IntCC;
use crate::ir::types;
use crate::ir::Inst as IRInst;
use crate::ir::{InstructionData, Opcode, Type};

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

fn intCC_to_x64_CC(cc: IntCC) -> CC {
    match cc {
        IntCC::Equal => CC::Z,
        IntCC::NotEqual => CC::NZ,
        IntCC::SignedGreaterThanOrEqual => CC::NL,
        IntCC::SignedGreaterThan => CC::NLE,
        IntCC::SignedLessThanOrEqual => CC::LE,
        IntCC::SignedLessThan => CC::L,
        IntCC::UnsignedGreaterThanOrEqual => CC::NB,
        IntCC::UnsignedGreaterThan => CC::NBE,
        IntCC::UnsignedLessThanOrEqual => CC::BE,
        IntCC::UnsignedLessThan => CC::B,
        IntCC::Overflow => CC::O,
        IntCC::NotOverflow => CC::NO,
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
fn lower_insn_to_regs<'a>(ctx: Ctx<'a>, iri: IRInst) {
    let op = ctx.data(iri).opcode();
    let ty = if ctx.num_outputs(iri) == 1 {
        Some(ctx.output_ty(iri, 0))
    } else {
        None
    };

    // This is all outstandingly feeble.  TODO: much better!
    match op {
        Opcode::Iconst => {
            if let Some(w64) = iri_to_u64_immediate(ctx, iri) {
                // Get exactly the bit pattern in 'w64' into the dest.  No
                // monkeying with sign extension etc.
                let dstIs64 = w64 > 0xFFFF_FFFF;
                let regD = output_to_reg(ctx, iri, 0);
                ctx.emit(Inst::imm_r(dstIs64, w64, regD));
            } else {
                unimplemented!();
            }
        }

        Opcode::Iadd | Opcode::Isub => {
            let regD = output_to_reg(ctx, iri, 0);
            let regL = input_to_reg(ctx, iri, 0);
            let regR = input_to_reg(ctx, iri, 1);
            let is64 = int_ty_is_64(ty.unwrap());
            let how = if op == Opcode::Iadd {
                RMI_R_Op::Add
            } else {
                RMI_R_Op::Sub
            };
            ctx.emit(Inst::mov_r_r(true, regL, regD));
            ctx.emit(Inst::alu_rmi_r(is64, how, RMI::reg(regR), regD));
        }

        Opcode::Ishl | Opcode::Ushr | Opcode::Sshr => {
            // TODO: implement imm shift value into insn
            let tySL = ctx.input_ty(iri, 0);
            let tyD = ctx.output_ty(iri, 0); // should be the same as tySL
            let regSL = input_to_reg(ctx, iri, 0);
            let regSR = input_to_reg(ctx, iri, 1);
            let regD = output_to_reg(ctx, iri, 0);
            if tyD == tySL && (tyD == types::I32 || tyD == types::I64) {
                let how = match op {
                    Opcode::Ishl => ShiftKind::Left,
                    Opcode::Ushr => ShiftKind::RightZ,
                    Opcode::Sshr => ShiftKind::RightS,
                    _ => unreachable!(),
                };
                let is64 = tyD == types::I64;
                let r_rcx = regs::rcx();
                let w_rcx = Writable::<Reg>::from_reg(r_rcx);
                ctx.emit(Inst::mov_r_r(true, regSL, regD));
                ctx.emit(Inst::mov_r_r(true, regSR, w_rcx));
                ctx.emit(Inst::shift_r(is64, how, None /*%cl*/, regD));
            } else {
                unimplemented!()
            }
        }

        Opcode::Uextend | Opcode::Sextend => {
            // TODO: this is all extremely lame, all because Mov{ZX,SX}_M_R
            // don't accept a register source operand.  They should be changed
            // so as to have _RM_R form.
            // TODO2: if the source operand is a load, incorporate that.
            let isZX = op == Opcode::Uextend;
            let tyS = ctx.input_ty(iri, 0);
            let tyD = ctx.output_ty(iri, 0);
            let regS = input_to_reg(ctx, iri, 0);
            let regD = output_to_reg(ctx, iri, 0);
            ctx.emit(Inst::mov_r_r(true, regS, regD));
            match (tyS, tyD, isZX) {
                (types::I8, types::I64, false) => {
                    ctx.emit(Inst::shift_r(true, ShiftKind::Left, Some(56), regD));
                    ctx.emit(Inst::shift_r(true, ShiftKind::RightS, Some(56), regD));
                }
                _ => unimplemented!(),
            }
        }

        Opcode::FallthroughReturn | Opcode::Return => {
            for i in 0..ctx.num_inputs(iri) {
                let src_reg = input_to_reg(ctx, iri, i);
                let retval_reg = ctx.retval(i);
                if src_reg.get_class() == RegClass::I64 {
                    ctx.emit(Inst::mov_r_r(true, src_reg, retval_reg));
                } else if src_reg.get_class() == RegClass::V128 {
                    ctx.emit(Inst::xmm_r_r(SSE_Op::SSE2_Movsd, src_reg, retval_reg));
                }
            }
            // N.B.: the Ret itself is generated by the ABI.
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
        Opcode::Fadd | Opcode::Fsub => {
            let regD = output_to_reg(ctx, iri, 0);
            let regL = input_to_reg(ctx, iri, 0);
            let regR = input_to_reg(ctx, iri, 1);
            let is64 = flt_ty_is_64(ty.unwrap());
            if !is64 {
                let inst = if op == Opcode::Fadd {
                    SSE_Op::SSE_Addss
                } else {
                    SSE_Op::SSE_Subss
                };
                ctx.emit(Inst::xmm_r_r(SSE_Op::SSE_Movss, regL, regD));
                ctx.emit(Inst::xmm_rm_r(inst, RM::reg(regR), regD));
            } else {
                unimplemented!("unimplemented lowering for opcode {:?}", op);
            }
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

        let mut unimplemented = false;

        if branches.len() == 2 {
            // Must be a conditional branch followed by an unconditional branch.
            let op0 = ctx.data(branches[0]).opcode();
            let op1 = ctx.data(branches[1]).opcode();

            println!(
                "QQQQ lowering two-branch group: opcodes are {:?} and {:?}",
                op0, op1
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
                    let tyS = ctx.input_ty(branches[0], 0);
                    if is_int_ty(tyS) {
                        let rS = input_to_reg(ctx, branches[0], 0);
                        let cc = match op0 {
                            Opcode::Brz => CC::Z,
                            Opcode::Brnz => CC::NZ,
                            _ => unreachable!(),
                        };
                        let sizeB = int_ty_to_sizeB(tyS);
                        ctx.emit(Inst::cmp_rmi_r(sizeB, RMI::imm(0), rS));
                        ctx.emit(Inst::jmp_cond_symm(cc, taken, not_taken));
                    } else {
                        unimplemented = true;
                    }
                }
                Opcode::BrIcmp => {
                    let tyS = ctx.input_ty(branches[0], 0);
                    if is_int_ty(tyS) {
                        let rSL = input_to_reg(ctx, branches[0], 0);
                        let rSR = input_to_reg(ctx, branches[0], 1);
                        let cc = intCC_to_x64_CC(inst_condcode(ctx.data(branches[0])));
                        let sizeB = int_ty_to_sizeB(tyS);
                        // FIXME verify rSR vs rSL ordering
                        ctx.emit(Inst::cmp_rmi_r(sizeB, RMI::reg(rSR), rSL));
                        ctx.emit(Inst::jmp_cond_symm(cc, taken, not_taken));
                    } else {
                        unimplemented = true;
                    }
                }
                // TODO: Brif/icmp, Brff/icmp, jump tables
                _ => {
                    unimplemented = true;
                }
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
                    unimplemented = true;
                }
                _ => panic!("Unknown branch type!"),
            }
        }

        if unimplemented {
            unimplemented!("lower_branch_group(x64): can't handle: {:?}", branches);
        }

        Ok(())
    }
}
