//! Lowering rules for AArch64.
//!
//! TODO: opportunities for better code generation:
//!
//! - Smarter use of addressing modes. Recognize a+SCALE*b patterns; recognize
//!   and incorporate sign/zero extension on indices. Recognize pre/post-index
//!   opportunities.
//!
//! - Floating-point immediates (FIMM instruction).

use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::types::*;
use crate::ir::Inst as IRInst;
use crate::ir::{InstructionData, Opcode, TrapCode, Type};
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::{CodegenError, CodegenResult};

use crate::isa::aarch64::inst::*;
use crate::isa::aarch64::AArch64Backend;

use super::lower_inst;

use log::debug;
use regalloc::{Reg, RegClass, Writable};

//============================================================================
// Result enum types.
//
// Lowering of a given value results in one of these enums, depending on the
// modes in which we can accept the value.

/// A lowering result: register, register-shift.  An SSA value can always be
/// lowered into one of these options; the register form is the fallback.
#[derive(Clone, Debug)]
enum ResultRS {
    Reg(Reg),
    RegShift(Reg, ShiftOpAndAmt),
}

/// A lowering result: register, register-shift, register-extend.  An SSA value can always be
/// lowered into one of these options; the register form is the fallback.
#[derive(Clone, Debug)]
enum ResultRSE {
    Reg(Reg),
    RegShift(Reg, ShiftOpAndAmt),
    RegExtend(Reg, ExtendOp),
}

impl ResultRSE {
    fn from_rs(rs: ResultRS) -> ResultRSE {
        match rs {
            ResultRS::Reg(r) => ResultRSE::Reg(r),
            ResultRS::RegShift(r, s) => ResultRSE::RegShift(r, s),
        }
    }
}

/// A lowering result: register, register-shift, register-extend, or 12-bit immediate form.
/// An SSA value can always be lowered into one of these options; the register form is the
/// fallback.
#[derive(Clone, Debug)]
pub(crate) enum ResultRSEImm12 {
    Reg(Reg),
    RegShift(Reg, ShiftOpAndAmt),
    RegExtend(Reg, ExtendOp),
    Imm12(Imm12),
}

impl ResultRSEImm12 {
    fn from_rse(rse: ResultRSE) -> ResultRSEImm12 {
        match rse {
            ResultRSE::Reg(r) => ResultRSEImm12::Reg(r),
            ResultRSE::RegShift(r, s) => ResultRSEImm12::RegShift(r, s),
            ResultRSE::RegExtend(r, e) => ResultRSEImm12::RegExtend(r, e),
        }
    }
}

/// A lowering result: register, register-shift, or logical immediate form.
/// An SSA value can always be lowered into one of these options; the register form is the
/// fallback.
#[derive(Clone, Debug)]
pub(crate) enum ResultRSImmLogic {
    Reg(Reg),
    RegShift(Reg, ShiftOpAndAmt),
    ImmLogic(ImmLogic),
}

impl ResultRSImmLogic {
    fn from_rs(rse: ResultRS) -> ResultRSImmLogic {
        match rse {
            ResultRS::Reg(r) => ResultRSImmLogic::Reg(r),
            ResultRS::RegShift(r, s) => ResultRSImmLogic::RegShift(r, s),
        }
    }
}

/// A lowering result: register or immediate shift amount (arg to a shift op).
/// An SSA value can always be lowered into one of these options; the register form is the
/// fallback.
#[derive(Clone, Debug)]
pub(crate) enum ResultRegImmShift {
    Reg(Reg),
    ImmShift(ImmShift),
}

//============================================================================
// Instruction input "slots".
//
// We use these types to refer to operand numbers, and result numbers, together
// with the associated instruction, in a type-safe way.

/// Identifier for a particular input of an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InsnInput {
    pub(crate) insn: IRInst,
    pub(crate) input: usize,
}

/// Identifier for a particular output of an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InsnOutput {
    pub(crate) insn: IRInst,
    pub(crate) output: usize,
}

//============================================================================
// Lowering: convert instruction inputs to forms that we can use.

/// Lower an instruction input to a 64-bit constant, if possible.
pub(crate) fn input_to_const<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Option<u64> {
    let input = ctx.get_input(input.insn, input.input);
    input.constant
}

/// Lower an instruction input to a constant register-shift amount, if possible.
pub(crate) fn input_to_shiftimm<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
) -> Option<ShiftOpShiftImm> {
    input_to_const(ctx, input).and_then(ShiftOpShiftImm::maybe_from_shift)
}

pub(crate) fn output_to_const_f128<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    out: InsnOutput,
) -> Option<u128> {
    if out.output > 0 {
        None
    } else {
        let inst_data = ctx.data(out.insn);

        match inst_data {
            &InstructionData::UnaryConst {
                opcode: _,
                constant_handle,
            } => {
                let mut bytes = [0u8; 16];
                let c = ctx.get_constant_data(constant_handle).clone().into_vec();
                assert_eq!(c.len(), 16);
                bytes.copy_from_slice(&c);
                Some(u128::from_le_bytes(bytes))
            }
            _ => None,
        }
    }
}

/// How to handle narrow values loaded into registers; see note on `narrow_mode`
/// parameter to `put_input_in_*` below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NarrowValueMode {
    None,
    /// Zero-extend to 32 bits if original is < 32 bits.
    ZeroExtend32,
    /// Sign-extend to 32 bits if original is < 32 bits.
    SignExtend32,
    /// Zero-extend to 64 bits if original is < 64 bits.
    ZeroExtend64,
    /// Sign-extend to 64 bits if original is < 64 bits.
    SignExtend64,
}

impl NarrowValueMode {
    fn is_32bit(&self) -> bool {
        match self {
            NarrowValueMode::None => false,
            NarrowValueMode::ZeroExtend32 | NarrowValueMode::SignExtend32 => true,
            NarrowValueMode::ZeroExtend64 | NarrowValueMode::SignExtend64 => false,
        }
    }
}

/// Allocate a register for an instruction output and return it.
pub(crate) fn get_output_reg<C: LowerCtx<I = Inst>>(ctx: &mut C, out: InsnOutput) -> Writable<Reg> {
    ctx.get_output(out.insn, out.output)
}

/// Lower an instruction input to a reg.
///
/// The given register will be extended appropriately, according to
/// `narrow_mode` and the input's type. If extended, the value is
/// always extended to 64 bits, for simplicity.
pub(crate) fn put_input_in_reg<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
    narrow_mode: NarrowValueMode,
) -> Reg {
    debug!("put_input_in_reg: input {:?}", input);
    let ty = ctx.input_ty(input.insn, input.input);
    let from_bits = ty_bits(ty) as u8;
    let inputs = ctx.get_input(input.insn, input.input);
    let in_reg = if let Some(c) = inputs.constant {
        let masked = if from_bits < 64 {
            c & ((1u64 << from_bits) - 1)
        } else {
            c
        };
        // Generate constants fresh at each use to minimize long-range register pressure.
        let to_reg = ctx.alloc_tmp(Inst::rc_for_type(ty).unwrap(), ty);
        for inst in Inst::gen_constant(to_reg, masked, ty).into_iter() {
            ctx.emit(inst);
        }
        to_reg.to_reg()
    } else {
        ctx.use_input_reg(inputs);
        inputs.reg
    };

    match (narrow_mode, from_bits) {
        (NarrowValueMode::None, _) => in_reg,
        (NarrowValueMode::ZeroExtend32, n) if n < 32 => {
            let tmp = ctx.alloc_tmp(RegClass::I64, I32);
            ctx.emit(Inst::Extend {
                rd: tmp,
                rn: in_reg,
                signed: false,
                from_bits,
                to_bits: 32,
            });
            tmp.to_reg()
        }
        (NarrowValueMode::SignExtend32, n) if n < 32 => {
            let tmp = ctx.alloc_tmp(RegClass::I64, I32);
            ctx.emit(Inst::Extend {
                rd: tmp,
                rn: in_reg,
                signed: true,
                from_bits,
                to_bits: 32,
            });
            tmp.to_reg()
        }
        (NarrowValueMode::ZeroExtend32, 32) | (NarrowValueMode::SignExtend32, 32) => in_reg,

        (NarrowValueMode::ZeroExtend64, n) if n < 64 => {
            if inputs.constant.is_some() {
                // Constants are zero-extended to full 64-bit width on load already.
                in_reg
            } else {
                let tmp = ctx.alloc_tmp(RegClass::I64, I32);
                ctx.emit(Inst::Extend {
                    rd: tmp,
                    rn: in_reg,
                    signed: false,
                    from_bits,
                    to_bits: 64,
                });
                tmp.to_reg()
            }
        }
        (NarrowValueMode::SignExtend64, n) if n < 64 => {
            let tmp = ctx.alloc_tmp(RegClass::I64, I32);
            ctx.emit(Inst::Extend {
                rd: tmp,
                rn: in_reg,
                signed: true,
                from_bits,
                to_bits: 64,
            });
            tmp.to_reg()
        }
        (_, 64) => in_reg,
        (_, 128) => in_reg,

        _ => panic!(
            "Unsupported input width: input ty {} bits {} mode {:?}",
            ty, from_bits, narrow_mode
        ),
    }
}

/// Lower an instruction input to a reg or reg/shift, or reg/extend operand.
///
/// The `narrow_mode` flag indicates whether the consumer of this value needs
/// the high bits clear. For many operations, such as an add/sub/mul or any
/// bitwise logical operation, the low-bit results depend only on the low-bit
/// inputs, so e.g. we can do an 8 bit add on 32 bit registers where the 8-bit
/// value is stored in the low 8 bits of the register and the high 24 bits are
/// undefined. If the op truly needs the high N bits clear (such as for a
/// divide or a right-shift or a compare-to-zero), `narrow_mode` should be
/// set to `ZeroExtend` or `SignExtend` as appropriate, and the resulting
/// register will be provided the extended value.
fn put_input_in_rs<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
    narrow_mode: NarrowValueMode,
) -> ResultRS {
    let inputs = ctx.get_input(input.insn, input.input);
    if let Some((insn, 0)) = inputs.inst {
        let op = ctx.data(insn).opcode();

        if op == Opcode::Ishl {
            let shiftee = InsnInput { insn, input: 0 };
            let shift_amt = InsnInput { insn, input: 1 };

            // Can we get the shift amount as an immediate?
            if let Some(shiftimm) = input_to_shiftimm(ctx, shift_amt) {
                let reg = put_input_in_reg(ctx, shiftee, narrow_mode);
                return ResultRS::RegShift(reg, ShiftOpAndAmt::new(ShiftOp::LSL, shiftimm));
            }
        }
    }

    ResultRS::Reg(put_input_in_reg(ctx, input, narrow_mode))
}

/// Lower an instruction input to a reg or reg/shift, or reg/extend operand.
/// This does not actually codegen the source instruction; it just uses the
/// vreg into which the source instruction will generate its value.
///
/// See note on `put_input_in_rs` for a description of `narrow_mode`.
fn put_input_in_rse<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
    narrow_mode: NarrowValueMode,
) -> ResultRSE {
    let inputs = ctx.get_input(input.insn, input.input);
    if let Some((insn, 0)) = inputs.inst {
        let op = ctx.data(insn).opcode();
        let out_ty = ctx.output_ty(insn, 0);
        let out_bits = ty_bits(out_ty);

        // If `out_ty` is smaller than 32 bits and we need to zero- or sign-extend,
        // then get the result into a register and return an Extend-mode operand on
        // that register.
        if narrow_mode != NarrowValueMode::None
            && ((narrow_mode.is_32bit() && out_bits < 32)
                || (!narrow_mode.is_32bit() && out_bits < 64))
        {
            let reg = put_input_in_reg(ctx, InsnInput { insn, input: 0 }, NarrowValueMode::None);
            let extendop = match (narrow_mode, out_bits) {
                (NarrowValueMode::SignExtend32, 1) | (NarrowValueMode::SignExtend64, 1) => {
                    ExtendOp::SXTB
                }
                (NarrowValueMode::ZeroExtend32, 1) | (NarrowValueMode::ZeroExtend64, 1) => {
                    ExtendOp::UXTB
                }
                (NarrowValueMode::SignExtend32, 8) | (NarrowValueMode::SignExtend64, 8) => {
                    ExtendOp::SXTB
                }
                (NarrowValueMode::ZeroExtend32, 8) | (NarrowValueMode::ZeroExtend64, 8) => {
                    ExtendOp::UXTB
                }
                (NarrowValueMode::SignExtend32, 16) | (NarrowValueMode::SignExtend64, 16) => {
                    ExtendOp::SXTH
                }
                (NarrowValueMode::ZeroExtend32, 16) | (NarrowValueMode::ZeroExtend64, 16) => {
                    ExtendOp::UXTH
                }
                (NarrowValueMode::SignExtend64, 32) => ExtendOp::SXTW,
                (NarrowValueMode::ZeroExtend64, 32) => ExtendOp::UXTW,
                _ => unreachable!(),
            };
            return ResultRSE::RegExtend(reg, extendop);
        }

        // Is this a zero-extend or sign-extend and can we handle that with a register-mode operator?
        if op == Opcode::Uextend || op == Opcode::Sextend {
            assert!(out_bits == 32 || out_bits == 64);
            let sign_extend = op == Opcode::Sextend;
            let inner_ty = ctx.input_ty(insn, 0);
            let inner_bits = ty_bits(inner_ty);
            assert!(inner_bits < out_bits);
            let extendop = match (sign_extend, inner_bits) {
                (true, 1) => ExtendOp::SXTB,
                (false, 1) => ExtendOp::UXTB,
                (true, 8) => ExtendOp::SXTB,
                (false, 8) => ExtendOp::UXTB,
                (true, 16) => ExtendOp::SXTH,
                (false, 16) => ExtendOp::UXTH,
                (true, 32) => ExtendOp::SXTW,
                (false, 32) => ExtendOp::UXTW,
                _ => unreachable!(),
            };
            let reg = put_input_in_reg(ctx, InsnInput { insn, input: 0 }, NarrowValueMode::None);
            return ResultRSE::RegExtend(reg, extendop);
        }
    }

    ResultRSE::from_rs(put_input_in_rs(ctx, input, narrow_mode))
}

pub(crate) fn put_input_in_rse_imm12<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
    narrow_mode: NarrowValueMode,
) -> ResultRSEImm12 {
    if let Some(imm_value) = input_to_const(ctx, input) {
        if let Some(i) = Imm12::maybe_from_u64(imm_value) {
            return ResultRSEImm12::Imm12(i);
        }
    }

    ResultRSEImm12::from_rse(put_input_in_rse(ctx, input, narrow_mode))
}

pub(crate) fn put_input_in_rs_immlogic<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
    narrow_mode: NarrowValueMode,
) -> ResultRSImmLogic {
    if let Some(imm_value) = input_to_const(ctx, input) {
        let ty = ctx.input_ty(input.insn, input.input);
        let ty = if ty_bits(ty) < 32 { I32 } else { ty };
        if let Some(i) = ImmLogic::maybe_from_u64(imm_value, ty) {
            return ResultRSImmLogic::ImmLogic(i);
        }
    }

    ResultRSImmLogic::from_rs(put_input_in_rs(ctx, input, narrow_mode))
}

pub(crate) fn put_input_in_reg_immshift<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
    shift_width_bits: usize,
) -> ResultRegImmShift {
    if let Some(imm_value) = input_to_const(ctx, input) {
        let imm_value = imm_value & ((shift_width_bits - 1) as u64);
        if let Some(immshift) = ImmShift::maybe_from_u64(imm_value) {
            return ResultRegImmShift::ImmShift(immshift);
        }
    }

    ResultRegImmShift::Reg(put_input_in_reg(ctx, input, NarrowValueMode::None))
}

//============================================================================
// ALU instruction constructors.

pub(crate) fn alu_inst_imm12(op: ALUOp, rd: Writable<Reg>, rn: Reg, rm: ResultRSEImm12) -> Inst {
    match rm {
        ResultRSEImm12::Imm12(imm12) => Inst::AluRRImm12 {
            alu_op: op,
            rd,
            rn,
            imm12,
        },
        ResultRSEImm12::Reg(rm) => Inst::AluRRR {
            alu_op: op,
            rd,
            rn,
            rm,
        },
        ResultRSEImm12::RegShift(rm, shiftop) => Inst::AluRRRShift {
            alu_op: op,
            rd,
            rn,
            rm,
            shiftop,
        },
        ResultRSEImm12::RegExtend(rm, extendop) => Inst::AluRRRExtend {
            alu_op: op,
            rd,
            rn,
            rm,
            extendop,
        },
    }
}

pub(crate) fn alu_inst_immlogic(
    op: ALUOp,
    rd: Writable<Reg>,
    rn: Reg,
    rm: ResultRSImmLogic,
) -> Inst {
    match rm {
        ResultRSImmLogic::ImmLogic(imml) => Inst::AluRRImmLogic {
            alu_op: op,
            rd,
            rn,
            imml,
        },
        ResultRSImmLogic::Reg(rm) => Inst::AluRRR {
            alu_op: op,
            rd,
            rn,
            rm,
        },
        ResultRSImmLogic::RegShift(rm, shiftop) => Inst::AluRRRShift {
            alu_op: op,
            rd,
            rn,
            rm,
            shiftop,
        },
    }
}

pub(crate) fn alu_inst_immshift(
    op: ALUOp,
    rd: Writable<Reg>,
    rn: Reg,
    rm: ResultRegImmShift,
) -> Inst {
    match rm {
        ResultRegImmShift::ImmShift(immshift) => Inst::AluRRImmShift {
            alu_op: op,
            rd,
            rn,
            immshift,
        },
        ResultRegImmShift::Reg(rm) => Inst::AluRRR {
            alu_op: op,
            rd,
            rn,
            rm,
        },
    }
}

//============================================================================
// Lowering: addressing mode support. Takes instruction directly, rather
// than an `InsnInput`, to do more introspection.

/// Lower the address of a load or store.
pub(crate) fn lower_address<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    elem_ty: Type,
    addends: &[InsnInput],
    offset: i32,
) -> MemArg {
    // TODO: support base_reg + scale * index_reg. For this, we would need to pattern-match shl or
    // mul instructions (Load/StoreComplex don't include scale factors).

    // Handle one reg and offset.
    if addends.len() == 1 {
        let reg = put_input_in_reg(ctx, addends[0], NarrowValueMode::ZeroExtend64);
        return MemArg::RegOffset(reg, offset as i64, elem_ty);
    }

    // Handle two regs and a zero offset with built-in extend, if possible.
    if addends.len() == 2 && offset == 0 {
        // r1, r2 (to be extended), r2_bits, is_signed
        let mut parts: Option<(Reg, Reg, usize, bool)> = None;
        // Handle extension of either first or second addend.
        for i in 0..2 {
            if let Some((op, ext_insn)) =
                maybe_input_insn_multi(ctx, addends[i], &[Opcode::Uextend, Opcode::Sextend])
            {
                // Non-extended addend.
                let r1 = put_input_in_reg(ctx, addends[1 - i], NarrowValueMode::ZeroExtend64);
                // Extended addend.
                let r2 = put_input_in_reg(
                    ctx,
                    InsnInput {
                        insn: ext_insn,
                        input: 0,
                    },
                    NarrowValueMode::None,
                );
                let r2_bits = ty_bits(ctx.input_ty(ext_insn, 0));
                parts = Some((
                    r1,
                    r2,
                    r2_bits,
                    /* is_signed = */ op == Opcode::Sextend,
                ));
                break;
            }
        }

        if let Some((r1, r2, r2_bits, is_signed)) = parts {
            match (r2_bits, is_signed) {
                (32, false) => {
                    return MemArg::RegExtended(r1, r2, ExtendOp::UXTW);
                }
                (32, true) => {
                    return MemArg::RegExtended(r1, r2, ExtendOp::SXTW);
                }
                _ => {}
            }
        }
    }

    // Handle two regs and a zero offset in the general case, if possible.
    if addends.len() == 2 && offset == 0 {
        let ra = put_input_in_reg(ctx, addends[0], NarrowValueMode::ZeroExtend64);
        let rb = put_input_in_reg(ctx, addends[1], NarrowValueMode::ZeroExtend64);
        return MemArg::reg_plus_reg(ra, rb);
    }

    // Otherwise, generate add instructions.
    let addr = ctx.alloc_tmp(RegClass::I64, I64);

    // Get the const into a reg.
    lower_constant_u64(ctx, addr.clone(), offset as u64);

    // Add each addend to the address.
    for addend in addends {
        let reg = put_input_in_reg(ctx, *addend, NarrowValueMode::ZeroExtend64);

        // In an addition, the stack register is the zero register, so divert it to another
        // register just before doing the actual add.
        let reg = if reg == stack_reg() {
            let tmp = ctx.alloc_tmp(RegClass::I64, I64);
            ctx.emit(Inst::Mov {
                rd: tmp,
                rm: stack_reg(),
            });
            tmp.to_reg()
        } else {
            reg
        };

        ctx.emit(Inst::AluRRR {
            alu_op: ALUOp::Add64,
            rd: addr.clone(),
            rn: addr.to_reg(),
            rm: reg.clone(),
        });
    }

    MemArg::reg(addr.to_reg())
}

pub(crate) fn lower_constant_u64<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    rd: Writable<Reg>,
    value: u64,
) {
    for inst in Inst::load_constant(rd, value) {
        ctx.emit(inst);
    }
}

pub(crate) fn lower_constant_f32<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    rd: Writable<Reg>,
    value: f32,
) {
    ctx.emit(Inst::load_fp_constant32(rd, value));
}

pub(crate) fn lower_constant_f64<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    rd: Writable<Reg>,
    value: f64,
) {
    ctx.emit(Inst::load_fp_constant64(rd, value));
}

pub(crate) fn lower_constant_f128<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    rd: Writable<Reg>,
    value: u128,
) {
    ctx.emit(Inst::load_fp_constant128(rd, value));
}

pub(crate) fn lower_condcode(cc: IntCC) -> Cond {
    match cc {
        IntCC::Equal => Cond::Eq,
        IntCC::NotEqual => Cond::Ne,
        IntCC::SignedGreaterThanOrEqual => Cond::Ge,
        IntCC::SignedGreaterThan => Cond::Gt,
        IntCC::SignedLessThanOrEqual => Cond::Le,
        IntCC::SignedLessThan => Cond::Lt,
        IntCC::UnsignedGreaterThanOrEqual => Cond::Hs,
        IntCC::UnsignedGreaterThan => Cond::Hi,
        IntCC::UnsignedLessThanOrEqual => Cond::Ls,
        IntCC::UnsignedLessThan => Cond::Lo,
        IntCC::Overflow => Cond::Vs,
        IntCC::NotOverflow => Cond::Vc,
    }
}

pub(crate) fn lower_fp_condcode(cc: FloatCC) -> Cond {
    // Refer to `codegen/shared/src/condcodes.rs` and to the `FCMP` AArch64 docs.
    // The FCMP instruction sets:
    //               NZCV
    // - PCSR.NZCV = 0011 on UN (unordered),
    //               0110 on EQ,
    //               1000 on LT,
    //               0010 on GT.
    match cc {
        // EQ | LT | GT. Vc => V clear.
        FloatCC::Ordered => Cond::Vc,
        // UN. Vs => V set.
        FloatCC::Unordered => Cond::Vs,
        // EQ. Eq => Z set.
        FloatCC::Equal => Cond::Eq,
        // UN | LT | GT. Ne => Z clear.
        FloatCC::NotEqual => Cond::Ne,
        // LT | GT.
        FloatCC::OrderedNotEqual => unimplemented!(),
        //  UN | EQ
        FloatCC::UnorderedOrEqual => unimplemented!(),
        // LT. Mi => N set.
        FloatCC::LessThan => Cond::Mi,
        // LT | EQ. Ls => C clear or Z set.
        FloatCC::LessThanOrEqual => Cond::Ls,
        // GT. Gt => Z clear, N = V.
        FloatCC::GreaterThan => Cond::Gt,
        // GT | EQ. Ge => N = V.
        FloatCC::GreaterThanOrEqual => Cond::Ge,
        // UN | LT
        FloatCC::UnorderedOrLessThan => unimplemented!(),
        // UN | LT | EQ
        FloatCC::UnorderedOrLessThanOrEqual => unimplemented!(),
        // UN | GT
        FloatCC::UnorderedOrGreaterThan => unimplemented!(),
        // UN | GT | EQ
        FloatCC::UnorderedOrGreaterThanOrEqual => unimplemented!(),
    }
}

pub(crate) fn lower_vector_compare<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    rd: Writable<Reg>,
    mut rn: Reg,
    mut rm: Reg,
    ty: Type,
    cond: Cond,
) -> CodegenResult<()> {
    match ty {
        F32X4 | F64X2 | I8X16 | I16X8 | I32X4 => {}
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "unsupported SIMD type: {:?}",
                ty
            )));
        }
    };

    let is_float = match ty {
        F32X4 | F64X2 => true,
        _ => false,
    };
    // 'Less than' operations are implemented by swapping
    // the order of operands and using the 'greater than'
    // instructions.
    // 'Not equal' is implemented with 'equal' and inverting
    // the result.
    let (alu_op, swap) = match (is_float, cond) {
        (false, Cond::Eq) => (VecALUOp::Cmeq, false),
        (false, Cond::Ne) => (VecALUOp::Cmeq, false),
        (false, Cond::Ge) => (VecALUOp::Cmge, false),
        (false, Cond::Gt) => (VecALUOp::Cmgt, false),
        (false, Cond::Le) => (VecALUOp::Cmge, true),
        (false, Cond::Lt) => (VecALUOp::Cmgt, true),
        (false, Cond::Hs) => (VecALUOp::Cmhs, false),
        (false, Cond::Hi) => (VecALUOp::Cmhi, false),
        (false, Cond::Ls) => (VecALUOp::Cmhs, true),
        (false, Cond::Lo) => (VecALUOp::Cmhi, true),
        (true, Cond::Eq) => (VecALUOp::Fcmeq, false),
        (true, Cond::Ne) => (VecALUOp::Fcmeq, false),
        (true, Cond::Mi) => (VecALUOp::Fcmgt, true),
        (true, Cond::Ls) => (VecALUOp::Fcmge, true),
        (true, Cond::Ge) => (VecALUOp::Fcmge, false),
        (true, Cond::Gt) => (VecALUOp::Fcmgt, false),
        _ => unreachable!(),
    };

    if swap {
        std::mem::swap(&mut rn, &mut rm);
    }

    ctx.emit(Inst::VecRRR {
        alu_op,
        rd,
        rn,
        rm,
        ty,
    });

    if cond == Cond::Ne {
        ctx.emit(Inst::VecMisc {
            op: VecMisc2::Not,
            rd,
            rn: rd.to_reg(),
            ty: I8X16,
        });
    }

    Ok(())
}

/// Determines whether this condcode interprets inputs as signed or
/// unsigned.  See the documentation for the `icmp` instruction in
/// cranelift-codegen/meta/src/shared/instructions.rs for further insights
/// into this.
pub fn condcode_is_signed(cc: IntCC) -> bool {
    match cc {
        IntCC::Equal => false,
        IntCC::NotEqual => false,
        IntCC::SignedGreaterThanOrEqual => true,
        IntCC::SignedGreaterThan => true,
        IntCC::SignedLessThanOrEqual => true,
        IntCC::SignedLessThan => true,
        IntCC::UnsignedGreaterThanOrEqual => false,
        IntCC::UnsignedGreaterThan => false,
        IntCC::UnsignedLessThanOrEqual => false,
        IntCC::UnsignedLessThan => false,
        IntCC::Overflow => true,
        IntCC::NotOverflow => true,
    }
}

//=============================================================================
// Helpers for instruction lowering.

/// Returns the size (in bits) of a given type.
pub fn ty_bits(ty: Type) -> usize {
    match ty {
        B1 => 1,
        B8 | I8 => 8,
        B16 | I16 => 16,
        B32 | I32 | F32 | R32 => 32,
        B64 | I64 | F64 | R64 => 64,
        B128 | I128 => 128,
        IFLAGS | FFLAGS => 32,
        B8X8 | I8X8 | B16X4 | I16X4 | B32X2 | I32X2 => 64,
        B8X16 | I8X16 | B16X8 | I16X8 | B32X4 | I32X4 | B64X2 | I64X2 => 128,
        F32X4 | F64X2 => 128,
        _ => panic!("ty_bits() on unknown type: {:?}", ty),
    }
}

pub(crate) fn ty_is_int(ty: Type) -> bool {
    match ty {
        B1 | B8 | I8 | B16 | I16 | B32 | I32 | B64 | I64 | R32 | R64 => true,
        F32 | F64 | B128 | I128 | I8X8 | I8X16 | I16X4 | I16X8 | I32X2 | I32X4 | I64X2 => false,
        IFLAGS | FFLAGS => panic!("Unexpected flags type"),
        _ => panic!("ty_is_int() on unknown type: {:?}", ty),
    }
}

pub(crate) fn ty_is_float(ty: Type) -> bool {
    !ty_is_int(ty)
}

pub(crate) fn choose_32_64<T: Copy>(ty: Type, op32: T, op64: T) -> T {
    let bits = ty_bits(ty);
    if bits <= 32 {
        op32
    } else if bits == 64 {
        op64
    } else {
        panic!("choose_32_64 on > 64 bits!")
    }
}

pub(crate) fn ldst_offset(data: &InstructionData) -> Option<i32> {
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

pub(crate) fn inst_condcode(data: &InstructionData) -> Option<IntCC> {
    match data {
        &InstructionData::IntCond { cond, .. }
        | &InstructionData::BranchIcmp { cond, .. }
        | &InstructionData::IntCompare { cond, .. }
        | &InstructionData::IntCondTrap { cond, .. }
        | &InstructionData::BranchInt { cond, .. }
        | &InstructionData::IntSelect { cond, .. }
        | &InstructionData::IntCompareImm { cond, .. } => Some(cond),
        _ => None,
    }
}

pub(crate) fn inst_fp_condcode(data: &InstructionData) -> Option<FloatCC> {
    match data {
        &InstructionData::BranchFloat { cond, .. }
        | &InstructionData::FloatCompare { cond, .. }
        | &InstructionData::FloatCond { cond, .. }
        | &InstructionData::FloatCondTrap { cond, .. } => Some(cond),
        _ => None,
    }
}

pub(crate) fn inst_trapcode(data: &InstructionData) -> Option<TrapCode> {
    match data {
        &InstructionData::Trap { code, .. }
        | &InstructionData::CondTrap { code, .. }
        | &InstructionData::IntCondTrap { code, .. }
        | &InstructionData::FloatCondTrap { code, .. } => Some(code),
        _ => None,
    }
}

/// Checks for an instance of `op` feeding the given input.
pub(crate) fn maybe_input_insn<C: LowerCtx<I = Inst>>(
    c: &mut C,
    input: InsnInput,
    op: Opcode,
) -> Option<IRInst> {
    let inputs = c.get_input(input.insn, input.input);
    debug!(
        "maybe_input_insn: input {:?} has options {:?}; looking for op {:?}",
        input, inputs, op
    );
    if let Some((src_inst, _)) = inputs.inst {
        let data = c.data(src_inst);
        debug!(" -> input inst {:?}", data);
        if data.opcode() == op {
            return Some(src_inst);
        }
    }
    None
}

/// Checks for an instance of any one of `ops` feeding the given input.
pub(crate) fn maybe_input_insn_multi<C: LowerCtx<I = Inst>>(
    c: &mut C,
    input: InsnInput,
    ops: &[Opcode],
) -> Option<(Opcode, IRInst)> {
    for &op in ops {
        if let Some(inst) = maybe_input_insn(c, input, op) {
            return Some((op, inst));
        }
    }
    None
}

/// Checks for an instance of `op` feeding the given input, possibly via a conversion `conv` (e.g.,
/// Bint or a bitcast).
///
/// FIXME cfallin 2020-03-30: this is really ugly. Factor out tree-matching stuff and make it
/// a bit more generic.
pub(crate) fn maybe_input_insn_via_conv<C: LowerCtx<I = Inst>>(
    c: &mut C,
    input: InsnInput,
    op: Opcode,
    conv: Opcode,
) -> Option<IRInst> {
    let inputs = c.get_input(input.insn, input.input);
    if let Some((src_inst, _)) = inputs.inst {
        let data = c.data(src_inst);
        if data.opcode() == op {
            return Some(src_inst);
        }
        if data.opcode() == conv {
            let inputs = c.get_input(src_inst, 0);
            if let Some((src_inst, _)) = inputs.inst {
                let data = c.data(src_inst);
                if data.opcode() == op {
                    return Some(src_inst);
                }
            }
        }
    }
    None
}

pub(crate) fn lower_icmp_or_ifcmp_to_flags<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    insn: IRInst,
    is_signed: bool,
) {
    debug!("lower_icmp_or_ifcmp_to_flags: insn {}", insn);
    let ty = ctx.input_ty(insn, 0);
    let bits = ty_bits(ty);
    let narrow_mode = match (bits <= 32, is_signed) {
        (true, true) => NarrowValueMode::SignExtend32,
        (true, false) => NarrowValueMode::ZeroExtend32,
        (false, true) => NarrowValueMode::SignExtend64,
        (false, false) => NarrowValueMode::ZeroExtend64,
    };
    let inputs = [InsnInput { insn, input: 0 }, InsnInput { insn, input: 1 }];
    let ty = ctx.input_ty(insn, 0);
    let rn = put_input_in_reg(ctx, inputs[0], narrow_mode);
    let rm = put_input_in_rse_imm12(ctx, inputs[1], narrow_mode);
    debug!("lower_icmp_or_ifcmp_to_flags: rn = {:?} rm = {:?}", rn, rm);
    let alu_op = choose_32_64(ty, ALUOp::SubS32, ALUOp::SubS64);
    let rd = writable_zero_reg();
    ctx.emit(alu_inst_imm12(alu_op, rd, rn, rm));
}

pub(crate) fn lower_fcmp_or_ffcmp_to_flags<C: LowerCtx<I = Inst>>(ctx: &mut C, insn: IRInst) {
    let ty = ctx.input_ty(insn, 0);
    let bits = ty_bits(ty);
    let inputs = [InsnInput { insn, input: 0 }, InsnInput { insn, input: 1 }];
    let rn = put_input_in_reg(ctx, inputs[0], NarrowValueMode::None);
    let rm = put_input_in_reg(ctx, inputs[1], NarrowValueMode::None);
    match bits {
        32 => {
            ctx.emit(Inst::FpuCmp32 { rn, rm });
        }
        64 => {
            ctx.emit(Inst::FpuCmp64 { rn, rm });
        }
        _ => panic!("Unknown float size"),
    }
}

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for AArch64Backend {
    type MInst = Inst;

    fn lower<C: LowerCtx<I = Inst>>(&self, ctx: &mut C, ir_inst: IRInst) -> CodegenResult<()> {
        lower_inst::lower_insn_to_regs(ctx, ir_inst)
    }

    fn lower_branch_group<C: LowerCtx<I = Inst>>(
        &self,
        ctx: &mut C,
        branches: &[IRInst],
        targets: &[MachLabel],
        fallthrough: Option<MachLabel>,
    ) -> CodegenResult<()> {
        lower_inst::lower_branch(ctx, branches, targets, fallthrough)
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        Some(xreg(PINNED_REG))
    }
}
