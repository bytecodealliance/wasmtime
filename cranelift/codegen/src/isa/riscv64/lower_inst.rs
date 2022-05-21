//! Lower a single Cranelift instruction into vcode.

use crate::machinst::Writable;
use alloc::vec;
use alloc::vec::Vec;

use crate::ir::Inst as IRInst;
use crate::ir::InstructionData;
use crate::ir::Opcode;
use crate::isa::riscv64::settings as aarch64_settings;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::settings::Flags;
use crate::CodegenError;
use crate::CodegenResult;
use std::boxed::Box;

use crate::ir::types::{
    B1, B128, B16, B32, B64, B8, F32, F64, FFLAGS, I128, I16, I32, I64, I8, IFLAGS, R32, R64,
};

use super::lower::*;
use crate::isa::riscv64::abi::*;
use crate::isa::riscv64::inst::*;

pub(crate) fn is_valid_atomic_transaction_ty(ty: Type) -> bool {
    match ty {
        I32 | I64 => true,
        _ => false,
    }
}

// pub(crate) fn lower_icmp<C: LowerCtx<I = Inst>>(
//     ctx: &mut C,
//     insn: IRInst,
//     flags: &Flags,
//     isa_flags: &aarch64_settings::Flags,
// ) {
//     let op = ctx.data(insn).opcode();
//     assert(op == crate::ir::Opcode::Icmp);
//     let inputs = insn_inputs(ctx, insn);
//     let outputs = insn_outputs(ctx, insn);
//     let ty = ctx.output_ty(insn, 0);
// }

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
            gen_load(dst, base, off, out_ty, flags)
                .into_iter()
                .for_each(|i| ctx.emit(i));
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
            gen_store(src, base, off, elem_ty, flags)
                .into_iter()
                .for_each(|i| ctx.emit(i));
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
            /*
                todo:: where is the memory ordering parameter. ?????????
            */
            let r_dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let r_addr = ctx.put_input_in_regs(insn, 0).only_reg().unwrap();

            let ty_access = ty.unwrap();
            assert!(is_valid_atomic_transaction_ty(ty_access));
            let op = ctx.data(insn).atomic_rmw_op().unwrap();
            let arg2: Reg = if ty_access == I32
                && (op == crate::ir::AtomicRmwOp::Umin || op == crate::ir::AtomicRmwOp::Umax)
            {
                // we must narrow down int to fit u32
                let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
                let arg2 = ctx.put_input_in_regs(insn, 1).only_reg().unwrap();
                Inst::narrow_down_int(tmp, arg2, ty_access)
                    .into_iter()
                    .for_each(|i| ctx.emit(i));
                tmp.to_reg()
            } else {
                ctx.put_input_in_regs(insn, 1).only_reg().unwrap()
            };

            let risc_op = AtomicOP::from_atomicrmw_type_and_op(ty_access, op);
            if let Some(op) = risc_op {
                let i = Inst::Atomic {
                    op,
                    rd: r_dst,
                    addr: r_addr,
                    src: arg2,
                    aq: false,
                    rl: false,
                };
                ctx.emit(i);
            } else if op == crate::ir::AtomicRmwOp::Sub {
                let sub_op = if ty_access.bits() == 64 {
                    AluOPRRR::Sub
                } else {
                    AluOPRRR::Subw
                };
                let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
                ctx.emit(Inst::AluRRR {
                    alu_op: sub_op,
                    rd: tmp,
                    rs1: zero_reg(),
                    rs2: arg2,
                });
                let add_op = if ty_access.bits() == 64 {
                    AtomicOP::AmoaddD
                } else {
                    AtomicOP::AmoaddW
                };
                ctx.emit(Inst::Atomic {
                    op: add_op,
                    rd: r_dst,
                    addr: r_addr,
                    src: tmp.to_reg(),
                    aq: false,
                    rl: false,
                });
            } else if op == crate::ir::AtomicRmwOp::Nand {
                let lr_op = if ty_access.bits() == 64 {
                    AtomicOP::LrD
                } else {
                    AtomicOP::LrW
                };
                // load origin value
                let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
                ctx.emit(Inst::Atomic {
                    op: lr_op,
                    rd: tmp,
                    addr: r_addr,
                    src: zero_reg(),
                    aq: false,
                    rl: true,
                });
                // tmp = tmp & arg2
                ctx.emit(Inst::AluRRR {
                    alu_op: AluOPRRR::And,
                    rd: tmp,
                    rs1: tmp.to_reg(),
                    rs2: arg2,
                });
                // tmp = bit_not tmp;
                ctx.emit(Inst::construct_bit_not(tmp, tmp.to_reg()));
                let st_op = if ty_access.bits() == 64 {
                    AtomicOP::AmoswapD
                } else {
                    AtomicOP::AmoswapW
                };
                ctx.emit(Inst::Atomic {
                    op: st_op,
                    rd: r_dst,
                    src: tmp.to_reg(),
                    addr: r_addr,
                    aq: false,
                    rl: true,
                });
            } else {
                unreachable!();
            }
        }

        Opcode::AtomicCas => {
            let ty_access = ty.unwrap();
            assert!(is_valid_atomic_transaction_ty(ty_access));
            let r_dst = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let r_addr = ctx.put_input_in_regs(insn, 0).only_reg().unwrap();
            let r_expected = ctx.put_input_in_regs(insn, 1).only_reg().unwrap();
            let r_replacement = ctx.put_input_in_regs(insn, 2).only_reg().unwrap();
            let t0 = ctx.alloc_tmp(I64).only_reg().unwrap();
            ctx.emit(Inst::AtomicCas {
                t0,
                dst: r_dst,
                e: r_expected,
                addr: r_addr,
                v: r_replacement,
                ty: ty_access,
            });
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

        Opcode::StackLoad => {
            // panic!("Direct stack memory access not supported; should not be used by Wasm");
            /*
                other platform did not implemented this.
                but test file cranelift\filetests\filetests\runtests\atomic-cas.clif
                use this instruction
            */
            let offset = ctx.data(insn).load_store_offset().unwrap() as i64;
            let flags = ctx.memflags(insn).unwrap_or(MemFlags::new());
            gen_load(
                get_output_reg(ctx, outputs[0]),
                stack_reg(),
                offset,
                ctx.output_ty(insn, 0),
                flags,
            )
            .into_iter()
            .for_each(|i| ctx.emit(i));
        }
        Opcode::StackStore => {
            // panic!("Direct stack memory access not supported; should not be used by Wasm");
            /*
                other platform did not implemented this.
                but test file cranelift\filetests\filetests\runtests\atomic-cas.clif
                use this instruction
            */
            let offset = ctx.data(insn).load_store_offset().unwrap() as i64;
            let flags = ctx.memflags(insn).unwrap_or(MemFlags::new());

            gen_store(
                ctx.put_input_in_regs(insn, 0),
                stack_reg(),
                offset,
                ctx.input_ty(insn, 0),
                flags,
            )
            .into_iter()
            .for_each(|i| ctx.emit(i));
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
            let tmp1 = ctx.alloc_tmp(I64).only_reg().unwrap();
            let tmp2 = ctx.alloc_tmp(I64).only_reg().unwrap();
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let rcond = put_input_in_reg(ctx, inputs[0]);
            let x = put_input_in_reg(ctx, inputs[1]);
            let y = put_input_in_reg(ctx, inputs[2]);
            // get all x part
            ctx.emit(Inst::AluRRR {
                alu_op: AluOPRRR::And,
                rd: tmp1,
                rs1: rcond,
                rs2: x,
            });
            // bit not
            ctx.emit(Inst::construct_bit_not(tmp2, rcond));
            // get y  part
            ctx.emit(Inst::AluRRR {
                alu_op: AluOPRRR::And,
                rd: tmp2,
                rs1: tmp2.to_reg(),
                rs2: y,
            });
            ctx.emit(Inst::AluRRR {
                alu_op: AluOPRRR::Or,
                rd: rd,
                rs1: tmp1.to_reg(),
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
            let _ty = ctx.input_ty(insn, 0);
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

        Opcode::Breduce | Opcode::Ireduce => {
            // Smaller integers/booleans are stored with high-order bits
            // undefined, so we can simply do a copy.
            let rn = put_input_in_regs(ctx, inputs[0]).regs()[0];
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            ctx.emit(Inst::gen_move(rd, rn, ty.unwrap()));
        }

        Opcode::Bextend | Opcode::Bmask => {
            let from_ty = ctx.input_ty(insn, 0);
            let to_ty = ctx.output_ty(insn, 0);
            let from_bits = ty_bits(from_ty);
            let to_bits = ty_bits(to_ty);

            if from_ty.is_vector() || from_bits > 64 || to_bits > 64 {
                return Err(CodegenError::Unsupported(format!(
                    "{}: Unsupported type: {:?}",
                    op, from_ty
                )));
            }
            assert!(from_bits <= to_bits);
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let rn = put_input_in_reg(ctx, inputs[0]);
            if from_bits == to_bits {
                ctx.emit(Inst::gen_move(rd, rn, to_ty));
            } else {
                let to_bits = if to_bits > 32 { 64 } else { 32 };
                let from_bits = from_bits as u8;
                ctx.emit(Inst::Extend {
                    rd,
                    rn,
                    signed: true,
                    from_bits,
                    to_bits,
                });
            }
        }

        Opcode::Bint => {
            let ty = ty.unwrap();
            if ty.is_vector() {
                return Err(CodegenError::Unsupported(format!(
                    "Bint: Unsupported type: {:?}",
                    ty
                )));
            }
            let input = put_input_in_regs(ctx, inputs[0]);
            let output = get_output_reg(ctx, outputs[0]);
            ctx.emit(Inst::AluRRImm12 {
                alu_op: AluOPRRI::Andi,
                rd: Writable::from(output.regs()[0]),
                rs: input.regs()[0],
                imm12: Imm12::from_bits(1),
            });
            if ty_bits(ty) == 128 {
                ctx.emit(Inst::load_constant_imm12(
                    Writable::from(output.regs()[1]),
                    Imm12::zero(),
                ));
            }
        }

        Opcode::Bitcast => {
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let ity = ctx.input_ty(insn, 0);
            let oty = ctx.output_ty(insn, 0);
            let ity_bits = ty_bits(ity);
            let ity_vec_reg = ty_has_float_or_vec_representation(ity);
            let oty_bits = ty_bits(oty);
            let oty_vec_reg = ty_has_float_or_vec_representation(oty);
            debug_assert_eq!(ity_bits, oty_bits);
            match (ity_vec_reg, oty_vec_reg) {
                (true, true) => {
                    unimplemented!()
                }
                (false, false) => {
                    let rm = put_input_in_reg(ctx, inputs[0]);
                    ctx.emit(gen_move(rd, oty, rm, ity));
                }

                (false, true) => {
                    unimplemented!()
                }
                (true, false) => {
                    unimplemented!()
                }
            }
        }

        Opcode::FallthroughReturn | Opcode::Return => {
            for i in 0..ctx.num_inputs(insn) {
                let src_reg = ctx.put_input_in_regs(insn, 0);
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

        Opcode::Icmp => {
            let ty = ctx.input_ty(insn, 0);
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let a = put_input_in_regs(ctx, inputs[0]);
            let b = put_input_in_regs(ctx, inputs[1]);
            let cc = ctx.data(insn).cond_code().unwrap();
            ctx.emit(Inst::Icmp { cc, rd, a, b, ty });
        }

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

        Opcode::Debugtrap => {
            unimplemented!()
        }

        Opcode::Trap | Opcode::ResumableTrap => {
            let trap_code = ctx.data(insn).trap_code().unwrap();
            ctx.emit(Inst::Udf { trap_code });
        }

        Opcode::Trapif | Opcode::Trapff => {
            unimplemented!()
        }

        Opcode::Trapz | Opcode::Trapnz | Opcode::ResumableTrapnz => {
            panic!("trapz / trapnz / resumable_trapnz should have been removed by legalization!");
        }

        Opcode::FuncAddr => {
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let (extname, _) = ctx.call_target(insn).unwrap();
            let extname = extname.clone();
            ctx.emit(Inst::LoadExtName {
                rd,
                name: Box::new(extname),
                offset: 0,
            });
        }

        Opcode::GlobalValue => {
            panic!("global_value should have been removed by legalization!");
        }

        Opcode::SymbolValue => {
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let (extname, _, offset) = ctx.symbol_value(insn).unwrap();
            let extname = extname.clone();
            ctx.emit(Inst::LoadExtName {
                rd,
                name: Box::new(extname),
                offset,
            });
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

        Opcode::GetPinnedReg => {
            // let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            // ctx.emit(Inst::gen_move(rd, x_reg(PINNED_REG), I64));
            pinned_register_not_used()
        }

        Opcode::SetPinnedReg => {
            // let rm = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
            // ctx.emit(Inst::gen_move(writable_xreg(PINNED_REG), rm, I64));
            pinned_register_not_used()
        }

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
            let rm = put_input_in_reg(ctx, inputs[0]);
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let ity = ctx.input_ty(insn, 0);
            let oty = ctx.output_ty(insn, 0);
            ctx.emit(gen_move(rd, oty, rm, ity));
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
            let input_ty = ctx.input_ty(insn, 0);
            if input_ty != I128 {
                return Err(CodegenError::Unsupported(format!(
                    "Isplit: Unsupported type: {:?}",
                    input_ty
                )));
            }

            assert_eq!(ctx.output_ty(insn, 0), I64);
            assert_eq!(ctx.output_ty(insn, 1), I64);

            let src_regs = put_input_in_regs(ctx, inputs[0]);
            let dst_lo = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            let dst_hi = get_output_reg(ctx, outputs[1]).only_reg().unwrap();

            ctx.emit(Inst::gen_move(dst_lo, src_regs.regs()[0], I64));
            ctx.emit(Inst::gen_move(dst_hi, src_regs.regs()[1], I64));
        }

        Opcode::Iconcat => {
            let ty = ty.unwrap();

            if ty != I128 {
                return Err(CodegenError::Unsupported(format!(
                    "Iconcat: Unsupported type: {:?}",
                    ty
                )));
            }
            assert_eq!(ctx.input_ty(insn, 0), I64);
            assert_eq!(ctx.input_ty(insn, 1), I64);

            let src_lo = put_input_in_reg(ctx, inputs[0]);
            let src_hi = put_input_in_reg(ctx, inputs[1]);
            let dst = get_output_reg(ctx, outputs[0]);

            ctx.emit(Inst::gen_move(dst.regs()[0], src_lo, I64));
            ctx.emit(Inst::gen_move(dst.regs()[1], src_hi, I64));
        }

        Opcode::Imax | Opcode::Umax | Opcode::Umin | Opcode::Imin => {
            let ty = ty.unwrap();
            if ty.is_int() {
                let dst = ctx.get_output(insn, 0);
                let dst: Vec<_> = dst.regs().iter().map(|r| *r).collect();
                let x = put_input_in_regs(ctx, inputs[0]);
                let y = put_input_in_regs(ctx, inputs[1]);
                ctx.emit(Inst::IntSelect {
                    op: IntSelectOP::from_ir_op(op),
                    dst: dst,
                    x,
                    y,
                    ty,
                });
            } else {
                unimplemented!()
            }
        }

        Opcode::IaddPairwise => {
            todo!()
        }

        Opcode::WideningPairwiseDotProductS => {
            todo!()
        }

        Opcode::Fadd | Opcode::Fsub | Opcode::Fmul | Opcode::Fdiv | Opcode::Fmin | Opcode::Fmax => {
            let ty = ty.unwrap();
            let rs1 = put_input_in_reg(ctx, inputs[0]);
            let rs2 = put_input_in_reg(ctx, inputs[1]);
            let rd = get_output_reg(ctx, outputs[0]).only_reg().unwrap();
            if !ty.is_vector() {
                let ty_32 = ty == F32;
                let op = match op {
                    Opcode::Fadd => {
                        if ty_32 {
                            AluOPRRR::FaddS
                        } else {
                            AluOPRRR::FaddD
                        }
                    }
                    Opcode::Fsub => {
                        if ty_32 {
                            AluOPRRR::FsubS
                        } else {
                            AluOPRRR::FsubD
                        }
                    }
                    Opcode::Fmul => {
                        if ty_32 {
                            AluOPRRR::FmulS
                        } else {
                            AluOPRRR::FmulD
                        }
                    }
                    Opcode::Fdiv => {
                        if ty_32 {
                            AluOPRRR::FdivS
                        } else {
                            AluOPRRR::FdivD
                        }
                    }
                    Opcode::Fmin => {
                        if ty_32 {
                            AluOPRRR::FminS
                        } else {
                            AluOPRRR::FminD
                        }
                    }
                    Opcode::Fmax => {
                        if ty_32 {
                            AluOPRRR::FmaxS
                        } else {
                            AluOPRRR::FmaxD
                        }
                    }
                    _ => unreachable!(),
                };
                ctx.emit(Inst::AluRRR {
                    alu_op: op,
                    rd,
                    rs1,
                    rs2,
                });
            } else {
                unimplemented!()
            }
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
            panic!(
                "op:{:?} ALU+imm and ALU+carry ops should not appear here!",
                op
            );
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
                if ty.bits() as u32 > Riscv64MachineDeps::word_bits() {
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
                ir_iflags_conflict(op0.opcode());
            }
            Opcode::Brff => {
                ir_fflags_conflict(op0.opcode());
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
fn gen_load(
    dst: ValueRegs<Writable<Reg>>,
    base: Reg,
    off: i64,
    out_ty: Type,
    flags: MemFlags,
) -> SmallInstVec<Inst> {
    let mut insts = SmallInstVec::new();
    match out_ty.bits() {
        128 => {
            insts.push(Inst::Load {
                rd: dst.regs()[0],
                op: LoadOP::Ld,
                flags,
                from: AMode::RegOffset(base, off, I64),
            });
            insts.push(Inst::Load {
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
            insts.push(Inst::Load {
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
            insts.push(Inst::Load {
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
            insts.push(Inst::Load {
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
            insts.push(Inst::Load {
                rd: dst.regs()[0],
                op: op,
                flags,
                from: AMode::RegOffset(base, off, I64),
            });
        }
        _ => unreachable!(),
    }

    insts
}

pub(crate) fn gen_store(
    src: ValueRegs<Reg>,
    base: Reg,
    off: i64,
    elem_ty: Type,
    flags: MemFlags,
) -> SmallInstVec<Inst> {
    let mut insts = SmallInstVec::new();
    match elem_ty.bits() {
        128 => {
            insts.push(Inst::Store {
                to: AMode::RegOffset(base, off, I64),
                op: StoreOP::Sd,
                flags,
                src: src.regs()[0],
            });
            insts.push(Inst::Store {
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
            insts.push(Inst::Store {
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
            insts.push(Inst::Store {
                to: AMode::RegOffset(base, off, I64),
                op,
                flags,
                src: src.regs()[0],
            });
        }
        16 => {
            let op = StoreOP::Sh;
            insts.push(Inst::Store {
                to: AMode::RegOffset(base, off, I64),
                op,
                flags,
                src: src.regs()[0],
            });
        }
        8 | 1 => {
            let op = StoreOP::Sb;
            insts.push(Inst::Store {
                to: AMode::RegOffset(base, off, I64),
                op,
                flags,
                src: src.regs()[0],
            });
        }
        _ => unreachable!(),
    }
    insts
}

fn pinned_register_not_used() -> ! {
    unimplemented!();
}

#[cfg(test)]
mod test {
    #[test]
    fn compile_ok() {}
}
