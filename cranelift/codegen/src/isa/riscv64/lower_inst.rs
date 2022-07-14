//! Lower a single Cranelift instruction into vcode.

use alloc::vec::Vec;

use crate::ir::Inst as IRInst;

use crate::ir::Opcode;
use crate::isa::riscv64::settings as aarch64_settings;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::settings::Flags;
use crate::CodegenResult;

use crate::ir::types::I64;

use super::lower::*;
use crate::isa::riscv64::abi::*;
use crate::isa::riscv64::inst::*;

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
            implemented_in_isle(ctx);
        }

        Opcode::Store | Opcode::Istore8 | Opcode::Istore16 | Opcode::Istore32 => {
            implemented_in_isle(ctx);
        }

        Opcode::StackAddr => {
            implemented_in_isle(ctx);
        }

        Opcode::AtomicRmw => {
            implemented_in_isle(ctx);
        }

        Opcode::AtomicCas => {
            implemented_in_isle(ctx);
        }

        Opcode::AtomicLoad => {
            implemented_in_isle(ctx);
        }

        Opcode::AtomicStore => {
            implemented_in_isle(ctx);
        }

        Opcode::Fence => {
            implemented_in_isle(ctx);
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
            implemented_in_isle(ctx);
        }

        Opcode::Selectif | Opcode::SelectifSpectreGuard => {
            implemented_in_isle(ctx);
        }

        Opcode::Bitselect => {
            implemented_in_isle(ctx);
        }

        Opcode::Vselect => {
            todo!()
        }

        Opcode::Trueif => {
            implemented_in_isle(ctx);
        }

        Opcode::Trueff => {
            implemented_in_isle(ctx);
        }

        Opcode::IsNull | Opcode::IsInvalid => {
            implemented_in_isle(ctx);
        }

        Opcode::Copy => {
            implemented_in_isle(ctx);
        }

        Opcode::Breduce | Opcode::Ireduce => {
            implemented_in_isle(ctx);
        }

        Opcode::Bextend | Opcode::Bmask => {
            implemented_in_isle(ctx);
        }

        Opcode::Bint => {
            implemented_in_isle(ctx);
        }

        Opcode::Bitcast => {
            implemented_in_isle(ctx);
        }

        Opcode::FallthroughReturn | Opcode::Return => {
            implemented_in_isle(ctx);
        }

        Opcode::Ifcmp | Opcode::Ffcmp => {
            panic!("Should never reach ifcmp as isel root!");
        }

        Opcode::Icmp => {
            implemented_in_isle(ctx);
        }

        Opcode::Fcmp => {
            implemented_in_isle(ctx);
        }

        Opcode::Debugtrap => {
            implemented_in_isle(ctx);
        }

        Opcode::Trap | Opcode::ResumableTrap => {
            implemented_in_isle(ctx);
        }

        Opcode::Trapif => {
            implemented_in_isle(ctx);
        }

        Opcode::Trapff => {
            implemented_in_isle(ctx);
        }

        Opcode::Trapz | Opcode::Trapnz | Opcode::ResumableTrapnz => {
            panic!("trapz / trapnz / resumable_trapnz should have been removed by legalization!");
        }

        Opcode::FuncAddr => {
            implemented_in_isle(ctx);
        }

        Opcode::GlobalValue => {
            panic!("global_value should have been removed by legalization!");
        }

        Opcode::SymbolValue => {
            implemented_in_isle(ctx);
        }

        Opcode::Call | Opcode::CallIndirect => {
            let caller_conv = ctx.abi().call_conv();
            let (mut abi, inputs) = match op {
                Opcode::Call => {
                    let (extname, dist) = ctx.call_target(insn).unwrap();
                    let extname = extname.clone();
                    let sig = ctx.call_sig(insn).unwrap();
                    assert!(inputs.len() == sig.params.len());
                    assert!(outputs.len() == sig.returns.len());
                    (
                        Riscv64ABICaller::from_func(sig, &extname, dist, caller_conv, flags)?,
                        &inputs[..],
                    )
                }
                Opcode::CallIndirect => {
                    let ptr = put_input_in_reg(ctx, inputs[0]);
                    let sig = ctx.call_sig(insn).unwrap();
                    assert!(inputs.len() - 1 == sig.params.len());
                    assert!(outputs.len() == sig.returns.len());
                    (
                        Riscv64ABICaller::from_ptr(sig, ptr, op, caller_conv, flags)?,
                        &inputs[1..],
                    )
                }
                _ => unreachable!(),
            };
            abi.emit_stack_pre_adjust(ctx);
            assert!(inputs.len() == abi.num_args());
            for i in abi.get_copy_to_arg_order() {
                let input = inputs[i];
                let arg_regs = put_input_in_regs(ctx, input);
                abi.emit_copy_regs_to_arg(ctx, i, arg_regs);
            }
            abi.emit_call(ctx);
            for (i, output) in outputs.iter().enumerate() {
                let retval_regs = get_output_reg(ctx, *output);
                abi.emit_copy_retval_to_regs(ctx, i, retval_regs);
            }
            abi.emit_stack_post_adjust(ctx);
        }

        Opcode::GetPinnedReg => pinned_register_not_used(),

        Opcode::SetPinnedReg => pinned_register_not_used(),

        Opcode::Jump
        | Opcode::Brz
        | Opcode::Brnz
        | Opcode::BrIcmp
        | Opcode::Brif
        | Opcode::Brff
        | Opcode::BrTable => {
            panic!("Branch opcode reached non-branch lowering logic!");
        }

        Opcode::Vconst => {
            unimplemented!()
        }

        Opcode::RawBitcast => {
            implemented_in_isle(ctx);
        }

        Opcode::Extractlane => {
            unimplemented!()
        }

        Opcode::Insertlane => {
            unimplemented!()
        }

        Opcode::Splat => {
            unimplemented!()
        }

        Opcode::ScalarToVector => {
            unimplemented!()
        }

        Opcode::VanyTrue | Opcode::VallTrue => {
            unimplemented!()
        }

        Opcode::VhighBits => {
            unimplemented!()
        }

        Opcode::Shuffle => {
            unimplemented!()
        }

        Opcode::Swizzle => {
            unimplemented!()
        }

        Opcode::Isplit => {
            implemented_in_isle(ctx);
        }

        Opcode::Iconcat => {
            implemented_in_isle(ctx);
        }

        Opcode::Imax | Opcode::Umax | Opcode::Umin | Opcode::Imin => {
            implemented_in_isle(ctx);
        }

        Opcode::IaddPairwise => {
            todo!()
        }

        Opcode::WideningPairwiseDotProductS => {
            todo!()
        }

        Opcode::Fadd | Opcode::Fsub | Opcode::Fmul | Opcode::Fdiv | Opcode::Fmin | Opcode::Fmax => {
            implemented_in_isle(ctx);
        }

        Opcode::FminPseudo | Opcode::FmaxPseudo => {
            todo!();
        }

        Opcode::Sqrt | Opcode::Fneg | Opcode::Fabs => {
            implemented_in_isle(ctx);
        }
        Opcode::Fpromote | Opcode::Fdemote => {
            implemented_in_isle(ctx);
        }

        Opcode::Ceil | Opcode::Floor | Opcode::Trunc | Opcode::Nearest => {
            implemented_in_isle(ctx);
        }

        Opcode::Fma => {
            implemented_in_isle(ctx);
        }
        Opcode::Fcopysign => {
            implemented_in_isle(ctx);
        }

        Opcode::FcvtToUint | Opcode::FcvtToSint => {
            implemented_in_isle(ctx);
        }
        Opcode::FcvtFromUint | Opcode::FcvtFromSint => {
            implemented_in_isle(ctx);
        }

        Opcode::FcvtToUintSat | Opcode::FcvtToSintSat => {
            implemented_in_isle(ctx);
        }

        Opcode::IaddIfcout => {
            implemented_in_isle(ctx);
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
            panic!(
                "op:{:?} ALU+imm and ALU+carry ops should not appear here!",
                op
            );
        }

        Opcode::Iabs => {
            implemented_in_isle(ctx);
        }
        Opcode::AvgRound => {
            unimplemented!();
        }

        Opcode::Snarrow | Opcode::Unarrow | Opcode::Uunarrow => {
            unimplemented!();
        }

        Opcode::SwidenLow | Opcode::SwidenHigh | Opcode::UwidenLow | Opcode::UwidenHigh => {
            unimplemented!();
        }

        Opcode::TlsValue => {}

        Opcode::SqmulRoundSat => {
            unimplemented!();
        }

        Opcode::FcvtLowFromSint => {
            unimplemented!();
        }

        Opcode::FvpromoteLow => {
            unimplemented!();
        }

        Opcode::Fvdemote => {
            unimplemented!();
        }

        Opcode::ConstAddr | Opcode::Vconcat | Opcode::Vsplit | Opcode::IfcmpSp => {
            unimplemented!();
        }
        Opcode::DynamicStackLoad => todo!(),
        Opcode::DynamicStackStore => todo!(),
        Opcode::DynamicStackAddr => todo!(),
        Opcode::ExtractVector => todo!(),
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
                if ty.bits() > Riscv64MachineDeps::word_bits() {
                    let insts = Inst::lower_br_icmp(
                        if op0.opcode() == Opcode::Brz {
                            IntCC::Equal
                        } else {
                            IntCC::NotEqual
                        },
                        reg,
                        ValueRegs::two(zero_reg(), zero_reg()),
                        taken,
                        not_taken,
                        ty,
                    );
                    insts.into_iter().for_each(|i| ctx.emit(i));
                } else {
                    let cond = if op0.opcode() == Opcode::Brz {
                        IntegerCompare {
                            rs1: reg.only_reg().unwrap(),
                            rs2: zero_reg(),
                            kind: IntCC::Equal,
                        }
                    } else {
                        IntegerCompare {
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
            }
            Opcode::BrIcmp => {
                let ty = ctx.input_ty(branches[0], 0);
                let a = ctx.put_input_in_regs(branches[0], 0);
                let b = ctx.put_input_in_regs(branches[0], 1);
                let cc = op0.cond_code().unwrap();
                Inst::lower_br_icmp(cc, a, b, taken, not_taken, ty)
                    .into_iter()
                    .for_each(|i| ctx.emit(i));
            }
            Opcode::Brif => {
                let (x, y, ty) = maybe_input_insn(
                    ctx,
                    InsnInput {
                        insn: branches[0],
                        input: 0,
                    },
                    crate::ir::Opcode::Ifcmp,
                )
                .map(|inst| get_ifcmp_parameters(ctx, inst))
                .unwrap();
                let cc = ctx.data(branches[0]).cond_code().unwrap();
                Inst::lower_br_icmp(cc, x, y, taken, not_taken, ty)
                    .into_iter()
                    .for_each(|i| ctx.emit(i));
            }
            Opcode::Brff => {
                let (x, y, ty) = maybe_input_insn(
                    ctx,
                    InsnInput {
                        insn: branches[0],
                        input: 0,
                    },
                    crate::ir::Opcode::Ffcmp,
                )
                .map(|inst| get_ffcmp_parameters(ctx, inst))
                .unwrap();
                let cc = ctx.data(branches[0]).fp_cond_code().unwrap();
                let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();

                Inst::lower_br_fcmp(cc, x, y, taken, not_taken, ty, tmp)
                    .into_iter()
                    .for_each(|i| ctx.emit(i));
            }
            _ => unreachable!(),
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
                let jt_size = targets.len() - 1;
                assert!(jt_size <= std::u32::MAX as usize);
                let ridx = put_input_in_reg(
                    ctx,
                    InsnInput {
                        insn: branches[0],
                        input: 0,
                    },
                );

                let tmp1 = ctx.alloc_tmp(I64).only_reg().unwrap();
                let jt_targets: Vec<BranchTarget> = targets
                    .iter()
                    .skip(1)
                    .map(|bix| BranchTarget::Label(*bix))
                    .collect();
                ctx.emit(Inst::BrTable {
                    index: ridx,
                    tmp1,
                    default_: BranchTarget::Label(targets[0]),
                    targets: jt_targets,
                });
            }
            _ => panic!("Unknown branch type!"),
        }
    }
    Ok(())
}

fn pinned_register_not_used() -> ! {
    unreachable!()
}
