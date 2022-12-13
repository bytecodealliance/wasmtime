//! Lowering rules for AArch64.
//!
//! TODO: opportunities for better code generation:
//!
//! - Smarter use of addressing modes. Recognize a+SCALE*b patterns. Recognize
//!   pre/post-index opportunities.
//!
//! - Floating-point immediates (FIMM instruction).

use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::types::*;
use crate::ir::Inst as IRInst;
use crate::ir::{Opcode, Type, Value};
use crate::isa::aarch64::inst::*;
use crate::isa::aarch64::AArch64Backend;
use crate::machinst::lower::*;
use crate::machinst::{Reg, Writable};
use crate::CodegenError;
use crate::CodegenResult;
use crate::{machinst::*, trace};
use smallvec::{smallvec, SmallVec};

pub mod isle;

//============================================================================
// Lowering: convert instruction inputs to forms that we can use.

/// How to handle narrow values loaded into registers; see note on `narrow_mode`
/// parameter to `put_input_in_*` below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NarrowValueMode {
    None,
    /// Zero-extend to 64 bits if original is < 64 bits.
    ZeroExtend64,
}

impl NarrowValueMode {
    fn is_32bit(&self) -> bool {
        match self {
            NarrowValueMode::None => false,
            NarrowValueMode::ZeroExtend64 => false,
        }
    }
}

/// Extends a register according to `narrow_mode`.
/// If extended, the value is always extended to 64 bits, for simplicity.
fn extend_reg(
    ctx: &mut Lower<Inst>,
    ty: Type,
    in_reg: Reg,
    is_const: bool,
    narrow_mode: NarrowValueMode,
) -> Reg {
    let from_bits = ty_bits(ty) as u8;
    match (narrow_mode, from_bits) {
        (NarrowValueMode::None, _) => in_reg,

        (NarrowValueMode::ZeroExtend64, n) if n < 64 => {
            if is_const {
                // Constants are zero-extended to full 64-bit width on load already.
                in_reg
            } else {
                let tmp = ctx.alloc_tmp(I32).only_reg().unwrap();
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
        (_, 64) => in_reg,
        (_, 128) => in_reg,

        _ => panic!(
            "Unsupported input width: input ty {} bits {} mode {:?}",
            ty, from_bits, narrow_mode
        ),
    }
}

/// Lowers an instruction input to multiple regs
fn lower_value_to_regs(ctx: &mut Lower<Inst>, value: Value) -> (ValueRegs<Reg>, Type, bool) {
    trace!("lower_value_to_regs: value {:?}", value);
    let ty = ctx.value_ty(value);
    let inputs = ctx.get_value_as_source_or_const(value);
    let is_const = inputs.constant.is_some();

    let in_regs = if let Some(c) = inputs.constant {
        // Generate constants fresh at each use to minimize long-range register pressure.
        let from_bits = ty_bits(ty);
        let c = if from_bits < 64 {
            c & ((1u64 << from_bits) - 1)
        } else {
            c
        };
        match ty {
            I8 | I16 | I32 | I64 | R32 | R64 => {
                let cst_copy = ctx.alloc_tmp(ty);
                lower_constant_u64(ctx, cst_copy.only_reg().unwrap(), c);
                non_writable_value_regs(cst_copy)
            }
            _ => unreachable!(), // Only used for addresses.
        }
    } else {
        ctx.put_value_in_regs(value)
    };

    (in_regs, ty, is_const)
}

/// Lower an instruction input to a register
///
/// The given register will be extended appropriately, according to
/// `narrow_mode` and the input's type. If extended, the value is
/// always extended to 64 bits, for simplicity.
pub(crate) fn put_input_in_reg(
    ctx: &mut Lower<Inst>,
    input: InsnInput,
    narrow_mode: NarrowValueMode,
) -> Reg {
    let value = ctx.input_as_value(input.insn, input.input);
    put_value_in_reg(ctx, value, narrow_mode)
}

/// Like above, only for values
fn put_value_in_reg(ctx: &mut Lower<Inst>, value: Value, narrow_mode: NarrowValueMode) -> Reg {
    let (in_regs, ty, is_const) = lower_value_to_regs(ctx, value);
    let reg = in_regs
        .only_reg()
        .expect("Multi-register value not expected");

    extend_reg(ctx, ty, reg, is_const, narrow_mode)
}

fn get_as_extended_value(
    ctx: &mut Lower<Inst>,
    val: Value,
    narrow_mode: NarrowValueMode,
) -> Option<(Value, ExtendOp)> {
    let inputs = ctx.get_value_as_source_or_const(val);
    let (insn, n) = inputs.inst.as_inst()?;
    if n != 0 {
        return None;
    }
    let op = ctx.data(insn).opcode();
    let out_ty = ctx.output_ty(insn, 0);
    let out_bits = ty_bits(out_ty);

    // Is this a zero-extend or sign-extend and can we handle that with a register-mode operator?
    if op == Opcode::Uextend || op == Opcode::Sextend {
        let sign_extend = op == Opcode::Sextend;
        let inner_ty = ctx.input_ty(insn, 0);
        let inner_bits = ty_bits(inner_ty);
        assert!(inner_bits < out_bits);
        if match (sign_extend, narrow_mode) {
            // A single zero-extend or sign-extend is equal to itself.
            (_, NarrowValueMode::None) => true,
            // Two zero-extends or sign-extends in a row is equal to a single zero-extend or sign-extend.
            (false, NarrowValueMode::ZeroExtend64) => true,
            (true, NarrowValueMode::ZeroExtend64) => false,
        } {
            let extendop = match (sign_extend, inner_bits) {
                (true, 8) => ExtendOp::SXTB,
                (false, 8) => ExtendOp::UXTB,
                (true, 16) => ExtendOp::SXTH,
                (false, 16) => ExtendOp::UXTH,
                (true, 32) => ExtendOp::SXTW,
                (false, 32) => ExtendOp::UXTW,
                _ => unreachable!(),
            };
            return Some((ctx.input_as_value(insn, 0), extendop));
        }
    }

    // If `out_ty` is smaller than 32 bits and we need to zero- or sign-extend,
    // then get the result into a register and return an Extend-mode operand on
    // that register.
    if narrow_mode != NarrowValueMode::None
        && ((narrow_mode.is_32bit() && out_bits < 32) || (!narrow_mode.is_32bit() && out_bits < 64))
    {
        let extendop = match (narrow_mode, out_bits) {
            (NarrowValueMode::ZeroExtend64, 1) => ExtendOp::UXTB,
            (NarrowValueMode::ZeroExtend64, 8) => ExtendOp::UXTB,
            (NarrowValueMode::ZeroExtend64, 16) => ExtendOp::UXTH,
            (NarrowValueMode::ZeroExtend64, 32) => ExtendOp::UXTW,
            _ => unreachable!(),
        };
        return Some((val, extendop));
    }
    None
}

//============================================================================
// Lowering: addressing mode support. Takes instruction directly, rather
// than an `InsnInput`, to do more introspection.

/// 32-bit addends that make up an address: an input, and an extension mode on that
/// input.
type AddressAddend32List = SmallVec<[(Reg, ExtendOp); 4]>;
/// 64-bit addends that make up an address: just an input.
type AddressAddend64List = SmallVec<[Reg; 4]>;

/// Collect all addends that feed into an address computation, with extend-modes
/// on each.  Note that a load/store may have multiple address components (and
/// the CLIF semantics are that these components are added to form the final
/// address), but sometimes the CLIF that we receive still has arguments that
/// refer to `iadd` instructions. We also want to handle uextend/sextend below
/// the add(s).
///
/// We match any 64-bit add (and descend into its inputs), and we match any
/// 32-to-64-bit sign or zero extension. The returned addend-list will use
/// NarrowValueMode values to indicate how to extend each input:
///
/// - NarrowValueMode::None: the associated input is 64 bits wide; no extend.
/// - NarrowValueMode::SignExtend64: the associated input is 32 bits wide;
///                                  do a sign-extension.
/// - NarrowValueMode::ZeroExtend64: the associated input is 32 bits wide;
///                                  do a zero-extension.
///
/// We do not descend further into the inputs of extensions (unless it is a constant),
/// because supporting (e.g.) a 32-bit add that is later extended would require
/// additional masking of high-order bits, which is too complex. So, in essence, we
/// descend any number of adds from the roots, collecting all 64-bit address addends;
/// then possibly support extensions at these leaves.
fn collect_address_addends(
    ctx: &mut Lower<Inst>,
    root: Value,
) -> (AddressAddend64List, AddressAddend32List, i64) {
    let mut result32: AddressAddend32List = SmallVec::new();
    let mut result64: AddressAddend64List = SmallVec::new();
    let mut offset: i64 = 0;

    let mut workqueue: SmallVec<[Value; 4]> = smallvec![root];

    while let Some(value) = workqueue.pop() {
        debug_assert_eq!(ty_bits(ctx.value_ty(value)), 64);
        if let Some((op, insn)) = maybe_value_multi(
            ctx,
            value,
            &[
                Opcode::Uextend,
                Opcode::Sextend,
                Opcode::Iadd,
                Opcode::Iconst,
            ],
        ) {
            match op {
                Opcode::Uextend | Opcode::Sextend if ty_bits(ctx.input_ty(insn, 0)) == 32 => {
                    let extendop = if op == Opcode::Uextend {
                        ExtendOp::UXTW
                    } else {
                        ExtendOp::SXTW
                    };
                    let extendee_input = InsnInput { insn, input: 0 };
                    // If the input is a zero-extension of a constant, add the value to the known
                    // offset.
                    // Only do this for zero-extension, as generating a sign-extended
                    // constant may be more instructions than using the 'SXTW' addressing mode.
                    if let (Some(insn), ExtendOp::UXTW) = (
                        maybe_input_insn(ctx, extendee_input, Opcode::Iconst),
                        extendop,
                    ) {
                        let value = (ctx.get_constant(insn).unwrap() & 0xFFFF_FFFF_u64) as i64;
                        offset += value;
                    } else {
                        let reg = put_input_in_reg(ctx, extendee_input, NarrowValueMode::None);
                        result32.push((reg, extendop));
                    }
                }
                Opcode::Uextend | Opcode::Sextend => {
                    let reg = put_value_in_reg(ctx, value, NarrowValueMode::None);
                    result64.push(reg);
                }
                Opcode::Iadd => {
                    for input in 0..ctx.num_inputs(insn) {
                        let addend = ctx.input_as_value(insn, input);
                        workqueue.push(addend);
                    }
                }
                Opcode::Iconst => {
                    let value: i64 = ctx.get_constant(insn).unwrap() as i64;
                    offset += value;
                }
                _ => panic!("Unexpected opcode from maybe_input_insn_multi"),
            }
        } else {
            let reg = put_value_in_reg(ctx, value, NarrowValueMode::ZeroExtend64);
            result64.push(reg);
        }
    }

    (result64, result32, offset)
}

/// Lower the address of a pair load or store.
pub(crate) fn lower_pair_address(ctx: &mut Lower<Inst>, addr: Value, offset: i32) -> PairAMode {
    // Collect addends through an arbitrary tree of 32-to-64-bit sign/zero
    // extends and addition ops. We update these as we consume address
    // components, so they represent the remaining addends not yet handled.
    let (mut addends64, mut addends32, args_offset) = collect_address_addends(ctx, addr);
    let offset = args_offset + (offset as i64);

    trace!(
        "lower_pair_address: addends64 {:?}, addends32 {:?}, offset {}",
        addends64,
        addends32,
        offset
    );

    // Pairs basically only have reg + imm formats so we only have to worry about those

    let base_reg = if let Some(reg64) = addends64.pop() {
        reg64
    } else if let Some((reg32, extendop)) = addends32.pop() {
        let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
        let signed = match extendop {
            ExtendOp::SXTW => true,
            ExtendOp::UXTW => false,
            _ => unreachable!(),
        };
        ctx.emit(Inst::Extend {
            rd: tmp,
            rn: reg32,
            signed,
            from_bits: 32,
            to_bits: 64,
        });
        tmp.to_reg()
    } else {
        zero_reg()
    };

    // We have the base register, if we have any others, we need to add them
    let addr = lower_add_addends(ctx, base_reg, addends64, addends32);

    // Figure out what offset we should emit
    let (addr, imm7) = if let Some(imm7) = SImm7Scaled::maybe_from_i64(offset, I64) {
        (addr, imm7)
    } else {
        let res = lower_add_immediate(ctx, addr, offset);
        (res, SImm7Scaled::maybe_from_i64(0, I64).unwrap())
    };

    PairAMode::SignedOffset(addr, imm7)
}

/// Lower the address of a load or store.
pub(crate) fn lower_address(
    ctx: &mut Lower<Inst>,
    elem_ty: Type,
    addr: Value,
    offset: i32,
) -> AMode {
    // TODO: support base_reg + scale * index_reg. For this, we would need to
    // pattern-match shl or mul instructions.

    // Collect addends through an arbitrary tree of 32-to-64-bit sign/zero
    // extends and addition ops. We update these as we consume address
    // components, so they represent the remaining addends not yet handled.
    let (mut addends64, mut addends32, args_offset) = collect_address_addends(ctx, addr);
    let mut offset = args_offset + (offset as i64);

    trace!(
        "lower_address: addends64 {:?}, addends32 {:?}, offset {}",
        addends64,
        addends32,
        offset
    );

    // First, decide what the `AMode` will be. Take one extendee and one 64-bit
    // reg, or two 64-bit regs, or a 64-bit reg and a 32-bit reg with extension,
    // or some other combination as appropriate.
    let memarg = if addends64.len() > 0 {
        if addends32.len() > 0 {
            let (reg32, extendop) = addends32.pop().unwrap();
            let reg64 = addends64.pop().unwrap();
            AMode::RegExtended {
                rn: reg64,
                rm: reg32,
                extendop,
            }
        } else if offset > 0 && offset < 0x1000 {
            let reg64 = addends64.pop().unwrap();
            let off = offset;
            offset = 0;
            AMode::RegOffset {
                rn: reg64,
                off,
                ty: elem_ty,
            }
        } else if addends64.len() >= 2 {
            let reg1 = addends64.pop().unwrap();
            let reg2 = addends64.pop().unwrap();
            AMode::RegReg { rn: reg1, rm: reg2 }
        } else {
            let reg1 = addends64.pop().unwrap();
            AMode::reg(reg1)
        }
    } else
    /* addends64.len() == 0 */
    {
        if addends32.len() > 0 {
            let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
            let (reg1, extendop) = addends32.pop().unwrap();
            let signed = match extendop {
                ExtendOp::SXTW => true,
                ExtendOp::UXTW => false,
                _ => unreachable!(),
            };
            ctx.emit(Inst::Extend {
                rd: tmp,
                rn: reg1,
                signed,
                from_bits: 32,
                to_bits: 64,
            });
            if let Some((reg2, extendop)) = addends32.pop() {
                AMode::RegExtended {
                    rn: tmp.to_reg(),
                    rm: reg2,
                    extendop,
                }
            } else {
                AMode::reg(tmp.to_reg())
            }
        } else
        /* addends32.len() == 0 */
        {
            let off_reg = ctx.alloc_tmp(I64).only_reg().unwrap();
            lower_constant_u64(ctx, off_reg, offset as u64);
            offset = 0;
            AMode::reg(off_reg.to_reg())
        }
    };

    // At this point, if we have any remaining components, we need to allocate a
    // temp, replace one of the registers in the AMode with the temp, and emit
    // instructions to add together the remaining components. Return immediately
    // if this is *not* the case.
    if offset == 0 && addends32.len() == 0 && addends64.len() == 0 {
        return memarg;
    }

    // Extract the first register from the memarg so that we can add all the
    // immediate values to it.
    let addr = match memarg {
        AMode::RegExtended { rn, .. } => rn,
        AMode::RegOffset { rn, .. } => rn,
        AMode::RegReg { rm, .. } => rm,
        AMode::UnsignedOffset { rn, .. } => rn,
        _ => unreachable!(),
    };

    // If there is any offset, load that first into `addr`, and add the `reg`
    // that we kicked out of the `AMode`; otherwise, start with that reg.
    let addr = if offset != 0 {
        lower_add_immediate(ctx, addr, offset)
    } else {
        addr
    };

    // Now handle reg64 and reg32-extended components.
    let addr = lower_add_addends(ctx, addr, addends64, addends32);

    // Shoehorn addr into the AMode.
    match memarg {
        AMode::RegExtended { rm, extendop, .. } => AMode::RegExtended {
            rn: addr,
            rm,
            extendop,
        },
        AMode::RegOffset { off, ty, .. } => AMode::RegOffset { rn: addr, off, ty },
        AMode::RegReg { rn, .. } => AMode::RegReg { rn: addr, rm: rn },
        AMode::UnsignedOffset { uimm12, .. } => AMode::UnsignedOffset { rn: addr, uimm12 },
        _ => unreachable!(),
    }
}

fn lower_add_addends(
    ctx: &mut Lower<Inst>,
    init: Reg,
    addends64: AddressAddend64List,
    addends32: AddressAddend32List,
) -> Reg {
    let init = addends64.into_iter().fold(init, |prev, reg| {
        // If the register is the stack reg, we must move it to another reg
        // before adding it.
        let reg = if reg == stack_reg() {
            let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
            ctx.emit(Inst::gen_move(tmp, stack_reg(), I64));
            tmp.to_reg()
        } else {
            reg
        };

        let rd = ctx.alloc_tmp(I64).only_reg().unwrap();

        ctx.emit(Inst::AluRRR {
            alu_op: ALUOp::Add,
            size: OperandSize::Size64,
            rd,
            rn: prev,
            rm: reg,
        });

        rd.to_reg()
    });

    addends32.into_iter().fold(init, |prev, (reg, extendop)| {
        assert!(reg != stack_reg());

        let rd = ctx.alloc_tmp(I64).only_reg().unwrap();

        ctx.emit(Inst::AluRRRExtend {
            alu_op: ALUOp::Add,
            size: OperandSize::Size64,
            rd,
            rn: prev,
            rm: reg,
            extendop,
        });

        rd.to_reg()
    })
}

/// Adds into `rd` a signed imm pattern matching the best instruction for it.
// TODO: This function is duplicated in ctx.gen_add_imm
fn lower_add_immediate(ctx: &mut Lower<Inst>, src: Reg, imm: i64) -> Reg {
    let dst = ctx.alloc_tmp(I64).only_reg().unwrap();

    // If we can fit offset or -offset in an imm12, use an add-imm
    // Otherwise, lower the constant first then add.
    if let Some(imm12) = Imm12::maybe_from_u64(imm as u64) {
        ctx.emit(Inst::AluRRImm12 {
            alu_op: ALUOp::Add,
            size: OperandSize::Size64,
            rd: dst,
            rn: src,
            imm12,
        });
    } else if let Some(imm12) = Imm12::maybe_from_u64(imm.wrapping_neg() as u64) {
        ctx.emit(Inst::AluRRImm12 {
            alu_op: ALUOp::Sub,
            size: OperandSize::Size64,
            rd: dst,
            rn: src,
            imm12,
        });
    } else {
        let tmp = ctx.alloc_tmp(I64).only_reg().unwrap();
        lower_constant_u64(ctx, tmp, imm as u64);
        ctx.emit(Inst::AluRRR {
            alu_op: ALUOp::Add,
            size: OperandSize::Size64,
            rd: dst,
            rn: tmp.to_reg(),
            rm: src,
        });
    }

    dst.to_reg()
}

pub(crate) fn lower_constant_u64(ctx: &mut Lower<Inst>, rd: Writable<Reg>, value: u64) {
    for inst in Inst::load_constant(rd, value, &mut |ty| ctx.alloc_tmp(ty).only_reg().unwrap()) {
        ctx.emit(inst);
    }
}

pub(crate) fn lower_constant_f32(ctx: &mut Lower<Inst>, rd: Writable<Reg>, value: f32) {
    let alloc_tmp = |ty| ctx.alloc_tmp(ty).only_reg().unwrap();

    for inst in Inst::load_fp_constant32(rd, value.to_bits(), alloc_tmp) {
        ctx.emit(inst);
    }
}

pub(crate) fn lower_constant_f64(ctx: &mut Lower<Inst>, rd: Writable<Reg>, value: f64) {
    let alloc_tmp = |ty| ctx.alloc_tmp(ty).only_reg().unwrap();

    for inst in Inst::load_fp_constant64(rd, value.to_bits(), alloc_tmp) {
        ctx.emit(inst);
    }
}

pub(crate) fn lower_constant_f128(ctx: &mut Lower<Inst>, rd: Writable<Reg>, value: u128) {
    if value == 0 {
        // Fast-track a common case.  The general case, viz, calling `Inst::load_fp_constant128`,
        // is potentially expensive.
        ctx.emit(Inst::VecDupImm {
            rd,
            imm: ASIMDMovModImm::zero(ScalarSize::Size8),
            invert: false,
            size: VectorSize::Size8x16,
        });
    } else {
        let alloc_tmp = |ty| ctx.alloc_tmp(ty).only_reg().unwrap();
        for inst in Inst::load_fp_constant128(rd, value, alloc_tmp) {
            ctx.emit(inst);
        }
    }
}

pub(crate) fn lower_splat_const(
    ctx: &mut Lower<Inst>,
    rd: Writable<Reg>,
    value: u64,
    size: VectorSize,
) {
    let (value, narrow_size) = match size.lane_size() {
        ScalarSize::Size8 => (value as u8 as u64, ScalarSize::Size128),
        ScalarSize::Size16 => (value as u16 as u64, ScalarSize::Size8),
        ScalarSize::Size32 => (value as u32 as u64, ScalarSize::Size16),
        ScalarSize::Size64 => (value, ScalarSize::Size32),
        _ => unreachable!(),
    };
    let (value, size) = match Inst::get_replicated_vector_pattern(value as u128, narrow_size) {
        Some((value, lane_size)) => (
            value,
            VectorSize::from_lane_size(lane_size, size.is_128bits()),
        ),
        None => (value, size),
    };
    let alloc_tmp = |ty| ctx.alloc_tmp(ty).only_reg().unwrap();

    for inst in Inst::load_replicated_vector_pattern(rd, value, size, alloc_tmp) {
        ctx.emit(inst);
    }
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

//=============================================================================
// Helpers for instruction lowering.

/// Checks for an instance of `op` feeding the given input.
pub(crate) fn maybe_input_insn(
    c: &mut Lower<Inst>,
    input: InsnInput,
    op: Opcode,
) -> Option<IRInst> {
    let inputs = c.get_input_as_source_or_const(input.insn, input.input);
    trace!(
        "maybe_input_insn: input {:?} has options {:?}; looking for op {:?}",
        input,
        inputs,
        op
    );
    if let Some((src_inst, _)) = inputs.inst.as_inst() {
        let data = c.data(src_inst);
        trace!(" -> input inst {:?}", data);
        if data.opcode() == op {
            return Some(src_inst);
        }
    }
    None
}

/// Checks for an instance of `op` defining the given value.
pub(crate) fn maybe_value(c: &mut Lower<Inst>, value: Value, op: Opcode) -> Option<IRInst> {
    let inputs = c.get_value_as_source_or_const(value);
    if let Some((src_inst, _)) = inputs.inst.as_inst() {
        let data = c.data(src_inst);
        if data.opcode() == op {
            return Some(src_inst);
        }
    }
    None
}

/// Checks for an instance of any one of `ops` defining the given value.
pub(crate) fn maybe_value_multi(
    c: &mut Lower<Inst>,
    value: Value,
    ops: &[Opcode],
) -> Option<(Opcode, IRInst)> {
    for &op in ops {
        if let Some(inst) = maybe_value(c, value, op) {
            return Some((op, inst));
        }
    }
    None
}

//=============================================================================
// Lowering-backend trait implementation.

impl LowerBackend for AArch64Backend {
    type MInst = Inst;

    fn lower(&self, ctx: &mut Lower<Inst>, ir_inst: IRInst) -> CodegenResult<InstOutput> {
        if let Some(temp_regs) = super::lower::isle::lower(ctx, self, ir_inst) {
            return Ok(temp_regs);
        }

        let op = ctx.data(ir_inst).opcode();
        let ty = if ctx.num_outputs(ir_inst) > 0 {
            Some(ctx.output_ty(ir_inst, 0))
        } else {
            None
        };

        match op {
            Opcode::Iconst
            | Opcode::Null
            | Opcode::F32const
            | Opcode::F64const
            | Opcode::GetFramePointer
            | Opcode::GetStackPointer
            | Opcode::GetReturnAddress
            | Opcode::Iadd
            | Opcode::Isub
            | Opcode::UaddSat
            | Opcode::SaddSat
            | Opcode::UsubSat
            | Opcode::SsubSat
            | Opcode::Ineg
            | Opcode::Imul
            | Opcode::Umulhi
            | Opcode::Smulhi
            | Opcode::Udiv
            | Opcode::Sdiv
            | Opcode::Urem
            | Opcode::Srem
            | Opcode::Uextend
            | Opcode::Sextend
            | Opcode::Bnot
            | Opcode::Band
            | Opcode::Bor
            | Opcode::Bxor
            | Opcode::BandNot
            | Opcode::BorNot
            | Opcode::BxorNot
            | Opcode::Ishl
            | Opcode::Ushr
            | Opcode::Sshr
            | Opcode::Rotr
            | Opcode::Rotl
            | Opcode::Bitrev
            | Opcode::Clz
            | Opcode::Cls
            | Opcode::Ctz
            | Opcode::Bswap
            | Opcode::Popcnt
            | Opcode::Load
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
            | Opcode::Uload32x2
            | Opcode::Store
            | Opcode::Istore8
            | Opcode::Istore16
            | Opcode::Istore32
            | Opcode::StackAddr
            | Opcode::DynamicStackAddr
            | Opcode::AtomicRmw
            | Opcode::AtomicCas
            | Opcode::AtomicLoad
            | Opcode::AtomicStore
            | Opcode::Fence
            | Opcode::Nop
            | Opcode::Select
            | Opcode::SelectSpectreGuard
            | Opcode::Bitselect
            | Opcode::Vselect
            | Opcode::IsNull
            | Opcode::IsInvalid
            | Opcode::Ireduce
            | Opcode::Bmask
            | Opcode::Bitcast
            | Opcode::Return
            | Opcode::Icmp
            | Opcode::Fcmp
            | Opcode::Debugtrap
            | Opcode::Trap
            | Opcode::ResumableTrap
            | Opcode::FuncAddr
            | Opcode::SymbolValue
            | Opcode::Call
            | Opcode::CallIndirect
            | Opcode::GetPinnedReg
            | Opcode::SetPinnedReg
            | Opcode::Vconst
            | Opcode::Extractlane
            | Opcode::Insertlane
            | Opcode::Splat
            | Opcode::ScalarToVector
            | Opcode::VallTrue
            | Opcode::VanyTrue
            | Opcode::VhighBits
            | Opcode::Shuffle
            | Opcode::Swizzle
            | Opcode::Isplit
            | Opcode::Iconcat
            | Opcode::Smax
            | Opcode::Umax
            | Opcode::Umin
            | Opcode::Smin
            | Opcode::IaddPairwise
            | Opcode::WideningPairwiseDotProductS
            | Opcode::Fadd
            | Opcode::Fsub
            | Opcode::Fmul
            | Opcode::Fdiv
            | Opcode::Fmin
            | Opcode::Fmax
            | Opcode::FminPseudo
            | Opcode::FmaxPseudo
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
            | Opcode::FcvtToUint
            | Opcode::FcvtToSint
            | Opcode::FcvtFromUint
            | Opcode::FcvtFromSint
            | Opcode::FcvtToUintSat
            | Opcode::FcvtToSintSat
            | Opcode::UaddOverflowTrap
            | Opcode::IaddCout
            | Opcode::Iabs
            | Opcode::AvgRound
            | Opcode::Snarrow
            | Opcode::Unarrow
            | Opcode::Uunarrow
            | Opcode::SwidenLow
            | Opcode::SwidenHigh
            | Opcode::UwidenLow
            | Opcode::UwidenHigh
            | Opcode::TlsValue
            | Opcode::SqmulRoundSat
            | Opcode::FcvtLowFromSint
            | Opcode::FvpromoteLow
            | Opcode::Fvdemote
            | Opcode::ExtractVector => {
                unreachable!(
                    "implemented in ISLE: inst = `{}`, type = `{:?}`",
                    ctx.dfg().display_inst(ir_inst),
                    ty
                );
            }

            Opcode::StackLoad
            | Opcode::StackStore
            | Opcode::DynamicStackStore
            | Opcode::DynamicStackLoad => {
                panic!("Direct stack memory access not supported; should not be used by Wasm");
            }
            Opcode::HeapLoad | Opcode::HeapStore | Opcode::HeapAddr => {
                panic!("heap access instructions should have been removed by legalization!");
            }
            Opcode::TableAddr => {
                panic!("table_addr should have been removed by legalization!");
            }
            Opcode::Trapz | Opcode::Trapnz | Opcode::ResumableTrapnz => {
                panic!(
                    "trapz / trapnz / resumable_trapnz should have been removed by legalization!"
                );
            }
            Opcode::GlobalValue => {
                panic!("global_value should have been removed by legalization!");
            }
            Opcode::Jump | Opcode::Brz | Opcode::Brnz | Opcode::BrTable => {
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
            | Opcode::IaddCarry
            | Opcode::IsubBin
            | Opcode::IsubBout
            | Opcode::IsubBorrow
            | Opcode::BandImm
            | Opcode::BorImm
            | Opcode::BxorImm
            | Opcode::RotlImm
            | Opcode::RotrImm
            | Opcode::IshlImm
            | Opcode::UshrImm
            | Opcode::SshrImm
            | Opcode::IcmpImm => {
                panic!("ALU+imm and ALU+carry ops should not appear here!");
            }

            Opcode::Vconcat | Opcode::Vsplit => {
                return Err(CodegenError::Unsupported(format!(
                    "Unimplemented lowering: {}",
                    op
                )));
            }
        }
    }

    fn lower_branch_group(
        &self,
        ctx: &mut Lower<Inst>,
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
            let op1 = ctx.data(branches[1]).opcode();
            assert!(op1 == Opcode::Jump);
        }

        if let Some(temp_regs) = super::lower::isle::lower_branch(ctx, self, branches[0], targets) {
            assert!(temp_regs.len() == 0);
            return Ok(());
        }

        unreachable!(
            "implemented in ISLE: branch = `{}`",
            ctx.dfg().display_inst(branches[0]),
        );
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        Some(regs::pinned_reg())
    }
}
