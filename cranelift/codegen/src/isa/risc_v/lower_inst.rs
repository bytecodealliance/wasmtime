//! Lower a single Cranelift instruction into vcode.

use crate::machinst::Writable;
use alloc::vec;
use alloc::vec::Vec;

use crate::ir::Inst as IRInst;
use crate::ir::InstructionData;
use crate::ir::Opcode;
use crate::isa::risc_v::settings as aarch64_settings;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::settings::Flags;
use crate::CodegenResult;

use crate::ir::types::{
    B1, B128, B16, B32, B64, B8, F32, F64, FFLAGS, I128, I16, I32, I64, I8, IFLAGS, R32, R64,
};

use super::lower::*;
use crate::isa::risc_v::abi::*;
use crate::isa::risc_v::inst::*;

pub(crate) fn is_valid_atomic_transaction_ty(ty: Type) -> bool {
    match ty {
        I8 | I16 | I32 | I64 => true,
        _ => false,
    }
}
/// Actually codegen an instruction's results into registers.
pub(crate) fn lower_insn_to_regs<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    insn: IRInst,
    flags: &Flags,
    isa_flags: &aarch64_settings::Flags,
) -> CodegenResult<()> {
    let op = ctx.data(insn).opcode();
    let inputs = insn_inputs(ctx, insn);
    let outputs = insn_outputs(ctx, insn);

    let ty = if outputs.len() > 0 {
        Some(ctx.output_ty(insn, 0))
    } else {
        None
    };

    if let Ok(()) = super::lower::isle::lower(ctx, flags, isa_flags, &outputs, insn) {
        return Ok(());
    }

    let implemented_in_isle = |ctx: &mut C| -> ! {
        unreachable!(
            "implemented in ISLE: inst = `{}`, type = `{:?}`",
            ctx.dfg().display_inst(insn),
            ty
        );
    };

    match op {
        Opcode::Iconst | Opcode::Bconst | Opcode::Null => implemented_in_isle(ctx),

        Opcode::F32const | Opcode::F64const => unreachable!(
            "Should never see constant ops at top level lowering entry
            point, as constants are rematerialized at use-sites"
        ),

        Opcode::Iadd => implemented_in_isle(ctx),
        Opcode::Isub => implemented_in_isle(ctx),
        Opcode::UaddSat | Opcode::SaddSat | Opcode::UsubSat | Opcode::SsubSat => {
            implemented_in_isle(ctx)
        }

        Opcode::Ineg => implemented_in_isle(ctx),

        Opcode::Imul => implemented_in_isle(ctx),

        Opcode::Umulhi | Opcode::Smulhi => implemented_in_isle(ctx),

        Opcode::Udiv | Opcode::Sdiv | Opcode::Urem | Opcode::Srem => implemented_in_isle(ctx),

        Opcode::Uextend | Opcode::Sextend => implemented_in_isle(ctx),

        Opcode::Bnot => implemented_in_isle(ctx),

        Opcode::Band
        | Opcode::Bor
        | Opcode::Bxor
        | Opcode::BandNot
        | Opcode::BorNot
        | Opcode::BxorNot => implemented_in_isle(ctx),

        Opcode::Ishl | Opcode::Ushr | Opcode::Sshr => implemented_in_isle(ctx),

        Opcode::Rotr | Opcode::Rotl => implemented_in_isle(ctx),

        Opcode::Bitrev | Opcode::Clz | Opcode::Cls | Opcode::Ctz => implemented_in_isle(ctx),

        Opcode::Popcnt => implemented_in_isle(ctx),

        Opcode::Load
        | Opcode::Uload8
        | Opcode::Sload8
        | Opcode::Uload16
        | Opcode::Sload16
        | Opcode::Uload32
        | Opcode::Sload32
        | Opcode::Sload8x8
        | Opcode::Uload8x8
        | Opcode::Sload16x4
        | Opcode::Uload16x4
        | Opcode::Sload32x2
        | Opcode::Uload32x2 => {
            let out_ty = ctx.output_ty(insn, 0);
            let flags = ctx
                .memflags(insn)
                .expect("Load instruction should have memflags");
            let base = put_input_in_reg(ctx, inputs[0]);
            let off = ctx.data(insn).load_store_offset().unwrap() as i64;

            let dst = get_output_reg(ctx, outputs[0]);
            // compute address
            match out_ty.bits() {
                128 => {
                    ctx.emit(Inst::Load {
                        rd: dst.regs()[0],
                        op: LoadOP::Ld,
                        flags,
                        from: AMode::RegOffset(base, off, I64),
                    });
                    ctx.emit(Inst::Load {
                        rd: dst.regs()[1],
                        op: LoadOP::Ld,
                        flags,
                        from: AMode::RegOffset(base, off + 8, I64),
                    })
                }
                64 => {
                    let op = if out_ty.is_float() {
                        LoadOP::Fld
                    } else {
                        LoadOP::Ld
                    };
                    ctx.emit(Inst::Load {
                        rd: dst.regs()[0],
                        op: LoadOP::Ld,
                        flags,
                        from: AMode::RegOffset(base, off, I64),
                    });
                }
                32 => {
                    let op = if out_ty.is_float() {
                        LoadOP::Flw
                    } else if is_int_and_type_signed(out_ty) {
                        LoadOP::Lw
                    } else {
                        LoadOP::Lwu
                    };
                    ctx.emit(Inst::Load {
                        rd: dst.regs()[0],
                        op: op,
                        flags,
                        from: AMode::RegOffset(base, off, I64),
                    });
                }
                16 => {
                    let op = if is_int_and_type_signed(out_ty) {
                        LoadOP::Lh
                    } else {
                        LoadOP::Lhu
                    };
                    ctx.emit(Inst::Load {
                        rd: dst.regs()[0],
                        op: op,
                        flags,
                        from: AMode::RegOffset(base, off, I64),
                    });
                }
                8 | 1 => {
                    let op = if is_int_and_type_signed(out_ty) {
                        LoadOP::Lb
                    } else {
                        LoadOP::Lbu
                    };
                    ctx.emit(Inst::Load {
                        rd: dst.regs()[0],
                        op: op,
                        flags,
                        from: AMode::RegOffset(base, off, I64),
                    });
                }
                _ => unreachable!(),
            }
        }

        Opcode::Store | Opcode::Istore8 | Opcode::Istore16 | Opcode::Istore32 => {
            let flags = ctx
                .memflags(insn)
                .expect("Load instruction should have memflags");

            let src = put_input_in_regs(ctx, inputs[0]);
            let base = put_input_in_reg(ctx, inputs[1]);
            let off = ctx.data(insn).load_store_offset().unwrap() as i64;
            let elem_ty = match op {
                Opcode::Istore8 => I8,
                Opcode::Istore16 => I16,
                Opcode::Istore32 => I32,
                Opcode::Store => ctx.input_ty(insn, 0),
                _ => unreachable!(),
            };
            match elem_ty.bits() {
                128 => {
                    ctx.emit(Inst::Store {
                        to: AMode::RegOffset(base, off, I64),
                        op: StoreOP::Sd,
                        flags,
                        src: src.regs()[0],
                    });
                    ctx.emit(Inst::Store {
                        to: AMode::RegOffset(base, off + 8, I64),
                        op: StoreOP::Sd,
                        flags,
                        src: src.regs()[1],
                    });
                }

                64 => {
                    let op = if elem_ty.is_float() {
                        StoreOP::Fsd
                    } else {
                        StoreOP::Sd
                    };
                    ctx.emit(Inst::Store {
                        to: AMode::RegOffset(base, off, I64),
                        op,
                        flags,
                        src: src.regs()[0],
                    });
                }
                32 => {
                    let op = if elem_ty.is_float() {
                        StoreOP::Fsw
                    } else {
                        StoreOP::Sw
                    };
                    ctx.emit(Inst::Store {
                        to: AMode::RegOffset(base, off, I64),
                        op,
                        flags,
                        src: src.regs()[0],
                    });
                }
                16 => {
                    let op = StoreOP::Sh;
                    ctx.emit(Inst::Store {
                        to: AMode::RegOffset(base, off, I64),
                        op,
                        flags,
                        src: src.regs()[0],
                    });
                }
                8 | 1 => {
                    let op = StoreOP::Sb;
                    ctx.emit(Inst::Store {
                        to: AMode::RegOffset(base, off, I64),
                        op,
                        flags,
                        src: src.regs()[0],
                    });
                }
                _ => unreachable!(),
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
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let offset: i32 = offset.into();
            let inst = ctx
                .abi()
                .stackslot_addr(stack_slot, u32::try_from(offset).unwrap(), rd);
            ctx.emit(inst);
        }

        Opcode::AtomicRmw => {
            let r_dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let r_addr = ctx.put_input_in_regs(insn, 0).only_reg().unwrap();
            let arg2 = ctx.put_input_in_regs(insn, 1).only_reg().unwrap();
            let ty_access = ty.unwrap();
            assert!(is_valid_atomic_transaction_ty(ty_access));
            let op = ctx.data(insn).atomic_rmw_op().unwrap();
            let mut insts = SmallInstVec::new();
            let risc_op = AtomicOP::from_atomicrmw_type_and_op(ty_access, op);
            match op {
                // special cases
                // sub will use add atomic instruction
                crate::ir::AtomicRmwOp::Sub => {
                    insts.push(Inst::AluRRR {
                        alu_op: AluOPRRR::Sub,
                        rd: Writable::from_reg(arg2),
                        rs1: zero_reg(),
                        rs2: arg2,
                    });
                    insts.push(Inst::AluRRR {
                        alu_op: AluOPRRR::Sub,
                        rd: Writable::from_reg(arg2),
                        rs1: zero_reg(),
                        rs2: arg2,
                    });
                }
                // AtomicRmwOp::Nand => {
                //     /*
                //     a = !(a&b);
                //     equals a = (!a)  | (!b)
                //     here are truth table.
                //         a = !(a&b);
                //         A	B	Y
                //         0	0	1
                //         0	1	1
                //         1	0	1
                //         1	1	0

                //         a = (!a)  | (!b)
                //         A	B	Y
                //         0	0	1
                //         0	1	1
                //         1	0	1
                //         1	1	0
                //              */

                //     //
                //     unimplemented!("nand not implemented.")
                // }
                // handled
                _ => {}
            }
            insts.push(Inst::Atomic {
                op: risc_op,
                rd: r_dst,
                addr: r_addr,
                src: arg2,
                /*
                todo::
                    where are the memory order parameter??
                */
                aq: false,
                rl: false,
            });
            for i in insts {
                ctx.emit(i);
            }
        }

        Opcode::AtomicCas => {
            let r_dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let r_addr = ctx.put_input_in_regs(insn, 0).only_reg().unwrap();
            let r_expected = ctx.put_input_in_regs(insn, 1).only_reg().unwrap();
            let r_replacement = ctx.put_input_in_regs(insn, 2).only_reg().unwrap();
            let ty_access = ty.unwrap();
            assert!(is_valid_atomic_transaction_ty(ty_access));
            unimplemented!()
        }

        Opcode::AtomicLoad => {
            let r_dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let r_addr = ctx.put_input_in_regs(insn, 0).only_reg().unwrap();
            let ty_access = ty.unwrap();
            assert!(is_valid_atomic_transaction_ty(ty_access));
            ctx.emit(Inst::Atomic {
                op: if ty_access.bits() == 32 {
                    AtomicOP::LrW
                } else {
                    AtomicOP::LrD
                },
                rd: r_dst,
                addr: r_addr,
                src: zero_reg(),
                aq: false,
                rl: false,
            });
        }

        Opcode::AtomicStore => {
            let r_dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let r_addr = ctx.put_input_in_regs(insn, 0).only_reg().unwrap();
            let arg2 = ctx.put_input_in_regs(insn, 1).only_reg().unwrap();
            let ty_access = ty.unwrap();
            assert!(is_valid_atomic_transaction_ty(ty_access));
            ctx.emit(Inst::Atomic {
                op: if ty_access.bits() == 32 {
                    AtomicOP::ScW
                } else {
                    AtomicOP::ScD
                },
                rd: r_dst,
                addr: r_addr,
                src: arg2,
                aq: false,
                rl: false,
            });
        }

        Opcode::Fence => {
            ctx.emit(Inst::Fence);
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

        Opcode::Nop => {
            // Nothing.
        }

        Opcode::Select => {
            assert!(ctx.input_ty(insn, 0).is_bool() || ctx.input_ty(insn, 0).is_int());
            let dst: Vec<_> = ctx
                .get_output(insn, 0)
                .regs()
                .into_iter()
                .map(|r| *r)
                .collect();

            let ty = ty.unwrap();
            let conditon = put_input_in_reg(ctx, inputs[0]);
            let x = ctx.put_input_in_regs(insn, 0);
            let y = ctx.put_input_in_regs(insn, 1);
            ctx.emit(Inst::Select {
                dst,
                conditon,
                x,
                y,
                ty,
            });
        }

        Opcode::Selectif | Opcode::SelectifSpectreGuard => ir_iflags_conflict(op),

        Opcode::Bitselect => {
            debug_assert_ne!(Opcode::Vselect, op);
            let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
            let tmp2 = ctx.alloc_tmp(I64).only_reg().unwrap();
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let rcond = put_input_in_reg(ctx, inputs[0]);
            let x = put_input_in_reg(ctx, inputs[1]);
            let y = put_input_in_reg(ctx, inputs[2]);
            // x part
            ctx.emit(Inst::AluRRR {
                alu_op: AluOPRRR::And,
                rd: tmp,
                rs1: rcond,
                rs2: x,
            });
            // bit not
            ctx.emit(Inst::construct_bit_not(tmp2, rcond));
            // y part
            ctx.emit(Inst::AluRRR {
                alu_op: AluOPRRR::And,
                rd: tmp2,
                rs1: rcond,
                rs2: y,
            });
            ctx.emit(Inst::AluRRR {
                alu_op: AluOPRRR::Or,
                rd: rd,
                rs1: tmp.to_reg(),
                rs2: tmp2.to_reg(),
            });
        }

        Opcode::Vselect => {
            todo!()
        }

        Opcode::Trueif => ir_iflags_conflict(op),

        Opcode::Trueff => ir_fflags_conflict(op),

        Opcode::IsNull | Opcode::IsInvalid => {
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let rs = put_input_in_reg(ctx, inputs[0]);
            let ty = ctx.input_ty(insn, 0);
            ctx.emit(Inst::ReferenceValid {
                op: ReferenceValidOP::from_ir_op(op),
                rd,
                x: rs,
            });
        }

        Opcode::Copy => {
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let rn = put_input_in_reg(ctx, inputs[0]);
            let ty = ctx.input_ty(insn, 0);
            ctx.emit(Inst::gen_move(rd, rn, ty));
        }

        Opcode::Breduce | Opcode::Ireduce => {}

        Opcode::Bextend | Opcode::Bmask => {}

        Opcode::Bint => {}

        Opcode::Bitcast => {}

        Opcode::FallthroughReturn | Opcode::Return => {
            for i in 0..ctx.num_inputs(insn) {
                let src_reg = put_input_in_regs(ctx, inputs[i]);
                let retval_reg = ctx.retval(i);
                let ty = ctx.input_ty(insn, i);
                assert!(src_reg.len() == retval_reg.len());
                let (_, tys) = Inst::rc_for_type(ty)?;
                for ((&src, &dst), &ty) in src_reg
                    .regs()
                    .iter()
                    .zip(retval_reg.regs().iter())
                    .zip(tys.iter())
                {
                    ctx.emit(Inst::gen_move(dst, src, ty));
                }
            }
        }

        Opcode::Ifcmp | Opcode::Ffcmp => {
            panic!("Should never reach ifcmp as isel root!");
        }

        Opcode::Icmp => {}

        Opcode::Fcmp => {
            let ty = ctx.input_ty(insn, 0);
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let rs1 = put_input_in_reg(ctx, inputs[0]);
            let rs2 = put_input_in_reg(ctx, inputs[1]);
            let cc = ctx.data(insn).fp_cond_code().unwrap();
            ctx.emit(Inst::Ffcmp {
                rd,
                cc,
                ty,
                rs1,
                rs2,
            });
        }

        Opcode::Debugtrap => {}

        Opcode::Trap | Opcode::ResumableTrap => {}

        Opcode::Trapif | Opcode::Trapff => {}

        Opcode::Trapz | Opcode::Trapnz | Opcode::ResumableTrapnz => {
            panic!("trapz / trapnz / resumable_trapnz should have been removed by legalization!");
        }

        Opcode::FuncAddr => {}

        Opcode::GlobalValue => {
            panic!("global_value should have been removed by legalization!");
        }

        Opcode::SymbolValue => {}

        Opcode::Call | Opcode::CallIndirect => {}

        Opcode::GetPinnedReg => {}

        Opcode::SetPinnedReg => {}

        Opcode::Jump
        | Opcode::Brz
        | Opcode::Brnz
        | Opcode::BrIcmp
        | Opcode::Brif
        | Opcode::Brff
        | Opcode::BrTable => {
            panic!("Branch opcode reached non-branch lowering logic!");
        }

        Opcode::Vconst => {}

        Opcode::RawBitcast => {}

        Opcode::Extractlane => {}

        Opcode::Insertlane => {}

        Opcode::Splat => {}

        Opcode::ScalarToVector => {}

        Opcode::VallTrue if ctx.input_ty(insn, 0).lane_bits() == 64 => {}

        Opcode::VanyTrue | Opcode::VallTrue => {}

        Opcode::VhighBits => {}

        Opcode::Shuffle => {}

        Opcode::Swizzle => {}

        Opcode::Isplit => {}

        Opcode::Iconcat => {}

        Opcode::Imax | Opcode::Umax | Opcode::Umin | Opcode::Imin => {}

        Opcode::IaddPairwise => {}

        Opcode::WideningPairwiseDotProductS => {}

        Opcode::Fadd | Opcode::Fsub | Opcode::Fmul | Opcode::Fdiv | Opcode::Fmin | Opcode::Fmax => {
        }

        Opcode::FminPseudo | Opcode::FmaxPseudo => {}

        Opcode::Sqrt | Opcode::Fneg | Opcode::Fabs | Opcode::Fpromote | Opcode::Fdemote => {}

        Opcode::Ceil | Opcode::Floor | Opcode::Trunc | Opcode::Nearest => {}

        Opcode::Fma => {}

        Opcode::Fcopysign => {}

        Opcode::FcvtToUint | Opcode::FcvtToSint => {}

        Opcode::FcvtFromUint | Opcode::FcvtFromSint => {}

        Opcode::FcvtToUintSat | Opcode::FcvtToSintSat => {}

        Opcode::IaddIfcout => {}

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

        Opcode::Iabs => {
            implemented_in_isle(ctx);
        }
        Opcode::AvgRound => {}

        Opcode::Snarrow | Opcode::Unarrow | Opcode::Uunarrow => {}

        Opcode::SwidenLow | Opcode::SwidenHigh | Opcode::UwidenLow | Opcode::UwidenHigh => {}

        Opcode::TlsValue => {}

        Opcode::SqmulRoundSat => {}

        Opcode::FcvtLowFromSint => {}

        Opcode::FvpromoteLow => {}

        Opcode::Fvdemote => {}

        Opcode::ConstAddr | Opcode::Vconcat | Opcode::Vsplit | Opcode::IfcmpSp => {}
    }

    Ok(())
}

/*
    todo::int128 compare
    gcc generate this.

    ```
    int main(int argc, char **argv)
    {
        __int128_t a;
        __int128_t b;
        if (a > b)
        {
            return 1;
        }
        else
        {
            return 2;
        }
        return 0;
    }
    ```


    .file	"main.c"
    .option pic
    .text
    .align	1
    .globl	main
    .type	main, @function
main:
    addi	sp,sp,-64
    sd	s0,56(sp)
    addi	s0,sp,64
    mv	a5,a0
    sd	a1,-64(s0)
    sw	a5,-52(s0)
    ld	a4,-40(s0)
    ld	a5,-24(s0)
    bgt	a4,a5,.L5
    ld	a4,-40(s0)
    ld	a5,-24(s0)
    bne	a4,a5,.L2
    ld	a4,-48(s0)
    ld	a5,-32(s0)
    bleu	a4,a5,.L2
.L5:
    li	a5,1
    j	.L4
.L2:
    li	a5,2
.L4:
    mv	a0,a5
    ld	s0,56(sp)
    addi	sp,sp,64
    jr	ra
    .size	main, .-main
    .ident	"GCC: (Ubuntu 10.3.0-1ubuntu1~20.04) 10.3.0"
    .section	.note.GNU-stack,"",@progbits
```
*/

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
        let op0 = ctx.data(branches[0]).clone();
        let op1 = ctx.data(branches[1]).clone();

        assert!(op1.opcode() == Opcode::Jump);
        let taken = BranchTarget::Label(targets[0]);
        // not_taken target is the target of the second branch, even if it is a Fallthrough
        // instruction: because we reorder blocks while we lower, the fallthrough in the new
        // order is not (necessarily) the same as the fallthrough in CLIF. So we use the
        // explicitly-provided target.
        let not_taken = BranchTarget::Label(targets[1]);

        match op0.opcode() {
            Opcode::Brz | Opcode::Brnz => {
                let ty = ctx.input_ty(branches[0], 0);
                let reg = ctx.put_input_in_regs(branches[0], 0);
                if ty.bits() as u32 >= Riscv64MachineDeps::word_bits() {
                    unimplemented!("");
                }
                let cond = if op0.opcode() == Opcode::Brz {
                    CondBrKind {
                        rs1: reg.only_reg().unwrap(),
                        rs2: zero_reg(),
                        kind: IntCC::Equal,
                    }
                } else {
                    CondBrKind {
                        rs1: reg.only_reg().unwrap(),
                        rs2: zero_reg(),
                        kind: IntCC::NotEqual,
                    }
                };
                let inst = Inst::CondBr {
                    taken,
                    not_taken,
                    kind: cond,
                };
                ctx.emit(inst);
            }
            Opcode::BrIcmp => {
                let ty = ctx.input_ty(branches[0], 0);
                assert!(ty.is_int());
                if ty.bits() as u32 <= Riscv64MachineDeps::word_bits() {
                    let rs1 = ctx.put_input_in_regs(branches[0], 0);
                    let rs2 = ctx.put_input_in_regs(branches[0], 1);
                    let rs1 = rs1.only_reg().unwrap();
                    let rs2 = rs2.only_reg().unwrap();
                    let cc = op0.cond_code().unwrap();
                    let inst = Inst::CondBr {
                        taken,
                        not_taken,
                        kind: CondBrKind { kind: cc, rs1, rs2 },
                    };
                    ctx.emit(inst);
                } else {
                    unimplemented!();
                }
            }
            Opcode::Brif => {
                unreachable!("risc-v has no compare iflag");
            }
            Opcode::Brff => {
                unreachable!("risc-v has no compare fflag");
            }
            _ => unimplemented!(),
        }
    } else {
        // Must be an unconditional branch or an indirect branch.
        let op = ctx.data(branches[0]).opcode();
        match op {
            Opcode::Jump => {
                assert!(branches.len() == 1);
                ctx.emit(Inst::Jal {
                    dest: BranchTarget::Label(targets[0]),
                });
            }
            Opcode::BrTable => {
                unimplemented!()
            }
            _ => panic!("Unknown branch type!"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn compile_ok() {}
}

// let left = ctx.put_input_in_regs(insn, 0).only_reg().unwrap();
// let right = ctx.put_input_in_regs(insn, 1).only_reg().unwrap();

// let left_tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
// let right_tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
// let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();

// let mut insts = vec![];
// let mut unordered_b = vec![];
// {
//     // check left is_nan
//     let class_op = if ctx.input_ty(insn, 0) == F32 {
//         AluOPRR::FclassS
//     } else {
//         AluOPRR::FclassD
//     };
//     // if left is nan
//     insts.push(Inst::AluRR {
//         alu_op: class_op,
//         rd: left_tmp,
//         rs: left,
//     });
//     //
//     insts.push(Inst::AluRRImm12 {
//         alu_op: AluOPRRI::Andi,
//         rd: tmp,
//         rs: left_tmp.to_reg(),
//         imm12: Imm12::from_bits(FClassResult::is_nan_bits() as i16),
//     });
//     // left is nan
//     unordered_b.push(insts.len());
//     insts.push(Inst::CondBr {
//         taken: BranchTarget::patch(),
//         not_taken: BranchTarget::zero(),
//         kind: CondBrKind {
//             kind: IntCC::NotEqual,
//             rs1: tmp.to_reg(),
//             rs2: zero_reg(),
//         },
//     });
// }

// {
//     // if right is nan
//     insts.push(Inst::AluRR {
//         alu_op: class_op,
//         rd: right_tmp,
//         rs: right,
//     });
//     insts.push(Inst::AluRRImm12 {
//         alu_op: AluOPRRI::Andi,
//         rd: tmp,
//         rs: right_tmp,
//         imm12: Imm12::from_bits(FClassResult::is_nan_bits() as i16),
//     });
//     // right is nan
//     unordered_b.push(insts.len());
//     insts.push(Inst::CondBr {
//         taken: BranchTarget::patch(),
//         not_taken: BranchTarget::zero(),
//         kind: CondBrKind {
//             kind: IntCC::NotEqual,
//             rs1: tmp,
//             rs2: zero_reg(),
//         },
//     });
// }

// {
//     // if left is pos infinite and right is pos infinite , or both neg infinite
//     insts.push(Inst::AluRRR {
//         alu_op: AluOPRRR::And,
//         rd: tmp,
//         rs1: left_tmp,
//         rs2: right_tmp,
//     });
//     insts.push(Inst::AluRRImm12 {
//         alu_op: AluOPRRI::Andi,
//         rd: tmp,
//         rs: tmp,
//         imm12: Imm12::from_bits(FClassResult::is_infinite_bits() as i16),
//     });
//     unordered_b.push(insts.len());
//     insts.push(Inst::CondBr {
//         taken: BranchTarget::patch(),
//         not_taken: BranchTarget::zero(),
//         kind: CondBrKind {
//             kind: IntCC::NotEqual,
//             rs1: tmp,
//             rs2: zero_reg(),
//         },
//     });
// }

// let jump_to_final_compare;

// {
//     // now we can use left_class_result for another purpose
//     {
//         // compute eq
//         // at this point
//         let eq_op = if ctx.input_ty(insn, 0) == F32 {
//             AluOPRRR::FeqS
//         } else {
//             AluOPRRR::FeqD
//         };
//         insts.push(Inst::AluRRR {
//             alu_op: eq_op,
//             rd: left_tmp,
//             rs1: left,
//             rs2: right,
//         });
//         insts.push(Inst::AluRRImm12 {
//             alu_op: AluOPRRI::Slli,
//             rd: tmp,
//             rs: left_tmp,
//             imm12: Imm12::from_bits(FloatCCBit::EQ.shift()),
//         });
//     }

//     {
//         // compute lt
//         let lt_op = if ctx.input_ty(insn, 0) == F32 {
//             AluOPRRR::FltS
//         } else {
//             AluOPRRR::FltD
//         };
//         insts.push(Inst::AluRRR {
//             alu_op: lt_op,
//             rd: left_tmp,
//             rs1: left,
//             rs2: right,
//         });

//         insts.push(Inst::AluRRImm12 {
//             alu_op: AluOPRRI::Slli,
//             rd: left_tmp,
//             rs: left_tmp,
//             imm12: Imm12::from_bits(FloatCCBit::LT.shift()),
//         });

//         insts.push(Inst::AluOPRRR {
//             alu_op: AluOPRRR::Or,
//             rd: tmp,
//             rs1: tmp,
//             rs2: left_tmp,
//         });
//     }
//     {
//         //
//         insts.push(Inst::load_constant_imm12(
//             left_tmp,
//             (FloatCCBit::EQ.bit() | FloatCCBit::LT.bit()) as u32,
//         ));
//         insts.push(Inst::AluRRR {
//             alu_op: AluOPRRR::And,
//             rd: left_tmp,
//             rs1: left_tmp,
//             rs2: tmp,
//         });
//         //compute gt
//         insts.push(Inst::CondBr {
//             taken: BranchTarget::offset(Instructions::instruction_size()),
//             not_taken: BranchTarget::zero(),
//             kind: CondBrKind {
//                 /*
//                  */
//                 kind: IntCC::NotEqual,
//                 rs1: left_tmp,
//                 rs2: zero_reg(),
//             },
//         });
//         insts.push(Inst::AluRRImm12 {
//             alu_op: AluOPRRI::Ori,
//             rd: tmp,
//             rs: tmp,
//             imm12: Imm12::from_bits(FloatCCBit::GT.bit() as i16),
//         });
//     }

//     jump_to_final_compare = insts.len();
//     insts.push(Inst::Jal {
//         dest: BranchTarget::patch(),
//     });
// };
// let patch = |i: &mut Inst| match &mut i {
//     &mut Inst::CondBr {
//         ref mut not_taken, ..
//     } => match not_taken {
//         &mut BranchTarget::ResolvedOffset(ref mut off) => {
//             *off = (Inst::instruction_size() * Inst::instruction_size()) as i32;
//         }
//         _ => unreachable!(),
//     },
//     _ => unreachable!(),
// };
// // patch
// for i in unordered_b {
//     // let length = insts.len();
//     // match &mut insns[i] {
//     //     &mut Inst::CondBr {
//     //         ref mut not_taken, ..
//     //     } => match not_taken {
//     //         &mut BranchTarget::ResolvedOffset(ref mut off) => {
//     //             *off = (Inst::instruction_size() * Inst::instruction_size()) as i32;
//     //         }
//     //         _ => unreachable!(),
//     //     },
//     //     _ => unreachable!(),
//     // }
//     patch(&mut insts[i]);
// }
// // make tmp as UN
// insts.push(&Inst::load_constant_imm12(
//     tmp,
//     Imm12::from_bits(FloatCCBit::UN),
// ));

// // path
// patch(&mut insts[jump_to_final_compare]);

// insts.push(Inst::AluRRImm12 {
//     alu_op: AluOPRRI::Ori,
//     rd: tmp,
//     rs: tmp.to_reg(),
//     imm12: Imm12::from_bits(FloatCCBit::floatcc_2_mask_bits(ctx)),
// });

// insts.push(Inst::CondBr {
//     taken: BranchTarget::offset(Inst::instruction_size() as u32),
//     not_taken: BranchTarget::zero(),
//     kind: CondBrKind {
//         kind: IntCC::NotEqual, // means match , conditon is true
//         rs1: tmp,
//         rs2: zero_reg(),
//     },
// });

// insts.push(Inst::load_constant_imm12(result_reg, Imm12::from_bits(0)));

// insts.push(Inst::load_constant_imm12(result_reg, Imm12::from_bits(1)));
// for i in insts {
//     ctx.emit(i);
// }
