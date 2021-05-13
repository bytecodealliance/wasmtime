//! Lower a single Cranelift instruction into vcode.

use crate::ir::types::*;
use crate::ir::Inst as IRInst;
use crate::ir::Opcode;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::settings::Flags;
use crate::CodegenResult;

use crate::isa::arm32::abi::*;
use crate::isa::arm32::inst::*;

use smallvec::SmallVec;

use super::lower::*;

/// Actually codegen an instruction's results into registers.
pub(crate) fn lower_insn_to_regs<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    insn: IRInst,
    flags: &Flags,
) -> CodegenResult<()> {
    let op = ctx.data(insn).opcode();
    let inputs: SmallVec<[InsnInput; 4]> = (0..ctx.num_inputs(insn))
        .map(|i| InsnInput { insn, input: i })
        .collect();
    let outputs: SmallVec<[InsnOutput; 2]> = (0..ctx.num_outputs(insn))
        .map(|i| InsnOutput { insn, output: i })
        .collect();
    let ty = if outputs.len() > 0 {
        let ty = ctx.output_ty(insn, 0);
        if ty.bits() > 32 || ty.is_float() {
            panic!("Cannot lower inst with type {}!", ty);
        }
        Some(ty)
    } else {
        None
    };

    match op {
        Opcode::Iconst | Opcode::Bconst | Opcode::Null => {
            let value = output_to_const(ctx, outputs[0]).unwrap();
            let rd = output_to_reg(ctx, outputs[0]);
            lower_constant(ctx, rd, value);
        }
        Opcode::Iadd
        | Opcode::IaddIfcin
        | Opcode::IaddIfcout
        | Opcode::IaddIfcarry
        | Opcode::Isub
        | Opcode::IsubIfbin
        | Opcode::IsubIfbout
        | Opcode::IsubIfborrow
        | Opcode::Band
        | Opcode::Bor
        | Opcode::Bxor
        | Opcode::BandNot
        | Opcode::BorNot => {
            let rd = output_to_reg(ctx, outputs[0]);
            let rn = input_to_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = input_to_reg(ctx, inputs[1], NarrowValueMode::None);

            let alu_op = match op {
                Opcode::Iadd => ALUOp::Add,
                Opcode::IaddIfcin => ALUOp::Adc,
                Opcode::IaddIfcout => ALUOp::Adds,
                Opcode::IaddIfcarry => ALUOp::Adcs,
                Opcode::Isub => ALUOp::Sub,
                Opcode::IsubIfbin => ALUOp::Sbc,
                Opcode::IsubIfbout => ALUOp::Subs,
                Opcode::IsubIfborrow => ALUOp::Sbcs,
                Opcode::Band => ALUOp::And,
                Opcode::Bor => ALUOp::Orr,
                Opcode::Bxor => ALUOp::Eor,
                Opcode::BandNot => ALUOp::Bic,
                Opcode::BorNot => ALUOp::Orn,
                _ => unreachable!(),
            };
            ctx.emit(Inst::AluRRRShift {
                alu_op,
                rd,
                rn,
                rm,
                shift: None,
            });
        }
        Opcode::Imul | Opcode::Udiv | Opcode::Sdiv => {
            let rd = output_to_reg(ctx, outputs[0]);
            let rn = input_to_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = input_to_reg(ctx, inputs[1], NarrowValueMode::None);

            let alu_op = match op {
                Opcode::Imul => ALUOp::Mul,
                Opcode::Udiv => ALUOp::Udiv,
                Opcode::Sdiv => ALUOp::Sdiv,
                _ => unreachable!(),
            };
            ctx.emit(Inst::AluRRR { alu_op, rd, rn, rm });
        }
        Opcode::Ineg => {
            let rd = output_to_reg(ctx, outputs[0]);
            let rn = input_to_reg(ctx, inputs[0], NarrowValueMode::None);

            ctx.emit(Inst::AluRRImm8 {
                alu_op: ALUOp::Rsb,
                rd,
                rn,
                imm8: UImm8::maybe_from_i64(0).unwrap(),
            });
        }
        Opcode::Ishl | Opcode::Ushr | Opcode::Sshr => {
            let (alu_op, ext) = match op {
                Opcode::Ishl => (ALUOp::Lsl, NarrowValueMode::None),
                Opcode::Ushr => (ALUOp::Lsr, NarrowValueMode::ZeroExtend),
                Opcode::Sshr => (ALUOp::Asr, NarrowValueMode::SignExtend),
                _ => unreachable!(),
            };
            let rd = output_to_reg(ctx, outputs[0]);
            let rn = input_to_reg(ctx, inputs[0], ext);
            let rm = input_to_reg(ctx, inputs[1], NarrowValueMode::ZeroExtend);
            ctx.emit(Inst::AluRRR { alu_op, rd, rn, rm });
        }
        Opcode::Rotr => {
            if ty.unwrap().bits() != 32 {
                unimplemented!()
            }
            let rd = output_to_reg(ctx, outputs[0]);
            let rn = input_to_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = input_to_reg(ctx, inputs[1], NarrowValueMode::None);
            ctx.emit(Inst::AluRRR {
                alu_op: ALUOp::Ror,
                rd,
                rn,
                rm,
            });
        }
        Opcode::Rotl => {
            if ty.unwrap().bits() != 32 {
                unimplemented!()
            }
            let rd = output_to_reg(ctx, outputs[0]);
            let rn = input_to_reg(ctx, inputs[0], NarrowValueMode::None);
            let rm = input_to_reg(ctx, inputs[1], NarrowValueMode::None);
            let tmp = ctx.alloc_tmp(I32).only_reg().unwrap();

            // ror rd, rn, 32 - (rm & 31)
            ctx.emit(Inst::AluRRImm8 {
                alu_op: ALUOp::And,
                rd: tmp,
                rn: rm,
                imm8: UImm8::maybe_from_i64(31).unwrap(),
            });
            ctx.emit(Inst::AluRRImm8 {
                alu_op: ALUOp::Rsb,
                rd: tmp,
                rn: tmp.to_reg(),
                imm8: UImm8::maybe_from_i64(32).unwrap(),
            });
            ctx.emit(Inst::AluRRR {
                alu_op: ALUOp::Ror,
                rd,
                rn,
                rm: tmp.to_reg(),
            });
        }
        Opcode::Smulhi | Opcode::Umulhi => {
            let ty = ty.unwrap();
            let is_signed = op == Opcode::Smulhi;
            match ty {
                I32 => {
                    let rd_hi = output_to_reg(ctx, outputs[0]);
                    let rd_lo = ctx.alloc_tmp(ty).only_reg().unwrap();
                    let rn = input_to_reg(ctx, inputs[0], NarrowValueMode::None);
                    let rm = input_to_reg(ctx, inputs[1], NarrowValueMode::None);

                    let alu_op = if is_signed {
                        ALUOp::Smull
                    } else {
                        ALUOp::Umull
                    };
                    ctx.emit(Inst::AluRRRR {
                        alu_op,
                        rd_hi,
                        rd_lo,
                        rn,
                        rm,
                    });
                }
                I16 | I8 => {
                    let narrow_mode = if is_signed {
                        NarrowValueMode::SignExtend
                    } else {
                        NarrowValueMode::ZeroExtend
                    };
                    let rd = output_to_reg(ctx, outputs[0]);
                    let rn = input_to_reg(ctx, inputs[0], narrow_mode);
                    let rm = input_to_reg(ctx, inputs[1], narrow_mode);

                    ctx.emit(Inst::AluRRR {
                        alu_op: ALUOp::Mul,
                        rd,
                        rn,
                        rm,
                    });
                    let shift_amt = if ty == I16 { 16 } else { 8 };
                    let imm8 = UImm8::maybe_from_i64(shift_amt).unwrap();
                    let alu_op = if is_signed { ALUOp::Asr } else { ALUOp::Lsr };

                    ctx.emit(Inst::AluRRImm8 {
                        alu_op,
                        rd,
                        rn: rd.to_reg(),
                        imm8,
                    });
                }
                _ => panic!("Unexpected type {} in lower {}!", ty, op),
            }
        }
        Opcode::Bnot => {
            let rd = output_to_reg(ctx, outputs[0]);
            let rm = input_to_reg(ctx, inputs[0], NarrowValueMode::None);

            ctx.emit(Inst::AluRRShift {
                alu_op: ALUOp1::Mvn,
                rd,
                rm,
                shift: None,
            });
        }
        Opcode::Clz | Opcode::Ctz => {
            let rd = output_to_reg(ctx, outputs[0]);
            let rm = input_to_reg(ctx, inputs[0], NarrowValueMode::ZeroExtend);
            let ty = ctx.output_ty(insn, 0);

            let in_reg = if op == Opcode::Ctz {
                ctx.emit(Inst::BitOpRR {
                    bit_op: BitOp::Rbit,
                    rd,
                    rm,
                });
                rd.to_reg()
            } else {
                rm
            };
            ctx.emit(Inst::BitOpRR {
                bit_op: BitOp::Clz,
                rd,
                rm: in_reg,
            });

            if ty.bits() < 32 {
                let imm12 = UImm12::maybe_from_i64(32 - ty.bits() as i64).unwrap();
                ctx.emit(Inst::AluRRImm12 {
                    alu_op: ALUOp::Sub,
                    rd,
                    rn: rd.to_reg(),
                    imm12,
                });
            }
        }
        Opcode::Bitrev => {
            let rd = output_to_reg(ctx, outputs[0]);
            let rm = input_to_reg(ctx, inputs[0], NarrowValueMode::None);
            let ty = ctx.output_ty(insn, 0);
            let bit_op = BitOp::Rbit;

            match ty.bits() {
                32 => ctx.emit(Inst::BitOpRR { bit_op, rd, rm }),
                n if n < 32 => {
                    let shift = ShiftOpAndAmt::new(
                        ShiftOp::LSL,
                        ShiftOpShiftImm::maybe_from_shift(32 - n as u32).unwrap(),
                    );
                    ctx.emit(Inst::AluRRShift {
                        alu_op: ALUOp1::Mov,
                        rd,
                        rm,
                        shift: Some(shift),
                    });
                    ctx.emit(Inst::BitOpRR {
                        bit_op,
                        rd,
                        rm: rd.to_reg(),
                    });
                }
                _ => panic!("Unexpected output type {}", ty),
            }
        }
        Opcode::Icmp | Opcode::Ifcmp => {
            let condcode = inst_condcode(ctx.data(insn)).unwrap();
            let cond = lower_condcode(condcode);
            let is_signed = condcode_is_signed(condcode);

            let narrow_mode = if is_signed {
                NarrowValueMode::SignExtend
            } else {
                NarrowValueMode::ZeroExtend
            };
            let rd = output_to_reg(ctx, outputs[0]);
            let rn = input_to_reg(ctx, inputs[0], narrow_mode);
            let rm = input_to_reg(ctx, inputs[1], narrow_mode);

            ctx.emit(Inst::Cmp { rn, rm });

            if op == Opcode::Icmp {
                let mut it_insts = vec![];
                it_insts.push(CondInst::new(Inst::MovImm16 { rd, imm16: 1 }, true));
                it_insts.push(CondInst::new(Inst::MovImm16 { rd, imm16: 0 }, false));
                ctx.emit(Inst::It {
                    cond,
                    insts: it_insts,
                });
            }
        }
        Opcode::Trueif => {
            let cmp_insn = ctx
                .get_input_as_source_or_const(inputs[0].insn, inputs[0].input)
                .inst
                .unwrap()
                .0;
            debug_assert_eq!(ctx.data(cmp_insn).opcode(), Opcode::Ifcmp);
            emit_cmp(ctx, cmp_insn);

            let condcode = inst_condcode(ctx.data(insn)).unwrap();
            let cond = lower_condcode(condcode);
            let rd = output_to_reg(ctx, outputs[0]);

            let mut it_insts = vec![];
            it_insts.push(CondInst::new(Inst::MovImm16 { rd, imm16: 1 }, true));
            it_insts.push(CondInst::new(Inst::MovImm16 { rd, imm16: 0 }, false));

            ctx.emit(Inst::It {
                cond,
                insts: it_insts,
            });
        }
        Opcode::Select | Opcode::Selectif => {
            let cond = if op == Opcode::Select {
                let rn = input_to_reg(ctx, inputs[0], NarrowValueMode::ZeroExtend);
                ctx.emit(Inst::CmpImm8 { rn, imm8: 0 });
                Cond::Ne
            } else {
                // Verification ensures that the input is always a single-def ifcmp.
                let cmp_insn = ctx
                    .get_input_as_source_or_const(inputs[0].insn, inputs[0].input)
                    .inst
                    .unwrap()
                    .0;
                debug_assert_eq!(ctx.data(cmp_insn).opcode(), Opcode::Ifcmp);
                emit_cmp(ctx, cmp_insn);

                let condcode = inst_condcode(ctx.data(insn)).unwrap();
                lower_condcode(condcode)
            };
            let r1 = input_to_reg(ctx, inputs[1], NarrowValueMode::None);
            let r2 = input_to_reg(ctx, inputs[2], NarrowValueMode::None);
            let out_reg = output_to_reg(ctx, outputs[0]);

            let mut it_insts = vec![];
            it_insts.push(CondInst::new(Inst::mov(out_reg, r1), true));
            it_insts.push(CondInst::new(Inst::mov(out_reg, r2), false));

            ctx.emit(Inst::It {
                cond,
                insts: it_insts,
            });
        }
        Opcode::Store | Opcode::Istore8 | Opcode::Istore16 | Opcode::Istore32 => {
            let off = ldst_offset(ctx.data(insn)).unwrap();
            let elem_ty = match op {
                Opcode::Istore8 => I8,
                Opcode::Istore16 => I16,
                Opcode::Istore32 => I32,
                Opcode::Store => ctx.input_ty(insn, 0),
                _ => unreachable!(),
            };
            if elem_ty.bits() > 32 {
                unimplemented!()
            }
            let bits = elem_ty.bits() as u8;

            assert_eq!(inputs.len(), 2, "only one input for store memory operands");
            let rt = input_to_reg(ctx, inputs[0], NarrowValueMode::None);
            let base = input_to_reg(ctx, inputs[1], NarrowValueMode::None);

            let mem = AMode::RegOffset(base, i64::from(off));

            ctx.emit(Inst::Store { rt, mem, bits });
        }
        Opcode::Load
        | Opcode::Uload8
        | Opcode::Sload8
        | Opcode::Uload16
        | Opcode::Sload16
        | Opcode::Uload32
        | Opcode::Sload32 => {
            let off = ldst_offset(ctx.data(insn)).unwrap();
            let elem_ty = match op {
                Opcode::Sload8 | Opcode::Uload8 => I8,
                Opcode::Sload16 | Opcode::Uload16 => I16,
                Opcode::Sload32 | Opcode::Uload32 => I32,
                Opcode::Load => ctx.output_ty(insn, 0),
                _ => unreachable!(),
            };
            if elem_ty.bits() > 32 {
                unimplemented!()
            }
            let bits = elem_ty.bits() as u8;

            let sign_extend = match op {
                Opcode::Sload8 | Opcode::Sload16 | Opcode::Sload32 => true,
                _ => false,
            };
            let out_reg = output_to_reg(ctx, outputs[0]);

            assert_eq!(inputs.len(), 2, "only one input for store memory operands");
            let base = input_to_reg(ctx, inputs[1], NarrowValueMode::None);
            let mem = AMode::RegOffset(base, i64::from(off));

            ctx.emit(Inst::Load {
                rt: out_reg,
                mem,
                bits,
                sign_extend,
            });
        }
        Opcode::Uextend | Opcode::Sextend => {
            let output_ty = ty.unwrap();
            let input_ty = ctx.input_ty(insn, 0);
            let from_bits = input_ty.bits() as u8;
            let to_bits = 32;
            let signed = op == Opcode::Sextend;

            let rm = input_to_reg(ctx, inputs[0], NarrowValueMode::None);
            let rd = output_to_reg(ctx, outputs[0]);

            if output_ty.bits() > 32 {
                panic!("Unexpected output type {}", output_ty);
            }
            if from_bits < to_bits {
                ctx.emit(Inst::Extend {
                    rd,
                    rm,
                    from_bits,
                    signed,
                });
            }
        }
        Opcode::Bint | Opcode::Breduce | Opcode::Bextend | Opcode::Ireduce => {
            let rn = input_to_reg(ctx, inputs[0], NarrowValueMode::ZeroExtend);
            let rd = output_to_reg(ctx, outputs[0]);
            let ty = ctx.input_ty(insn, 0);

            ctx.emit(Inst::gen_move(rd, rn, ty));
        }
        Opcode::Copy => {
            let rd = output_to_reg(ctx, outputs[0]);
            let rn = input_to_reg(ctx, inputs[0], NarrowValueMode::None);
            let ty = ctx.input_ty(insn, 0);

            ctx.emit(Inst::gen_move(rd, rn, ty));
        }
        Opcode::Debugtrap => {
            ctx.emit(Inst::Bkpt);
        }
        Opcode::Trap => {
            let trap_info = inst_trapcode(ctx.data(insn)).unwrap();
            ctx.emit(Inst::Udf { trap_info })
        }
        Opcode::Trapif => {
            let cmp_insn = ctx
                .get_input_as_source_or_const(inputs[0].insn, inputs[0].input)
                .inst
                .unwrap()
                .0;
            debug_assert_eq!(ctx.data(cmp_insn).opcode(), Opcode::Ifcmp);
            emit_cmp(ctx, cmp_insn);

            let trap_info = inst_trapcode(ctx.data(insn)).unwrap();
            let condcode = inst_condcode(ctx.data(insn)).unwrap();
            let cond = lower_condcode(condcode);

            ctx.emit(Inst::TrapIf { cond, trap_info });
        }
        Opcode::FallthroughReturn | Opcode::Return => {
            for (i, input) in inputs.iter().enumerate() {
                let reg = input_to_reg(ctx, *input, NarrowValueMode::None);
                let retval_reg = ctx.retval(i).only_reg().unwrap();
                let ty = ctx.input_ty(insn, i);

                ctx.emit(Inst::gen_move(retval_reg, reg, ty));
            }
        }
        Opcode::Call | Opcode::CallIndirect => {
            let caller_conv = ctx.abi().call_conv();
            let (mut abi, inputs) = match op {
                Opcode::Call => {
                    let (extname, dist) = ctx.call_target(insn).unwrap();
                    let extname = extname.clone();
                    let sig = ctx.call_sig(insn).unwrap();
                    assert_eq!(inputs.len(), sig.params.len());
                    assert_eq!(outputs.len(), sig.returns.len());
                    (
                        Arm32ABICaller::from_func(sig, &extname, dist, caller_conv, flags)?,
                        &inputs[..],
                    )
                }
                Opcode::CallIndirect => {
                    let ptr = input_to_reg(ctx, inputs[0], NarrowValueMode::ZeroExtend);
                    let sig = ctx.call_sig(insn).unwrap();
                    assert_eq!(inputs.len() - 1, sig.params.len());
                    assert_eq!(outputs.len(), sig.returns.len());
                    (
                        Arm32ABICaller::from_ptr(sig, ptr, op, caller_conv, flags)?,
                        &inputs[1..],
                    )
                }
                _ => unreachable!(),
            };
            assert_eq!(inputs.len(), abi.num_args());
            for (i, input) in inputs.iter().enumerate().filter(|(i, _)| *i <= 3) {
                let arg_reg = input_to_reg(ctx, *input, NarrowValueMode::None);
                abi.emit_copy_regs_to_arg(ctx, i, ValueRegs::one(arg_reg));
            }
            abi.emit_call(ctx);
            for (i, output) in outputs.iter().enumerate() {
                let retval_reg = output_to_reg(ctx, *output);
                abi.emit_copy_retval_to_regs(ctx, i, ValueRegs::one(retval_reg));
            }
        }
        _ => panic!("lowering {} unimplemented!", op),
    }

    Ok(())
}

pub(crate) fn lower_branch<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
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
        // Must be a conditional branch followed by an unconditional branch.
        let op0 = ctx.data(branches[0]).opcode();
        let op1 = ctx.data(branches[1]).opcode();

        assert!(op1 == Opcode::Jump || op1 == Opcode::Fallthrough);
        let taken = BranchTarget::Label(targets[0]);
        let not_taken = BranchTarget::Label(targets[1]);

        match op0 {
            Opcode::Brz | Opcode::Brnz => {
                let rn = input_to_reg(
                    ctx,
                    InsnInput {
                        insn: branches[0],
                        input: 0,
                    },
                    NarrowValueMode::ZeroExtend,
                );
                let cond = if op0 == Opcode::Brz {
                    Cond::Eq
                } else {
                    Cond::Ne
                };

                ctx.emit(Inst::CmpImm8 { rn, imm8: 0 });
                ctx.emit(Inst::CondBr {
                    taken,
                    not_taken,
                    cond,
                });
            }
            _ => unimplemented!(),
        }
    } else {
        // Must be an unconditional branch or an indirect branch.
        let op = ctx.data(branches[0]).opcode();
        match op {
            Opcode::Jump | Opcode::Fallthrough => {
                assert_eq!(branches.len(), 1);
                // In the Fallthrough case, the machine-independent driver
                // fills in `targets[0]` with our fallthrough block, so this
                // is valid for both Jump and Fallthrough.
                ctx.emit(Inst::Jump {
                    dest: BranchTarget::Label(targets[0]),
                });
            }
            _ => unimplemented!(),
        }
    }

    Ok(())
}
