//! Lower a single Cranelift instruction into vcode.

use crate::binemit::CodeOffset;
use crate::ir::condcodes::FloatCC;
use crate::ir::types::*;
use crate::ir::Inst as IRInst;
use crate::ir::{InstructionData, Opcode, TrapCode};
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::CodegenResult;

use crate::isa::aarch64::abi::*;
use crate::isa::aarch64::inst::*;

use regalloc::RegClass;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::convert::TryFrom;
use smallvec::SmallVec;

use super::lower::*;

/// Actually codegen an instruction's results into registers.
pub(crate) fn lower_insn_to_regs<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    insn: IRInst,
) -> CodegenResult<()> {
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
        Opcode::Iconst | Opcode::Bconst | Opcode::Null => {
            let value = ctx.get_constant(insn).unwrap();
            let rd = get_output_reg(ctx, outputs[0]);
            lower_constant_u64(ctx, rd, value);
        }
        Opcode::F32const => {
            let value = f32::from_bits(ctx.get_constant(insn).unwrap() as u32);
            let rd = get_output_reg(ctx, outputs[0]);
            lower_constant_f32(ctx, rd, value);
        }
        Opcode::F64const => {
            let value = f64::from_bits(ctx.get_constant(insn).unwrap());
            let rd = get_output_reg(ctx, outputs[0]);
            lower_constant_f64(ctx, rd, value);
        }
        Opcode::Iadd => {
            let rd = get_output_reg(ctx, outputs[0]);
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let ty = ty.unwrap();
            if ty_bits(ty) < 128 {
                let rm = put_input_in_rse_imm12(ctx, inputs[1], NarrowValueMode::None);
                let alu_op = choose_32_64(ty, ALUOp::Add32, ALUOp::Add64);
                ctx.emit(alu_inst_imm12(alu_op, rd, rn, rm));
            } else {
                let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
                ctx.emit(Inst::VecRRR {
                    rd,
                    rn,
                    rm,
                    alu_op: VecALUOp::Add,
                    ty,
                });
            }
        }
        Opcode::Isub => {
            let rd = get_output_reg(ctx, outputs[0]);
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let ty = ty.unwrap();
            if ty_bits(ty) < 128 {
                let rm = put_input_in_rse_imm12(ctx, inputs[1], NarrowValueMode::None);
                let alu_op = choose_32_64(ty, ALUOp::Sub32, ALUOp::Sub64);
                ctx.emit(alu_inst_imm12(alu_op, rd, rn, rm));
            } else {
                let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
                ctx.emit(Inst::VecRRR {
                    rd,
                    rn,
                    rm,
                    alu_op: VecALUOp::Sub,
                    ty,
                });
            }
        }
        Opcode::UaddSat | Opcode::SaddSat => {
            // We use the vector instruction set's saturating adds (UQADD /
            // SQADD), which require vector registers.
            let is_signed = op == Opcode::SaddSat;
            let narrow_mode = if is_signed {
                NarrowValueMode::SignExtend64
            } else {
                NarrowValueMode::ZeroExtend64
            };
            let alu_op = if is_signed {
                VecALUOp::SQAddScalar
            } else {
                VecALUOp::UQAddScalar
            };
            let va = ctx.alloc_tmp(RegClass::V128, I128);
            let vb = ctx.alloc_tmp(RegClass::V128, I128);
            let ra = put_input_in_reg(ctx, inputs[0], narrow_mode);
            let rb = put_input_in_reg(ctx, inputs[1], narrow_mode);
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::MovToVec64 { rd: va, rn: ra });
            ctx.emit(Inst::MovToVec64 { rd: vb, rn: rb });
            ctx.emit(Inst::VecRRR {
                rd: va,
                rn: va.to_reg(),
                rm: vb.to_reg(),
                alu_op,
                ty: I64,
            });
            ctx.emit(Inst::MovFromVec {
                rd,
                rn: va.to_reg(),
                idx: 0,
                ty: I64,
            });
        }

        Opcode::UsubSat | Opcode::SsubSat => {
            let is_signed = op == Opcode::SsubSat;
            let narrow_mode = if is_signed {
                NarrowValueMode::SignExtend64
            } else {
                NarrowValueMode::ZeroExtend64
            };
            let alu_op = if is_signed {
                VecALUOp::SQSubScalar
            } else {
                VecALUOp::UQSubScalar
            };
            let va = ctx.alloc_tmp(RegClass::V128, I128);
            let vb = ctx.alloc_tmp(RegClass::V128, I128);
            let ra = put_input_in_reg(ctx, inputs[0], narrow_mode);
            let rb = put_input_in_reg(ctx, inputs[1], narrow_mode);
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::MovToVec64 { rd: va, rn: ra });
            ctx.emit(Inst::MovToVec64 { rd: vb, rn: rb });
            ctx.emit(Inst::VecRRR {
                rd: va,
                rn: va.to_reg(),
                rm: vb.to_reg(),
                alu_op,
                ty: I64,
            });
            ctx.emit(Inst::MovFromVec {
                rd,
                rn: va.to_reg(),
                idx: 0,
                ty: I64,
            });
        }

        Opcode::Ineg => {
            let rd = get_output_reg(ctx, outputs[0]);
            let ty = ty.unwrap();
            if ty_bits(ty) < 128 {
                let rn = zero_reg();
                let rm = put_input_in_rse_imm12(ctx, inputs[0], NarrowValueMode::None);
                let alu_op = choose_32_64(ty, ALUOp::Sub32, ALUOp::Sub64);
                ctx.emit(alu_inst_imm12(alu_op, rd, rn, rm));
            } else {
                let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                ctx.emit(Inst::VecMisc {
                    op: VecMisc2::Neg,
                    rd,
                    rn,
                    ty,
                });
            }
        }

        Opcode::Imul => {
            let rd = get_output_reg(ctx, outputs[0]);
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
            let ty = ty.unwrap();
            if ty_bits(ty) < 128 {
                let alu_op = choose_32_64(ty, ALUOp::MAdd32, ALUOp::MAdd64);
                ctx.emit(Inst::AluRRRR {
                    alu_op,
                    rd,
                    rn,
                    rm,
                    ra: zero_reg(),
                });
            } else {
                ctx.emit(Inst::VecRRR {
                    alu_op: VecALUOp::Mul,
                    rd,
                    rn,
                    rm,
                    ty,
                });
            }
        }

        Opcode::Umulhi | Opcode::Smulhi => {
            let rd = get_output_reg(ctx, outputs[0]);
            let is_signed = op == Opcode::Smulhi;
            let input_ty = ctx.input_ty(insn, 0);
            assert!(ctx.input_ty(insn, 1) == input_ty);
            assert!(ctx.output_ty(insn, 0) == input_ty);

            match input_ty {
                I64 => {
                    let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                    let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
                    let ra = zero_reg();
                    let alu_op = if is_signed {
                        ALUOp::SMulH
                    } else {
                        ALUOp::UMulH
                    };
                    ctx.emit(Inst::AluRRRR {
                        alu_op,
                        rd,
                        rn,
                        rm,
                        ra,
                    });
                }
                I32 | I16 | I8 => {
                    let narrow_mode = if is_signed {
                        NarrowValueMode::SignExtend64
                    } else {
                        NarrowValueMode::ZeroExtend64
                    };
                    let rn = put_input_in_reg(ctx, inputs[0], narrow_mode);
                    let rm = put_input_in_reg(ctx, inputs[1], narrow_mode);
                    let ra = zero_reg();
                    ctx.emit(Inst::AluRRRR {
                        alu_op: ALUOp::MAdd64,
                        rd,
                        rn,
                        rm,
                        ra,
                    });
                    let shift_op = if is_signed {
                        ALUOp::Asr64
                    } else {
                        ALUOp::Lsr64
                    };
                    let shift_amt = match input_ty {
                        I32 => 32,
                        I16 => 16,
                        I8 => 8,
                        _ => unreachable!(),
                    };
                    ctx.emit(Inst::AluRRImmShift {
                        alu_op: shift_op,
                        rd,
                        rn: rd.to_reg(),
                        immshift: ImmShift::maybe_from_u64(shift_amt).unwrap(),
                    });
                }
                _ => {
                    panic!("Unsupported argument type for umulhi/smulhi: {}", input_ty);
                }
            }
        }

        Opcode::Udiv | Opcode::Sdiv | Opcode::Urem | Opcode::Srem => {
            let is_signed = match op {
                Opcode::Udiv | Opcode::Urem => false,
                Opcode::Sdiv | Opcode::Srem => true,
                _ => unreachable!(),
            };
            let is_rem = match op {
                Opcode::Udiv | Opcode::Sdiv => false,
                Opcode::Urem | Opcode::Srem => true,
                _ => unreachable!(),
            };
            let narrow_mode = if is_signed {
                NarrowValueMode::SignExtend64
            } else {
                NarrowValueMode::ZeroExtend64
            };
            // TODO: Add SDiv32 to implement 32-bit directly, rather
            // than extending the input.
            let div_op = if is_signed {
                ALUOp::SDiv64
            } else {
                ALUOp::UDiv64
            };

            let rd = get_output_reg(ctx, outputs[0]);
            let rn = put_input_in_reg(ctx, inputs[0], narrow_mode);
            let rm = put_input_in_reg(ctx, inputs[1], narrow_mode);
            // The div instruction does not trap on divide by zero or signed overflow
            // so checks are inserted below.
            //
            //   div rd, rn, rm
            ctx.emit(Inst::AluRRR {
                alu_op: div_op,
                rd,
                rn,
                rm,
            });

            if is_rem {
                // Remainder (rn % rm) is implemented as:
                //
                //   tmp = rn / rm
                //   rd = rn - (tmp*rm)
                //
                // use 'rd' for tmp and you have:
                //
                //   div rd, rn, rm       ; rd = rn / rm
                //   cbnz rm, #8          ; branch over trap
                //   udf                  ; divide by zero
                //   msub rd, rd, rm, rn  ; rd = rn - rd * rm

                // Check for divide by 0.
                let trap_info = (ctx.srcloc(insn), TrapCode::IntegerDivisionByZero);
                ctx.emit(Inst::TrapIf {
                    trap_info,
                    kind: CondBrKind::Zero(rm),
                });

                ctx.emit(Inst::AluRRRR {
                    alu_op: ALUOp::MSub64,
                    rd: rd,
                    rn: rd.to_reg(),
                    rm: rm,
                    ra: rn,
                });
            } else {
                if div_op == ALUOp::SDiv64 {
                    //   cbnz rm, #8
                    //   udf ; divide by zero
                    //   cmn rm, 1
                    //   ccmp rn, 1, #nzcv, eq
                    //   b.vc #8
                    //   udf ; signed overflow

                    // Check for divide by 0.
                    let trap_info = (ctx.srcloc(insn), TrapCode::IntegerDivisionByZero);
                    ctx.emit(Inst::TrapIf {
                        trap_info,
                        kind: CondBrKind::Zero(rm),
                    });

                    // Check for signed overflow. The only case is min_value / -1.
                    let ty = ty.unwrap();
                    // The following checks must be done in 32-bit or 64-bit, depending
                    // on the input type. Even though the initial div instruction is
                    // always done in 64-bit currently.
                    let size = InstSize::from_ty(ty);
                    // Check RHS is -1.
                    ctx.emit(Inst::AluRRImm12 {
                        alu_op: choose_32_64(ty, ALUOp::AddS32, ALUOp::AddS64),
                        rd: writable_zero_reg(),
                        rn: rm,
                        imm12: Imm12::maybe_from_u64(1).unwrap(),
                    });
                    // Check LHS is min_value, by subtracting 1 and branching if
                    // there is overflow.
                    ctx.emit(Inst::CCmpImm {
                        size,
                        rn,
                        imm: UImm5::maybe_from_u8(1).unwrap(),
                        nzcv: NZCV::new(false, false, false, false),
                        cond: Cond::Eq,
                    });
                    let trap_info = (ctx.srcloc(insn), TrapCode::IntegerOverflow);
                    ctx.emit(Inst::TrapIf {
                        trap_info,
                        kind: CondBrKind::Cond(Cond::Vs),
                    });
                } else {
                    //   cbnz rm, #8
                    //   udf ; divide by zero

                    // Check for divide by 0.
                    let trap_info = (ctx.srcloc(insn), TrapCode::IntegerDivisionByZero);
                    ctx.emit(Inst::TrapIf {
                        trap_info,
                        kind: CondBrKind::Zero(rm),
                    });
                }
            }
        }

        Opcode::Uextend | Opcode::Sextend => {
            let output_ty = ty.unwrap();
            let input_ty = ctx.input_ty(insn, 0);
            let from_bits = ty_bits(input_ty) as u8;
            let to_bits = ty_bits(output_ty) as u8;
            let to_bits = std::cmp::max(32, to_bits);
            assert!(from_bits <= to_bits);
            if from_bits < to_bits {
                let signed = op == Opcode::Sextend;
                // If we reach this point, we weren't able to incorporate the extend as
                // a register-mode on another instruction, so we have a 'None'
                // narrow-value/extend mode here, and we emit the explicit instruction.
                let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                let rd = get_output_reg(ctx, outputs[0]);
                ctx.emit(Inst::Extend {
                    rd,
                    rn,
                    signed,
                    from_bits,
                    to_bits,
                });
            }
        }

        Opcode::Bnot => {
            let rd = get_output_reg(ctx, outputs[0]);
            let ty = ty.unwrap();
            if ty_bits(ty) < 128 {
                let rm = put_input_in_rs_immlogic(ctx, inputs[0], NarrowValueMode::None);
                let alu_op = choose_32_64(ty, ALUOp::OrrNot32, ALUOp::OrrNot64);
                // NOT rd, rm ==> ORR_NOT rd, zero, rm
                ctx.emit(alu_inst_immlogic(alu_op, rd, zero_reg(), rm));
            } else {
                let rm = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                ctx.emit(Inst::VecMisc {
                    op: VecMisc2::Not,
                    rd,
                    rn: rm,
                    ty,
                });
            }
        }

        Opcode::Band
        | Opcode::Bor
        | Opcode::Bxor
        | Opcode::BandNot
        | Opcode::BorNot
        | Opcode::BxorNot => {
            let rd = get_output_reg(ctx, outputs[0]);
            let ty = ty.unwrap();
            if ty_bits(ty) < 128 {
                let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                let rm = put_input_in_rs_immlogic(ctx, inputs[1], NarrowValueMode::None);
                let alu_op = match op {
                    Opcode::Band => choose_32_64(ty, ALUOp::And32, ALUOp::And64),
                    Opcode::Bor => choose_32_64(ty, ALUOp::Orr32, ALUOp::Orr64),
                    Opcode::Bxor => choose_32_64(ty, ALUOp::Eor32, ALUOp::Eor64),
                    Opcode::BandNot => choose_32_64(ty, ALUOp::AndNot32, ALUOp::AndNot64),
                    Opcode::BorNot => choose_32_64(ty, ALUOp::OrrNot32, ALUOp::OrrNot64),
                    Opcode::BxorNot => choose_32_64(ty, ALUOp::EorNot32, ALUOp::EorNot64),
                    _ => unreachable!(),
                };
                ctx.emit(alu_inst_immlogic(alu_op, rd, rn, rm));
            } else {
                let alu_op = match op {
                    Opcode::Band => VecALUOp::And,
                    Opcode::BandNot => VecALUOp::Bic,
                    Opcode::Bor => VecALUOp::Orr,
                    Opcode::Bxor => VecALUOp::Eor,
                    _ => unreachable!(),
                };

                let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
                let rd = get_output_reg(ctx, outputs[0]);

                ctx.emit(Inst::VecRRR {
                    alu_op,
                    rd,
                    rn,
                    rm,
                    ty,
                });
            }
        }

        Opcode::Ishl | Opcode::Ushr | Opcode::Sshr => {
            let ty = ty.unwrap();
            let size = InstSize::from_bits(ty_bits(ty));
            let narrow_mode = match (op, size) {
                (Opcode::Ishl, _) => NarrowValueMode::None,
                (Opcode::Ushr, InstSize::Size64) => NarrowValueMode::ZeroExtend64,
                (Opcode::Ushr, InstSize::Size32) => NarrowValueMode::ZeroExtend32,
                (Opcode::Sshr, InstSize::Size64) => NarrowValueMode::SignExtend64,
                (Opcode::Sshr, InstSize::Size32) => NarrowValueMode::SignExtend32,
                _ => unreachable!(),
            };
            let rd = get_output_reg(ctx, outputs[0]);
            let rn = put_input_in_reg(ctx, inputs[0], narrow_mode);
            let rm = put_input_in_reg_immshift(ctx, inputs[1], ty_bits(ty));
            let alu_op = match op {
                Opcode::Ishl => choose_32_64(ty, ALUOp::Lsl32, ALUOp::Lsl64),
                Opcode::Ushr => choose_32_64(ty, ALUOp::Lsr32, ALUOp::Lsr64),
                Opcode::Sshr => choose_32_64(ty, ALUOp::Asr32, ALUOp::Asr64),
                _ => unreachable!(),
            };
            ctx.emit(alu_inst_immshift(alu_op, rd, rn, rm));
        }

        Opcode::Rotr | Opcode::Rotl => {
            // aarch64 doesn't have a left-rotate instruction, but a left rotation of K places is
            // effectively a right rotation of N - K places, if N is the integer's bit size. We
            // implement left rotations with this trick.
            //
            // For a 32-bit or 64-bit rotate-right, we can use the ROR instruction directly.
            //
            // For a < 32-bit rotate-right, we synthesize this as:
            //
            //    rotr rd, rn, rm
            //
            //       =>
            //
            //    zero-extend rn, <32-or-64>
            //    and tmp_masked_rm, rm, <bitwidth - 1>
            //    sub tmp1, tmp_masked_rm, <bitwidth>
            //    sub tmp1, zero, tmp1  ; neg
            //    lsr tmp2, rn, tmp_masked_rm
            //    lsl rd, rn, tmp1
            //    orr rd, rd, tmp2
            //
            // For a constant amount, we can instead do:
            //
            //    zero-extend rn, <32-or-64>
            //    lsr tmp2, rn, #<shiftimm>
            //    lsl rd, rn, <bitwidth - shiftimm>
            //    orr rd, rd, tmp2

            let is_rotl = op == Opcode::Rotl;

            let ty = ty.unwrap();
            let ty_bits_size = ty_bits(ty) as u8;

            let rd = get_output_reg(ctx, outputs[0]);
            let rn = put_input_in_reg(
                ctx,
                inputs[0],
                if ty_bits_size <= 32 {
                    NarrowValueMode::ZeroExtend32
                } else {
                    NarrowValueMode::ZeroExtend64
                },
            );
            let rm = put_input_in_reg_immshift(ctx, inputs[1], ty_bits(ty));

            if ty_bits_size == 32 || ty_bits_size == 64 {
                let alu_op = choose_32_64(ty, ALUOp::RotR32, ALUOp::RotR64);
                match rm {
                    ResultRegImmShift::ImmShift(mut immshift) => {
                        if is_rotl {
                            immshift.imm = ty_bits_size.wrapping_sub(immshift.value());
                        }
                        immshift.imm &= ty_bits_size - 1;
                        ctx.emit(Inst::AluRRImmShift {
                            alu_op,
                            rd,
                            rn,
                            immshift,
                        });
                    }

                    ResultRegImmShift::Reg(rm) => {
                        let rm = if is_rotl {
                            // Really ty_bits_size - rn, but the upper bits of the result are
                            // ignored (because of the implicit masking done by the instruction),
                            // so this is equivalent to negating the input.
                            let alu_op = choose_32_64(ty, ALUOp::Sub32, ALUOp::Sub64);
                            let tmp = ctx.alloc_tmp(RegClass::I64, ty);
                            ctx.emit(Inst::AluRRR {
                                alu_op,
                                rd: tmp,
                                rn: zero_reg(),
                                rm,
                            });
                            tmp.to_reg()
                        } else {
                            rm
                        };
                        ctx.emit(Inst::AluRRR { alu_op, rd, rn, rm });
                    }
                }
            } else {
                debug_assert!(ty_bits_size < 32);

                match rm {
                    ResultRegImmShift::Reg(reg) => {
                        let reg = if is_rotl {
                            // Really ty_bits_size - rn, but the upper bits of the result are
                            // ignored (because of the implicit masking done by the instruction),
                            // so this is equivalent to negating the input.
                            let tmp = ctx.alloc_tmp(RegClass::I64, I32);
                            ctx.emit(Inst::AluRRR {
                                alu_op: ALUOp::Sub32,
                                rd: tmp,
                                rn: zero_reg(),
                                rm: reg,
                            });
                            tmp.to_reg()
                        } else {
                            reg
                        };

                        // Explicitly mask the rotation count.
                        let tmp_masked_rm = ctx.alloc_tmp(RegClass::I64, I32);
                        ctx.emit(Inst::AluRRImmLogic {
                            alu_op: ALUOp::And32,
                            rd: tmp_masked_rm,
                            rn: reg,
                            imml: ImmLogic::maybe_from_u64((ty_bits_size - 1) as u64, I32).unwrap(),
                        });
                        let tmp_masked_rm = tmp_masked_rm.to_reg();

                        let tmp1 = ctx.alloc_tmp(RegClass::I64, I32);
                        let tmp2 = ctx.alloc_tmp(RegClass::I64, I32);
                        ctx.emit(Inst::AluRRImm12 {
                            alu_op: ALUOp::Sub32,
                            rd: tmp1,
                            rn: tmp_masked_rm,
                            imm12: Imm12::maybe_from_u64(ty_bits_size as u64).unwrap(),
                        });
                        ctx.emit(Inst::AluRRR {
                            alu_op: ALUOp::Sub32,
                            rd: tmp1,
                            rn: zero_reg(),
                            rm: tmp1.to_reg(),
                        });
                        ctx.emit(Inst::AluRRR {
                            alu_op: ALUOp::Lsr32,
                            rd: tmp2,
                            rn,
                            rm: tmp_masked_rm,
                        });
                        ctx.emit(Inst::AluRRR {
                            alu_op: ALUOp::Lsl32,
                            rd,
                            rn,
                            rm: tmp1.to_reg(),
                        });
                        ctx.emit(Inst::AluRRR {
                            alu_op: ALUOp::Orr32,
                            rd,
                            rn: rd.to_reg(),
                            rm: tmp2.to_reg(),
                        });
                    }

                    ResultRegImmShift::ImmShift(mut immshift) => {
                        if is_rotl {
                            immshift.imm = ty_bits_size.wrapping_sub(immshift.value());
                        }
                        immshift.imm &= ty_bits_size - 1;

                        let tmp1 = ctx.alloc_tmp(RegClass::I64, I32);
                        ctx.emit(Inst::AluRRImmShift {
                            alu_op: ALUOp::Lsr32,
                            rd: tmp1,
                            rn,
                            immshift: immshift.clone(),
                        });

                        let amount = immshift.value() & (ty_bits_size - 1);
                        let opp_shift =
                            ImmShift::maybe_from_u64(ty_bits_size as u64 - amount as u64).unwrap();
                        ctx.emit(Inst::AluRRImmShift {
                            alu_op: ALUOp::Lsl32,
                            rd,
                            rn,
                            immshift: opp_shift,
                        });

                        ctx.emit(Inst::AluRRR {
                            alu_op: ALUOp::Orr32,
                            rd,
                            rn: rd.to_reg(),
                            rm: tmp1.to_reg(),
                        });
                    }
                }
            }
        }

        Opcode::Bitrev | Opcode::Clz | Opcode::Cls | Opcode::Ctz => {
            let rd = get_output_reg(ctx, outputs[0]);
            let needs_zext = match op {
                Opcode::Bitrev | Opcode::Ctz => false,
                Opcode::Clz | Opcode::Cls => true,
                _ => unreachable!(),
            };
            let ty = ty.unwrap();
            let narrow_mode = if needs_zext && ty_bits(ty) == 64 {
                NarrowValueMode::ZeroExtend64
            } else if needs_zext {
                NarrowValueMode::ZeroExtend32
            } else {
                NarrowValueMode::None
            };
            let rn = put_input_in_reg(ctx, inputs[0], narrow_mode);
            let op_ty = match ty {
                I8 | I16 | I32 => I32,
                I64 => I64,
                _ => panic!("Unsupported type for Bitrev/Clz/Cls"),
            };
            let bitop = match op {
                Opcode::Clz | Opcode::Cls | Opcode::Bitrev => BitOp::from((op, op_ty)),
                Opcode::Ctz => BitOp::from((Opcode::Bitrev, op_ty)),
                _ => unreachable!(),
            };
            ctx.emit(Inst::BitRR { rd, rn, op: bitop });

            // Both bitrev and ctz use a bit-reverse (rbit) instruction; ctz to reduce the problem
            // to a clz, and bitrev as the main operation.
            if op == Opcode::Bitrev || op == Opcode::Ctz {
                // Reversing an n-bit value (n < 32) with a 32-bit bitrev instruction will place
                // the reversed result in the highest n bits, so we need to shift them down into
                // place.
                let right_shift = match ty {
                    I8 => Some(24),
                    I16 => Some(16),
                    I32 => None,
                    I64 => None,
                    _ => panic!("Unsupported type for Bitrev"),
                };
                if let Some(s) = right_shift {
                    ctx.emit(Inst::AluRRImmShift {
                        alu_op: ALUOp::Lsr32,
                        rd,
                        rn: rd.to_reg(),
                        immshift: ImmShift::maybe_from_u64(s).unwrap(),
                    });
                }
            }

            if op == Opcode::Ctz {
                ctx.emit(Inst::BitRR {
                    op: BitOp::from((Opcode::Clz, op_ty)),
                    rd,
                    rn: rd.to_reg(),
                });
            }
        }

        Opcode::Popcnt => {
            // Lower popcount using the following algorithm:
            //
            //   x -= (x >> 1) & 0x5555555555555555
            //   x = (x & 0x3333333333333333) + ((x >> 2) & 0x3333333333333333)
            //   x = (x + (x >> 4)) & 0x0f0f0f0f0f0f0f0f
            //   x += x << 8
            //   x += x << 16
            //   x += x << 32
            //   x >> 56
            let ty = ty.unwrap();
            let rd = get_output_reg(ctx, outputs[0]);
            // FIXME(#1537): zero-extend 8/16/32-bit operands only to 32 bits,
            // and fix the sequence below to work properly for this.
            let narrow_mode = NarrowValueMode::ZeroExtend64;
            let rn = put_input_in_reg(ctx, inputs[0], narrow_mode);
            let tmp = ctx.alloc_tmp(RegClass::I64, I64);

            // If this is a 32-bit Popcnt, use Lsr32 to clear the top 32 bits of the register, then
            // the rest of the code is identical to the 64-bit version.
            // lsr [wx]d, [wx]n, #1
            ctx.emit(Inst::AluRRImmShift {
                alu_op: choose_32_64(ty, ALUOp::Lsr32, ALUOp::Lsr64),
                rd: rd,
                rn: rn,
                immshift: ImmShift::maybe_from_u64(1).unwrap(),
            });

            // and xd, xd, #0x5555555555555555
            ctx.emit(Inst::AluRRImmLogic {
                alu_op: ALUOp::And64,
                rd: rd,
                rn: rd.to_reg(),
                imml: ImmLogic::maybe_from_u64(0x5555555555555555, I64).unwrap(),
            });

            // sub xd, xn, xd
            ctx.emit(Inst::AluRRR {
                alu_op: ALUOp::Sub64,
                rd: rd,
                rn: rn,
                rm: rd.to_reg(),
            });

            // and xt, xd, #0x3333333333333333
            ctx.emit(Inst::AluRRImmLogic {
                alu_op: ALUOp::And64,
                rd: tmp,
                rn: rd.to_reg(),
                imml: ImmLogic::maybe_from_u64(0x3333333333333333, I64).unwrap(),
            });

            // lsr xd, xd, #2
            ctx.emit(Inst::AluRRImmShift {
                alu_op: ALUOp::Lsr64,
                rd: rd,
                rn: rd.to_reg(),
                immshift: ImmShift::maybe_from_u64(2).unwrap(),
            });

            // and xd, xd, #0x3333333333333333
            ctx.emit(Inst::AluRRImmLogic {
                alu_op: ALUOp::And64,
                rd: rd,
                rn: rd.to_reg(),
                imml: ImmLogic::maybe_from_u64(0x3333333333333333, I64).unwrap(),
            });

            // add xt, xd, xt
            ctx.emit(Inst::AluRRR {
                alu_op: ALUOp::Add64,
                rd: tmp,
                rn: rd.to_reg(),
                rm: tmp.to_reg(),
            });

            // add xt, xt, xt LSR #4
            ctx.emit(Inst::AluRRRShift {
                alu_op: ALUOp::Add64,
                rd: tmp,
                rn: tmp.to_reg(),
                rm: tmp.to_reg(),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSR,
                    ShiftOpShiftImm::maybe_from_shift(4).unwrap(),
                ),
            });

            // and xt, xt, #0x0f0f0f0f0f0f0f0f
            ctx.emit(Inst::AluRRImmLogic {
                alu_op: ALUOp::And64,
                rd: tmp,
                rn: tmp.to_reg(),
                imml: ImmLogic::maybe_from_u64(0x0f0f0f0f0f0f0f0f, I64).unwrap(),
            });

            // add xt, xt, xt, LSL #8
            ctx.emit(Inst::AluRRRShift {
                alu_op: ALUOp::Add64,
                rd: tmp,
                rn: tmp.to_reg(),
                rm: tmp.to_reg(),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(8).unwrap(),
                ),
            });

            // add xt, xt, xt, LSL #16
            ctx.emit(Inst::AluRRRShift {
                alu_op: ALUOp::Add64,
                rd: tmp,
                rn: tmp.to_reg(),
                rm: tmp.to_reg(),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(16).unwrap(),
                ),
            });

            // add xt, xt, xt, LSL #32
            ctx.emit(Inst::AluRRRShift {
                alu_op: ALUOp::Add64,
                rd: tmp,
                rn: tmp.to_reg(),
                rm: tmp.to_reg(),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(32).unwrap(),
                ),
            });

            // lsr xd, xt, #56
            ctx.emit(Inst::AluRRImmShift {
                alu_op: ALUOp::Lsr64,
                rd: rd,
                rn: tmp.to_reg(),
                immshift: ImmShift::maybe_from_u64(56).unwrap(),
            });
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
        | Opcode::Sload32Complex
        | Opcode::Sload8x8
        | Opcode::Uload8x8
        | Opcode::Sload16x4
        | Opcode::Uload16x4
        | Opcode::Sload32x2
        | Opcode::Uload32x2 => {
            let off = ldst_offset(ctx.data(insn)).unwrap();
            let elem_ty = match op {
                Opcode::Sload8 | Opcode::Uload8 | Opcode::Sload8Complex | Opcode::Uload8Complex => {
                    I8
                }
                Opcode::Sload16
                | Opcode::Uload16
                | Opcode::Sload16Complex
                | Opcode::Uload16Complex => I16,
                Opcode::Sload32
                | Opcode::Uload32
                | Opcode::Sload32Complex
                | Opcode::Uload32Complex => I32,
                Opcode::Load | Opcode::LoadComplex => ctx.output_ty(insn, 0),
                Opcode::Sload8x8 | Opcode::Uload8x8 => I8X8,
                Opcode::Sload16x4 | Opcode::Uload16x4 => I16X4,
                Opcode::Sload32x2 | Opcode::Uload32x2 => I32X2,
                _ => unreachable!(),
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
            let is_float = ty_is_float(elem_ty);

            let mem = lower_address(ctx, elem_ty, &inputs[..], off);
            let rd = get_output_reg(ctx, outputs[0]);

            let memflags = ctx.memflags(insn).expect("memory flags");
            let srcloc = if !memflags.notrap() {
                Some(ctx.srcloc(insn))
            } else {
                None
            };

            ctx.emit(match (ty_bits(elem_ty), sign_extend, is_float) {
                (1, _, _) => Inst::ULoad8 { rd, mem, srcloc },
                (8, false, _) => Inst::ULoad8 { rd, mem, srcloc },
                (8, true, _) => Inst::SLoad8 { rd, mem, srcloc },
                (16, false, _) => Inst::ULoad16 { rd, mem, srcloc },
                (16, true, _) => Inst::SLoad16 { rd, mem, srcloc },
                (32, false, false) => Inst::ULoad32 { rd, mem, srcloc },
                (32, true, false) => Inst::SLoad32 { rd, mem, srcloc },
                (32, _, true) => Inst::FpuLoad32 { rd, mem, srcloc },
                (64, _, false) => Inst::ULoad64 { rd, mem, srcloc },
                // Note that we treat some of the vector loads as scalar floating-point loads,
                // which is correct in a little endian environment.
                (64, _, true) => Inst::FpuLoad64 { rd, mem, srcloc },
                (128, _, _) => Inst::FpuLoad128 { rd, mem, srcloc },
                _ => panic!("Unsupported size in load"),
            });

            let vec_extend = match op {
                Opcode::Sload8x8 => Some(VecExtendOp::Sxtl8),
                Opcode::Uload8x8 => Some(VecExtendOp::Uxtl8),
                Opcode::Sload16x4 => Some(VecExtendOp::Sxtl16),
                Opcode::Uload16x4 => Some(VecExtendOp::Uxtl16),
                Opcode::Sload32x2 => Some(VecExtendOp::Sxtl32),
                Opcode::Uload32x2 => Some(VecExtendOp::Uxtl32),
                _ => None,
            };

            if let Some(t) = vec_extend {
                ctx.emit(Inst::VecExtend {
                    t,
                    rd,
                    rn: rd.to_reg(),
                });
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
            let off = ldst_offset(ctx.data(insn)).unwrap();
            let elem_ty = match op {
                Opcode::Istore8 | Opcode::Istore8Complex => I8,
                Opcode::Istore16 | Opcode::Istore16Complex => I16,
                Opcode::Istore32 | Opcode::Istore32Complex => I32,
                Opcode::Store | Opcode::StoreComplex => ctx.input_ty(insn, 0),
                _ => unreachable!(),
            };
            let is_float = ty_is_float(elem_ty);

            let mem = lower_address(ctx, elem_ty, &inputs[1..], off);
            let rd = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);

            let memflags = ctx.memflags(insn).expect("memory flags");
            let srcloc = if !memflags.notrap() {
                Some(ctx.srcloc(insn))
            } else {
                None
            };

            ctx.emit(match (ty_bits(elem_ty), is_float) {
                (1, _) | (8, _) => Inst::Store8 { rd, mem, srcloc },
                (16, _) => Inst::Store16 { rd, mem, srcloc },
                (32, false) => Inst::Store32 { rd, mem, srcloc },
                (32, true) => Inst::FpuStore32 { rd, mem, srcloc },
                (64, false) => Inst::Store64 { rd, mem, srcloc },
                (64, true) => Inst::FpuStore64 { rd, mem, srcloc },
                (128, _) => Inst::FpuStore128 { rd, mem, srcloc },
                _ => panic!("Unsupported size in store"),
            });
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
            let rd = get_output_reg(ctx, outputs[0]);
            let offset: i32 = offset.into();
            let inst = ctx
                .abi()
                .stackslot_addr(stack_slot, u32::try_from(offset).unwrap(), rd);
            ctx.emit(inst);
        }

        Opcode::StackLoad | Opcode::StackStore => {
            panic!("Direct stack memory access not supported; should not be used by Wasm");
        }

        Opcode::HeapAddr => {
            panic!("heap_addr should have been removed by legalization!");
        }

        Opcode::TableAddr => {
            panic!("table_addr should have been removed by legalization!");
        }

        Opcode::ConstAddr => unimplemented!(),

        Opcode::Nop => {
            // Nothing.
        }

        Opcode::Select | Opcode::Selectif | Opcode::SelectifSpectreGuard => {
            let cond = if op == Opcode::Select {
                let (cmp_op, narrow_mode) = if ty_bits(ctx.input_ty(insn, 0)) > 32 {
                    (ALUOp::SubS64, NarrowValueMode::ZeroExtend64)
                } else {
                    (ALUOp::SubS32, NarrowValueMode::ZeroExtend32)
                };

                let rcond = put_input_in_reg(ctx, inputs[0], narrow_mode);
                // cmp rcond, #0
                ctx.emit(Inst::AluRRR {
                    alu_op: cmp_op,
                    rd: writable_zero_reg(),
                    rn: rcond,
                    rm: zero_reg(),
                });
                Cond::Ne
            } else {
                let condcode = inst_condcode(ctx.data(insn)).unwrap();
                let cond = lower_condcode(condcode);
                let is_signed = condcode_is_signed(condcode);
                // Verification ensures that the input is always a
                // single-def ifcmp.
                let ifcmp_insn = maybe_input_insn(ctx, inputs[0], Opcode::Ifcmp).unwrap();
                lower_icmp_or_ifcmp_to_flags(ctx, ifcmp_insn, is_signed);
                cond
            };

            // csel.COND rd, rn, rm
            let rd = get_output_reg(ctx, outputs[0]);
            let rn = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
            let rm = put_input_in_reg(ctx, inputs[2], NarrowValueMode::None);
            let ty = ctx.output_ty(insn, 0);
            let bits = ty_bits(ty);
            if ty_is_float(ty) && bits == 32 {
                ctx.emit(Inst::FpuCSel32 { cond, rd, rn, rm });
            } else if ty_is_float(ty) && bits == 64 {
                ctx.emit(Inst::FpuCSel64 { cond, rd, rn, rm });
            } else {
                ctx.emit(Inst::CSel { cond, rd, rn, rm });
            }
        }

        Opcode::Bitselect => {
            let ty = ty.unwrap();
            if ty_bits(ty) < 128 {
                let tmp = ctx.alloc_tmp(RegClass::I64, I64);
                let rd = get_output_reg(ctx, outputs[0]);
                let rcond = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                let rn = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
                let rm = put_input_in_reg(ctx, inputs[2], NarrowValueMode::None);
                // AND rTmp, rn, rcond
                ctx.emit(Inst::AluRRR {
                    alu_op: ALUOp::And64,
                    rd: tmp,
                    rn,
                    rm: rcond,
                });
                // BIC rd, rm, rcond
                ctx.emit(Inst::AluRRR {
                    alu_op: ALUOp::AndNot64,
                    rd,
                    rn: rm,
                    rm: rcond,
                });
                // ORR rd, rd, rTmp
                ctx.emit(Inst::AluRRR {
                    alu_op: ALUOp::Orr64,
                    rd,
                    rn: rd.to_reg(),
                    rm: tmp.to_reg(),
                });
            } else {
                let rcond = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                let rn = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
                let rm = put_input_in_reg(ctx, inputs[2], NarrowValueMode::None);
                let rd = get_output_reg(ctx, outputs[0]);
                ctx.emit(Inst::gen_move(rd, rcond, ty));

                ctx.emit(Inst::VecRRR {
                    alu_op: VecALUOp::Bsl,
                    rd,
                    rn,
                    rm,
                    ty,
                });
            }
        }

        Opcode::Trueif => {
            let condcode = inst_condcode(ctx.data(insn)).unwrap();
            let cond = lower_condcode(condcode);
            let is_signed = condcode_is_signed(condcode);
            // Verification ensures that the input is always a
            // single-def ifcmp.
            let ifcmp_insn = maybe_input_insn(ctx, inputs[0], Opcode::Ifcmp).unwrap();
            lower_icmp_or_ifcmp_to_flags(ctx, ifcmp_insn, is_signed);
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::CSet { rd, cond });
        }

        Opcode::Trueff => {
            let condcode = inst_fp_condcode(ctx.data(insn)).unwrap();
            let cond = lower_fp_condcode(condcode);
            let ffcmp_insn = maybe_input_insn(ctx, inputs[0], Opcode::Ffcmp).unwrap();
            lower_fcmp_or_ffcmp_to_flags(ctx, ffcmp_insn);
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::CSet { rd, cond });
        }

        Opcode::IsNull | Opcode::IsInvalid => {
            panic!("Reference types not supported");
        }

        Opcode::Copy => {
            let rd = get_output_reg(ctx, outputs[0]);
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let ty = ctx.input_ty(insn, 0);
            ctx.emit(Inst::gen_move(rd, rn, ty));
        }

        Opcode::Bint | Opcode::Breduce | Opcode::Bextend | Opcode::Ireduce => {
            // All of these ops are simply a move from a zero-extended source.
            // Here is why this works, in each case:
            //
            // - Bint: Bool-to-int. We always represent a bool as a 0 or 1, so we
            //   merely need to zero-extend here.
            //
            // - Breduce, Bextend: changing width of a boolean. We represent a
            //   bool as a 0 or 1, so again, this is a zero-extend / no-op.
            //
            // - Ireduce: changing width of an integer. Smaller ints are stored
            //   with undefined high-order bits, so we can simply do a copy.

            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::ZeroExtend64);
            let rd = get_output_reg(ctx, outputs[0]);
            let ty = ctx.input_ty(insn, 0);
            ctx.emit(Inst::gen_move(rd, rn, ty));
        }

        Opcode::Bmask => {
            // Bool is {0, 1}, so we can subtract from 0 to get all-1s.
            let rd = get_output_reg(ctx, outputs[0]);
            let rm = put_input_in_reg(ctx, inputs[0], NarrowValueMode::ZeroExtend64);
            ctx.emit(Inst::AluRRR {
                alu_op: ALUOp::Sub64,
                rd,
                rn: zero_reg(),
                rm,
            });
        }

        Opcode::Bitcast => {
            let rd = get_output_reg(ctx, outputs[0]);
            let ity = ctx.input_ty(insn, 0);
            let oty = ctx.output_ty(insn, 0);
            match (ty_is_float(ity), ty_is_float(oty)) {
                (true, true) => {
                    let narrow_mode = if ty_bits(ity) <= 32 && ty_bits(oty) <= 32 {
                        NarrowValueMode::ZeroExtend32
                    } else {
                        NarrowValueMode::ZeroExtend64
                    };
                    let rm = put_input_in_reg(ctx, inputs[0], narrow_mode);
                    ctx.emit(Inst::gen_move(rd, rm, oty));
                }
                (false, false) => {
                    let rm = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                    ctx.emit(Inst::gen_move(rd, rm, oty));
                }
                (false, true) => {
                    let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::ZeroExtend64);
                    ctx.emit(Inst::MovToVec64 { rd, rn });
                }
                (true, false) => {
                    let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                    ctx.emit(Inst::MovFromVec {
                        rd,
                        rn,
                        idx: 0,
                        ty: I64,
                    });
                }
            }
        }

        Opcode::FallthroughReturn | Opcode::Return => {
            for (i, input) in inputs.iter().enumerate() {
                // N.B.: according to the AArch64 ABI, the top bits of a register
                // (above the bits for the value's type) are undefined, so we
                // need not extend the return values.
                let reg = put_input_in_reg(ctx, *input, NarrowValueMode::None);
                let retval_reg = ctx.retval(i);
                let ty = ctx.input_ty(insn, i);
                ctx.emit(Inst::gen_move(retval_reg, reg, ty));
            }
            // N.B.: the Ret itself is generated by the ABI.
        }

        Opcode::Ifcmp | Opcode::Ffcmp => {
            // An Ifcmp/Ffcmp must always be seen as a use of a brif/brff or trueif/trueff
            // instruction. This will always be the case as long as the IR uses an Ifcmp/Ffcmp from
            // the same block, or a dominating block. In other words, it cannot pass through a BB
            // param (phi). The flags pass of the verifier will ensure this.
            panic!("Should never reach ifcmp as isel root!");
        }

        Opcode::Icmp => {
            let condcode = inst_condcode(ctx.data(insn)).unwrap();
            let cond = lower_condcode(condcode);
            let is_signed = condcode_is_signed(condcode);
            let rd = get_output_reg(ctx, outputs[0]);
            let ty = ctx.input_ty(insn, 0);
            let bits = ty_bits(ty);
            let narrow_mode = match (bits <= 32, is_signed) {
                (true, true) => NarrowValueMode::SignExtend32,
                (true, false) => NarrowValueMode::ZeroExtend32,
                (false, true) => NarrowValueMode::SignExtend64,
                (false, false) => NarrowValueMode::ZeroExtend64,
            };
            let rn = put_input_in_reg(ctx, inputs[0], narrow_mode);

            if ty_bits(ty) < 128 {
                let alu_op = choose_32_64(ty, ALUOp::SubS32, ALUOp::SubS64);
                let rm = put_input_in_rse_imm12(ctx, inputs[1], narrow_mode);
                ctx.emit(alu_inst_imm12(alu_op, writable_zero_reg(), rn, rm));
                ctx.emit(Inst::CondSet { cond, rd });
            } else {
                let rm = put_input_in_reg(ctx, inputs[1], narrow_mode);
                lower_vector_compare(ctx, rd, rn, rm, ty, cond)?;
            }
        }

        Opcode::Fcmp => {
            let condcode = inst_fp_condcode(ctx.data(insn)).unwrap();
            let cond = lower_fp_condcode(condcode);
            let ty = ctx.input_ty(insn, 0);
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);

            if ty_bits(ty) < 128 {
                match ty_bits(ty) {
                    32 => {
                        ctx.emit(Inst::FpuCmp32 { rn, rm });
                    }
                    64 => {
                        ctx.emit(Inst::FpuCmp64 { rn, rm });
                    }
                    _ => panic!("Bad float size"),
                }
                ctx.emit(Inst::CondSet { cond, rd });
            } else {
                lower_vector_compare(ctx, rd, rn, rm, ty, cond)?;
            }
        }

        Opcode::JumpTableEntry | Opcode::JumpTableBase => {
            panic!("Should not appear: we handle BrTable directly");
        }

        Opcode::Debugtrap => {
            ctx.emit(Inst::Brk);
        }

        Opcode::Trap | Opcode::ResumableTrap => {
            let trap_info = (ctx.srcloc(insn), inst_trapcode(ctx.data(insn)).unwrap());
            ctx.emit(Inst::Udf { trap_info })
        }

        Opcode::Trapif | Opcode::Trapff => {
            let trap_info = (ctx.srcloc(insn), inst_trapcode(ctx.data(insn)).unwrap());

            let cond = if maybe_input_insn(ctx, inputs[0], Opcode::IaddIfcout).is_some() {
                let condcode = inst_condcode(ctx.data(insn)).unwrap();
                let cond = lower_condcode(condcode);
                // The flags must not have been clobbered by any other
                // instruction between the iadd_ifcout and this instruction, as
                // verified by the CLIF validator; so we can simply use the
                // flags here.
                cond
            } else if op == Opcode::Trapif {
                let condcode = inst_condcode(ctx.data(insn)).unwrap();
                let cond = lower_condcode(condcode);
                let is_signed = condcode_is_signed(condcode);

                // Verification ensures that the input is always a single-def ifcmp.
                let ifcmp_insn = maybe_input_insn(ctx, inputs[0], Opcode::Ifcmp).unwrap();
                lower_icmp_or_ifcmp_to_flags(ctx, ifcmp_insn, is_signed);
                cond
            } else {
                let condcode = inst_fp_condcode(ctx.data(insn)).unwrap();
                let cond = lower_fp_condcode(condcode);

                // Verification ensures that the input is always a
                // single-def ffcmp.
                let ffcmp_insn = maybe_input_insn(ctx, inputs[0], Opcode::Ffcmp).unwrap();
                lower_fcmp_or_ffcmp_to_flags(ctx, ffcmp_insn);
                cond
            };

            ctx.emit(Inst::TrapIf {
                trap_info,
                kind: CondBrKind::Cond(cond),
            });
        }

        Opcode::Safepoint => {
            panic!("safepoint support not implemented!");
        }

        Opcode::Trapz | Opcode::Trapnz | Opcode::ResumableTrapnz => {
            panic!("trapz / trapnz / resumable_trapnz should have been removed by legalization!");
        }

        Opcode::FuncAddr => {
            let rd = get_output_reg(ctx, outputs[0]);
            let (extname, _) = ctx.call_target(insn).unwrap();
            let extname = extname.clone();
            let loc = ctx.srcloc(insn);
            ctx.emit(Inst::LoadExtName {
                rd,
                name: Box::new(extname),
                srcloc: loc,
                offset: 0,
            });
        }

        Opcode::GlobalValue => {
            panic!("global_value should have been removed by legalization!");
        }

        Opcode::SymbolValue => {
            let rd = get_output_reg(ctx, outputs[0]);
            let (extname, _, offset) = ctx.symbol_value(insn).unwrap();
            let extname = extname.clone();
            let loc = ctx.srcloc(insn);
            ctx.emit(Inst::LoadExtName {
                rd,
                name: Box::new(extname),
                srcloc: loc,
                offset,
            });
        }

        Opcode::Call | Opcode::CallIndirect => {
            let loc = ctx.srcloc(insn);
            let (mut abi, inputs) = match op {
                Opcode::Call => {
                    let (extname, dist) = ctx.call_target(insn).unwrap();
                    let extname = extname.clone();
                    let sig = ctx.call_sig(insn).unwrap();
                    assert!(inputs.len() == sig.params.len());
                    assert!(outputs.len() == sig.returns.len());
                    (
                        AArch64ABICall::from_func(sig, &extname, dist, loc)?,
                        &inputs[..],
                    )
                }
                Opcode::CallIndirect => {
                    let ptr = put_input_in_reg(ctx, inputs[0], NarrowValueMode::ZeroExtend64);
                    let sig = ctx.call_sig(insn).unwrap();
                    assert!(inputs.len() - 1 == sig.params.len());
                    assert!(outputs.len() == sig.returns.len());
                    (AArch64ABICall::from_ptr(sig, ptr, loc, op)?, &inputs[1..])
                }
                _ => unreachable!(),
            };

            abi.emit_stack_pre_adjust(ctx);
            assert!(inputs.len() == abi.num_args());
            for (i, input) in inputs.iter().enumerate() {
                let arg_reg = put_input_in_reg(ctx, *input, NarrowValueMode::None);
                abi.emit_copy_reg_to_arg(ctx, i, arg_reg);
            }
            abi.emit_call(ctx);
            for (i, output) in outputs.iter().enumerate() {
                let retval_reg = get_output_reg(ctx, *output);
                abi.emit_copy_retval_to_reg(ctx, i, retval_reg);
            }
            abi.emit_stack_post_adjust(ctx);
        }

        Opcode::GetPinnedReg => {
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::mov(rd, xreg(PINNED_REG)));
        }

        Opcode::SetPinnedReg => {
            let rm = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            ctx.emit(Inst::mov(writable_xreg(PINNED_REG), rm));
        }

        Opcode::Spill
        | Opcode::Fill
        | Opcode::FillNop
        | Opcode::Regmove
        | Opcode::CopySpecial
        | Opcode::CopyToSsa
        | Opcode::CopyNop
        | Opcode::AdjustSpDown
        | Opcode::AdjustSpUpImm
        | Opcode::AdjustSpDownImm
        | Opcode::IfcmpSp
        | Opcode::Regspill
        | Opcode::Regfill => {
            panic!("Unused opcode should not be encountered.");
        }

        Opcode::Jump
        | Opcode::Fallthrough
        | Opcode::Brz
        | Opcode::Brnz
        | Opcode::BrIcmp
        | Opcode::Brif
        | Opcode::Brff
        | Opcode::IndirectJumpTableBr
        | Opcode::BrTable => {
            panic!("Branch opcode reached non-branch lowering logic!");
        }

        Opcode::Vconst => {
            let value = output_to_const_f128(ctx, outputs[0]).unwrap();
            let rd = get_output_reg(ctx, outputs[0]);
            lower_constant_f128(ctx, rd, value);
        }

        Opcode::RawBitcast => {
            let rm = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);
            let ty = ctx.input_ty(insn, 0);
            ctx.emit(Inst::gen_move(rd, rm, ty));
        }

        Opcode::Extractlane => {
            if let InstructionData::BinaryImm8 { imm, .. } = ctx.data(insn) {
                let idx = *imm;
                let rd = get_output_reg(ctx, outputs[0]);
                let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
                let ty = ty.unwrap();

                if ty_is_int(ty) {
                    ctx.emit(Inst::MovFromVec { rd, rn, idx, ty });
                // Plain moves are faster on some processors.
                } else if idx == 0 {
                    ctx.emit(Inst::gen_move(rd, rn, ty));
                } else {
                    ctx.emit(Inst::FpuMoveFromVec { rd, rn, idx, ty });
                }
            } else {
                unreachable!();
            }
        }

        Opcode::Splat => {
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);
            let ty = ctx.input_ty(insn, 0);
            let inst = if ty_is_int(ty) {
                Inst::VecDup { rd, rn, ty }
            } else {
                Inst::VecDupFromFpu { rd, rn, ty }
            };
            ctx.emit(inst);
        }

        Opcode::VanyTrue | Opcode::VallTrue => {
            let rd = get_output_reg(ctx, outputs[0]);
            let rm = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let tmp = ctx.alloc_tmp(RegClass::V128, ty.unwrap());

            // This operation is implemented by using umaxp or uminv to
            // create a scalar value, which is then compared against zero.
            //
            // umaxp vn.16b, vm.16, vm.16 / uminv bn, vm.16b
            // mov xm, vn.d[0]
            // cmp xm, #0
            // cset xm, ne

            let input_ty = ctx.input_ty(insn, 0);
            if op == Opcode::VanyTrue {
                ctx.emit(Inst::VecRRR {
                    alu_op: VecALUOp::Umaxp,
                    rd: tmp,
                    rn: rm,
                    rm: rm,
                    ty: input_ty,
                });
            } else {
                ctx.emit(Inst::VecLanes {
                    op: VecLanesOp::Uminv,
                    rd: tmp,
                    rn: rm,
                    ty: input_ty,
                });
            };

            ctx.emit(Inst::MovFromVec {
                rd,
                rn: tmp.to_reg(),
                idx: 0,
                ty: I64,
            });

            ctx.emit(Inst::AluRRImm12 {
                alu_op: ALUOp::SubS64,
                rd: writable_zero_reg(),
                rn: rd.to_reg(),
                imm12: Imm12::zero(),
            });

            ctx.emit(Inst::CSet { rd, cond: Cond::Ne });
        }

        Opcode::Shuffle
        | Opcode::Vsplit
        | Opcode::Vconcat
        | Opcode::Vselect
        | Opcode::Insertlane
        | Opcode::ScalarToVector
        | Opcode::Swizzle
        | Opcode::Uload8x8Complex
        | Opcode::Sload8x8Complex
        | Opcode::Uload16x4Complex
        | Opcode::Sload16x4Complex
        | Opcode::Uload32x2Complex
        | Opcode::Sload32x2Complex => {
            // TODO
            panic!("Vector ops not implemented.");
        }

        Opcode::Isplit | Opcode::Iconcat => panic!("Vector ops not supported."),
        Opcode::Imax | Opcode::Imin | Opcode::Umin | Opcode::Umax => {
            panic!("Vector ops not supported.")
        }

        Opcode::Fadd | Opcode::Fsub | Opcode::Fmul | Opcode::Fdiv | Opcode::Fmin | Opcode::Fmax => {
            let bits = ty_bits(ctx.output_ty(insn, 0));
            let fpu_op = match (op, bits) {
                (Opcode::Fadd, 32) => FPUOp2::Add32,
                (Opcode::Fadd, 64) => FPUOp2::Add64,
                (Opcode::Fsub, 32) => FPUOp2::Sub32,
                (Opcode::Fsub, 64) => FPUOp2::Sub64,
                (Opcode::Fmul, 32) => FPUOp2::Mul32,
                (Opcode::Fmul, 64) => FPUOp2::Mul64,
                (Opcode::Fdiv, 32) => FPUOp2::Div32,
                (Opcode::Fdiv, 64) => FPUOp2::Div64,
                (Opcode::Fmin, 32) => FPUOp2::Min32,
                (Opcode::Fmin, 64) => FPUOp2::Min64,
                (Opcode::Fmax, 32) => FPUOp2::Max32,
                (Opcode::Fmax, 64) => FPUOp2::Max64,
                _ => panic!("Unknown op/bits combination"),
            };
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::FpuRRR { fpu_op, rd, rn, rm });
        }

        Opcode::Sqrt | Opcode::Fneg | Opcode::Fabs | Opcode::Fpromote | Opcode::Fdemote => {
            let bits = ty_bits(ctx.output_ty(insn, 0));
            let fpu_op = match (op, bits) {
                (Opcode::Sqrt, 32) => FPUOp1::Sqrt32,
                (Opcode::Sqrt, 64) => FPUOp1::Sqrt64,
                (Opcode::Fneg, 32) => FPUOp1::Neg32,
                (Opcode::Fneg, 64) => FPUOp1::Neg64,
                (Opcode::Fabs, 32) => FPUOp1::Abs32,
                (Opcode::Fabs, 64) => FPUOp1::Abs64,
                (Opcode::Fpromote, 32) => panic!("Cannot promote to 32 bits"),
                (Opcode::Fpromote, 64) => FPUOp1::Cvt32To64,
                (Opcode::Fdemote, 32) => FPUOp1::Cvt64To32,
                (Opcode::Fdemote, 64) => panic!("Cannot demote to 64 bits"),
                _ => panic!("Unknown op/bits combination"),
            };
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::FpuRR { fpu_op, rd, rn });
        }

        Opcode::Ceil | Opcode::Floor | Opcode::Trunc | Opcode::Nearest => {
            let bits = ty_bits(ctx.output_ty(insn, 0));
            let op = match (op, bits) {
                (Opcode::Ceil, 32) => FpuRoundMode::Plus32,
                (Opcode::Ceil, 64) => FpuRoundMode::Plus64,
                (Opcode::Floor, 32) => FpuRoundMode::Minus32,
                (Opcode::Floor, 64) => FpuRoundMode::Minus64,
                (Opcode::Trunc, 32) => FpuRoundMode::Zero32,
                (Opcode::Trunc, 64) => FpuRoundMode::Zero64,
                (Opcode::Nearest, 32) => FpuRoundMode::Nearest32,
                (Opcode::Nearest, 64) => FpuRoundMode::Nearest64,
                _ => panic!("Unknown op/bits combination"),
            };
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::FpuRound { op, rd, rn });
        }

        Opcode::Fma => {
            let bits = ty_bits(ctx.output_ty(insn, 0));
            let fpu_op = match bits {
                32 => FPUOp3::MAdd32,
                64 => FPUOp3::MAdd64,
                _ => panic!("Unknown op size"),
            };
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
            let ra = put_input_in_reg(ctx, inputs[2], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::FpuRRRR {
                fpu_op,
                rn,
                rm,
                ra,
                rd,
            });
        }

        Opcode::Fcopysign => {
            // Copy the sign bit from inputs[1] to inputs[0]. We use the following sequence:
            //
            // This is a scalar Fcopysign.
            // This uses scalar NEON operations for 64-bit and vector operations (2S) for 32-bit.
            //
            //  mov vd, vn
            //  ushr vtmp, vm, #63 / #31
            //  sli vd, vtmp, #63 / #31

            let ty = ctx.output_ty(insn, 0);
            let bits = ty_bits(ty) as u8;
            assert!(bits == 32 || bits == 64);
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);
            let tmp = ctx.alloc_tmp(RegClass::V128, F64);

            // Copy LHS to rd.
            ctx.emit(Inst::FpuMove64 { rd, rn });

            // Copy the sign bit to the lowest bit in tmp.
            let imm = FPURightShiftImm::maybe_from_u8(bits - 1, bits).unwrap();
            ctx.emit(Inst::FpuRRI {
                fpu_op: choose_32_64(ty, FPUOpRI::UShr32(imm), FPUOpRI::UShr64(imm)),
                rd: tmp,
                rn: rm,
            });

            // Insert the bit from tmp into the sign bit of rd.
            let imm = FPULeftShiftImm::maybe_from_u8(bits - 1, bits).unwrap();
            ctx.emit(Inst::FpuRRI {
                fpu_op: choose_32_64(ty, FPUOpRI::Sli32(imm), FPUOpRI::Sli64(imm)),
                rd,
                rn: tmp.to_reg(),
            });
        }

        Opcode::FcvtToUint | Opcode::FcvtToSint => {
            let in_bits = ty_bits(ctx.input_ty(insn, 0));
            let out_bits = ty_bits(ctx.output_ty(insn, 0));
            let signed = op == Opcode::FcvtToSint;
            let op = match (signed, in_bits, out_bits) {
                (false, 32, 32) => FpuToIntOp::F32ToU32,
                (true, 32, 32) => FpuToIntOp::F32ToI32,
                (false, 32, 64) => FpuToIntOp::F32ToU64,
                (true, 32, 64) => FpuToIntOp::F32ToI64,
                (false, 64, 32) => FpuToIntOp::F64ToU32,
                (true, 64, 32) => FpuToIntOp::F64ToI32,
                (false, 64, 64) => FpuToIntOp::F64ToU64,
                (true, 64, 64) => FpuToIntOp::F64ToI64,
                _ => panic!("Unknown input/output-bits combination"),
            };

            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);

            // First, check the output: it's important to carry the NaN conversion before the
            // in-bounds conversion, per wasm semantics.

            // Check that the input is not a NaN.
            if in_bits == 32 {
                ctx.emit(Inst::FpuCmp32 { rn, rm: rn });
            } else {
                ctx.emit(Inst::FpuCmp64 { rn, rm: rn });
            }
            let trap_info = (ctx.srcloc(insn), TrapCode::BadConversionToInteger);
            ctx.emit(Inst::TrapIf {
                trap_info,
                kind: CondBrKind::Cond(lower_fp_condcode(FloatCC::Unordered)),
            });

            let tmp = ctx.alloc_tmp(RegClass::V128, I128);

            // Check that the input is in range, with "truncate towards zero" semantics. This means
            // we allow values that are slightly out of range:
            // - for signed conversions, we allow values strictly greater than INT_MIN-1 (when this
            // can be represented), and strictly less than INT_MAX+1 (when this can be
            // represented).
            // - for unsigned conversions, we allow values strictly greater than -1, and strictly
            // less than UINT_MAX+1 (when this can be represented).

            if in_bits == 32 {
                // From float32.
                let (low_bound, low_cond, high_bound) = match (signed, out_bits) {
                    (true, 32) => (
                        i32::min_value() as f32, // I32_MIN - 1 isn't precisely representable as a f32.
                        FloatCC::GreaterThanOrEqual,
                        i32::max_value() as f32 + 1.,
                    ),
                    (true, 64) => (
                        i64::min_value() as f32, // I64_MIN - 1 isn't precisely representable as a f32.
                        FloatCC::GreaterThanOrEqual,
                        i64::max_value() as f32 + 1.,
                    ),
                    (false, 32) => (-1., FloatCC::GreaterThan, u32::max_value() as f32 + 1.),
                    (false, 64) => (-1., FloatCC::GreaterThan, u64::max_value() as f32 + 1.),
                    _ => panic!("Unknown input/output-bits combination"),
                };

                // >= low_bound
                lower_constant_f32(ctx, tmp, low_bound);
                ctx.emit(Inst::FpuCmp32 {
                    rn,
                    rm: tmp.to_reg(),
                });
                let trap_info = (ctx.srcloc(insn), TrapCode::IntegerOverflow);
                ctx.emit(Inst::TrapIf {
                    trap_info,
                    kind: CondBrKind::Cond(lower_fp_condcode(low_cond).invert()),
                });

                // <= high_bound
                lower_constant_f32(ctx, tmp, high_bound);
                ctx.emit(Inst::FpuCmp32 {
                    rn,
                    rm: tmp.to_reg(),
                });
                let trap_info = (ctx.srcloc(insn), TrapCode::IntegerOverflow);
                ctx.emit(Inst::TrapIf {
                    trap_info,
                    kind: CondBrKind::Cond(lower_fp_condcode(FloatCC::LessThan).invert()),
                });
            } else {
                // From float64.
                let (low_bound, low_cond, high_bound) = match (signed, out_bits) {
                    (true, 32) => (
                        i32::min_value() as f64 - 1.,
                        FloatCC::GreaterThan,
                        i32::max_value() as f64 + 1.,
                    ),
                    (true, 64) => (
                        i64::min_value() as f64, // I64_MIN - 1 is not precisely representable as an i64.
                        FloatCC::GreaterThanOrEqual,
                        i64::max_value() as f64 + 1.,
                    ),
                    (false, 32) => (-1., FloatCC::GreaterThan, u32::max_value() as f64 + 1.),
                    (false, 64) => (-1., FloatCC::GreaterThan, u64::max_value() as f64 + 1.),
                    _ => panic!("Unknown input/output-bits combination"),
                };

                // >= low_bound
                lower_constant_f64(ctx, tmp, low_bound);
                ctx.emit(Inst::FpuCmp64 {
                    rn,
                    rm: tmp.to_reg(),
                });
                let trap_info = (ctx.srcloc(insn), TrapCode::IntegerOverflow);
                ctx.emit(Inst::TrapIf {
                    trap_info,
                    kind: CondBrKind::Cond(lower_fp_condcode(low_cond).invert()),
                });

                // <= high_bound
                lower_constant_f64(ctx, tmp, high_bound);
                ctx.emit(Inst::FpuCmp64 {
                    rn,
                    rm: tmp.to_reg(),
                });
                let trap_info = (ctx.srcloc(insn), TrapCode::IntegerOverflow);
                ctx.emit(Inst::TrapIf {
                    trap_info,
                    kind: CondBrKind::Cond(lower_fp_condcode(FloatCC::LessThan).invert()),
                });
            };

            // Do the conversion.
            ctx.emit(Inst::FpuToInt { op, rd, rn });
        }

        Opcode::FcvtFromUint | Opcode::FcvtFromSint => {
            let in_bits = ty_bits(ctx.input_ty(insn, 0));
            let out_bits = ty_bits(ctx.output_ty(insn, 0));
            let signed = op == Opcode::FcvtFromSint;
            let op = match (signed, in_bits, out_bits) {
                (false, 32, 32) => IntToFpuOp::U32ToF32,
                (true, 32, 32) => IntToFpuOp::I32ToF32,
                (false, 32, 64) => IntToFpuOp::U32ToF64,
                (true, 32, 64) => IntToFpuOp::I32ToF64,
                (false, 64, 32) => IntToFpuOp::U64ToF32,
                (true, 64, 32) => IntToFpuOp::I64ToF32,
                (false, 64, 64) => IntToFpuOp::U64ToF64,
                (true, 64, 64) => IntToFpuOp::I64ToF64,
                _ => panic!("Unknown input/output-bits combination"),
            };
            let narrow_mode = match (signed, in_bits) {
                (false, 32) => NarrowValueMode::ZeroExtend32,
                (true, 32) => NarrowValueMode::SignExtend32,
                (false, 64) => NarrowValueMode::ZeroExtend64,
                (true, 64) => NarrowValueMode::SignExtend64,
                _ => panic!("Unknown input size"),
            };
            let rn = put_input_in_reg(ctx, inputs[0], narrow_mode);
            let rd = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::IntToFpu { op, rd, rn });
        }

        Opcode::FcvtToUintSat | Opcode::FcvtToSintSat => {
            let in_ty = ctx.input_ty(insn, 0);
            let in_bits = ty_bits(in_ty);
            let out_ty = ctx.output_ty(insn, 0);
            let out_bits = ty_bits(out_ty);
            let out_signed = op == Opcode::FcvtToSintSat;
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rd = get_output_reg(ctx, outputs[0]);

            // FIMM Vtmp1, u32::MAX or u64::MAX or i32::MAX or i64::MAX
            // FMIN Vtmp2, Vin, Vtmp1
            // FIMM Vtmp1, 0 or 0 or i32::MIN or i64::MIN
            // FMAX Vtmp2, Vtmp2, Vtmp1
            // (if signed) FIMM Vtmp1, 0
            // FCMP Vin, Vin
            // FCSEL Vtmp2, Vtmp1, Vtmp2, NE  // on NaN, select 0
            // convert Rout, Vtmp2

            assert!(in_bits == 32 || in_bits == 64);
            assert!(out_bits == 32 || out_bits == 64);

            let min: f64 = match (out_bits, out_signed) {
                (32, true) => std::i32::MIN as f64,
                (32, false) => 0.0,
                (64, true) => std::i64::MIN as f64,
                (64, false) => 0.0,
                _ => unreachable!(),
            };

            let max = match (out_bits, out_signed) {
                (32, true) => std::i32::MAX as f64,
                (32, false) => std::u32::MAX as f64,
                (64, true) => std::i64::MAX as f64,
                (64, false) => std::u64::MAX as f64,
                _ => unreachable!(),
            };

            let rtmp1 = ctx.alloc_tmp(RegClass::V128, in_ty);
            let rtmp2 = ctx.alloc_tmp(RegClass::V128, in_ty);

            if in_bits == 32 {
                ctx.emit(Inst::LoadFpuConst32 {
                    rd: rtmp1,
                    const_data: max as f32,
                });
            } else {
                ctx.emit(Inst::LoadFpuConst64 {
                    rd: rtmp1,
                    const_data: max,
                });
            }
            ctx.emit(Inst::FpuRRR {
                fpu_op: choose_32_64(in_ty, FPUOp2::Min32, FPUOp2::Min64),
                rd: rtmp2,
                rn: rn,
                rm: rtmp1.to_reg(),
            });
            if in_bits == 32 {
                ctx.emit(Inst::LoadFpuConst32 {
                    rd: rtmp1,
                    const_data: min as f32,
                });
            } else {
                ctx.emit(Inst::LoadFpuConst64 {
                    rd: rtmp1,
                    const_data: min,
                });
            }
            ctx.emit(Inst::FpuRRR {
                fpu_op: choose_32_64(in_ty, FPUOp2::Max32, FPUOp2::Max64),
                rd: rtmp2,
                rn: rtmp2.to_reg(),
                rm: rtmp1.to_reg(),
            });
            if out_signed {
                if in_bits == 32 {
                    ctx.emit(Inst::LoadFpuConst32 {
                        rd: rtmp1,
                        const_data: 0.0,
                    });
                } else {
                    ctx.emit(Inst::LoadFpuConst64 {
                        rd: rtmp1,
                        const_data: 0.0,
                    });
                }
            }
            if in_bits == 32 {
                ctx.emit(Inst::FpuCmp32 { rn: rn, rm: rn });
                ctx.emit(Inst::FpuCSel32 {
                    rd: rtmp2,
                    rn: rtmp1.to_reg(),
                    rm: rtmp2.to_reg(),
                    cond: Cond::Ne,
                });
            } else {
                ctx.emit(Inst::FpuCmp64 { rn: rn, rm: rn });
                ctx.emit(Inst::FpuCSel64 {
                    rd: rtmp2,
                    rn: rtmp1.to_reg(),
                    rm: rtmp2.to_reg(),
                    cond: Cond::Ne,
                });
            }

            let cvt = match (in_bits, out_bits, out_signed) {
                (32, 32, false) => FpuToIntOp::F32ToU32,
                (32, 32, true) => FpuToIntOp::F32ToI32,
                (32, 64, false) => FpuToIntOp::F32ToU64,
                (32, 64, true) => FpuToIntOp::F32ToI64,
                (64, 32, false) => FpuToIntOp::F64ToU32,
                (64, 32, true) => FpuToIntOp::F64ToI32,
                (64, 64, false) => FpuToIntOp::F64ToU64,
                (64, 64, true) => FpuToIntOp::F64ToI64,
                _ => unreachable!(),
            };
            ctx.emit(Inst::FpuToInt {
                op: cvt,
                rd,
                rn: rtmp2.to_reg(),
            });
        }

        Opcode::IaddIfcout => {
            // This is a two-output instruction that is needed for the
            // legalizer's explicit heap-check sequence, among possible other
            // uses. Its second output is a flags output only ever meant to
            // check for overflow using the
            // `backend.unsigned_add_overflow_condition()` condition.
            //
            // Note that the CLIF validation will ensure that no flag-setting
            // operation comes between this IaddIfcout and its use (e.g., a
            // Trapif). Thus, we can rely on implicit communication through the
            // processor flags rather than explicitly generating flags into a
            // register. We simply use the variant of the add instruction that
            // sets flags (`adds`) here.

            // Ensure that the second output isn't directly called for: it
            // should only be used by a flags-consuming op, which will directly
            // understand this instruction and merge the comparison.
            assert!(!ctx.is_reg_needed(insn, ctx.get_output(insn, 1).to_reg()));

            // Now handle the iadd as above, except use an AddS opcode that sets
            // flags.
            let rd = get_output_reg(ctx, outputs[0]);
            let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = put_input_in_rse_imm12(ctx, inputs[1], NarrowValueMode::None);
            let ty = ty.unwrap();
            let alu_op = choose_32_64(ty, ALUOp::AddS32, ALUOp::AddS64);
            ctx.emit(alu_inst_imm12(alu_op, rd, rn, rm));
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
        | Opcode::SshrImm
        | Opcode::IcmpImm
        | Opcode::IfcmpImm => {
            panic!("ALU+imm and ALU+carry ops should not appear here!");
        }

        #[cfg(feature = "x86")]
        Opcode::X86Udivmodx
        | Opcode::X86Sdivmodx
        | Opcode::X86Umulx
        | Opcode::X86Smulx
        | Opcode::X86Cvtt2si
        | Opcode::X86Fmin
        | Opcode::X86Fmax
        | Opcode::X86Push
        | Opcode::X86Pop
        | Opcode::X86Bsr
        | Opcode::X86Bsf
        | Opcode::X86Pblendw
        | Opcode::X86Pshufd
        | Opcode::X86Pshufb
        | Opcode::X86Pextr
        | Opcode::X86Pinsr
        | Opcode::X86Insertps
        | Opcode::X86Movsd
        | Opcode::X86Movlhps
        | Opcode::X86Psll
        | Opcode::X86Psrl
        | Opcode::X86Psra
        | Opcode::X86Ptest
        | Opcode::X86Pmaxs
        | Opcode::X86Pmaxu
        | Opcode::X86Pmins
        | Opcode::X86Pminu
        | Opcode::X86Pmullq
        | Opcode::X86Pmuludq
        | Opcode::X86Punpckh
        | Opcode::X86Punpckl
        | Opcode::X86Vcvtudq2ps
        | Opcode::X86ElfTlsGetAddr
        | Opcode::X86MachoTlsGetAddr => {
            panic!("x86-specific opcode in supposedly arch-neutral IR!");
        }

        Opcode::AvgRound => unimplemented!(),
        Opcode::Iabs => unimplemented!(),
        Opcode::Snarrow | Opcode::Unarrow => unimplemented!(),
        Opcode::TlsValue => unimplemented!(),
    }

    Ok(())
}

pub(crate) fn lower_branch<C: LowerCtx<I = Inst>>(
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

        assert!(op1 == Opcode::Jump || op1 == Opcode::Fallthrough);
        let taken = BranchTarget::Label(targets[0]);
        let not_taken = match op1 {
            Opcode::Jump => BranchTarget::Label(targets[1]),
            Opcode::Fallthrough => BranchTarget::Label(fallthrough.unwrap()),
            _ => unreachable!(), // assert above.
        };

        match op0 {
            Opcode::Brz | Opcode::Brnz => {
                let flag_input = InsnInput {
                    insn: branches[0],
                    input: 0,
                };
                if let Some(icmp_insn) =
                    maybe_input_insn_via_conv(ctx, flag_input, Opcode::Icmp, Opcode::Bint)
                {
                    let condcode = inst_condcode(ctx.data(icmp_insn)).unwrap();
                    let cond = lower_condcode(condcode);
                    let is_signed = condcode_is_signed(condcode);
                    let negated = op0 == Opcode::Brz;
                    let cond = if negated { cond.invert() } else { cond };

                    lower_icmp_or_ifcmp_to_flags(ctx, icmp_insn, is_signed);
                    ctx.emit(Inst::CondBr {
                        taken,
                        not_taken,
                        kind: CondBrKind::Cond(cond),
                    });
                } else if let Some(fcmp_insn) =
                    maybe_input_insn_via_conv(ctx, flag_input, Opcode::Fcmp, Opcode::Bint)
                {
                    let condcode = inst_fp_condcode(ctx.data(fcmp_insn)).unwrap();
                    let cond = lower_fp_condcode(condcode);
                    let negated = op0 == Opcode::Brz;
                    let cond = if negated { cond.invert() } else { cond };

                    lower_fcmp_or_ffcmp_to_flags(ctx, fcmp_insn);
                    ctx.emit(Inst::CondBr {
                        taken,
                        not_taken,
                        kind: CondBrKind::Cond(cond),
                    });
                } else {
                    let rt = put_input_in_reg(
                        ctx,
                        InsnInput {
                            insn: branches[0],
                            input: 0,
                        },
                        NarrowValueMode::ZeroExtend64,
                    );
                    let kind = match op0 {
                        Opcode::Brz => CondBrKind::Zero(rt),
                        Opcode::Brnz => CondBrKind::NotZero(rt),
                        _ => unreachable!(),
                    };
                    ctx.emit(Inst::CondBr {
                        taken,
                        not_taken,
                        kind,
                    });
                }
            }
            Opcode::BrIcmp => {
                let condcode = inst_condcode(ctx.data(branches[0])).unwrap();
                let cond = lower_condcode(condcode);
                let kind = CondBrKind::Cond(cond);

                let is_signed = condcode_is_signed(condcode);
                let ty = ctx.input_ty(branches[0], 0);
                let bits = ty_bits(ty);
                let narrow_mode = match (bits <= 32, is_signed) {
                    (true, true) => NarrowValueMode::SignExtend32,
                    (true, false) => NarrowValueMode::ZeroExtend32,
                    (false, true) => NarrowValueMode::SignExtend64,
                    (false, false) => NarrowValueMode::ZeroExtend64,
                };
                let rn = put_input_in_reg(
                    ctx,
                    InsnInput {
                        insn: branches[0],
                        input: 0,
                    },
                    narrow_mode,
                );
                let rm = put_input_in_rse_imm12(
                    ctx,
                    InsnInput {
                        insn: branches[0],
                        input: 1,
                    },
                    narrow_mode,
                );

                let alu_op = choose_32_64(ty, ALUOp::SubS32, ALUOp::SubS64);
                let rd = writable_zero_reg();
                ctx.emit(alu_inst_imm12(alu_op, rd, rn, rm));
                ctx.emit(Inst::CondBr {
                    taken,
                    not_taken,
                    kind,
                });
            }

            Opcode::Brif => {
                let condcode = inst_condcode(ctx.data(branches[0])).unwrap();
                let cond = lower_condcode(condcode);
                let kind = CondBrKind::Cond(cond);

                let is_signed = condcode_is_signed(condcode);
                let flag_input = InsnInput {
                    insn: branches[0],
                    input: 0,
                };
                if let Some(ifcmp_insn) = maybe_input_insn(ctx, flag_input, Opcode::Ifcmp) {
                    lower_icmp_or_ifcmp_to_flags(ctx, ifcmp_insn, is_signed);
                    ctx.emit(Inst::CondBr {
                        taken,
                        not_taken,
                        kind,
                    });
                } else {
                    // If the ifcmp result is actually placed in a
                    // register, we need to move it back into the flags.
                    let rn = put_input_in_reg(ctx, flag_input, NarrowValueMode::None);
                    ctx.emit(Inst::MovToNZCV { rn });
                    ctx.emit(Inst::CondBr {
                        taken,
                        not_taken,
                        kind,
                    });
                }
            }

            Opcode::Brff => {
                let condcode = inst_fp_condcode(ctx.data(branches[0])).unwrap();
                let cond = lower_fp_condcode(condcode);
                let kind = CondBrKind::Cond(cond);
                let flag_input = InsnInput {
                    insn: branches[0],
                    input: 0,
                };
                if let Some(ffcmp_insn) = maybe_input_insn(ctx, flag_input, Opcode::Ffcmp) {
                    lower_fcmp_or_ffcmp_to_flags(ctx, ffcmp_insn);
                    ctx.emit(Inst::CondBr {
                        taken,
                        not_taken,
                        kind,
                    });
                } else {
                    // If the ffcmp result is actually placed in a
                    // register, we need to move it back into the flags.
                    let rn = put_input_in_reg(ctx, flag_input, NarrowValueMode::None);
                    ctx.emit(Inst::MovToNZCV { rn });
                    ctx.emit(Inst::CondBr {
                        taken,
                        not_taken,
                        kind,
                    });
                }
            }

            _ => unimplemented!(),
        }
    } else {
        // Must be an unconditional branch or an indirect branch.
        let op = ctx.data(branches[0]).opcode();
        match op {
            Opcode::Jump | Opcode::Fallthrough => {
                assert!(branches.len() == 1);
                // In the Fallthrough case, the machine-independent driver
                // fills in `targets[0]` with our fallthrough block, so this
                // is valid for both Jump and Fallthrough.
                ctx.emit(Inst::Jump {
                    dest: BranchTarget::Label(targets[0]),
                });
            }

            Opcode::BrTable => {
                // Expand `br_table index, default, JT` to:
                //
                //   emit_island  // this forces an island at this point
                //                // if the jumptable would push us past
                //                // the deadline
                //   subs idx, #jt_size
                //   b.hs default
                //   adr vTmp1, PC+16
                //   ldr vTmp2, [vTmp1, idx, lsl #2]
                //   add vTmp2, vTmp2, vTmp1
                //   br vTmp2
                //   [jumptable offsets relative to JT base]
                let jt_size = targets.len() - 1;
                assert!(jt_size <= std::u32::MAX as usize);

                ctx.emit(Inst::EmitIsland {
                    needed_space: 4 * (6 + jt_size) as CodeOffset,
                });

                let ridx = put_input_in_reg(
                    ctx,
                    InsnInput {
                        insn: branches[0],
                        input: 0,
                    },
                    NarrowValueMode::ZeroExtend32,
                );

                let rtmp1 = ctx.alloc_tmp(RegClass::I64, I32);
                let rtmp2 = ctx.alloc_tmp(RegClass::I64, I32);

                // Bounds-check, leaving condition codes for JTSequence's
                // branch to default target below.
                if let Some(imm12) = Imm12::maybe_from_u64(jt_size as u64) {
                    ctx.emit(Inst::AluRRImm12 {
                        alu_op: ALUOp::SubS32,
                        rd: writable_zero_reg(),
                        rn: ridx,
                        imm12,
                    });
                } else {
                    lower_constant_u64(ctx, rtmp1, jt_size as u64);
                    ctx.emit(Inst::AluRRR {
                        alu_op: ALUOp::SubS32,
                        rd: writable_zero_reg(),
                        rn: ridx,
                        rm: rtmp1.to_reg(),
                    });
                }

                // Emit the compound instruction that does:
                //
                // b.hs default
                // adr rA, jt
                // ldrsw rB, [rA, rIndex, UXTW 2]
                // add rA, rA, rB
                // br rA
                // [jt entries]
                //
                // This must be *one* instruction in the vcode because
                // we cannot allow regalloc to insert any spills/fills
                // in the middle of the sequence; otherwise, the ADR's
                // PC-rel offset to the jumptable would be incorrect.
                // (The alternative is to introduce a relocation pass
                // for inlined jumptables, which is much worse, IMHO.)

                let jt_targets: Vec<BranchTarget> = targets
                    .iter()
                    .skip(1)
                    .map(|bix| BranchTarget::Label(*bix))
                    .collect();
                let default_target = BranchTarget::Label(targets[0]);
                let targets_for_term: Vec<MachLabel> = targets.to_vec();
                ctx.emit(Inst::JTSequence {
                    ridx,
                    rtmp1,
                    rtmp2,
                    info: Box::new(JTSequenceInfo {
                        targets: jt_targets,
                        default_target,
                        targets_for_term: targets_for_term,
                    }),
                });
            }

            _ => panic!("Unknown branch type!"),
        }
    }

    Ok(())
}
