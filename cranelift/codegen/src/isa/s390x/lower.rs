//! Lowering rules for S390x.

use crate::ir::condcodes::IntCC;
use crate::ir::Inst as IRInst;
use crate::ir::{types, Endianness, MemFlags, Opcode, Type};
use crate::isa::s390x::abi::*;
use crate::isa::s390x::inst::*;
use crate::isa::s390x::settings as s390x_settings;
use crate::isa::s390x::S390xBackend;
use crate::machinst::lower::*;
use crate::machinst::*;
use crate::settings::Flags;
use crate::CodegenResult;
use alloc::boxed::Box;
use core::convert::TryFrom;
use regalloc::{Reg, Writable};
use smallvec::SmallVec;

pub mod isle;

//=============================================================================
// Helpers for instruction lowering.

fn choose_32_64<T: Copy>(ty: Type, op32: T, op64: T) -> T {
    let bits = ty_bits(ty);
    if bits <= 32 {
        op32
    } else if bits == 64 {
        op64
    } else {
        panic!("choose_32_64 on > 64 bits!")
    }
}

//============================================================================
// Lowering: convert instruction inputs to forms that we can use.

/// Lower an instruction input to a 64-bit constant, if possible.
fn input_matches_const<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Option<u64> {
    let input = ctx.get_input_as_source_or_const(input.insn, input.input);
    input.constant
}

/// Lower an instruction input to a 64-bit signed constant, if possible.
fn input_matches_sconst<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Option<i64> {
    if let Some(imm) = input_matches_const(ctx, input) {
        let ty = ctx.input_ty(input.insn, input.input);
        Some(sign_extend_to_u64(imm, ty_bits(ty) as u8) as i64)
    } else {
        None
    }
}

/// Lower an instruction input to a 16-bit signed constant, if possible.
fn input_matches_simm16<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Option<i16> {
    if let Some(imm_value) = input_matches_sconst(ctx, input) {
        if let Ok(imm) = i16::try_from(imm_value) {
            return Some(imm);
        }
    }
    None
}

/// Lower an instruction input to a 32-bit signed constant, if possible.
fn input_matches_simm32<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Option<i32> {
    if let Some(imm_value) = input_matches_sconst(ctx, input) {
        if let Ok(imm) = i32::try_from(imm_value) {
            return Some(imm);
        }
    }
    None
}

/// Lower an instruction input to a 32-bit unsigned constant, if possible.
fn input_matches_uimm32<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Option<u32> {
    if let Some(imm_value) = input_matches_const(ctx, input) {
        if let Ok(imm) = u32::try_from(imm_value) {
            return Some(imm);
        }
    }
    None
}

/// Checks for an instance of `op` feeding the given input.
fn input_matches_insn<C: LowerCtx<I = Inst>>(
    c: &mut C,
    input: InsnInput,
    op: Opcode,
) -> Option<IRInst> {
    let inputs = c.get_input_as_source_or_const(input.insn, input.input);
    if let Some((src_inst, _)) = inputs.inst {
        let data = c.data(src_inst);
        if data.opcode() == op {
            return Some(src_inst);
        }
    }
    None
}

/// Checks for an instance of `op` feeding the given input, possibly via a conversion `conv` (e.g.,
/// Bint or a bitcast).
fn input_matches_insn_via_conv<C: LowerCtx<I = Inst>>(
    c: &mut C,
    input: InsnInput,
    op: Opcode,
    conv: Opcode,
) -> Option<IRInst> {
    let inputs = c.get_input_as_source_or_const(input.insn, input.input);
    if let Some((src_inst, _)) = inputs.inst {
        let data = c.data(src_inst);
        if data.opcode() == op {
            return Some(src_inst);
        }
        if data.opcode() == conv {
            let inputs = c.get_input_as_source_or_const(src_inst, 0);
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

fn input_matches_load_insn<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
    op: Opcode,
) -> Option<MemArg> {
    if let Some(insn) = input_matches_insn(ctx, input, op) {
        let inputs: SmallVec<[InsnInput; 4]> = (0..ctx.num_inputs(insn))
            .map(|i| InsnInput { insn, input: i })
            .collect();
        let off = ctx.data(insn).load_store_offset().unwrap();
        let flags = ctx.memflags(insn).unwrap();
        let endianness = flags.endianness(Endianness::Big);
        if endianness == Endianness::Big {
            let mem = lower_address(ctx, &inputs[..], off, flags);
            ctx.sink_inst(insn);
            return Some(mem);
        }
    }
    None
}

fn input_matches_mem<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Option<MemArg> {
    if ty_bits(ctx.input_ty(input.insn, input.input)) >= 32 {
        return input_matches_load_insn(ctx, input, Opcode::Load);
    }
    None
}

fn input_matches_sext16_mem<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
) -> Option<MemArg> {
    if ty_bits(ctx.input_ty(input.insn, input.input)) == 16 {
        return input_matches_load_insn(ctx, input, Opcode::Load);
    }
    if ty_bits(ctx.input_ty(input.insn, input.input)) >= 32 {
        return input_matches_load_insn(ctx, input, Opcode::Sload16);
    }
    None
}

fn input_matches_sext32_mem<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
) -> Option<MemArg> {
    if ty_bits(ctx.input_ty(input.insn, input.input)) > 32 {
        return input_matches_load_insn(ctx, input, Opcode::Sload32);
    }
    None
}

fn input_matches_sext32_reg<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Option<Reg> {
    if let Some(insn) = input_matches_insn(ctx, input, Opcode::Sextend) {
        if ty_bits(ctx.input_ty(insn, 0)) == 32 {
            let reg = put_input_in_reg(ctx, InsnInput { insn, input: 0 }, NarrowValueMode::None);
            return Some(reg);
        }
    }
    None
}

fn input_matches_uext32_reg<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Option<Reg> {
    if let Some(insn) = input_matches_insn(ctx, input, Opcode::Uextend) {
        if ty_bits(ctx.input_ty(insn, 0)) == 32 {
            let reg = put_input_in_reg(ctx, InsnInput { insn, input: 0 }, NarrowValueMode::None);
            return Some(reg);
        }
    }
    None
}

fn input_matches_uext16_mem<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
) -> Option<MemArg> {
    if ty_bits(ctx.input_ty(input.insn, input.input)) == 16 {
        return input_matches_load_insn(ctx, input, Opcode::Load);
    }
    if ty_bits(ctx.input_ty(input.insn, input.input)) >= 32 {
        return input_matches_load_insn(ctx, input, Opcode::Uload16);
    }
    None
}

fn input_matches_uext32_mem<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
) -> Option<MemArg> {
    if ty_bits(ctx.input_ty(input.insn, input.input)) > 32 {
        return input_matches_load_insn(ctx, input, Opcode::Uload32);
    }
    None
}

//============================================================================
// Lowering: force instruction input into a register

/// How to handle narrow values loaded into registers; see note on `narrow_mode`
/// parameter to `put_input_in_*` below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NarrowValueMode {
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

fn extend_memory_to_reg<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    mem: MemArg,
    from_ty: Type,
    to_ty: Type,
    signed: bool,
) -> Reg {
    let rd = ctx.alloc_tmp(to_ty).only_reg().unwrap();
    ctx.emit(match (signed, ty_bits(to_ty), ty_bits(from_ty)) {
        (false, 32, 8) => Inst::Load32ZExt8 { rd, mem },
        (false, 32, 16) => Inst::Load32ZExt16 { rd, mem },
        (true, 32, 8) => Inst::Load32SExt8 { rd, mem },
        (true, 32, 16) => Inst::Load32SExt16 { rd, mem },
        (false, 64, 8) => Inst::Load64ZExt8 { rd, mem },
        (false, 64, 16) => Inst::Load64ZExt16 { rd, mem },
        (false, 64, 32) => Inst::Load64ZExt32 { rd, mem },
        (true, 64, 8) => Inst::Load64SExt8 { rd, mem },
        (true, 64, 16) => Inst::Load64SExt16 { rd, mem },
        (true, 64, 32) => Inst::Load64SExt32 { rd, mem },
        _ => panic!("Unsupported size in load"),
    });
    rd.to_reg()
}

/// Sign-extend the low `from_bits` bits of `value` to a full u64.
fn sign_extend_to_u64(value: u64, from_bits: u8) -> u64 {
    assert!(from_bits <= 64);
    if from_bits >= 64 {
        value
    } else {
        (((value << (64 - from_bits)) as i64) >> (64 - from_bits)) as u64
    }
}

/// Zero-extend the low `from_bits` bits of `value` to a full u64.
fn zero_extend_to_u64(value: u64, from_bits: u8) -> u64 {
    assert!(from_bits <= 64);
    if from_bits >= 64 {
        value
    } else {
        value & ((1u64 << from_bits) - 1)
    }
}

/// Lower an instruction input to a reg.
///
/// The given register will be extended appropriately, according to
/// `narrow_mode` and the input's type.
fn put_input_in_reg<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    input: InsnInput,
    narrow_mode: NarrowValueMode,
) -> Reg {
    let signed = match narrow_mode {
        NarrowValueMode::SignExtend32 | NarrowValueMode::SignExtend64 => true,
        NarrowValueMode::ZeroExtend32 | NarrowValueMode::ZeroExtend64 => false,
        _ => false,
    };
    let ty = ctx.input_ty(input.insn, input.input);
    let from_bits = ty_bits(ty) as u8;
    let ext_ty = match narrow_mode {
        NarrowValueMode::None => ty,
        NarrowValueMode::ZeroExtend32 | NarrowValueMode::SignExtend32 => types::I32,
        NarrowValueMode::ZeroExtend64 | NarrowValueMode::SignExtend64 => types::I64,
    };
    let to_bits = ty_bits(ext_ty) as u8;
    assert!(to_bits >= from_bits);

    if let Some(c) = input_matches_const(ctx, input) {
        let extended = if from_bits == to_bits {
            c
        } else if signed {
            sign_extend_to_u64(c, from_bits)
        } else {
            zero_extend_to_u64(c, from_bits)
        };
        let masked = zero_extend_to_u64(extended, to_bits);

        // Generate constants fresh at each use to minimize long-range register pressure.
        let to_reg = ctx.alloc_tmp(ext_ty).only_reg().unwrap();
        for inst in Inst::gen_constant(ValueRegs::one(to_reg), masked as u128, ext_ty, |ty| {
            ctx.alloc_tmp(ty).only_reg().unwrap()
        })
        .into_iter()
        {
            ctx.emit(inst);
        }
        to_reg.to_reg()
    } else if to_bits == from_bits {
        ctx.put_input_in_regs(input.insn, input.input)
            .only_reg()
            .unwrap()
    } else if let Some(mem) = input_matches_load_insn(ctx, input, Opcode::Load) {
        extend_memory_to_reg(ctx, mem, ty, ext_ty, signed)
    } else {
        let rd = ctx.alloc_tmp(ext_ty).only_reg().unwrap();
        let rn = ctx
            .put_input_in_regs(input.insn, input.input)
            .only_reg()
            .unwrap();
        ctx.emit(Inst::Extend {
            rd,
            rn,
            signed,
            from_bits,
            to_bits,
        });
        rd.to_reg()
    }
}

//============================================================================
// Lowering: addressing mode support. Takes instruction directly, rather
// than an `InsnInput`, to do more introspection.

/// Lower the address of a load or store.
fn lower_address<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    addends: &[InsnInput],
    offset: i32,
    flags: MemFlags,
) -> MemArg {
    // Handle one reg and offset.
    if addends.len() == 1 {
        if offset == 0 {
            if let Some(add) = input_matches_insn(ctx, addends[0], Opcode::Iadd) {
                debug_assert_eq!(ctx.output_ty(add, 0), types::I64);
                let add_inputs = &[
                    InsnInput {
                        insn: add,
                        input: 0,
                    },
                    InsnInput {
                        insn: add,
                        input: 1,
                    },
                ];

                let ra = put_input_in_reg(ctx, add_inputs[0], NarrowValueMode::None);
                let rb = put_input_in_reg(ctx, add_inputs[1], NarrowValueMode::None);
                return MemArg::reg_plus_reg(ra, rb, flags);
            }
        }

        if let Some(symbol) = input_matches_insn(ctx, addends[0], Opcode::SymbolValue) {
            let (extname, dist, ext_offset) = ctx.symbol_value(symbol).unwrap();
            let ext_offset = ext_offset + i64::from(offset);
            if dist == RelocDistance::Near && (ext_offset & 1) == 0 {
                if let Ok(offset) = i32::try_from(ext_offset) {
                    return MemArg::Symbol {
                        name: Box::new(extname.clone()),
                        offset,
                        flags,
                    };
                }
            }
        }

        let reg = put_input_in_reg(ctx, addends[0], NarrowValueMode::None);
        return MemArg::reg_plus_off(reg, offset as i64, flags);
    }

    // Handle two regs and a zero offset.
    if addends.len() == 2 && offset == 0 {
        let ra = put_input_in_reg(ctx, addends[0], NarrowValueMode::None);
        let rb = put_input_in_reg(ctx, addends[1], NarrowValueMode::None);
        return MemArg::reg_plus_reg(ra, rb, flags);
    }

    // Otherwise, generate add instructions.
    let addr = ctx.alloc_tmp(types::I64).only_reg().unwrap();

    // Get the const into a reg.
    lower_constant_u64(ctx, addr.clone(), offset as u64);

    // Add each addend to the address.
    for addend in addends {
        let reg = put_input_in_reg(ctx, *addend, NarrowValueMode::None);

        ctx.emit(Inst::AluRRR {
            alu_op: ALUOp::Add64,
            rd: addr.clone(),
            rn: addr.to_reg(),
            rm: reg.clone(),
        });
    }

    MemArg::reg(addr.to_reg(), flags)
}

//============================================================================
// Lowering: generating constants.

fn lower_constant_u64<C: LowerCtx<I = Inst>>(ctx: &mut C, rd: Writable<Reg>, value: u64) {
    for inst in Inst::load_constant64(rd, value) {
        ctx.emit(inst);
    }
}

//=============================================================================
// Lowering: comparisons

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

fn lower_icmp_to_flags<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    insn: IRInst,
    is_signed: bool,
    may_sink_memory: bool,
) {
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
    if is_signed {
        let op = choose_32_64(ty, CmpOp::CmpS32, CmpOp::CmpS64);
        // Try matching immedate operand.
        if let Some(imm) = input_matches_simm16(ctx, inputs[1]) {
            return ctx.emit(Inst::CmpRSImm16 { op, rn, imm });
        }
        if let Some(imm) = input_matches_simm32(ctx, inputs[1]) {
            return ctx.emit(Inst::CmpRSImm32 { op, rn, imm });
        }
        // If sinking memory loads is allowed, try matching memory operand.
        if may_sink_memory {
            if let Some(mem) = input_matches_mem(ctx, inputs[1]) {
                return ctx.emit(Inst::CmpRX { op, rn, mem });
            }
            if let Some(mem) = input_matches_sext16_mem(ctx, inputs[1]) {
                let op = choose_32_64(ty, CmpOp::CmpS32Ext16, CmpOp::CmpS64Ext16);
                return ctx.emit(Inst::CmpRX { op, rn, mem });
            }
            if let Some(mem) = input_matches_sext32_mem(ctx, inputs[1]) {
                return ctx.emit(Inst::CmpRX {
                    op: CmpOp::CmpS64Ext32,
                    rn,
                    mem,
                });
            }
        }
        // Try matching sign-extension in register.
        if let Some(rm) = input_matches_sext32_reg(ctx, inputs[1]) {
            return ctx.emit(Inst::CmpRR {
                op: CmpOp::CmpS64Ext32,
                rn,
                rm,
            });
        }
        // If no special case matched above, fall back to a register compare.
        let rm = put_input_in_reg(ctx, inputs[1], narrow_mode);
        return ctx.emit(Inst::CmpRR { op, rn, rm });
    } else {
        let op = choose_32_64(ty, CmpOp::CmpL32, CmpOp::CmpL64);
        // Try matching immedate operand.
        if let Some(imm) = input_matches_uimm32(ctx, inputs[1]) {
            return ctx.emit(Inst::CmpRUImm32 { op, rn, imm });
        }
        // If sinking memory loads is allowed, try matching memory operand.
        if may_sink_memory {
            if let Some(mem) = input_matches_mem(ctx, inputs[1]) {
                return ctx.emit(Inst::CmpRX { op, rn, mem });
            }
            if let Some(mem) = input_matches_uext16_mem(ctx, inputs[1]) {
                match &mem {
                    &MemArg::Symbol { .. } => {
                        let op = choose_32_64(ty, CmpOp::CmpL32Ext16, CmpOp::CmpL64Ext16);
                        return ctx.emit(Inst::CmpRX { op, rn, mem });
                    }
                    _ => {
                        let reg_ty = choose_32_64(ty, types::I32, types::I64);
                        let rm = extend_memory_to_reg(ctx, mem, types::I16, reg_ty, false);
                        return ctx.emit(Inst::CmpRR { op, rn, rm });
                    }
                }
            }
            if let Some(mem) = input_matches_uext32_mem(ctx, inputs[1]) {
                return ctx.emit(Inst::CmpRX {
                    op: CmpOp::CmpL64Ext32,
                    rn,
                    mem,
                });
            }
        }
        // Try matching zero-extension in register.
        if let Some(rm) = input_matches_uext32_reg(ctx, inputs[1]) {
            return ctx.emit(Inst::CmpRR {
                op: CmpOp::CmpL64Ext32,
                rn,
                rm,
            });
        }
        // If no special case matched above, fall back to a register compare.
        let rm = put_input_in_reg(ctx, inputs[1], narrow_mode);
        return ctx.emit(Inst::CmpRR { op, rn, rm });
    }
}

fn lower_fcmp_to_flags<C: LowerCtx<I = Inst>>(ctx: &mut C, insn: IRInst) {
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

fn lower_boolean_to_flags<C: LowerCtx<I = Inst>>(ctx: &mut C, input: InsnInput) -> Cond {
    if let Some(icmp_insn) = input_matches_insn_via_conv(ctx, input, Opcode::Icmp, Opcode::Bint) {
        // FIXME: If the Icmp (and Bint) only have a single use, we can still allow sinking memory
        let may_sink_memory = false;
        let condcode = ctx.data(icmp_insn).cond_code().unwrap();
        let is_signed = condcode_is_signed(condcode);
        lower_icmp_to_flags(ctx, icmp_insn, is_signed, may_sink_memory);
        Cond::from_intcc(condcode)
    } else if let Some(fcmp_insn) =
        input_matches_insn_via_conv(ctx, input, Opcode::Fcmp, Opcode::Bint)
    {
        let condcode = ctx.data(fcmp_insn).fp_cond_code().unwrap();
        lower_fcmp_to_flags(ctx, fcmp_insn);
        Cond::from_floatcc(condcode)
    } else {
        let ty = ctx.input_ty(input.insn, input.input);
        let narrow_mode = if ty.bits() < 32 {
            NarrowValueMode::ZeroExtend32
        } else {
            NarrowValueMode::None
        };
        let rn = put_input_in_reg(ctx, input, narrow_mode);
        let op = choose_32_64(ty, CmpOp::CmpS32, CmpOp::CmpS64);
        ctx.emit(Inst::CmpRSImm16 { op, rn, imm: 0 });
        Cond::from_intcc(IntCC::NotEqual)
    }
}

//============================================================================
// Lowering: main entry point for lowering a instruction

fn lower_insn_to_regs<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    insn: IRInst,
    flags: &Flags,
    isa_flags: &s390x_settings::Flags,
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

    if let Ok(()) = super::lower::isle::lower(ctx, flags, isa_flags, &outputs, insn) {
        return Ok(());
    }

    let implemented_in_isle = || {
        unreachable!(
            "implemented in ISLE: inst = `{}`, type = `{:?}`",
            ctx.dfg().display_inst(insn),
            ty
        );
    };

    match op {
        Opcode::Nop
        | Opcode::Copy
        | Opcode::Iconst
        | Opcode::Bconst
        | Opcode::F32const
        | Opcode::F64const
        | Opcode::Null
        | Opcode::Iadd
        | Opcode::IaddIfcout
        | Opcode::Isub
        | Opcode::Iabs
        | Opcode::Ineg
        | Opcode::Imul
        | Opcode::Umulhi
        | Opcode::Smulhi
        | Opcode::Udiv
        | Opcode::Urem
        | Opcode::Sdiv
        | Opcode::Srem
        | Opcode::Ishl
        | Opcode::Ushr
        | Opcode::Sshr
        | Opcode::Rotr
        | Opcode::Rotl
        | Opcode::Ireduce
        | Opcode::Uextend
        | Opcode::Sextend
        | Opcode::Bnot
        | Opcode::Band
        | Opcode::Bor
        | Opcode::Bxor
        | Opcode::BandNot
        | Opcode::BorNot
        | Opcode::BxorNot
        | Opcode::Bitselect
        | Opcode::Breduce
        | Opcode::Bextend
        | Opcode::Bmask
        | Opcode::Bint
        | Opcode::Clz
        | Opcode::Cls
        | Opcode::Ctz
        | Opcode::Popcnt
        | Opcode::Fadd
        | Opcode::Fsub
        | Opcode::Fmul
        | Opcode::Fdiv
        | Opcode::Fmin
        | Opcode::Fmax
        | Opcode::Sqrt
        | Opcode::Fneg
        | Opcode::Fabs
        | Opcode::Fpromote
        | Opcode::Fdemote
        | Opcode::Ceil
        | Opcode::Floor
        | Opcode::Trunc
        | Opcode::Nearest
        | Opcode::Fma
        | Opcode::Fcopysign
        | Opcode::FcvtFromUint
        | Opcode::FcvtFromSint
        | Opcode::FcvtToUint
        | Opcode::FcvtToSint
        | Opcode::FcvtToUintSat
        | Opcode::FcvtToSintSat
        | Opcode::Bitcast
        | Opcode::Load
        | Opcode::Uload8
        | Opcode::Sload8
        | Opcode::Uload16
        | Opcode::Sload16
        | Opcode::Uload32
        | Opcode::Sload32
        | Opcode::Store
        | Opcode::Istore8
        | Opcode::Istore16
        | Opcode::Istore32
        | Opcode::AtomicRmw
        | Opcode::AtomicCas
        | Opcode::AtomicLoad
        | Opcode::AtomicStore
        | Opcode::Fence
        | Opcode::Icmp
        | Opcode::Fcmp
        | Opcode::IsNull
        | Opcode::IsInvalid
        | Opcode::Select
        | Opcode::SelectifSpectreGuard
        | Opcode::StackAddr
        | Opcode::FuncAddr
        | Opcode::SymbolValue => implemented_in_isle(),

        Opcode::UaddSat | Opcode::SaddSat => unimplemented!(),
        Opcode::UsubSat | Opcode::SsubSat => unimplemented!(),

        Opcode::Bitrev => unimplemented!(),

        Opcode::FcvtLowFromSint => unimplemented!("FcvtLowFromSint"),

        Opcode::StackLoad | Opcode::StackStore => {
            panic!("Direct stack memory access not supported; should not be used by Wasm");
        }

        Opcode::ConstAddr => unimplemented!(),

        Opcode::HeapAddr => {
            panic!("heap_addr should have been removed by legalization!");
        }

        Opcode::TableAddr => {
            panic!("table_addr should have been removed by legalization!");
        }

        Opcode::GlobalValue => {
            panic!("global_value should have been removed by legalization!");
        }

        Opcode::TlsValue => {
            unimplemented!("Thread-local storage support not implemented!");
        }

        Opcode::GetPinnedReg | Opcode::SetPinnedReg => {
            unimplemented!("Pinned register support not implemented!");
        }

        Opcode::Trap | Opcode::ResumableTrap => {
            let trap_code = ctx.data(insn).trap_code().unwrap();
            ctx.emit_safepoint(Inst::Trap { trap_code })
        }

        Opcode::Trapz | Opcode::Trapnz | Opcode::ResumableTrapnz => {
            let cond = lower_boolean_to_flags(ctx, inputs[0]);
            let negated = op == Opcode::Trapz;
            let cond = if negated { cond.invert() } else { cond };
            let trap_code = ctx.data(insn).trap_code().unwrap();
            ctx.emit_safepoint(Inst::TrapIf { trap_code, cond });
        }

        Opcode::Trapif => {
            let condcode = ctx.data(insn).cond_code().unwrap();
            let mut cond = Cond::from_intcc(condcode);
            let is_signed = condcode_is_signed(condcode);

            let cmp_insn = ctx
                .get_input_as_source_or_const(inputs[0].insn, inputs[0].input)
                .inst
                .unwrap()
                .0;
            if ctx.data(cmp_insn).opcode() == Opcode::IaddIfcout {
                // The flags must not have been clobbered by any other instruction between the
                // iadd_ifcout and this instruction, as verified by the CLIF validator; so we
                // can simply rely on the condition code here.
                //
                // IaddIfcout is implemented via a ADD LOGICAL instruction, which sets the
                // the condition code as follows:
                //   0   Result zero; no carry
                //   1   Result not zero; no carry
                //   2   Result zero; carry
                //   3   Result not zero; carry
                // This means "carry" corresponds to condition code 2 or 3, i.e.
                // a condition mask of 2 | 1.
                //
                // As this does not match any of the encodings used with a normal integer
                // comparsion, this cannot be represented by any IntCC value.  We need to
                // remap the IntCC::UnsignedGreaterThan value that we have here as result
                // of the unsigned_add_overflow_condition call to the correct mask.
                assert!(condcode == IntCC::UnsignedGreaterThan);
                cond = Cond::from_mask(2 | 1);
            } else {
                // Verification ensures that the input is always a single-def ifcmp
                debug_assert_eq!(ctx.data(cmp_insn).opcode(), Opcode::Ifcmp);
                lower_icmp_to_flags(ctx, cmp_insn, is_signed, true);
            }

            let trap_code = ctx.data(insn).trap_code().unwrap();
            ctx.emit_safepoint(Inst::TrapIf { trap_code, cond });
        }

        Opcode::Debugtrap => {
            ctx.emit(Inst::Debugtrap);
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
                        S390xABICaller::from_func(sig, &extname, dist, caller_conv, flags)?,
                        &inputs[..],
                    )
                }
                Opcode::CallIndirect => {
                    let ptr = put_input_in_reg(ctx, inputs[0], NarrowValueMode::ZeroExtend64);
                    let sig = ctx.call_sig(insn).unwrap();
                    assert!(inputs.len() - 1 == sig.params.len());
                    assert!(outputs.len() == sig.returns.len());
                    (
                        S390xABICaller::from_ptr(sig, ptr, op, caller_conv, flags)?,
                        &inputs[1..],
                    )
                }
                _ => unreachable!(),
            };

            assert!(inputs.len() == abi.num_args());
            for (i, input) in inputs.iter().enumerate() {
                let arg_reg = put_input_in_reg(ctx, *input, NarrowValueMode::None);
                abi.emit_copy_regs_to_arg(ctx, i, ValueRegs::one(arg_reg));
            }
            abi.emit_call(ctx);
            for (i, output) in outputs.iter().enumerate() {
                let retval_reg = get_output_reg(ctx, *output).only_reg().unwrap();
                abi.emit_copy_retval_to_regs(ctx, i, ValueRegs::one(retval_reg));
            }
            abi.accumulate_outgoing_args_size(ctx);
        }

        Opcode::FallthroughReturn | Opcode::Return => {
            for (i, input) in inputs.iter().enumerate() {
                let reg = put_input_in_reg(ctx, *input, NarrowValueMode::None);
                let retval_reg = ctx.retval(i).only_reg().unwrap();
                let ty = ctx.input_ty(insn, i);
                ctx.emit(Inst::gen_move(retval_reg, reg, ty));
            }
            // N.B.: the Ret itself is generated by the ABI.
        }

        Opcode::RawBitcast
        | Opcode::Splat
        | Opcode::Swizzle
        | Opcode::Insertlane
        | Opcode::Extractlane
        | Opcode::Imin
        | Opcode::Umin
        | Opcode::Imax
        | Opcode::Umax
        | Opcode::AvgRound
        | Opcode::FminPseudo
        | Opcode::FmaxPseudo
        | Opcode::Uload8x8
        | Opcode::Uload8x8Complex
        | Opcode::Sload8x8
        | Opcode::Sload8x8Complex
        | Opcode::Uload16x4
        | Opcode::Uload16x4Complex
        | Opcode::Sload16x4
        | Opcode::Sload16x4Complex
        | Opcode::Uload32x2
        | Opcode::Uload32x2Complex
        | Opcode::Sload32x2
        | Opcode::Sload32x2Complex
        | Opcode::Vconst
        | Opcode::Shuffle
        | Opcode::Vsplit
        | Opcode::Vconcat
        | Opcode::Vselect
        | Opcode::VanyTrue
        | Opcode::VallTrue
        | Opcode::VhighBits
        | Opcode::ScalarToVector
        | Opcode::Snarrow
        | Opcode::Unarrow
        | Opcode::Uunarrow
        | Opcode::SwidenLow
        | Opcode::SwidenHigh
        | Opcode::UwidenLow
        | Opcode::UwidenHigh
        | Opcode::WideningPairwiseDotProductS
        | Opcode::SqmulRoundSat
        | Opcode::FvpromoteLow
        | Opcode::Fvdemote
        | Opcode::IaddPairwise => {
            // TODO
            unimplemented!("Vector ops not implemented.");
        }

        Opcode::Isplit | Opcode::Iconcat => unimplemented!("Wide integer ops not implemented."),

        Opcode::IfcmpSp => {
            panic!("Unused opcode should not be encountered.");
        }

        Opcode::LoadComplex
        | Opcode::Uload8Complex
        | Opcode::Sload8Complex
        | Opcode::Uload16Complex
        | Opcode::Sload16Complex
        | Opcode::Uload32Complex
        | Opcode::Sload32Complex
        | Opcode::StoreComplex
        | Opcode::Istore8Complex
        | Opcode::Istore16Complex
        | Opcode::Istore32Complex => {
            panic!("Load/store complex opcode should not be encountered.");
        }

        Opcode::Ifcmp
        | Opcode::Ffcmp
        | Opcode::Trapff
        | Opcode::Trueif
        | Opcode::Trueff
        | Opcode::Selectif => {
            panic!("Flags opcode should not be encountered.");
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
    }

    Ok(())
}

//============================================================================
// Lowering: main entry point for lowering a branch group

fn lower_branch<C: LowerCtx<I = Inst>>(
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

        assert!(op1 == Opcode::Jump);
        let taken = targets[0];
        let not_taken = targets[1];

        match op0 {
            Opcode::Brz | Opcode::Brnz => {
                let flag_input = InsnInput {
                    insn: branches[0],
                    input: 0,
                };
                let cond = lower_boolean_to_flags(ctx, flag_input);
                let negated = op0 == Opcode::Brz;
                let cond = if negated { cond.invert() } else { cond };
                ctx.emit(Inst::CondBr {
                    taken,
                    not_taken,
                    cond,
                });
            }

            Opcode::Brif => {
                let condcode = ctx.data(branches[0]).cond_code().unwrap();
                let cond = Cond::from_intcc(condcode);
                let is_signed = condcode_is_signed(condcode);

                // Verification ensures that the input is always a single-def ifcmp.
                let cmp_insn = ctx
                    .get_input_as_source_or_const(branches[0], 0)
                    .inst
                    .unwrap()
                    .0;
                debug_assert_eq!(ctx.data(cmp_insn).opcode(), Opcode::Ifcmp);
                lower_icmp_to_flags(ctx, cmp_insn, is_signed, true);

                ctx.emit(Inst::CondBr {
                    taken,
                    not_taken,
                    cond,
                });
            }

            Opcode::Brff => unreachable!(),

            _ => unimplemented!(),
        }
    } else {
        // Must be an unconditional branch or an indirect branch.
        let op = ctx.data(branches[0]).opcode();
        match op {
            Opcode::Jump => {
                assert!(branches.len() == 1);
                ctx.emit(Inst::Jump { dest: targets[0] });
            }

            Opcode::BrTable => {
                let jt_size = targets.len() - 1;
                assert!(jt_size <= std::u32::MAX as usize);

                // Load up jump table element index.
                let ridx = put_input_in_reg(
                    ctx,
                    InsnInput {
                        insn: branches[0],
                        input: 0,
                    },
                    NarrowValueMode::ZeroExtend64,
                );

                // Bounds-check index and branch to default.
                // This is an internal branch that is not a terminator insn.
                // Instead, the default target is listed a potential target
                // in the final JTSequence, which is the block terminator.
                ctx.emit(Inst::CmpRUImm32 {
                    op: CmpOp::CmpL64,
                    rn: ridx,
                    imm: jt_size as u32,
                });
                ctx.emit(Inst::OneWayCondBr {
                    target: targets[0],
                    cond: Cond::from_intcc(IntCC::UnsignedGreaterThanOrEqual),
                });

                // Compute index scaled by entry size.
                let rtmp = ctx.alloc_tmp(types::I64).only_reg().unwrap();
                ctx.emit(Inst::ShiftRR {
                    shift_op: ShiftOp::LShL64,
                    rd: rtmp,
                    rn: ridx,
                    shift_imm: 2,
                    shift_reg: zero_reg(),
                });

                // Emit the compound instruction that does:
                //
                // larl %r1, <jt-base>
                // agf %r1, 0(%r1, %rTmp)
                // br %r1
                // [jt entries]
                //
                // This must be *one* instruction in the vcode because
                // we cannot allow regalloc to insert any spills/fills
                // in the middle of the sequence; otherwise, the ADR's
                // PC-rel offset to the jumptable would be incorrect.
                // (The alternative is to introduce a relocation pass
                // for inlined jumptables, which is much worse, IMHO.)

                ctx.emit(Inst::JTSequence {
                    ridx: rtmp.to_reg(),
                    targets: targets.to_vec(),
                });
            }

            _ => panic!("Unknown branch type!"),
        }
    }

    Ok(())
}

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for S390xBackend {
    type MInst = Inst;

    fn lower<C: LowerCtx<I = Inst>>(&self, ctx: &mut C, ir_inst: IRInst) -> CodegenResult<()> {
        lower_insn_to_regs(ctx, ir_inst, &self.flags, &self.isa_flags)
    }

    fn lower_branch_group<C: LowerCtx<I = Inst>>(
        &self,
        ctx: &mut C,
        branches: &[IRInst],
        targets: &[MachLabel],
    ) -> CodegenResult<()> {
        lower_branch(ctx, branches, targets)
    }
}
