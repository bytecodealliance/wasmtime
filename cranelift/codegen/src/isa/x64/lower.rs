//! Lowering rules for X64.

#![allow(non_snake_case)]

use crate::ir;
use crate::ir::{
    condcodes::FloatCC, condcodes::IntCC, types, AbiParam, ArgumentPurpose, ExternalName,
    Inst as IRInst, InstructionData, LibCall, Opcode, Signature, TrapCode, Type,
};
use crate::isa::x64::abi::*;
use crate::isa::x64::inst::args::*;
use crate::isa::x64::inst::*;
use crate::isa::{x64::X64Backend, CallConv};
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::result::CodegenResult;
use crate::settings::Flags;
use alloc::boxed::Box;
use alloc::vec::Vec;
use cranelift_codegen_shared::condcodes::CondCode;
use log::trace;
use regalloc::{Reg, RegClass, Writable};
use smallvec::SmallVec;
use std::convert::TryFrom;
use target_lexicon::Triple;

/// Context passed to all lowering functions.
type Ctx<'a> = &'a mut dyn LowerCtx<I = Inst>;

//=============================================================================
// Helpers for instruction lowering.

fn is_int_ty(ty: Type) -> bool {
    match ty {
        types::I8 | types::I16 | types::I32 | types::I64 | types::R64 => true,
        types::R32 => panic!("shouldn't have 32-bits refs on x64"),
        _ => false,
    }
}

fn is_bool_ty(ty: Type) -> bool {
    match ty {
        types::B1 | types::B8 | types::B16 | types::B32 | types::B64 => true,
        types::R32 => panic!("shouldn't have 32-bits refs on x64"),
        _ => false,
    }
}

/// This is target-word-size dependent.  And it excludes booleans and reftypes.
fn is_valid_atomic_transaction_ty(ty: Type) -> bool {
    match ty {
        types::I8 | types::I16 | types::I32 | types::I64 => true,
        _ => false,
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

fn inst_fp_condcode(data: &InstructionData) -> FloatCC {
    match data {
        &InstructionData::BranchFloat { cond, .. }
        | &InstructionData::FloatCompare { cond, .. }
        | &InstructionData::FloatCond { cond, .. }
        | &InstructionData::FloatCondTrap { cond, .. } => cond,
        _ => panic!("inst_fp_condcode(x64): unhandled: {:?}", data),
    }
}

fn inst_atomic_rmw_op(data: &InstructionData) -> Option<ir::AtomicRmwOp> {
    match data {
        &InstructionData::AtomicRmw { op, .. } => Some(op),
        _ => None,
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

fn matches_input<C: LowerCtx<I = Inst>>(c: &mut C, input: InsnInput, op: Opcode) -> Option<IRInst> {
    let inputs = c.get_input(input.insn, input.input);
    if let Some((src_inst, _)) = inputs.inst {
        let data = c.data(src_inst);
        if data.opcode() == op {
            return Some(src_inst);
        }
    }
    None
}

/// Put the given input into a register, and mark it as used (side-effect).
fn input_to_reg(ctx: Ctx, spec: InsnInput) -> Reg {
    let inputs = ctx.get_input(spec.insn, spec.input);
    ctx.use_input_reg(inputs);
    inputs.reg
}

/// An extension specification for `extend_input_to_reg`.
enum ExtSpec {
    ZeroExtendTo32,
    ZeroExtendTo64,
    SignExtendTo32,
    SignExtendTo64,
}

/// Put the given input into a register, marking it as used, and do a zero- or signed- extension if
/// required. (This obviously causes side-effects.)
fn extend_input_to_reg(ctx: Ctx, spec: InsnInput, ext_spec: ExtSpec) -> Reg {
    let requested_size = match ext_spec {
        ExtSpec::ZeroExtendTo32 | ExtSpec::SignExtendTo32 => 32,
        ExtSpec::ZeroExtendTo64 | ExtSpec::SignExtendTo64 => 64,
    };
    let input_size = ctx.input_ty(spec.insn, spec.input).bits();

    let ext_mode = match (input_size, requested_size) {
        (a, b) if a == b => return input_to_reg(ctx, spec),
        (a, 32) if a == 1 || a == 8 => ExtMode::BL,
        (a, 64) if a == 1 || a == 8 => ExtMode::BQ,
        (16, 32) => ExtMode::WL,
        (16, 64) => ExtMode::WQ,
        (32, 64) => ExtMode::LQ,
        _ => unreachable!(),
    };

    let requested_ty = if requested_size == 32 {
        types::I32
    } else {
        types::I64
    };

    let src = input_to_reg_mem(ctx, spec);
    let dst = ctx.alloc_tmp(RegClass::I64, requested_ty);
    match ext_spec {
        ExtSpec::ZeroExtendTo32 | ExtSpec::ZeroExtendTo64 => {
            ctx.emit(Inst::movzx_rm_r(
                ext_mode, src, dst, /* infallible */ None,
            ))
        }
        ExtSpec::SignExtendTo32 | ExtSpec::SignExtendTo64 => {
            ctx.emit(Inst::movsx_rm_r(
                ext_mode, src, dst, /* infallible */ None,
            ))
        }
    }
    dst.to_reg()
}

/// Put the given input into a register or a memory operand.
/// Effectful: may mark the given input as used, when returning the register form.
fn input_to_reg_mem(ctx: Ctx, spec: InsnInput) -> RegMem {
    // TODO handle memory.
    RegMem::reg(input_to_reg(ctx, spec))
}

/// Returns whether the given input is an immediate that can be properly sign-extended, without any
/// possible side-effect.
fn input_to_sext_imm(ctx: Ctx, spec: InsnInput) -> Option<u32> {
    ctx.get_input(spec.insn, spec.input).constant.and_then(|x| {
        // For i64 instructions (prefixed with REX.W), require that the immediate will sign-extend
        // to 64 bits. For other sizes, it doesn't matter and we can just use the plain
        // constant.
        if ctx.input_ty(spec.insn, spec.input).bytes() != 8 || low32_will_sign_extend_to_64(x) {
            Some(x as u32)
        } else {
            None
        }
    })
}

/// Put the given input into an immediate, a register or a memory operand.
/// Effectful: may mark the given input as used, when returning the register form.
fn input_to_reg_mem_imm(ctx: Ctx, spec: InsnInput) -> RegMemImm {
    match input_to_sext_imm(ctx, spec) {
        Some(x) => RegMemImm::imm(x),
        None => match input_to_reg_mem(ctx, spec) {
            RegMem::Reg { reg } => RegMemImm::reg(reg),
            RegMem::Mem { addr } => RegMemImm::mem(addr),
        },
    }
}

fn output_to_reg(ctx: Ctx, spec: InsnOutput) -> Writable<Reg> {
    ctx.get_output(spec.insn, spec.output)
}

fn emit_cmp(ctx: Ctx, insn: IRInst) {
    let ty = ctx.input_ty(insn, 0);

    let inputs = [InsnInput { insn, input: 0 }, InsnInput { insn, input: 1 }];

    // TODO Try to commute the operands (and invert the condition) if one is an immediate.
    let lhs = input_to_reg(ctx, inputs[0]);
    let rhs = input_to_reg_mem_imm(ctx, inputs[1]);

    // Cranelift's icmp semantics want to compare lhs - rhs, while Intel gives
    // us dst - src at the machine instruction level, so invert operands.
    ctx.emit(Inst::cmp_rmi_r(ty.bytes() as u8, rhs, lhs));
}

#[derive(PartialEq)]
enum FcmpOperands {
    Swap,
    DontSwap,
}

fn emit_fcmp(ctx: Ctx, insn: IRInst, swap_operands: FcmpOperands) {
    // The only valid CC constructed with `from_floatcc` can be put in the flag
    // register with a direct float comparison; do this here.
    let input_ty = ctx.input_ty(insn, 0);
    let op = match input_ty {
        types::F32 => SseOpcode::Ucomiss,
        types::F64 => SseOpcode::Ucomisd,
        _ => panic!("Bad input type to Fcmp"),
    };
    let inputs = &[InsnInput { insn, input: 0 }, InsnInput { insn, input: 1 }];
    let (lhs, rhs) = if swap_operands == FcmpOperands::Swap {
        (
            input_to_reg(ctx, inputs[1]),
            input_to_reg_mem(ctx, inputs[0]),
        )
    } else {
        (
            input_to_reg(ctx, inputs[0]),
            input_to_reg_mem(ctx, inputs[1]),
        )
    };
    ctx.emit(Inst::xmm_cmp_rm_r(op, rhs, lhs));
}

fn make_libcall_sig(ctx: Ctx, insn: IRInst, call_conv: CallConv, ptr_ty: Type) -> Signature {
    let mut sig = Signature::new(call_conv);
    for i in 0..ctx.num_inputs(insn) {
        sig.params.push(AbiParam::new(ctx.input_ty(insn, i)));
    }
    for i in 0..ctx.num_outputs(insn) {
        sig.returns.push(AbiParam::new(ctx.output_ty(insn, i)));
    }
    if call_conv.extends_baldrdash() {
        // Adds the special VMContext parameter to the signature.
        sig.params
            .push(AbiParam::special(ptr_ty, ArgumentPurpose::VMContext));
    }
    sig
}

fn emit_vm_call<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    flags: &Flags,
    triple: &Triple,
    libcall: LibCall,
    insn: IRInst,
    inputs: SmallVec<[InsnInput; 4]>,
    outputs: SmallVec<[InsnOutput; 2]>,
) -> CodegenResult<()> {
    let extname = ExternalName::LibCall(libcall);

    let dist = if flags.use_colocated_libcalls() {
        RelocDistance::Near
    } else {
        RelocDistance::Far
    };

    // TODO avoid recreating signatures for every single Libcall function.
    let call_conv = CallConv::for_libcall(flags, CallConv::triple_default(triple));
    let sig = make_libcall_sig(ctx, insn, call_conv, types::I64);

    let loc = ctx.srcloc(insn);
    let mut abi = X64ABICall::from_func(&sig, &extname, dist, loc)?;

    abi.emit_stack_pre_adjust(ctx);

    let vm_context = if call_conv.extends_baldrdash() { 1 } else { 0 };
    assert_eq!(inputs.len() + vm_context, abi.num_args());

    for (i, input) in inputs.iter().enumerate() {
        let arg_reg = input_to_reg(ctx, *input);
        abi.emit_copy_reg_to_arg(ctx, i, arg_reg);
    }
    if call_conv.extends_baldrdash() {
        let vm_context_vreg = ctx
            .get_vm_context()
            .expect("should have a VMContext to pass to libcall funcs");
        abi.emit_copy_reg_to_arg(ctx, inputs.len(), vm_context_vreg);
    }

    abi.emit_call(ctx);
    for (i, output) in outputs.iter().enumerate() {
        let retval_reg = output_to_reg(ctx, *output);
        abi.emit_copy_retval_to_reg(ctx, i, retval_reg);
    }
    abi.emit_stack_post_adjust(ctx);

    Ok(())
}

//=============================================================================
// Top-level instruction lowering entry point, for one instruction.

/// Actually codegen an instruction's results into registers.
fn lower_insn_to_regs<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    insn: IRInst,
    flags: &Flags,
    triple: &Triple,
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
            if let Some(w64) = iri_to_u64_imm(ctx, insn) {
                let dst_is_64 = w64 > 0x7fffffff;
                let dst = output_to_reg(ctx, outputs[0]);
                ctx.emit(Inst::imm_r(dst_is_64, w64, dst));
            } else {
                unimplemented!();
            }
        }

        Opcode::Iadd
        | Opcode::IaddIfcout
        | Opcode::Isub
        | Opcode::Imul
        | Opcode::Band
        | Opcode::Bor
        | Opcode::Bxor => {
            let ty = ty.unwrap();
            if ty.lane_count() > 1 {
                let sse_op = match op {
                    Opcode::Iadd => match ty {
                        types::I8X16 => SseOpcode::Paddb,
                        types::I16X8 => SseOpcode::Paddw,
                        types::I32X4 => SseOpcode::Paddd,
                        types::I64X2 => SseOpcode::Paddq,
                        _ => panic!("Unsupported type for packed Iadd instruction"),
                    },
                    Opcode::Isub => match ty {
                        types::I8X16 => SseOpcode::Psubb,
                        types::I16X8 => SseOpcode::Psubw,
                        types::I32X4 => SseOpcode::Psubd,
                        types::I64X2 => SseOpcode::Psubq,
                        _ => panic!("Unsupported type for packed Isub instruction"),
                    },
                    Opcode::Imul => match ty {
                        types::I16X8 => SseOpcode::Pmullw,
                        types::I32X4 => SseOpcode::Pmulld,
                        _ => panic!("Unsupported type for packed Imul instruction"),
                    },
                    _ => panic!("Unsupported packed instruction"),
                };
                let lhs = input_to_reg(ctx, inputs[0]);
                let rhs = input_to_reg_mem(ctx, inputs[1]);
                let dst = output_to_reg(ctx, outputs[0]);

                // Move the `lhs` to the same register as `dst`.
                ctx.emit(Inst::gen_move(dst, lhs, ty));
                ctx.emit(Inst::xmm_rm_r(sse_op, rhs, dst));
            } else {
                let is_64 = ty == types::I64;
                let alu_op = match op {
                    Opcode::Iadd | Opcode::IaddIfcout => AluRmiROpcode::Add,
                    Opcode::Isub => AluRmiROpcode::Sub,
                    Opcode::Imul => AluRmiROpcode::Mul,
                    Opcode::Band => AluRmiROpcode::And,
                    Opcode::Bor => AluRmiROpcode::Or,
                    Opcode::Bxor => AluRmiROpcode::Xor,
                    _ => unreachable!(),
                };

                let (lhs, rhs) = match op {
                    Opcode::Iadd
                    | Opcode::IaddIfcout
                    | Opcode::Imul
                    | Opcode::Band
                    | Opcode::Bor
                    | Opcode::Bxor => {
                        // For commutative operations, try to commute operands if one is an
                        // immediate.
                        if let Some(imm) = input_to_sext_imm(ctx, inputs[0]) {
                            (input_to_reg(ctx, inputs[1]), RegMemImm::imm(imm))
                        } else {
                            (
                                input_to_reg(ctx, inputs[0]),
                                input_to_reg_mem_imm(ctx, inputs[1]),
                            )
                        }
                    }
                    Opcode::Isub => (
                        input_to_reg(ctx, inputs[0]),
                        input_to_reg_mem_imm(ctx, inputs[1]),
                    ),
                    _ => unreachable!(),
                };

                let dst = output_to_reg(ctx, outputs[0]);
                ctx.emit(Inst::mov_r_r(true, lhs, dst));
                ctx.emit(Inst::alu_rmi_r(is_64, alu_op, rhs, dst));
            }
        }

        Opcode::Ishl | Opcode::Ushr | Opcode::Sshr | Opcode::Rotl | Opcode::Rotr => {
            let dst_ty = ctx.output_ty(insn, 0);
            debug_assert_eq!(ctx.input_ty(insn, 0), dst_ty);
            debug_assert!(dst_ty == types::I32 || dst_ty == types::I64);

            let lhs = input_to_reg(ctx, inputs[0]);

            let (count, rhs) = if let Some(cst) = ctx.get_constant(inputs[1].insn) {
                let cst = if op == Opcode::Rotl || op == Opcode::Rotr {
                    // Mask rotation count, according to Cranelift's semantics.
                    (cst as u8) & (dst_ty.bits() as u8 - 1)
                } else {
                    cst as u8
                };
                (Some(cst), None)
            } else {
                (None, Some(input_to_reg(ctx, inputs[1])))
            };

            let dst = output_to_reg(ctx, outputs[0]);

            let shift_kind = match op {
                Opcode::Ishl => ShiftKind::ShiftLeft,
                Opcode::Ushr => ShiftKind::ShiftRightLogical,
                Opcode::Sshr => ShiftKind::ShiftRightArithmetic,
                Opcode::Rotl => ShiftKind::RotateLeft,
                Opcode::Rotr => ShiftKind::RotateRight,
                _ => unreachable!(),
            };

            let is_64 = dst_ty == types::I64;
            let w_rcx = Writable::from_reg(regs::rcx());
            ctx.emit(Inst::mov_r_r(true, lhs, dst));
            if count.is_none() {
                ctx.emit(Inst::mov_r_r(true, rhs.unwrap(), w_rcx));
            }
            ctx.emit(Inst::shift_r(is_64, shift_kind, count, dst));
        }

        Opcode::Clz => {
            // TODO when the x86 flags have use_lzcnt, we can use LZCNT.

            // General formula using bit-scan reverse (BSR):
            // mov -1, %dst
            // bsr %src, %tmp
            // cmovz %dst, %tmp
            // mov $(size_bits - 1), %dst
            // sub %tmp, %dst

            let (ext_spec, ty) = match ctx.input_ty(insn, 0) {
                types::I8 | types::I16 => (Some(ExtSpec::ZeroExtendTo32), types::I32),
                a if a == types::I32 || a == types::I64 => (None, a),
                _ => unreachable!(),
            };

            let src = if let Some(ext_spec) = ext_spec {
                RegMem::reg(extend_input_to_reg(ctx, inputs[0], ext_spec))
            } else {
                input_to_reg_mem(ctx, inputs[0])
            };
            let dst = output_to_reg(ctx, outputs[0]);

            let tmp = ctx.alloc_tmp(RegClass::I64, ty);
            ctx.emit(Inst::imm_r(ty == types::I64, u64::max_value(), dst));

            ctx.emit(Inst::unary_rm_r(
                ty.bytes() as u8,
                UnaryRmROpcode::Bsr,
                src,
                tmp,
            ));

            ctx.emit(Inst::cmove(
                ty.bytes() as u8,
                CC::Z,
                RegMem::reg(dst.to_reg()),
                tmp,
            ));

            ctx.emit(Inst::imm_r(ty == types::I64, ty.bits() as u64 - 1, dst));

            ctx.emit(Inst::alu_rmi_r(
                ty == types::I64,
                AluRmiROpcode::Sub,
                RegMemImm::reg(tmp.to_reg()),
                dst,
            ));
        }

        Opcode::Ctz => {
            // TODO when the x86 flags have use_bmi1, we can use TZCNT.

            // General formula using bit-scan forward (BSF):
            // bsf %src, %dst
            // mov $(size_bits), %tmp
            // cmovz %tmp, %dst
            let ty = ctx.input_ty(insn, 0);
            let ty = if ty.bits() < 32 { types::I32 } else { ty };
            debug_assert!(ty == types::I32 || ty == types::I64);

            let src = input_to_reg_mem(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);

            let tmp = ctx.alloc_tmp(RegClass::I64, ty);
            ctx.emit(Inst::imm_r(false /* 64 bits */, ty.bits() as u64, tmp));

            ctx.emit(Inst::unary_rm_r(
                ty.bytes() as u8,
                UnaryRmROpcode::Bsf,
                src,
                dst,
            ));

            ctx.emit(Inst::cmove(
                ty.bytes() as u8,
                CC::Z,
                RegMem::reg(tmp.to_reg()),
                dst,
            ));
        }

        Opcode::Popcnt => {
            // TODO when the x86 flags have use_popcnt, we can use the popcnt instruction.

            let (ext_spec, ty) = match ctx.input_ty(insn, 0) {
                types::I8 | types::I16 => (Some(ExtSpec::ZeroExtendTo32), types::I32),
                a if a == types::I32 || a == types::I64 => (None, a),
                _ => unreachable!(),
            };

            let src = if let Some(ext_spec) = ext_spec {
                RegMem::reg(extend_input_to_reg(ctx, inputs[0], ext_spec))
            } else {
                input_to_reg_mem(ctx, inputs[0])
            };
            let dst = output_to_reg(ctx, outputs[0]);

            if ty == types::I64 {
                let is_64 = true;

                let tmp1 = ctx.alloc_tmp(RegClass::I64, types::I64);
                let tmp2 = ctx.alloc_tmp(RegClass::I64, types::I64);
                let cst = ctx.alloc_tmp(RegClass::I64, types::I64);

                // mov src, tmp1
                ctx.emit(Inst::mov64_rm_r(src.clone(), tmp1, None));

                // shr $1, tmp1
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(1),
                    tmp1,
                ));

                // mov 0x7777_7777_7777_7777, cst
                ctx.emit(Inst::imm_r(is_64, 0x7777777777777777, cst));

                // andq cst, tmp1
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::And,
                    RegMemImm::reg(cst.to_reg()),
                    tmp1,
                ));

                // mov src, tmp2
                ctx.emit(Inst::mov64_rm_r(src, tmp2, None));

                // sub tmp1, tmp2
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Sub,
                    RegMemImm::reg(tmp1.to_reg()),
                    tmp2,
                ));

                // shr $1, tmp1
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(1),
                    tmp1,
                ));

                // and cst, tmp1
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::And,
                    RegMemImm::reg(cst.to_reg()),
                    tmp1,
                ));

                // sub tmp1, tmp2
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Sub,
                    RegMemImm::reg(tmp1.to_reg()),
                    tmp2,
                ));

                // shr $1, tmp1
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(1),
                    tmp1,
                ));

                // and cst, tmp1
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::And,
                    RegMemImm::reg(cst.to_reg()),
                    tmp1,
                ));

                // sub tmp1, tmp2
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Sub,
                    RegMemImm::reg(tmp1.to_reg()),
                    tmp2,
                ));

                // mov tmp2, dst
                ctx.emit(Inst::mov64_rm_r(RegMem::reg(tmp2.to_reg()), dst, None));

                // shr $4, dst
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(4),
                    dst,
                ));

                // add tmp2, dst
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Add,
                    RegMemImm::reg(tmp2.to_reg()),
                    dst,
                ));

                // mov $0x0F0F_0F0F_0F0F_0F0F, cst
                ctx.emit(Inst::imm_r(is_64, 0x0F0F0F0F0F0F0F0F, cst));

                // and cst, dst
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::And,
                    RegMemImm::reg(cst.to_reg()),
                    dst,
                ));

                // mov $0x0101_0101_0101_0101, cst
                ctx.emit(Inst::imm_r(is_64, 0x0101010101010101, cst));

                // mul cst, dst
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Mul,
                    RegMemImm::reg(cst.to_reg()),
                    dst,
                ));

                // shr $56, dst
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(56),
                    dst,
                ));
            } else {
                assert_eq!(ty, types::I32);
                let is_64 = false;

                let tmp1 = ctx.alloc_tmp(RegClass::I64, types::I64);
                let tmp2 = ctx.alloc_tmp(RegClass::I64, types::I64);

                // mov src, tmp1
                ctx.emit(Inst::mov64_rm_r(src.clone(), tmp1, None));

                // shr $1, tmp1
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(1),
                    tmp1,
                ));

                // andq $0x7777_7777, tmp1
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::And,
                    RegMemImm::imm(0x77777777),
                    tmp1,
                ));

                // mov src, tmp2
                ctx.emit(Inst::mov64_rm_r(src, tmp2, None));

                // sub tmp1, tmp2
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Sub,
                    RegMemImm::reg(tmp1.to_reg()),
                    tmp2,
                ));

                // shr $1, tmp1
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(1),
                    tmp1,
                ));

                // and 0x7777_7777, tmp1
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::And,
                    RegMemImm::imm(0x77777777),
                    tmp1,
                ));

                // sub tmp1, tmp2
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Sub,
                    RegMemImm::reg(tmp1.to_reg()),
                    tmp2,
                ));

                // shr $1, tmp1
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(1),
                    tmp1,
                ));

                // and $0x7777_7777, tmp1
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::And,
                    RegMemImm::imm(0x77777777),
                    tmp1,
                ));

                // sub tmp1, tmp2
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Sub,
                    RegMemImm::reg(tmp1.to_reg()),
                    tmp2,
                ));

                // mov tmp2, dst
                ctx.emit(Inst::mov64_rm_r(RegMem::reg(tmp2.to_reg()), dst, None));

                // shr $4, dst
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(4),
                    dst,
                ));

                // add tmp2, dst
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Add,
                    RegMemImm::reg(tmp2.to_reg()),
                    dst,
                ));

                // and $0x0F0F_0F0F, dst
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::And,
                    RegMemImm::imm(0x0F0F0F0F),
                    dst,
                ));

                // mul $0x0101_0101, dst
                ctx.emit(Inst::alu_rmi_r(
                    is_64,
                    AluRmiROpcode::Mul,
                    RegMemImm::imm(0x01010101),
                    dst,
                ));

                // shr $24, dst
                ctx.emit(Inst::shift_r(
                    is_64,
                    ShiftKind::ShiftRightLogical,
                    Some(24),
                    dst,
                ));
            }
        }

        Opcode::IsNull | Opcode::IsInvalid => {
            // Null references are represented by the constant value 0; invalid references are
            // represented by the constant value -1. See `define_reftypes()` in
            // `meta/src/isa/x86/encodings.rs` to confirm.
            let src = input_to_reg(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);
            let ty = ctx.input_ty(insn, 0);
            let imm = match op {
                Opcode::IsNull => {
                    // TODO could use tst src, src for IsNull
                    0
                }
                Opcode::IsInvalid => {
                    // We can do a 32-bit comparison even in 64-bits mode, as the constant is then
                    // sign-extended.
                    0xffffffff
                }
                _ => unreachable!(),
            };
            ctx.emit(Inst::cmp_rmi_r(ty.bytes() as u8, RegMemImm::imm(imm), src));
            ctx.emit(Inst::setcc(CC::Z, dst));
        }

        Opcode::Uextend
        | Opcode::Sextend
        | Opcode::Bint
        | Opcode::Breduce
        | Opcode::Bextend
        | Opcode::Ireduce => {
            let src_ty = ctx.input_ty(insn, 0);
            let dst_ty = ctx.output_ty(insn, 0);

            let src = input_to_reg_mem(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);

            let ext_mode = match (src_ty.bits(), dst_ty.bits()) {
                (1, 32) | (8, 32) => Some(ExtMode::BL),
                (1, 64) | (8, 64) => Some(ExtMode::BQ),
                (16, 32) => Some(ExtMode::WL),
                (16, 64) => Some(ExtMode::WQ),
                (32, 64) => Some(ExtMode::LQ),
                (x, y) if x >= y => None,
                _ => unreachable!(
                    "unexpected extension kind from {:?} to {:?}",
                    src_ty, dst_ty
                ),
            };

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

            if let Some(ext_mode) = ext_mode {
                if op == Opcode::Sextend {
                    ctx.emit(Inst::movsx_rm_r(
                        ext_mode, src, dst, /* infallible */ None,
                    ));
                } else {
                    ctx.emit(Inst::movzx_rm_r(
                        ext_mode, src, dst, /* infallible */ None,
                    ));
                }
            } else {
                ctx.emit(Inst::mov64_rm_r(src, dst, /* infallible */ None));
            }
        }

        Opcode::Icmp => {
            emit_cmp(ctx, insn);

            let condcode = inst_condcode(ctx.data(insn));
            let cc = CC::from_intcc(condcode);
            let dst = output_to_reg(ctx, outputs[0]);
            ctx.emit(Inst::setcc(cc, dst));
        }

        Opcode::Fcmp => {
            let condcode = inst_fp_condcode(ctx.data(insn));
            let input_ty = ctx.input_ty(insn, 0);
            if !input_ty.is_vector() {
                let op = match input_ty {
                    types::F32 => SseOpcode::Ucomiss,
                    types::F64 => SseOpcode::Ucomisd,
                    _ => panic!("Bad input type to fcmp: {}", input_ty),
                };

                // Unordered is returned by setting ZF, PF, CF <- 111
                // Greater than by ZF, PF, CF <- 000
                // Less than by ZF, PF, CF <- 001
                // Equal by ZF, PF, CF <- 100
                //
                // Checking the result of comiss is somewhat annoying because you don't have setcc
                // instructions that explicitly check simultaneously for the condition (i.e. eq, le,
                // gt, etc) *and* orderedness.
                //
                // So that might mean we need more than one setcc check and then a logical "and" or
                // "or" to determine both, in some cases.  However knowing that if the parity bit is
                // set, then the result was considered unordered and knowing that if the parity bit is
                // set, then both the ZF and CF flag bits must also be set we can get away with using
                // one setcc for most condition codes.

                match condcode {
                    FloatCC::LessThan
                    | FloatCC::LessThanOrEqual
                    | FloatCC::UnorderedOrGreaterThan
                    | FloatCC::UnorderedOrGreaterThanOrEqual => {
                        // setb and setbe for ordered LessThan and LessThanOrEqual check if CF = 1
                        // which doesn't exclude unorderdness. To get around this we can reverse the
                        // operands and the cc test to instead check if CF and ZF are 0 which would
                        // also excludes unorderedness. Using similiar logic we also reverse
                        // UnorderedOrGreaterThan and UnorderedOrGreaterThanOrEqual and assure that ZF
                        // or CF is 1 to exclude orderedness.
                        let lhs = input_to_reg_mem(ctx, inputs[0]);
                        let rhs = input_to_reg(ctx, inputs[1]);
                        let dst = output_to_reg(ctx, outputs[0]);
                        ctx.emit(Inst::xmm_cmp_rm_r(op, lhs, rhs));
                        let condcode = condcode.reverse();
                        let cc = CC::from_floatcc(condcode);
                        ctx.emit(Inst::setcc(cc, dst));
                    }

                    FloatCC::Equal => {
                        // Outlier case: equal means both the operands are ordered and equal; we cannot
                        // get around checking the parity bit to determine if the result was ordered.
                        let lhs = input_to_reg(ctx, inputs[0]);
                        let rhs = input_to_reg_mem(ctx, inputs[1]);
                        let dst = output_to_reg(ctx, outputs[0]);
                        let tmp_gpr1 = ctx.alloc_tmp(RegClass::I64, types::I32);
                        ctx.emit(Inst::xmm_cmp_rm_r(op, rhs, lhs));
                        ctx.emit(Inst::setcc(CC::NP, tmp_gpr1));
                        ctx.emit(Inst::setcc(CC::Z, dst));
                        ctx.emit(Inst::alu_rmi_r(
                            false,
                            AluRmiROpcode::And,
                            RegMemImm::reg(tmp_gpr1.to_reg()),
                            dst,
                        ));
                    }

                    FloatCC::NotEqual => {
                        // Outlier case: not equal means either the operands are unordered, or they're
                        // not the same value.
                        let lhs = input_to_reg(ctx, inputs[0]);
                        let rhs = input_to_reg_mem(ctx, inputs[1]);
                        let dst = output_to_reg(ctx, outputs[0]);
                        let tmp_gpr1 = ctx.alloc_tmp(RegClass::I64, types::I32);
                        ctx.emit(Inst::xmm_cmp_rm_r(op, rhs, lhs));
                        ctx.emit(Inst::setcc(CC::P, tmp_gpr1));
                        ctx.emit(Inst::setcc(CC::NZ, dst));
                        ctx.emit(Inst::alu_rmi_r(
                            false,
                            AluRmiROpcode::Or,
                            RegMemImm::reg(tmp_gpr1.to_reg()),
                            dst,
                        ));
                    }

                    _ => {
                        // For all remaining condition codes we can handle things with one check.
                        let lhs = input_to_reg(ctx, inputs[0]);
                        let rhs = input_to_reg_mem(ctx, inputs[1]);
                        let dst = output_to_reg(ctx, outputs[0]);
                        let cc = CC::from_floatcc(condcode);
                        ctx.emit(Inst::xmm_cmp_rm_r(op, rhs, lhs));
                        ctx.emit(Inst::setcc(cc, dst));
                    }
                }
            } else {
                let op = match input_ty {
                    types::F32X4 => SseOpcode::Cmpps,
                    types::F64X2 => SseOpcode::Cmppd,
                    _ => panic!("Bad input type to fcmp: {}", input_ty),
                };

                // Since some packed comparisons are not available, some of the condition codes
                // must be inverted, with a corresponding `flip` of the operands.
                let (imm, flip) = match condcode {
                    FloatCC::GreaterThan => (FcmpImm::LessThan, true),
                    FloatCC::GreaterThanOrEqual => (FcmpImm::LessThanOrEqual, true),
                    FloatCC::UnorderedOrLessThan => (FcmpImm::UnorderedOrGreaterThan, true),
                    FloatCC::UnorderedOrLessThanOrEqual => {
                        (FcmpImm::UnorderedOrGreaterThanOrEqual, true)
                    }
                    FloatCC::OrderedNotEqual | FloatCC::UnorderedOrEqual => {
                        panic!("unsupported float condition code: {}", condcode)
                    }
                    _ => (FcmpImm::from(condcode), false),
                };

                // Determine the operands of the comparison, possibly by flipping them.
                let (lhs, rhs) = if flip {
                    (
                        input_to_reg(ctx, inputs[1]),
                        input_to_reg_mem(ctx, inputs[0]),
                    )
                } else {
                    (
                        input_to_reg(ctx, inputs[0]),
                        input_to_reg_mem(ctx, inputs[1]),
                    )
                };

                // Move the `lhs` to the same register as `dst`; this may not emit an actual move
                // but ensures that the registers are the same to match x86's read-write operand
                // encoding.
                let dst = output_to_reg(ctx, outputs[0]);
                ctx.emit(Inst::gen_move(dst, lhs, input_ty));

                // Emit the comparison.
                ctx.emit(Inst::xmm_rm_r_imm(op, rhs, dst, imm.encode()));
            }
        }

        Opcode::FallthroughReturn | Opcode::Return => {
            for i in 0..ctx.num_inputs(insn) {
                let src_reg = input_to_reg(ctx, inputs[i]);
                let retval_reg = ctx.retval(i);
                let ty = ctx.input_ty(insn, i);
                ctx.emit(Inst::gen_move(retval_reg, src_reg, ty));
            }
            // N.B.: the Ret itself is generated by the ABI.
        }

        Opcode::Call | Opcode::CallIndirect => {
            let loc = ctx.srcloc(insn);
            let (mut abi, inputs) = match op {
                Opcode::Call => {
                    let (extname, dist) = ctx.call_target(insn).unwrap();
                    let sig = ctx.call_sig(insn).unwrap();
                    assert_eq!(inputs.len(), sig.params.len());
                    assert_eq!(outputs.len(), sig.returns.len());
                    (
                        X64ABICall::from_func(sig, &extname, dist, loc)?,
                        &inputs[..],
                    )
                }

                Opcode::CallIndirect => {
                    let ptr = input_to_reg(ctx, inputs[0]);
                    let sig = ctx.call_sig(insn).unwrap();
                    assert_eq!(inputs.len() - 1, sig.params.len());
                    assert_eq!(outputs.len(), sig.returns.len());
                    (X64ABICall::from_ptr(sig, ptr, loc, op)?, &inputs[1..])
                }

                _ => unreachable!(),
            };

            abi.emit_stack_pre_adjust(ctx);
            assert_eq!(inputs.len(), abi.num_args());
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

        Opcode::Trap | Opcode::ResumableTrap => {
            let trap_info = (ctx.srcloc(insn), inst_trapcode(ctx.data(insn)).unwrap());
            ctx.emit_safepoint(Inst::Ud2 { trap_info });
        }

        Opcode::Trapif | Opcode::Trapff => {
            let srcloc = ctx.srcloc(insn);
            let trap_code = inst_trapcode(ctx.data(insn)).unwrap();

            let cc = if matches_input(ctx, inputs[0], Opcode::IaddIfcout).is_some() {
                let condcode = inst_condcode(ctx.data(insn));
                // The flags must not have been clobbered by any other instruction between the
                // iadd_ifcout and this instruction, as verified by the CLIF validator; so we can
                // simply use the flags here.
                CC::from_intcc(condcode)
            } else if op == Opcode::Trapif {
                let condcode = inst_condcode(ctx.data(insn));
                let cc = CC::from_intcc(condcode);

                // Verification ensures that the input is always a single-def ifcmp.
                let ifcmp_insn = matches_input(ctx, inputs[0], Opcode::Ifcmp).unwrap();
                emit_cmp(ctx, ifcmp_insn);
                cc
            } else {
                let condcode = inst_fp_condcode(ctx.data(insn));
                let cc = CC::from_floatcc(condcode);

                // Verification ensures that the input is always a single-def ffcmp.
                let ffcmp_insn = matches_input(ctx, inputs[0], Opcode::Ffcmp).unwrap();
                emit_fcmp(ctx, ffcmp_insn, FcmpOperands::DontSwap);
                cc
            };

            ctx.emit_safepoint(Inst::TrapIf {
                trap_code,
                srcloc,
                cc,
            });
        }

        Opcode::F64const => {
            // TODO use cmpeqpd for all 1s.
            let value = ctx.get_constant(insn).unwrap();
            let dst = output_to_reg(ctx, outputs[0]);
            for inst in Inst::gen_constant(dst, value, types::F64, |reg_class, ty| {
                ctx.alloc_tmp(reg_class, ty)
            }) {
                ctx.emit(inst);
            }
        }

        Opcode::F32const => {
            // TODO use cmpeqps for all 1s.
            let value = ctx.get_constant(insn).unwrap();
            let dst = output_to_reg(ctx, outputs[0]);
            for inst in Inst::gen_constant(dst, value, types::F32, |reg_class, ty| {
                ctx.alloc_tmp(reg_class, ty)
            }) {
                ctx.emit(inst);
            }
        }

        Opcode::Fadd | Opcode::Fsub | Opcode::Fmul | Opcode::Fdiv => {
            let lhs = input_to_reg(ctx, inputs[0]);
            let rhs = input_to_reg_mem(ctx, inputs[1]);
            let dst = output_to_reg(ctx, outputs[0]);
            let ty = ty.unwrap();

            // Move the `lhs` to the same register as `dst`; this may not emit an actual move
            // but ensures that the registers are the same to match x86's read-write operand
            // encoding.
            ctx.emit(Inst::gen_move(dst, lhs, ty));

            // Note: min and max can't be handled here, because of the way Cranelift defines them:
            // if any operand is a NaN, they must return the NaN operand, while the x86 machine
            // instruction will return the second operand if either operand is a NaN.
            let sse_op = match ty {
                types::F32 => match op {
                    Opcode::Fadd => SseOpcode::Addss,
                    Opcode::Fsub => SseOpcode::Subss,
                    Opcode::Fmul => SseOpcode::Mulss,
                    Opcode::Fdiv => SseOpcode::Divss,
                    _ => unreachable!(),
                },
                types::F64 => match op {
                    Opcode::Fadd => SseOpcode::Addsd,
                    Opcode::Fsub => SseOpcode::Subsd,
                    Opcode::Fmul => SseOpcode::Mulsd,
                    Opcode::Fdiv => SseOpcode::Divsd,
                    _ => unreachable!(),
                },
                types::F32X4 => match op {
                    Opcode::Fadd => SseOpcode::Addps,
                    Opcode::Fsub => SseOpcode::Subps,
                    Opcode::Fmul => SseOpcode::Mulps,
                    Opcode::Fdiv => SseOpcode::Divps,
                    _ => unreachable!(),
                },
                types::F64X2 => match op {
                    Opcode::Fadd => SseOpcode::Addpd,
                    Opcode::Fsub => SseOpcode::Subpd,
                    Opcode::Fmul => SseOpcode::Mulpd,
                    Opcode::Fdiv => SseOpcode::Divpd,
                    _ => unreachable!(),
                },
                _ => panic!(
                    "invalid type: expected one of [F32, F64, F32X4, F64X2], found {}",
                    ty
                ),
            };
            ctx.emit(Inst::xmm_rm_r(sse_op, rhs, dst));
        }

        Opcode::Fmin | Opcode::Fmax => {
            let lhs = input_to_reg(ctx, inputs[0]);
            let rhs = input_to_reg(ctx, inputs[1]);
            let dst = output_to_reg(ctx, outputs[0]);
            let is_min = op == Opcode::Fmin;
            let output_ty = ty.unwrap();
            ctx.emit(Inst::gen_move(dst, rhs, output_ty));
            let op_size = match output_ty {
                types::F32 => OperandSize::Size32,
                types::F64 => OperandSize::Size64,
                _ => panic!("unexpected type {:?} for fmin/fmax", output_ty),
            };
            ctx.emit(Inst::xmm_min_max_seq(op_size, is_min, lhs, dst));
        }

        Opcode::Sqrt => {
            let src = input_to_reg_mem(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);
            let ty = ty.unwrap();

            let sse_op = match ty {
                types::F32 => SseOpcode::Sqrtss,
                types::F64 => SseOpcode::Sqrtsd,
                types::F32X4 => SseOpcode::Sqrtps,
                types::F64X2 => SseOpcode::Sqrtpd,
                _ => panic!(
                    "invalid type: expected one of [F32, F64, F32X4, F64X2], found {}",
                    ty
                ),
            };

            ctx.emit(Inst::xmm_unary_rm_r(sse_op, src, dst));
        }

        Opcode::Fpromote => {
            let src = input_to_reg_mem(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);
            ctx.emit(Inst::xmm_unary_rm_r(SseOpcode::Cvtss2sd, src, dst));
        }

        Opcode::Fdemote => {
            let src = input_to_reg_mem(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);
            ctx.emit(Inst::xmm_unary_rm_r(SseOpcode::Cvtsd2ss, src, dst));
        }

        Opcode::FcvtFromSint => {
            let (ext_spec, src_size) = match ctx.input_ty(insn, 0) {
                types::I8 | types::I16 => (Some(ExtSpec::SignExtendTo32), OperandSize::Size32),
                types::I32 => (None, OperandSize::Size32),
                types::I64 => (None, OperandSize::Size64),
                _ => unreachable!(),
            };

            let src = match ext_spec {
                Some(ext_spec) => RegMem::reg(extend_input_to_reg(ctx, inputs[0], ext_spec)),
                None => input_to_reg_mem(ctx, inputs[0]),
            };

            let output_ty = ty.unwrap();
            let opcode = if output_ty == types::F32 {
                SseOpcode::Cvtsi2ss
            } else {
                assert_eq!(output_ty, types::F64);
                SseOpcode::Cvtsi2sd
            };

            let dst = output_to_reg(ctx, outputs[0]);
            ctx.emit(Inst::gpr_to_xmm(opcode, src, src_size, dst));
        }

        Opcode::FcvtFromUint => {
            let dst = output_to_reg(ctx, outputs[0]);
            let ty = ty.unwrap();

            let input_ty = ctx.input_ty(insn, 0);
            match input_ty {
                types::I8 | types::I16 | types::I32 => {
                    // Conversion from an unsigned int smaller than 64-bit is easy: zero-extend +
                    // do a signed conversion (which won't overflow).
                    let opcode = if ty == types::F32 {
                        SseOpcode::Cvtsi2ss
                    } else {
                        assert_eq!(ty, types::F64);
                        SseOpcode::Cvtsi2sd
                    };

                    let src =
                        RegMem::reg(extend_input_to_reg(ctx, inputs[0], ExtSpec::ZeroExtendTo64));
                    ctx.emit(Inst::gpr_to_xmm(opcode, src, OperandSize::Size64, dst));
                }

                types::I64 => {
                    let src = input_to_reg(ctx, inputs[0]);

                    let src_copy = ctx.alloc_tmp(RegClass::I64, types::I64);
                    ctx.emit(Inst::gen_move(src_copy, src, types::I64));

                    let tmp_gpr1 = ctx.alloc_tmp(RegClass::I64, types::I64);
                    let tmp_gpr2 = ctx.alloc_tmp(RegClass::I64, types::I64);
                    ctx.emit(Inst::cvt_u64_to_float_seq(
                        ty == types::F64,
                        src_copy,
                        tmp_gpr1,
                        tmp_gpr2,
                        dst,
                    ));
                }

                _ => panic!("unexpected input type for FcvtFromUint: {:?}", input_ty),
            };
        }

        Opcode::FcvtToUint | Opcode::FcvtToUintSat | Opcode::FcvtToSint | Opcode::FcvtToSintSat => {
            let src = input_to_reg(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);

            let input_ty = ctx.input_ty(insn, 0);
            let src_size = if input_ty == types::F32 {
                OperandSize::Size32
            } else {
                assert_eq!(input_ty, types::F64);
                OperandSize::Size64
            };

            let output_ty = ty.unwrap();
            let dst_size = if output_ty == types::I32 {
                OperandSize::Size32
            } else {
                assert_eq!(output_ty, types::I64);
                OperandSize::Size64
            };

            let to_signed = op == Opcode::FcvtToSint || op == Opcode::FcvtToSintSat;
            let is_sat = op == Opcode::FcvtToUintSat || op == Opcode::FcvtToSintSat;

            let src_copy = ctx.alloc_tmp(RegClass::V128, input_ty);
            ctx.emit(Inst::gen_move(src_copy, src, input_ty));

            let tmp_xmm = ctx.alloc_tmp(RegClass::V128, input_ty);
            let tmp_gpr = ctx.alloc_tmp(RegClass::I64, output_ty);

            let srcloc = ctx.srcloc(insn);
            if to_signed {
                ctx.emit(Inst::cvt_float_to_sint_seq(
                    src_size, dst_size, is_sat, src_copy, dst, tmp_gpr, tmp_xmm, srcloc,
                ));
            } else {
                ctx.emit(Inst::cvt_float_to_uint_seq(
                    src_size, dst_size, is_sat, src_copy, dst, tmp_gpr, tmp_xmm, srcloc,
                ));
            }
        }

        Opcode::Bitcast => {
            let input_ty = ctx.input_ty(insn, 0);
            let output_ty = ctx.output_ty(insn, 0);
            match (input_ty, output_ty) {
                (types::F32, types::I32) => {
                    let src = input_to_reg(ctx, inputs[0]);
                    let dst = output_to_reg(ctx, outputs[0]);
                    ctx.emit(Inst::xmm_to_gpr(
                        SseOpcode::Movd,
                        src,
                        dst,
                        OperandSize::Size32,
                    ));
                }
                (types::I32, types::F32) => {
                    let src = input_to_reg_mem(ctx, inputs[0]);
                    let dst = output_to_reg(ctx, outputs[0]);
                    ctx.emit(Inst::gpr_to_xmm(
                        SseOpcode::Movd,
                        src,
                        OperandSize::Size32,
                        dst,
                    ));
                }
                (types::F64, types::I64) => {
                    let src = input_to_reg(ctx, inputs[0]);
                    let dst = output_to_reg(ctx, outputs[0]);
                    ctx.emit(Inst::xmm_to_gpr(
                        SseOpcode::Movq,
                        src,
                        dst,
                        OperandSize::Size64,
                    ));
                }
                (types::I64, types::F64) => {
                    let src = input_to_reg_mem(ctx, inputs[0]);
                    let dst = output_to_reg(ctx, outputs[0]);
                    ctx.emit(Inst::gpr_to_xmm(
                        SseOpcode::Movq,
                        src,
                        OperandSize::Size64,
                        dst,
                    ));
                }
                _ => unreachable!("invalid bitcast from {:?} to {:?}", input_ty, output_ty),
            }
        }

        Opcode::Fabs | Opcode::Fneg => {
            let src = input_to_reg_mem(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);

            // In both cases, generate a constant and apply a single binary instruction:
            // - to compute the absolute value, set all bits to 1 but the MSB to 0, and bit-AND the
            // src with it.
            // - to compute the negated value, set all bits to 0 but the MSB to 1, and bit-XOR the
            // src with it.
            let output_ty = ty.unwrap();
            if !output_ty.is_vector() {
                let (val, opcode) = match output_ty {
                    types::F32 => match op {
                        Opcode::Fabs => (0x7fffffff, SseOpcode::Andps),
                        Opcode::Fneg => (0x80000000, SseOpcode::Xorps),
                        _ => unreachable!(),
                    },
                    types::F64 => match op {
                        Opcode::Fabs => (0x7fffffffffffffff, SseOpcode::Andpd),
                        Opcode::Fneg => (0x8000000000000000, SseOpcode::Xorpd),
                        _ => unreachable!(),
                    },
                    _ => panic!("unexpected type {:?} for Fabs", output_ty),
                };

                for inst in Inst::gen_constant(dst, val, output_ty, |reg_class, ty| {
                    ctx.alloc_tmp(reg_class, ty)
                }) {
                    ctx.emit(inst);
                }

                ctx.emit(Inst::xmm_rm_r(opcode, src, dst));
            } else {
                // Eventually vector constants should be available in `gen_constant` and this block
                // can be merged with the one above (TODO).
                if output_ty.bits() == 128 {
                    // Move the `lhs` to the same register as `dst`; this may not emit an actual move
                    // but ensures that the registers are the same to match x86's read-write operand
                    // encoding.
                    let src = input_to_reg(ctx, inputs[0]);
                    ctx.emit(Inst::gen_move(dst, src, output_ty));

                    // Generate an all 1s constant in an XMM register. This uses CMPPS but could
                    // have used CMPPD with the same effect.
                    let tmp = ctx.alloc_tmp(RegClass::V128, output_ty);
                    let cond = FcmpImm::from(FloatCC::Equal);
                    let cmpps = Inst::xmm_rm_r_imm(
                        SseOpcode::Cmpps,
                        RegMem::reg(tmp.to_reg()),
                        tmp,
                        cond.encode(),
                    );
                    ctx.emit(cmpps);

                    // Shift the all 1s constant to generate the mask.
                    let lane_bits = output_ty.lane_bits();
                    let (shift_opcode, opcode, shift_by) = match (op, lane_bits) {
                        (Opcode::Fabs, 32) => (SseOpcode::Psrld, SseOpcode::Andps, 1),
                        (Opcode::Fabs, 64) => (SseOpcode::Psrlq, SseOpcode::Andpd, 1),
                        (Opcode::Fneg, 32) => (SseOpcode::Pslld, SseOpcode::Xorps, 31),
                        (Opcode::Fneg, 64) => (SseOpcode::Psllq, SseOpcode::Xorpd, 63),
                        _ => unreachable!(
                            "unexpected opcode and lane size: {:?}, {} bits",
                            op, lane_bits
                        ),
                    };
                    let shift = Inst::xmm_rmi_reg(shift_opcode, RegMemImm::imm(shift_by), tmp);
                    ctx.emit(shift);

                    // Apply shifted mask (XOR or AND).
                    let mask = Inst::xmm_rm_r(opcode, RegMem::reg(tmp.to_reg()), dst);
                    ctx.emit(mask);
                } else {
                    panic!("unexpected type {:?} for Fabs", output_ty);
                }
            }
        }

        Opcode::Fcopysign => {
            let dst = output_to_reg(ctx, outputs[0]);
            let lhs = input_to_reg(ctx, inputs[0]);
            let rhs = input_to_reg(ctx, inputs[1]);

            let ty = ty.unwrap();

            // We're going to generate the following sequence:
            //
            // movabs     $INT_MIN, tmp_gpr1
            // mov{d,q}   tmp_gpr1, tmp_xmm1
            // movap{s,d} tmp_xmm1, dst
            // andnp{s,d} src_1, dst
            // movap{s,d} src_2, tmp_xmm2
            // andp{s,d}  tmp_xmm1, tmp_xmm2
            // orp{s,d}   tmp_xmm2, dst

            let tmp_xmm1 = ctx.alloc_tmp(RegClass::V128, types::F32);
            let tmp_xmm2 = ctx.alloc_tmp(RegClass::V128, types::F32);

            let (sign_bit_cst, mov_op, and_not_op, and_op, or_op) = match ty {
                types::F32 => (
                    0x8000_0000,
                    SseOpcode::Movaps,
                    SseOpcode::Andnps,
                    SseOpcode::Andps,
                    SseOpcode::Orps,
                ),
                types::F64 => (
                    0x8000_0000_0000_0000,
                    SseOpcode::Movapd,
                    SseOpcode::Andnpd,
                    SseOpcode::Andpd,
                    SseOpcode::Orpd,
                ),
                _ => {
                    panic!("unexpected type {:?} for copysign", ty);
                }
            };

            for inst in Inst::gen_constant(tmp_xmm1, sign_bit_cst, ty, |reg_class, ty| {
                ctx.alloc_tmp(reg_class, ty)
            }) {
                ctx.emit(inst);
            }
            ctx.emit(Inst::xmm_mov(
                mov_op,
                RegMem::reg(tmp_xmm1.to_reg()),
                dst,
                None,
            ));
            ctx.emit(Inst::xmm_rm_r(and_not_op, RegMem::reg(lhs), dst));
            ctx.emit(Inst::xmm_mov(mov_op, RegMem::reg(rhs), tmp_xmm2, None));
            ctx.emit(Inst::xmm_rm_r(
                and_op,
                RegMem::reg(tmp_xmm1.to_reg()),
                tmp_xmm2,
            ));
            ctx.emit(Inst::xmm_rm_r(or_op, RegMem::reg(tmp_xmm2.to_reg()), dst));
        }

        Opcode::Ceil | Opcode::Floor | Opcode::Nearest | Opcode::Trunc => {
            // TODO use ROUNDSS/ROUNDSD after sse4.1.

            // Lower to VM calls when there's no access to SSE4.1.
            let ty = ty.unwrap();
            let libcall = match (ty, op) {
                (types::F32, Opcode::Ceil) => LibCall::CeilF32,
                (types::F64, Opcode::Ceil) => LibCall::CeilF64,
                (types::F32, Opcode::Floor) => LibCall::FloorF32,
                (types::F64, Opcode::Floor) => LibCall::FloorF64,
                (types::F32, Opcode::Nearest) => LibCall::NearestF32,
                (types::F64, Opcode::Nearest) => LibCall::NearestF64,
                (types::F32, Opcode::Trunc) => LibCall::TruncF32,
                (types::F64, Opcode::Trunc) => LibCall::TruncF64,
                _ => panic!(
                    "unexpected type/opcode {:?}/{:?} in Ceil/Floor/Nearest/Trunc",
                    ty, op
                ),
            };

            emit_vm_call(ctx, flags, triple, libcall, insn, inputs, outputs)?;
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

            let addr = match op {
                Opcode::Load
                | Opcode::Uload8
                | Opcode::Sload8
                | Opcode::Uload16
                | Opcode::Sload16
                | Opcode::Uload32
                | Opcode::Sload32 => {
                    assert_eq!(inputs.len(), 1, "only one input for load operands");
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
                    assert_eq!(
                        inputs.len(),
                        2,
                        "can't handle more than two inputs in complex load"
                    );
                    let base = input_to_reg(ctx, inputs[0]);
                    let index = input_to_reg(ctx, inputs[1]);
                    let shift = 0;
                    Amode::imm_reg_reg_shift(offset as u32, base, index, shift)
                }

                _ => unreachable!(),
            };

            let srcloc = Some(ctx.srcloc(insn));

            let dst = output_to_reg(ctx, outputs[0]);
            let is_xmm = elem_ty.is_float() || elem_ty.is_vector();
            match (sign_extend, is_xmm) {
                (true, false) => {
                    // The load is sign-extended only when the output size is lower than 64 bits,
                    // so ext-mode is defined in this case.
                    ctx.emit(Inst::movsx_rm_r(
                        ext_mode.unwrap(),
                        RegMem::mem(addr),
                        dst,
                        srcloc,
                    ));
                }
                (false, false) => {
                    if elem_ty.bytes() == 8 {
                        // Use a plain load.
                        ctx.emit(Inst::mov64_m_r(addr, dst, srcloc))
                    } else {
                        // Use a zero-extended load.
                        ctx.emit(Inst::movzx_rm_r(
                            ext_mode.unwrap(),
                            RegMem::mem(addr),
                            dst,
                            srcloc,
                        ))
                    }
                }
                (_, true) => {
                    ctx.emit(match elem_ty {
                        types::F32 => {
                            Inst::xmm_mov(SseOpcode::Movss, RegMem::mem(addr), dst, srcloc)
                        }
                        types::F64 => {
                            Inst::xmm_mov(SseOpcode::Movsd, RegMem::mem(addr), dst, srcloc)
                        }
                        _ if elem_ty.is_vector() && elem_ty.bits() == 128 => {
                            Inst::xmm_mov(SseOpcode::Movups, RegMem::mem(addr), dst, srcloc)
                        } // TODO Specialize for different types: MOVUPD, MOVDQU
                        _ => unreachable!("unexpected type for load: {:?}", elem_ty),
                    });
                }
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

            let addr = match op {
                Opcode::Store | Opcode::Istore8 | Opcode::Istore16 | Opcode::Istore32 => {
                    assert_eq!(inputs.len(), 2, "only one input for store memory operands");
                    let base = input_to_reg(ctx, inputs[1]);
                    // TODO sign?
                    Amode::imm_reg(offset as u32, base)
                }

                Opcode::StoreComplex
                | Opcode::Istore8Complex
                | Opcode::Istore16Complex
                | Opcode::Istore32Complex => {
                    assert_eq!(
                        inputs.len(),
                        3,
                        "can't handle more than two inputs in complex store"
                    );
                    let base = input_to_reg(ctx, inputs[1]);
                    let index = input_to_reg(ctx, inputs[2]);
                    let shift = 0;
                    Amode::imm_reg_reg_shift(offset as u32, base, index, shift)
                }

                _ => unreachable!(),
            };

            let src = input_to_reg(ctx, inputs[0]);

            let srcloc = Some(ctx.srcloc(insn));

            ctx.emit(match elem_ty {
                types::F32 => Inst::xmm_mov_r_m(SseOpcode::Movss, src, addr, srcloc),
                types::F64 => Inst::xmm_mov_r_m(SseOpcode::Movsd, src, addr, srcloc),
                _ if elem_ty.is_vector() && elem_ty.bits() == 128 => {
                    // TODO Specialize for different types: MOVUPD, MOVDQU, etc.
                    Inst::xmm_mov_r_m(SseOpcode::Movups, src, addr, srcloc)
                }
                _ => Inst::mov_r_m(elem_ty.bytes() as u8, src, addr, srcloc),
            });
        }

        Opcode::AtomicRmw => {
            // This is a simple, general-case atomic update, based on a loop involving
            // `cmpxchg`.  Note that we could do much better than this in the case where the old
            // value at the location (that is to say, the SSA `Value` computed by this CLIF
            // instruction) is not required.  In that case, we could instead implement this
            // using a single `lock`-prefixed x64 read-modify-write instruction.  Also, even in
            // the case where the old value is required, for the `add` and `sub` cases, we can
            // use the single instruction `lock xadd`.  However, those improvements have been
            // left for another day.
            // TODO: filed as https://github.com/bytecodealliance/wasmtime/issues/2153
            let dst = output_to_reg(ctx, outputs[0]);
            let mut addr = input_to_reg(ctx, inputs[0]);
            let mut arg2 = input_to_reg(ctx, inputs[1]);
            let ty_access = ty.unwrap();
            assert!(is_valid_atomic_transaction_ty(ty_access));
            let memflags = ctx.memflags(insn).expect("memory flags");
            let srcloc = if !memflags.notrap() {
                Some(ctx.srcloc(insn))
            } else {
                None
            };
            // Make sure that both args are in virtual regs, since in effect we have to do a
            // parallel copy to get them safely to the AtomicRmwSeq input regs, and that's not
            // guaranteed safe if either is in a real reg.
            addr = ctx.ensure_in_vreg(addr, types::I64);
            arg2 = ctx.ensure_in_vreg(arg2, types::I64);
            // Move the args to the preordained AtomicRMW input regs.  Note that `AtomicRmwSeq`
            // operates at whatever width is specified by `ty`, so there's no need to
            // zero-extend `arg2` in the case of `ty` being I8/I16/I32.
            ctx.emit(Inst::gen_move(
                Writable::from_reg(regs::r9()),
                addr,
                types::I64,
            ));
            ctx.emit(Inst::gen_move(
                Writable::from_reg(regs::r10()),
                arg2,
                types::I64,
            ));
            // Now the AtomicRmwSeq (pseudo-) instruction itself
            let op = inst_common::AtomicRmwOp::from(inst_atomic_rmw_op(ctx.data(insn)).unwrap());
            ctx.emit(Inst::AtomicRmwSeq {
                ty: ty_access,
                op,
                srcloc,
            });
            // And finally, copy the preordained AtomicRmwSeq output reg to its destination.
            ctx.emit(Inst::gen_move(dst, regs::rax(), types::I64));
        }

        Opcode::AtomicCas => {
            // This is very similar to, but not identical to, the `AtomicRmw` case.  As with
            // `AtomicRmw`, there's no need to zero-extend narrow values here.
            let dst = output_to_reg(ctx, outputs[0]);
            let addr = input_to_reg(ctx, inputs[0]);
            let expected = input_to_reg(ctx, inputs[1]);
            let replacement = input_to_reg(ctx, inputs[2]);
            let ty_access = ty.unwrap();
            assert!(is_valid_atomic_transaction_ty(ty_access));
            let memflags = ctx.memflags(insn).expect("memory flags");
            let srcloc = if !memflags.notrap() {
                Some(ctx.srcloc(insn))
            } else {
                None
            };
            // Move the expected value into %rax.  Because there's only one fixed register on
            // the input side, we don't have to use `ensure_in_vreg`, as is necessary in the
            // `AtomicRmw` case.
            ctx.emit(Inst::gen_move(
                Writable::from_reg(regs::rax()),
                expected,
                types::I64,
            ));
            ctx.emit(Inst::LockCmpxchg {
                ty: ty_access,
                src: replacement,
                dst: Amode::imm_reg(0, addr).into(),
                srcloc,
            });
            // And finally, copy the old value at the location to its destination reg.
            ctx.emit(Inst::gen_move(dst, regs::rax(), types::I64));
        }

        Opcode::AtomicLoad => {
            // This is a normal load.  The x86-TSO memory model provides sufficient sequencing
            // to satisfy the CLIF synchronisation requirements for `AtomicLoad` without the
            // need for any fence instructions.
            let data = output_to_reg(ctx, outputs[0]);
            let addr = input_to_reg(ctx, inputs[0]);
            let ty_access = ty.unwrap();
            assert!(is_valid_atomic_transaction_ty(ty_access));
            let memflags = ctx.memflags(insn).expect("memory flags");
            let srcloc = if !memflags.notrap() {
                Some(ctx.srcloc(insn))
            } else {
                None
            };
            // For the amode, we could do better, but for now just use `0(addr)`.
            let rm = RegMem::mem(Amode::imm_reg(0, addr));
            if ty_access == types::I64 {
                ctx.emit(Inst::mov64_rm_r(rm, data, srcloc));
            } else {
                let ext_mode = match ty_access {
                    types::I8 => ExtMode::BQ,
                    types::I16 => ExtMode::WQ,
                    types::I32 => ExtMode::LQ,
                    _ => panic!("lowering AtomicLoad: invalid type"),
                };
                ctx.emit(Inst::movzx_rm_r(ext_mode, rm, data, srcloc));
            }
        }

        Opcode::AtomicStore => {
            // This is a normal store, followed by an `mfence` instruction.
            let data = input_to_reg(ctx, inputs[0]);
            let addr = input_to_reg(ctx, inputs[1]);
            let ty_access = ctx.input_ty(insn, 0);
            assert!(is_valid_atomic_transaction_ty(ty_access));
            let memflags = ctx.memflags(insn).expect("memory flags");
            let srcloc = if !memflags.notrap() {
                Some(ctx.srcloc(insn))
            } else {
                None
            };
            // For the amode, we could do better, but for now just use `0(addr)`.
            ctx.emit(Inst::mov_r_m(
                ty_access.bytes() as u8,
                data,
                Amode::imm_reg(0, addr),
                srcloc,
            ));
            ctx.emit(Inst::Fence {
                kind: FenceKind::MFence,
            });
        }

        Opcode::Fence => {
            ctx.emit(Inst::Fence {
                kind: FenceKind::MFence,
            });
        }

        Opcode::FuncAddr => {
            let dst = output_to_reg(ctx, outputs[0]);
            let (extname, _) = ctx.call_target(insn).unwrap();
            let extname = extname.clone();
            let loc = ctx.srcloc(insn);
            ctx.emit(Inst::LoadExtName {
                dst,
                name: Box::new(extname),
                srcloc: loc,
                offset: 0,
            });
        }

        Opcode::SymbolValue => {
            let dst = output_to_reg(ctx, outputs[0]);
            let (extname, _, offset) = ctx.symbol_value(insn).unwrap();
            let extname = extname.clone();
            let loc = ctx.srcloc(insn);
            ctx.emit(Inst::LoadExtName {
                dst,
                name: Box::new(extname),
                srcloc: loc,
                offset,
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
            let dst = output_to_reg(ctx, outputs[0]);
            let offset: i32 = offset.into();
            let inst = ctx
                .abi()
                .stackslot_addr(stack_slot, u32::try_from(offset).unwrap(), dst);
            ctx.emit(inst);
        }

        Opcode::Select => {
            let flag_input = inputs[0];
            if let Some(fcmp) = matches_input(ctx, flag_input, Opcode::Fcmp) {
                let cond_code = inst_fp_condcode(ctx.data(fcmp));

                // See comments in the lowering of Fcmp.
                let (cond_code, swap_op, was_equal) = match cond_code {
                    FloatCC::LessThan
                    | FloatCC::LessThanOrEqual
                    | FloatCC::UnorderedOrGreaterThan
                    | FloatCC::UnorderedOrGreaterThanOrEqual => {
                        (cond_code.reverse(), FcmpOperands::Swap, false)
                    }
                    FloatCC::Equal => {
                        // Additionally, we invert Equal to NotEqual too: taking LHS if equal would
                        // mean take it if both CC::NP and CC::Z are set, the conjunction of which
                        // can't be modeled with a single cmov instruction. Instead, we'll swap LHS
                        // and RHS in the select operation, and invert the equal to a not-equal
                        // here.
                        (FloatCC::NotEqual, FcmpOperands::DontSwap, true)
                    }
                    _ => (cond_code, FcmpOperands::DontSwap, false),
                };
                emit_fcmp(ctx, fcmp, swap_op);

                let (lhs, rhs) = if was_equal {
                    // See comment above about inverting conditional code.
                    (
                        input_to_reg_mem(ctx, inputs[2]),
                        input_to_reg(ctx, inputs[1]),
                    )
                } else {
                    (
                        input_to_reg_mem(ctx, inputs[1]),
                        input_to_reg(ctx, inputs[2]),
                    )
                };

                let dst = output_to_reg(ctx, outputs[0]);

                let ty = ctx.output_ty(insn, 0);

                let lhs = if is_int_ty(ty) {
                    let size = ty.bytes() as u8;
                    if size == 1 {
                        // Sign-extend operands to 32, then do a cmove of size 4.
                        let lhs_se = ctx.alloc_tmp(RegClass::I64, types::I32);
                        ctx.emit(Inst::movsx_rm_r(ExtMode::BL, lhs, lhs_se, None));
                        ctx.emit(Inst::movsx_rm_r(ExtMode::BL, RegMem::reg(rhs), dst, None));
                        RegMem::reg(lhs_se.to_reg())
                    } else {
                        ctx.emit(Inst::gen_move(dst, rhs, ty));
                        lhs
                    }
                } else {
                    debug_assert!(ty == types::F32 || ty == types::F64);
                    ctx.emit(Inst::gen_move(dst, rhs, ty));
                    lhs
                };

                match cond_code {
                    FloatCC::Equal => {
                        // See comment above about inverting conditional code.
                        panic!("can't happen because of above guard");
                    }

                    FloatCC::NotEqual => {
                        // Take lhs if not-equal, that is CC::P or CC:NZ.
                        if is_int_ty(ty) {
                            let size = u8::max(ty.bytes() as u8, 4);
                            ctx.emit(Inst::cmove(size, CC::P, lhs.clone(), dst));
                            ctx.emit(Inst::cmove(size, CC::NZ, lhs, dst));
                        } else {
                            ctx.emit(Inst::xmm_cmove(ty == types::F64, CC::P, lhs.clone(), dst));
                            ctx.emit(Inst::xmm_cmove(ty == types::F64, CC::NZ, lhs, dst));
                        }
                    }

                    _ => {
                        let cc = CC::from_floatcc(cond_code);
                        if is_int_ty(ty) {
                            let size = u8::max(ty.bytes() as u8, 4);
                            ctx.emit(Inst::cmove(size, cc, lhs, dst));
                        } else {
                            ctx.emit(Inst::xmm_cmove(ty == types::F64, cc, lhs, dst));
                        }
                    }
                }
            } else {
                let cc = if let Some(icmp) = matches_input(ctx, flag_input, Opcode::Icmp) {
                    emit_cmp(ctx, icmp);
                    let cond_code = inst_condcode(ctx.data(icmp));
                    CC::from_intcc(cond_code)
                } else {
                    // The input is a boolean value, compare it against zero.
                    let size = ctx.input_ty(insn, 0).bytes() as u8;
                    let test = input_to_reg(ctx, inputs[0]);
                    ctx.emit(Inst::cmp_rmi_r(size, RegMemImm::imm(0), test));
                    CC::NZ
                };

                let lhs = input_to_reg_mem(ctx, inputs[1]);
                let rhs = input_to_reg(ctx, inputs[2]);
                let dst = output_to_reg(ctx, outputs[0]);

                let ty = ctx.output_ty(insn, 0);

                if is_int_ty(ty) {
                    let size = ty.bytes() as u8;
                    if size == 1 {
                        // Sign-extend operands to 32, then do a cmove of size 4.
                        let lhs_se = ctx.alloc_tmp(RegClass::I64, types::I32);
                        ctx.emit(Inst::movsx_rm_r(ExtMode::BL, lhs, lhs_se, None));
                        ctx.emit(Inst::movsx_rm_r(ExtMode::BL, RegMem::reg(rhs), dst, None));
                        ctx.emit(Inst::cmove(4, cc, RegMem::reg(lhs_se.to_reg()), dst));
                    } else {
                        ctx.emit(Inst::gen_move(dst, rhs, ty));
                        ctx.emit(Inst::cmove(size, cc, lhs, dst));
                    }
                } else {
                    debug_assert!(ty == types::F32 || ty == types::F64);
                    ctx.emit(Inst::gen_move(dst, rhs, ty));
                    ctx.emit(Inst::xmm_cmove(ty == types::F64, cc, lhs, dst));
                }
            }
        }

        Opcode::Selectif | Opcode::SelectifSpectreGuard => {
            // Verification ensures that the input is always a single-def ifcmp.
            let cmp_insn = ctx
                .get_input(inputs[0].insn, inputs[0].input)
                .inst
                .unwrap()
                .0;
            debug_assert_eq!(ctx.data(cmp_insn).opcode(), Opcode::Ifcmp);
            emit_cmp(ctx, cmp_insn);

            let cc = CC::from_intcc(inst_condcode(ctx.data(insn)));

            let lhs = input_to_reg_mem(ctx, inputs[1]);
            let rhs = input_to_reg(ctx, inputs[2]);
            let dst = output_to_reg(ctx, outputs[0]);

            let ty = ctx.output_ty(insn, 0);

            if is_int_ty(ty) {
                let size = ty.bytes() as u8;
                if size == 1 {
                    // Sign-extend operands to 32, then do a cmove of size 4.
                    let lhs_se = ctx.alloc_tmp(RegClass::I64, types::I32);
                    ctx.emit(Inst::movsx_rm_r(ExtMode::BL, lhs, lhs_se, None));
                    ctx.emit(Inst::movsx_rm_r(ExtMode::BL, RegMem::reg(rhs), dst, None));
                    ctx.emit(Inst::cmove(4, cc, RegMem::reg(lhs_se.to_reg()), dst));
                } else {
                    ctx.emit(Inst::gen_move(dst, rhs, ty));
                    ctx.emit(Inst::cmove(size, cc, lhs, dst));
                }
            } else {
                debug_assert!(ty == types::F32 || ty == types::F64);
                ctx.emit(Inst::gen_move(dst, rhs, ty));
                ctx.emit(Inst::xmm_cmove(ty == types::F64, cc, lhs, dst));
            }
        }

        Opcode::Udiv | Opcode::Urem | Opcode::Sdiv | Opcode::Srem => {
            let kind = match op {
                Opcode::Udiv => DivOrRemKind::UnsignedDiv,
                Opcode::Sdiv => DivOrRemKind::SignedDiv,
                Opcode::Urem => DivOrRemKind::UnsignedRem,
                Opcode::Srem => DivOrRemKind::SignedRem,
                _ => unreachable!(),
            };
            let is_div = kind.is_div();

            let input_ty = ctx.input_ty(insn, 0);
            let size = input_ty.bytes() as u8;

            let dividend = input_to_reg(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);

            let srcloc = ctx.srcloc(insn);
            ctx.emit(Inst::gen_move(
                Writable::from_reg(regs::rax()),
                dividend,
                input_ty,
            ));

            if flags.avoid_div_traps() {
                // A vcode meta-instruction is used to lower the inline checks, since they embed
                // pc-relative offsets that must not change, thus requiring regalloc to not
                // interfere by introducing spills and reloads.
                //
                // Note it keeps the result in $rax (for divide) or $rdx (for rem), so that
                // regalloc is aware of the coalescing opportunity between rax/rdx and the
                // destination register.
                let divisor = input_to_reg(ctx, inputs[1]);

                let divisor_copy = ctx.alloc_tmp(RegClass::I64, types::I64);
                ctx.emit(Inst::gen_move(divisor_copy, divisor, types::I64));

                let tmp = if op == Opcode::Sdiv && size == 8 {
                    Some(ctx.alloc_tmp(RegClass::I64, types::I64))
                } else {
                    None
                };
                ctx.emit(Inst::imm_r(true, 0, Writable::from_reg(regs::rdx())));
                ctx.emit(Inst::checked_div_or_rem_seq(
                    kind,
                    size,
                    divisor_copy,
                    tmp,
                    srcloc,
                ));
            } else {
                let divisor = input_to_reg_mem(ctx, inputs[1]);

                // Fill in the high parts:
                if kind.is_signed() {
                    // sign-extend the sign-bit of rax into rdx, for signed opcodes.
                    ctx.emit(Inst::sign_extend_rax_to_rdx(size));
                } else {
                    // zero for unsigned opcodes.
                    ctx.emit(Inst::imm_r(
                        true, /* is_64 */
                        0,
                        Writable::from_reg(regs::rdx()),
                    ));
                }

                // Emit the actual idiv.
                ctx.emit(Inst::div(size, kind.is_signed(), divisor, ctx.srcloc(insn)));
            }

            // Move the result back into the destination reg.
            if is_div {
                // The quotient is in rax.
                ctx.emit(Inst::gen_move(dst, regs::rax(), input_ty));
            } else {
                // The remainder is in rdx.
                ctx.emit(Inst::gen_move(dst, regs::rdx(), input_ty));
            }
        }

        Opcode::Umulhi | Opcode::Smulhi => {
            let input_ty = ctx.input_ty(insn, 0);
            let size = input_ty.bytes() as u8;

            let lhs = input_to_reg(ctx, inputs[0]);
            let rhs = input_to_reg_mem(ctx, inputs[1]);
            let dst = output_to_reg(ctx, outputs[0]);

            // Move lhs in %rax.
            ctx.emit(Inst::gen_move(
                Writable::from_reg(regs::rax()),
                lhs,
                input_ty,
            ));

            // Emit the actual mul or imul.
            let signed = op == Opcode::Smulhi;
            ctx.emit(Inst::mul_hi(size, signed, rhs));

            // Read the result from the high part (stored in %rdx).
            ctx.emit(Inst::gen_move(dst, regs::rdx(), input_ty));
        }

        Opcode::GetPinnedReg => {
            let dst = output_to_reg(ctx, outputs[0]);
            ctx.emit(Inst::gen_move(dst, regs::pinned_reg(), types::I64));
        }

        Opcode::SetPinnedReg => {
            let src = input_to_reg(ctx, inputs[0]);
            ctx.emit(Inst::gen_move(
                Writable::from_reg(regs::pinned_reg()),
                src,
                types::I64,
            ));
        }

        Opcode::Vconst => {
            let val = if let &InstructionData::UnaryConst {
                constant_handle, ..
            } = ctx.data(insn)
            {
                ctx.get_constant_data(constant_handle).clone().into_vec()
            } else {
                unreachable!("vconst should always have unary_const format")
            };
            let dst = output_to_reg(ctx, outputs[0]);
            let ty = ty.unwrap();
            ctx.emit(Inst::xmm_load_const_seq(val, dst, ty));
        }

        Opcode::RawBitcast => {
            // A raw_bitcast is just a mechanism for correcting the type of V128 values (see
            // https://github.com/bytecodealliance/wasmtime/issues/1147). As such, this IR
            // instruction should emit no machine code but a move is necessary to give the register
            // allocator a definition for the output virtual register.
            let src = input_to_reg(ctx, inputs[0]);
            let dst = output_to_reg(ctx, outputs[0]);
            let ty = ty.unwrap();
            ctx.emit(Inst::gen_move(dst, src, ty));
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
        lower_insn_to_regs(ctx, ir_inst, &self.flags, &self.triple)
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
                    let flag_input = InsnInput {
                        insn: branches[0],
                        input: 0,
                    };

                    let src_ty = ctx.input_ty(branches[0], 0);

                    if let Some(icmp) = matches_input(ctx, flag_input, Opcode::Icmp) {
                        emit_cmp(ctx, icmp);

                        let cond_code = inst_condcode(ctx.data(icmp));
                        let cond_code = if op0 == Opcode::Brz {
                            cond_code.inverse()
                        } else {
                            cond_code
                        };
                        let cc = CC::from_intcc(cond_code);
                        ctx.emit(Inst::jmp_cond(cc, taken, not_taken));
                    } else if let Some(fcmp) = matches_input(ctx, flag_input, Opcode::Fcmp) {
                        let cond_code = inst_fp_condcode(ctx.data(fcmp));

                        let cond_code = if op0 == Opcode::Brz {
                            cond_code.inverse()
                        } else {
                            cond_code
                        };

                        // See comments in the lowering of Fcmp.
                        let (cond_code, swap_op) = match cond_code {
                            FloatCC::LessThan
                            | FloatCC::LessThanOrEqual
                            | FloatCC::UnorderedOrGreaterThan
                            | FloatCC::UnorderedOrGreaterThanOrEqual => {
                                (cond_code.reverse(), FcmpOperands::Swap)
                            }
                            _ => (cond_code, FcmpOperands::DontSwap),
                        };
                        emit_fcmp(ctx, fcmp, swap_op);

                        match cond_code {
                            FloatCC::Equal => {
                                // Jump to taken if CC::NP and CC::Z, that is, jump to not-taken if
                                // CC::P or CC::NZ.
                                ctx.emit(Inst::jmp_if(CC::P, not_taken));
                                ctx.emit(Inst::jmp_cond(CC::NZ, not_taken, taken));
                            }

                            FloatCC::NotEqual => {
                                // Jump to taken if CC::P or CC::NZ.
                                ctx.emit(Inst::jmp_if(CC::P, taken));
                                ctx.emit(Inst::jmp_cond(CC::NZ, taken, not_taken));
                            }

                            _ => {
                                let cc = CC::from_floatcc(cond_code);
                                ctx.emit(Inst::jmp_cond(cc, taken, not_taken));
                            }
                        }
                    } else if is_int_ty(src_ty) || is_bool_ty(src_ty) {
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

                _ => panic!("unexpected branch opcode: {:?}", op0),
            }
        } else {
            assert_eq!(branches.len(), 1);

            // Must be an unconditional branch or trap.
            let op = ctx.data(branches[0]).opcode();
            match op {
                Opcode::Jump | Opcode::Fallthrough => {
                    ctx.emit(Inst::jmp_known(BranchTarget::Label(targets[0])));
                }

                Opcode::BrTable => {
                    let jt_size = targets.len() - 1;
                    assert!(jt_size <= u32::max_value() as usize);
                    let jt_size = jt_size as u32;

                    let idx = extend_input_to_reg(
                        ctx,
                        InsnInput {
                            insn: branches[0],
                            input: 0,
                        },
                        ExtSpec::ZeroExtendTo32,
                    );

                    // Bounds-check (compute flags from idx - jt_size) and branch to default.
                    ctx.emit(Inst::cmp_rmi_r(4, RegMemImm::imm(jt_size), idx));

                    // Emit the compound instruction that does:
                    //
                    // lea $jt, %rA
                    // movsbl [%rA, %rIndex, 2], %rB
                    // add %rB, %rA
                    // j *%rA
                    // [jt entries]
                    //
                    // This must be *one* instruction in the vcode because we cannot allow regalloc
                    // to insert any spills/fills in the middle of the sequence; otherwise, the
                    // lea PC-rel offset to the jumptable would be incorrect.  (The alternative
                    // is to introduce a relocation pass for inlined jumptables, which is much
                    // worse.)

                    // This temporary is used as a signed integer of 64-bits (to hold addresses).
                    let tmp1 = ctx.alloc_tmp(RegClass::I64, types::I64);
                    // This temporary is used as a signed integer of 32-bits (for the wasm-table
                    // index) and then 64-bits (address addend). The small lie about the I64 type
                    // is benign, since the temporary is dead after this instruction (and its
                    // Cranelift type is thus unused).
                    let tmp2 = ctx.alloc_tmp(RegClass::I64, types::I64);

                    let targets_for_term: Vec<MachLabel> = targets.to_vec();
                    let default_target = BranchTarget::Label(targets[0]);

                    let jt_targets: Vec<BranchTarget> = targets
                        .iter()
                        .skip(1)
                        .map(|bix| BranchTarget::Label(*bix))
                        .collect();

                    ctx.emit(Inst::JmpTableSeq {
                        idx,
                        tmp1,
                        tmp2,
                        default_target,
                        targets: jt_targets,
                        targets_for_term,
                    });
                }

                _ => panic!("Unknown branch type {:?}", op),
            }
        }

        Ok(())
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        Some(regs::pinned_reg())
    }
}
