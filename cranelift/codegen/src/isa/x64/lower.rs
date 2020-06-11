//! Lowering rules for X64.

#![allow(non_snake_case)]

use log::trace;
use regalloc::{Reg, RegClass, Writable};
use smallvec::SmallVec;
use std::convert::TryFrom;

use crate::ir::types;
use crate::ir::types::*;
use crate::ir::Inst as IRInst;
use crate::ir::{condcodes::IntCC, InstructionData, Opcode, TrapCode, Type};

use crate::machinst::lower::*;
use crate::machinst::*;
use crate::result::CodegenResult;

use crate::isa::x64::abi::*;
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

fn is_bool_ty(ty: Type) -> bool {
    match ty {
        types::B1 | types::B8 | types::B16 | types::B32 | types::B64 => true,
        _ => false,
    }
}

fn is_float_ty(ty: Type) -> bool {
    match ty {
        types::F32 | types::F64 => true,
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

fn iri_to_u64_imm(ctx: Ctx, inst: IRInst) -> Option<u64> {
    ctx.get_constant(inst)
}

fn inst_trapcode(data: &InstructionData) -> Option<TrapCode> {
    match data {
        &InstructionData::Trap { code, .. }
        | &InstructionData::CondTrap { code, .. }
        | &InstructionData::IntCondTrap { code, .. }
        | &InstructionData::FloatCondTrap { code, .. } => Some(code),
        _ => None,
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

fn ldst_offset(data: &InstructionData) -> Option<i32> {
    match data {
        &InstructionData::Load { offset, .. }
        | &InstructionData::StackLoad { offset, .. }
        | &InstructionData::LoadComplex { offset, .. }
        | &InstructionData::Store { offset, .. }
        | &InstructionData::StackStore { offset, .. }
        | &InstructionData::StoreComplex { offset, .. } => Some(offset.into()),
        _ => None,
    }
}

/// Identifier for a particular input of an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InsnInput {
    insn: IRInst,
    input: usize,
}

/// Identifier for a particular output of an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InsnOutput {
    insn: IRInst,
    output: usize,
}

fn input_to_reg<'a>(ctx: Ctx<'a>, spec: InsnInput) -> Reg {
    let inputs = ctx.get_input(spec.insn, spec.input);
    ctx.use_input_reg(inputs);
    inputs.reg
}

/// Try to use an immediate for constant inputs, and a register otherwise.
/// TODO: handle memory as well!
fn input_to_reg_mem_imm(ctx: Ctx, spec: InsnInput) -> RegMemImm {
    let imm = ctx.get_input(spec.insn, spec.input).constant.and_then(|x| {
        let as_u32 = x as u32;
        let extended = as_u32 as u64;
        // If the truncation and sign-extension don't change the value, use it.
        if extended == x {
            Some(as_u32)
        } else {
            None
        }
    });
    match imm {
        Some(x) => RegMemImm::imm(x),
        None => RegMemImm::reg(input_to_reg(ctx, spec)),
    }
}

fn output_to_reg<'a>(ctx: Ctx<'a>, spec: InsnOutput) -> Writable<Reg> {
    ctx.get_output(spec.insn, spec.output)
}

//=============================================================================
// Top-level instruction lowering entry point, for one instruction.

/// Actually codegen an instruction's results into registers.
fn lower_insn_to_regs<C: LowerCtx<I = Inst>>(ctx: &mut C, insn: IRInst) -> CodegenResult<()> {
    let op = ctx.data(insn).opcode();

    let inputs: SmallVec<[InsnInput; 4]> = (0..ctx.num_inputs(insn))
        .map(|i| InsnInput { insn, input: i })
        .collect();
    let outputs: SmallVec<[InsnOutput; 2]> = (0..ctx.num_outputs(insn))
        .map(|i| InsnOutput { insn, output: i })
        .collect();

    let ty = if outputs.len() > 0 {
        Some(ctx.output_ty(insn, 0))
    } else {
        None
    };

    match op {
        Opcode::Iconst => {
            if let Some(w64) = iri_to_u64_imm(ctx, insn) {
                // Get exactly the bit pattern in 'w64' into the dest.  No
                // monkeying with sign extension etc.
                let dst_is_64 = w64 > 0xFFFF_FFFF;
                let dst = output_to_reg(ctx, outputs[0]);
                ctx.emit(Inst::imm_r(dst_is_64, w64, dst));
            } else {
                unimplemented!();
            }
        }

        Opcode::Iadd | Opcode::Isub => {
            let lhs = input_to_reg(ctx, inputs[0]);
            let rhs = input_to_reg_mem_imm(ctx, inputs[1]);
            let dst = output_to_reg(ctx, outputs[0]);

            // TODO For add, try to commute the operands if one is an immediate.

            let is_64 = int_ty_is_64(ty.unwrap());
            let alu_op = if op == Opcode::Iadd {
                AluRmiROpcode::Add
            } else {
                AluRmiROpcode::Sub
            };

            ctx.emit(Inst::mov_r_r(true, lhs, dst));
            ctx.emit(Inst::alu_rmi_r(is_64, alu_op, rhs, dst));
        }

        Opcode::Ishl | Opcode::Ushr | Opcode::Sshr => {
            // TODO: implement imm shift value into insn
            let dst_ty = ctx.output_ty(insn, 0);
            assert_eq!(ctx.input_ty(insn, 0), dst_ty);
            assert!(dst_ty == types::I32 || dst_ty == types::I64);

            let lhs = input_to_reg(ctx, inputs[0]);
            let rhs = input_to_reg(ctx, inputs[1]);
            let dst = output_to_reg(ctx, outputs[0]);

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

        Opcode::Uextend
        | Opcode::Sextend
        | Opcode::Bint
        | Opcode::Breduce
        | Opcode::Bextend
        | Opcode::Ireduce => {
            let src_ty = ctx.input_ty(insn, 0);
            let dst_ty = ctx.output_ty(insn, 0);

            // TODO: if the source operand is a load, incorporate that.
            let src = input_to_reg(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);

            let ext_mode = match (src_ty.bits(), dst_ty.bits()) {
                (1, 32) | (8, 32) => ExtMode::BL,
                (1, 64) | (8, 64) => ExtMode::BQ,
                (16, 32) => ExtMode::WL,
                (16, 64) => ExtMode::WQ,
                (32, 64) => ExtMode::LQ,
                _ => unreachable!(
                    "unexpected extension kind from {:?} to {:?}",
                    src_ty, dst_ty
                ),
            };

            if op == Opcode::Sextend {
                ctx.emit(Inst::movsx_rm_r(ext_mode, RegMem::reg(src), dst));
            } else {
                // All of these other opcodes are simply a move from a zero-extended source.  Here
                // is why this works, in each case:
                //
                // - Bint: Bool-to-int. We always represent a bool as a 0 or 1, so we
                //   merely need to zero-extend here.
                //
                // - Breduce, Bextend: changing width of a boolean. We represent a
                //   bool as a 0 or 1, so again, this is a zero-extend / no-op.
                //
                // - Ireduce: changing width of an integer. Smaller ints are stored
                //   with undefined high-order bits, so we can simply do a copy.
                ctx.emit(Inst::movzx_rm_r(ext_mode, RegMem::reg(src), dst));
            }
        }

        Opcode::Icmp => {
            let condcode = inst_condcode(ctx.data(insn));
            let cc = CC::from_intcc(condcode);
            let ty = ctx.input_ty(insn, 0);

            // TODO Try to commute the operands (and invert the condition) if one is an immediate.
            let lhs = input_to_reg(ctx, inputs[0]);
            let rhs = input_to_reg_mem_imm(ctx, inputs[1]);
            let dst = output_to_reg(ctx, outputs[0]);

            // Cranelift's icmp semantics want to compare lhs - rhs, while Intel gives
            // us dst - src at the machine instruction level, so invert operands.
            ctx.emit(Inst::cmp_rmi_r(ty.bytes() as u8, rhs, lhs));
            ctx.emit(Inst::setcc(cc, dst));
        }

        Opcode::FallthroughReturn | Opcode::Return => {
            for i in 0..ctx.num_inputs(insn) {
                let src_reg = input_to_reg(ctx, inputs[i]);
                let retval_reg = ctx.retval(i);
                if src_reg.get_class() == RegClass::I64 {
                    ctx.emit(Inst::mov_r_r(true, src_reg, retval_reg));
                } else if src_reg.get_class() == RegClass::V128 {
                    ctx.emit(Inst::xmm_mov_rm_r(
                        SseOpcode::Movsd,
                        RegMem::reg(src_reg),
                        retval_reg,
                    ));
                }
            }
            // N.B.: the Ret itself is generated by the ABI.
        }

        Opcode::Call | Opcode::CallIndirect => {
            let loc = ctx.srcloc(insn);
            let (mut abi, inputs) = match op {
                Opcode::Call => {
                    let (extname, dist) = ctx.call_target(insn).unwrap();
                    let sig = ctx.call_sig(insn).unwrap();
                    assert!(inputs.len() == sig.params.len());
                    assert!(outputs.len() == sig.returns.len());
                    (
                        X64ABICall::from_func(sig, &extname, dist, loc)?,
                        &inputs[..],
                    )
                }

                Opcode::CallIndirect => {
                    let ptr = input_to_reg(ctx, inputs[0]);
                    let sig = ctx.call_sig(insn).unwrap();
                    assert!(inputs.len() - 1 == sig.params.len());
                    assert!(outputs.len() == sig.returns.len());
                    (X64ABICall::from_ptr(sig, ptr, loc, op)?, &inputs[1..])
                }

                _ => unreachable!(),
            };

            abi.emit_stack_pre_adjust(ctx);
            assert!(inputs.len() == abi.num_args());
            for (i, input) in inputs.iter().enumerate() {
                let arg_reg = input_to_reg(ctx, *input);
                abi.emit_copy_reg_to_arg(ctx, i, arg_reg);
            }
            abi.emit_call(ctx);
            for (i, output) in outputs.iter().enumerate() {
                let retval_reg = output_to_reg(ctx, *output);
                abi.emit_copy_retval_to_reg(ctx, i, retval_reg);
            }
            abi.emit_stack_post_adjust(ctx);
        }

        Opcode::Debugtrap => {
            ctx.emit(Inst::Hlt);
        }

        Opcode::Trap => {
            let trap_info = (ctx.srcloc(insn), inst_trapcode(ctx.data(insn)).unwrap());
            ctx.emit(Inst::Ud2 { trap_info })
        }

        Opcode::Fadd | Opcode::Fsub | Opcode::Fmul | Opcode::Fdiv => {
            let lhs = input_to_reg(ctx, inputs[0]);
            let rhs = input_to_reg(ctx, inputs[1]);
            let dst = output_to_reg(ctx, outputs[0]);
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
                ctx.emit(Inst::xmm_mov_rm_r(SseOpcode::Movss, RegMem::reg(lhs), dst));
                ctx.emit(Inst::xmm_rm_r(sse_op, RegMem::reg(rhs), dst));
            } else {
                unimplemented!("unimplemented lowering for opcode {:?}", op);
            }
        }

        Opcode::Fcopysign => {
            let dst = output_to_reg(ctx, outputs[0]);
            let lhs = input_to_reg(ctx, inputs[0]);
            let rhs = input_to_reg(ctx, inputs[1]);
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

        Opcode::Load
        | Opcode::Uload8
        | Opcode::Sload8
        | Opcode::Uload16
        | Opcode::Sload16
        | Opcode::Uload32
        | Opcode::Sload32
        | Opcode::LoadComplex
        | Opcode::Uload8Complex
        | Opcode::Sload8Complex
        | Opcode::Uload16Complex
        | Opcode::Sload16Complex
        | Opcode::Uload32Complex
        | Opcode::Sload32Complex => {
            let offset = ldst_offset(ctx.data(insn)).unwrap();

            let elem_ty = match op {
                Opcode::Sload8 | Opcode::Uload8 | Opcode::Sload8Complex | Opcode::Uload8Complex => {
                    types::I8
                }
                Opcode::Sload16
                | Opcode::Uload16
                | Opcode::Sload16Complex
                | Opcode::Uload16Complex => types::I16,
                Opcode::Sload32
                | Opcode::Uload32
                | Opcode::Sload32Complex
                | Opcode::Uload32Complex => types::I32,
                Opcode::Load | Opcode::LoadComplex => ctx.output_ty(insn, 0),
                _ => unimplemented!(),
            };

            let ext_mode = match elem_ty.bytes() {
                1 => Some(ExtMode::BQ),
                2 => Some(ExtMode::WQ),
                4 => Some(ExtMode::LQ),
                _ => None,
            };

            let sign_extend = match op {
                Opcode::Sload8
                | Opcode::Sload8Complex
                | Opcode::Sload16
                | Opcode::Sload16Complex
                | Opcode::Sload32
                | Opcode::Sload32Complex => true,
                _ => false,
            };

            let is_float = is_float_ty(elem_ty);

            let addr = match op {
                Opcode::Load
                | Opcode::Uload8
                | Opcode::Sload8
                | Opcode::Uload16
                | Opcode::Sload16
                | Opcode::Uload32
                | Opcode::Sload32 => {
                    assert!(inputs.len() == 1, "only one input for load operands");
                    let base = input_to_reg(ctx, inputs[0]);
                    Amode::imm_reg(offset as u32, base)
                }

                Opcode::LoadComplex
                | Opcode::Uload8Complex
                | Opcode::Sload8Complex
                | Opcode::Uload16Complex
                | Opcode::Sload16Complex
                | Opcode::Uload32Complex
                | Opcode::Sload32Complex => {
                    assert!(
                        inputs.len() == 2,
                        "can't handle more than two inputs in complex load"
                    );
                    let base = input_to_reg(ctx, inputs[0]);
                    let index = input_to_reg(ctx, inputs[1]);
                    let shift = 0;
                    Amode::imm_reg_reg_shift(offset as u32, base, index, shift)
                }

                _ => unreachable!(),
            };

            let dst = output_to_reg(ctx, outputs[0]);
            match (sign_extend, is_float) {
                (true, false) => {
                    // The load is sign-extended only when the output size is lower than 64 bits,
                    // so ext-mode is defined in this case.
                    ctx.emit(Inst::movsx_rm_r(ext_mode.unwrap(), RegMem::mem(addr), dst));
                }
                (false, false) => {
                    if elem_ty.bytes() == 8 {
                        // Use a plain load.
                        ctx.emit(Inst::mov64_m_r(addr, dst))
                    } else {
                        // Use a zero-extended load.
                        ctx.emit(Inst::movzx_rm_r(ext_mode.unwrap(), RegMem::mem(addr), dst))
                    }
                }
                (_, true) => unimplemented!("FPU loads"),
            }
        }

        Opcode::Store
        | Opcode::Istore8
        | Opcode::Istore16
        | Opcode::Istore32
        | Opcode::StoreComplex
        | Opcode::Istore8Complex
        | Opcode::Istore16Complex
        | Opcode::Istore32Complex => {
            let offset = ldst_offset(ctx.data(insn)).unwrap();

            let elem_ty = match op {
                Opcode::Istore8 | Opcode::Istore8Complex => types::I8,
                Opcode::Istore16 | Opcode::Istore16Complex => types::I16,
                Opcode::Istore32 | Opcode::Istore32Complex => types::I32,
                Opcode::Store | Opcode::StoreComplex => ctx.input_ty(insn, 0),
                _ => unreachable!(),
            };
            let is_float = is_float_ty(elem_ty);

            let addr = match op {
                Opcode::Store | Opcode::Istore8 | Opcode::Istore16 | Opcode::Istore32 => {
                    assert!(
                        inputs.len() == 2,
                        "only one input for store memory operands"
                    );
                    let base = input_to_reg(ctx, inputs[1]);
                    // TODO sign?
                    Amode::imm_reg(offset as u32, base)
                }

                Opcode::StoreComplex
                | Opcode::Istore8Complex
                | Opcode::Istore16Complex
                | Opcode::Istore32Complex => {
                    assert!(
                        inputs.len() == 3,
                        "can't handle more than two inputs in complex load"
                    );
                    let base = input_to_reg(ctx, inputs[1]);
                    let index = input_to_reg(ctx, inputs[2]);
                    let shift = 0;
                    Amode::imm_reg_reg_shift(offset as u32, base, index, shift)
                }

                _ => unreachable!(),
            };

            let src = input_to_reg(ctx, inputs[0]);

            if is_float {
                ctx.emit(match elem_ty {
                    F32 => Inst::xmm_mov_r_m(SseOpcode::Movss, src, addr),
                    _ => unimplemented!("FP store not 32-bit"),
                });
            } else {
                ctx.emit(Inst::mov_r_m(elem_ty.bytes() as u8, src, addr));
            }
        }

        Opcode::StackAddr => {
            let (stack_slot, offset) = match *ctx.data(insn) {
                InstructionData::StackLoad {
                    opcode: Opcode::StackAddr,
                    stack_slot,
                    offset,
                } => (stack_slot, offset),
                _ => unreachable!(),
            };
            let dst = output_to_reg(ctx, outputs[0]);
            let offset: i32 = offset.into();
            println!("stackslot_addr: {:?} @ off{}", stack_slot, offset);
            let inst = ctx
                .abi()
                .stackslot_addr(stack_slot, u32::try_from(offset).unwrap(), dst);
            ctx.emit(inst);
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

    Ok(())
}

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for X64Backend {
    type MInst = Inst;

    fn lower<C: LowerCtx<I = Inst>>(&self, ctx: &mut C, ir_inst: IRInst) -> CodegenResult<()> {
        lower_insn_to_regs(ctx, ir_inst)
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
                    if is_int_ty(src_ty) || is_bool_ty(src_ty) {
                        let src = input_to_reg(
                            ctx,
                            InsnInput {
                                insn: branches[0],
                                input: 0,
                            },
                        );
                        let cc = match op0 {
                            Opcode::Brz => CC::Z,
                            Opcode::Brnz => CC::NZ,
                            _ => unreachable!(),
                        };
                        let size_bytes = src_ty.bytes() as u8;
                        ctx.emit(Inst::cmp_rmi_r(size_bytes, RegMemImm::imm(0), src));
                        ctx.emit(Inst::jmp_cond(cc, taken, not_taken));
                    } else {
                        unimplemented!("brz/brnz with non-int type {:?}", src_ty);
                    }
                }

                Opcode::BrIcmp => {
                    let src_ty = ctx.input_ty(branches[0], 0);
                    if is_int_ty(src_ty) || is_bool_ty(src_ty) {
                        let lhs = input_to_reg(
                            ctx,
                            InsnInput {
                                insn: branches[0],
                                input: 0,
                            },
                        );
                        let rhs = input_to_reg_mem_imm(
                            ctx,
                            InsnInput {
                                insn: branches[0],
                                input: 1,
                            },
                        );
                        let cc = CC::from_intcc(inst_condcode(ctx.data(branches[0])));
                        let byte_size = src_ty.bytes() as u8;
                        // Cranelift's icmp semantics want to compare lhs - rhs, while Intel gives
                        // us dst - src at the machine instruction level, so invert operands.
                        ctx.emit(Inst::cmp_rmi_r(byte_size, rhs, lhs));
                        ctx.emit(Inst::jmp_cond(cc, taken, not_taken));
                    } else {
                        unimplemented!("bricmp with non-int type {:?}", src_ty);
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
                Opcode::Jump | Opcode::Fallthrough => {
                    ctx.emit(Inst::jmp_known(BranchTarget::Label(targets[0])));
                }
                _ => panic!("Unknown branch type!"),
            }
        }

        Ok(())
    }
}
