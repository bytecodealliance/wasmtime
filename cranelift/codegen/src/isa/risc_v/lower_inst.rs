//! Lower a single Cranelift instruction into vcode.

use crate::ir::Inst as IRInst;
use crate::ir::Opcode;
use crate::isa::risc_v::settings as aarch64_settings;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::settings::Flags;
use crate::CodegenResult;

use crate::isa::risc_v::abi::*;
use crate::isa::risc_v::inst::*;

use super::lower::*;

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
        | Opcode::Uload32x2
        | Opcode::Uload8x8Complex
        | Opcode::Sload8x8Complex
        | Opcode::Uload16x4Complex
        | Opcode::Sload16x4Complex
        | Opcode::Uload32x2Complex
        | Opcode::Sload32x2Complex => {}

        Opcode::Store
        | Opcode::Istore8
        | Opcode::Istore16
        | Opcode::Istore32
        | Opcode::StoreComplex
        | Opcode::Istore8Complex
        | Opcode::Istore16Complex
        | Opcode::Istore32Complex => {}

        Opcode::StackAddr => {}

        Opcode::AtomicRmw => {}

        Opcode::AtomicCas => {}

        Opcode::AtomicLoad => {}

        Opcode::AtomicStore => {}

        Opcode::Fence => {}

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

        Opcode::Select => {}

        Opcode::Selectif | Opcode::SelectifSpectreGuard => {}

        Opcode::Bitselect | Opcode::Vselect => {}

        Opcode::Trueif => {}

        Opcode::Trueff => {}

        Opcode::IsNull | Opcode::IsInvalid => {}

        Opcode::Copy => {}

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

        Opcode::Fcmp => {}

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
                    ty,
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
                        ty,
                    };
                    ctx.emit(inst);
                } else {
                    unimplemented!();
                }
            }
            Opcode::Brif => {
                unimplemented!("risc-v has no compare iflag");
            }
            Opcode::Brff => {
                /*
                   risc-v compare float value (in float registers) and write to a x register
                */
                // let condcode = ctx.data(branches[0]).fp_cond_code().unwrap();

                // let rs1 = ctx.put_input_in_regs(branches[0], 0);

                // match condcode {
                //     crate::ir::condcodes::FloatCC::Ordered => todo!(),
                //     crate::ir::condcodes::FloatCC::Unordered => todo!(),
                //     crate::ir::condcodes::FloatCC::Equal => todo!(),
                //     crate::ir::condcodes::FloatCC::NotEqual => todo!(),
                //     crate::ir::condcodes::FloatCC::OrderedNotEqual => todo!(),
                //     crate::ir::condcodes::FloatCC::UnorderedOrEqual => todo!(),
                //     crate::ir::condcodes::FloatCC::LessThan => todo!(),
                //     crate::ir::condcodes::FloatCC::LessThanOrEqual => todo!(),
                //     crate::ir::condcodes::FloatCC::GreaterThan => todo!(),
                //     crate::ir::condcodes::FloatCC::GreaterThanOrEqual => todo!(),
                //     crate::ir::condcodes::FloatCC::UnorderedOrLessThan => todo!(),
                //     crate::ir::condcodes::FloatCC::UnorderedOrLessThanOrEqual => todo!(),
                //     crate::ir::condcodes::FloatCC::UnorderedOrGreaterThan => todo!(),
                //     crate::ir::condcodes::FloatCC::UnorderedOrGreaterThanOrEqual => todo!(),
                // }
            }

            _ => unimplemented!(),
        }
    } else {
        // Must be an unconditional branch or an indirect branch.
        let op = ctx.data(branches[0]).opcode();
        match op {
            Opcode::Jump => {
                assert!(branches.len() == 1);
                ctx.emit(Inst::Jump {
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
