//! Encoding tables for x86 ISAs.

use super::registers::*;
use crate::bitset::BitSet;
use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::types::*;
use crate::ir::{self, Function, Inst, InstBuilder};
use crate::isa::constraints::*;
use crate::isa::enc_tables::*;
use crate::isa::encoding::base_size;
use crate::isa::encoding::{Encoding, RecipeSizing};
use crate::isa::RegUnit;
use crate::isa::{self, TargetIsa};
use crate::predicates;
use crate::regalloc::RegDiversions;

use cranelift_codegen_shared::isa::x86::EncodingBits;

include!(concat!(env!("OUT_DIR"), "/encoding-x86.rs"));
include!(concat!(env!("OUT_DIR"), "/legalize-x86.rs"));

/// Whether the REX prefix is needed for encoding extended registers (via REX.RXB).
///
/// Normal x86 instructions have only 3 bits for encoding a register.
/// The REX prefix adds REX.R, REX,X, and REX.B bits, interpreted as fourth bits.
pub fn is_extended_reg(reg: RegUnit) -> bool {
    // Extended registers have the fourth bit set.
    reg as u8 & 0b1000 != 0
}

pub fn needs_sib_byte(reg: RegUnit) -> bool {
    reg == RU::r12 as RegUnit || reg == RU::rsp as RegUnit
}
pub fn needs_offset(reg: RegUnit) -> bool {
    reg == RU::r13 as RegUnit || reg == RU::rbp as RegUnit
}
pub fn needs_sib_byte_or_offset(reg: RegUnit) -> bool {
    needs_sib_byte(reg) || needs_offset(reg)
}

fn test_input(
    op_index: usize,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
    condition_func: fn(RegUnit) -> bool,
) -> bool {
    let in_reg = divert.reg(func.dfg.inst_args(inst)[op_index], &func.locations);
    condition_func(in_reg)
}

fn test_result(
    result_index: usize,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
    condition_func: fn(RegUnit) -> bool,
) -> bool {
    let out_reg = divert.reg(func.dfg.inst_results(inst)[result_index], &func.locations);
    condition_func(out_reg)
}

fn size_plus_maybe_offset_for_inreg_0(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_offset = test_input(0, inst, divert, func, needs_offset);
    sizing.base_size + if needs_offset { 1 } else { 0 }
}
fn size_plus_maybe_offset_for_inreg_1(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_offset = test_input(1, inst, divert, func, needs_offset);
    sizing.base_size + if needs_offset { 1 } else { 0 }
}
fn size_plus_maybe_sib_for_inreg_0(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_sib = test_input(0, inst, divert, func, needs_sib_byte);
    sizing.base_size + if needs_sib { 1 } else { 0 }
}
fn size_plus_maybe_sib_for_inreg_1(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_sib = test_input(1, inst, divert, func, needs_sib_byte);
    sizing.base_size + if needs_sib { 1 } else { 0 }
}
fn size_plus_maybe_sib_or_offset_for_inreg_0(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_sib_or_offset = test_input(0, inst, divert, func, needs_sib_byte_or_offset);
    sizing.base_size + if needs_sib_or_offset { 1 } else { 0 }
}
fn size_plus_maybe_sib_or_offset_for_inreg_1(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_sib_or_offset = test_input(1, inst, divert, func, needs_sib_byte_or_offset);
    sizing.base_size + if needs_sib_or_offset { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, for use with one input reg.
///
/// A REX prefix is known to be emitted if either:
///  1. The EncodingBits specify that REX.W is to be set.
///  2. Registers are used that require REX.R or REX.B bits for encoding.
fn size_with_inferred_rex_for_inreg0(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_rex = (EncodingBits::from(enc.bits()).rex_w() != 0)
        || test_input(0, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, based on the second operand.
fn size_with_inferred_rex_for_inreg1(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_rex = (EncodingBits::from(enc.bits()).rex_w() != 0)
        || test_input(1, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, based on the third operand.
fn size_with_inferred_rex_for_inreg2(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_rex = (EncodingBits::from(enc.bits()).rex_w() != 0)
        || test_input(2, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, for use with two input registers.
///
/// A REX prefix is known to be emitted if either:
///  1. The EncodingBits specify that REX.W is to be set.
///  2. Registers are used that require REX.R or REX.B bits for encoding.
fn size_with_inferred_rex_for_inreg0_inreg1(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_rex = (EncodingBits::from(enc.bits()).rex_w() != 0)
        || test_input(0, inst, divert, func, is_extended_reg)
        || test_input(1, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, based on a single
/// input register and a single output register.
fn size_with_inferred_rex_for_inreg0_outreg0(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_rex = (EncodingBits::from(enc.bits()).rex_w() != 0)
        || test_input(0, inst, divert, func, is_extended_reg)
        || test_result(0, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, for use with CMOV.
///
/// CMOV uses 3 inputs, with the REX is inferred from reg1 and reg2.
fn size_with_inferred_rex_for_cmov(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    let needs_rex = (EncodingBits::from(enc.bits()).rex_w() != 0)
        || test_input(1, inst, divert, func, is_extended_reg)
        || test_input(2, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// If the value's definition is a constant immediate, returns its unpacked value, or None
/// otherwise.
fn maybe_iconst_imm(pos: &FuncCursor, value: ir::Value) -> Option<i64> {
    if let ir::ValueDef::Result(inst, _) = &pos.func.dfg.value_def(value) {
        if let ir::InstructionData::UnaryImm {
            opcode: ir::Opcode::Iconst,
            imm,
        } = &pos.func.dfg[*inst]
        {
            let value: i64 = (*imm).into();
            Some(value)
        } else {
            None
        }
    } else {
        None
    }
}

/// Expand the `sdiv` and `srem` instructions using `x86_sdivmodx`.
fn expand_sdivrem(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    let (x, y, is_srem) = match func.dfg[inst] {
        ir::InstructionData::Binary {
            opcode: ir::Opcode::Sdiv,
            args,
        } => (args[0], args[1], false),
        ir::InstructionData::Binary {
            opcode: ir::Opcode::Srem,
            args,
        } => (args[0], args[1], true),
        _ => panic!("Need sdiv/srem: {}", func.dfg.display_inst(inst, None)),
    };

    let old_ebb = func.layout.pp_ebb(inst);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);
    pos.func.dfg.clear_results(inst);

    let avoid_div_traps = isa.flags().avoid_div_traps();

    // If we can tolerate native division traps, sdiv doesn't need branching.
    if !avoid_div_traps && !is_srem {
        let xhi = pos.ins().sshr_imm(x, i64::from(ty.lane_bits()) - 1);
        pos.ins().with_result(result).x86_sdivmodx(x, xhi, y);
        pos.remove_inst();
        return;
    }

    // Try to remove checks if the input value is an immediate other than 0 or -1. For these two
    // immediates, we'd ideally replace conditional traps by traps, but this requires more
    // manipulation of the dfg/cfg, which is out of scope here.
    let (could_be_zero, could_be_minus_one) = if let Some(imm) = maybe_iconst_imm(&pos, y) {
        (imm == 0, imm == -1)
    } else {
        (true, true)
    };

    // Put in an explicit division-by-zero trap if the environment requires it.
    if avoid_div_traps && could_be_zero {
        pos.ins().trapz(y, ir::TrapCode::IntegerDivisionByZero);
    }

    if !could_be_minus_one {
        let xhi = pos.ins().sshr_imm(x, i64::from(ty.lane_bits()) - 1);
        let reuse = if is_srem {
            [None, Some(result)]
        } else {
            [Some(result), None]
        };
        pos.ins().with_results(reuse).x86_sdivmodx(x, xhi, y);
        pos.remove_inst();
        return;
    }

    // EBB handling the nominal case.
    let nominal = pos.func.dfg.make_ebb();

    // EBB handling the -1 divisor case.
    let minus_one = pos.func.dfg.make_ebb();

    // Final EBB with one argument representing the final result value.
    let done = pos.func.dfg.make_ebb();

    // Move the `inst` result value onto the `done` EBB.
    pos.func.dfg.attach_ebb_param(done, result);

    // Start by checking for a -1 divisor which needs to be handled specially.
    let is_m1 = pos.ins().ifcmp_imm(y, -1);
    pos.ins().brif(IntCC::Equal, is_m1, minus_one, &[]);
    pos.ins().jump(nominal, &[]);

    // Now it is safe to execute the `x86_sdivmodx` instruction which will still trap on division
    // by zero.
    pos.insert_ebb(nominal);
    let xhi = pos.ins().sshr_imm(x, i64::from(ty.lane_bits()) - 1);
    let (quot, rem) = pos.ins().x86_sdivmodx(x, xhi, y);
    let divres = if is_srem { rem } else { quot };
    pos.ins().jump(done, &[divres]);

    // Now deal with the -1 divisor case.
    pos.insert_ebb(minus_one);
    let m1_result = if is_srem {
        // x % -1 = 0.
        pos.ins().iconst(ty, 0)
    } else {
        // Explicitly check for overflow: Trap when x == INT_MIN.
        debug_assert!(avoid_div_traps, "Native trapping divide handled above");
        let f = pos.ins().ifcmp_imm(x, -1 << (ty.lane_bits() - 1));
        pos.ins()
            .trapif(IntCC::Equal, f, ir::TrapCode::IntegerOverflow);
        // x / -1 = -x.
        pos.ins().irsub_imm(x, 0)
    };

    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[m1_result]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, nominal);
    cfg.recompute_ebb(pos.func, minus_one);
    cfg.recompute_ebb(pos.func, done);
}

/// Expand the `udiv` and `urem` instructions using `x86_udivmodx`.
fn expand_udivrem(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    let (x, y, is_urem) = match func.dfg[inst] {
        ir::InstructionData::Binary {
            opcode: ir::Opcode::Udiv,
            args,
        } => (args[0], args[1], false),
        ir::InstructionData::Binary {
            opcode: ir::Opcode::Urem,
            args,
        } => (args[0], args[1], true),
        _ => panic!("Need udiv/urem: {}", func.dfg.display_inst(inst, None)),
    };
    let avoid_div_traps = isa.flags().avoid_div_traps();
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);
    pos.func.dfg.clear_results(inst);

    // Put in an explicit division-by-zero trap if the environment requires it.
    if avoid_div_traps {
        let zero_check = if let Some(imm) = maybe_iconst_imm(&pos, y) {
            // Ideally, we'd just replace the conditional trap with a trap when the immediate is
            // zero, but this requires more manipulation of the dfg/cfg, which is out of scope
            // here.
            imm == 0
        } else {
            true
        };
        if zero_check {
            pos.ins().trapz(y, ir::TrapCode::IntegerDivisionByZero);
        }
    }

    // Now it is safe to execute the `x86_udivmodx` instruction.
    let xhi = pos.ins().iconst(ty, 0);
    let reuse = if is_urem {
        [None, Some(result)]
    } else {
        [Some(result), None]
    };
    pos.ins().with_results(reuse).x86_udivmodx(x, xhi, y);
    pos.remove_inst();
}

/// Expand the `fmin` and `fmax` instructions using the x86 `x86_fmin` and `x86_fmax`
/// instructions.
fn expand_minmax(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    let (x, y, x86_opc, bitwise_opc) = match func.dfg[inst] {
        ir::InstructionData::Binary {
            opcode: ir::Opcode::Fmin,
            args,
        } => (args[0], args[1], ir::Opcode::X86Fmin, ir::Opcode::Bor),
        ir::InstructionData::Binary {
            opcode: ir::Opcode::Fmax,
            args,
        } => (args[0], args[1], ir::Opcode::X86Fmax, ir::Opcode::Band),
        _ => panic!("Expected fmin/fmax: {}", func.dfg.display_inst(inst, None)),
    };
    let old_ebb = func.layout.pp_ebb(inst);

    // We need to handle the following conditions, depending on how x and y compare:
    //
    // 1. LT or GT: The native `x86_opc` min/max instruction does what we need.
    // 2. EQ: We need to use `bitwise_opc` to make sure that
    //    fmin(0.0, -0.0) -> -0.0 and fmax(0.0, -0.0) -> 0.0.
    // 3. UN: We need to produce a quiet NaN that is canonical if the inputs are canonical.

    // EBB handling case 1) where operands are ordered but not equal.
    let one_ebb = func.dfg.make_ebb();

    // EBB handling case 3) where one operand is NaN.
    let uno_ebb = func.dfg.make_ebb();

    // EBB that handles the unordered or equal cases 2) and 3).
    let ueq_ebb = func.dfg.make_ebb();

    // EBB handling case 2) where operands are ordered and equal.
    let eq_ebb = func.dfg.make_ebb();

    // Final EBB with one argument representing the final result value.
    let done = func.dfg.make_ebb();

    // The basic blocks are laid out to minimize branching for the common cases:
    //
    // 1) One branch not taken, one jump.
    // 2) One branch taken.
    // 3) Two branches taken, one jump.

    // Move the `inst` result value onto the `done` EBB.
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);
    func.dfg.clear_results(inst);
    func.dfg.attach_ebb_param(done, result);

    // Test for case 1) ordered and not equal.
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);
    let cmp_ueq = pos.ins().fcmp(FloatCC::UnorderedOrEqual, x, y);
    pos.ins().brnz(cmp_ueq, ueq_ebb, &[]);
    pos.ins().jump(one_ebb, &[]);

    // Handle the common ordered, not equal (LT|GT) case.
    pos.insert_ebb(one_ebb);
    let one_inst = pos.ins().Binary(x86_opc, ty, x, y).0;
    let one_result = pos.func.dfg.first_result(one_inst);
    pos.ins().jump(done, &[one_result]);

    // Case 3) Unordered.
    // We know that at least one operand is a NaN that needs to be propagated. We simply use an
    // `fadd` instruction which has the same NaN propagation semantics.
    pos.insert_ebb(uno_ebb);
    let uno_result = pos.ins().fadd(x, y);
    pos.ins().jump(done, &[uno_result]);

    // Case 2) or 3).
    pos.insert_ebb(ueq_ebb);
    // Test for case 3) (UN) one value is NaN.
    // TODO: When we get support for flag values, we can reuse the above comparison.
    let cmp_uno = pos.ins().fcmp(FloatCC::Unordered, x, y);
    pos.ins().brnz(cmp_uno, uno_ebb, &[]);
    pos.ins().jump(eq_ebb, &[]);

    // We are now in case 2) where x and y compare EQ.
    // We need a bitwise operation to get the sign right.
    pos.insert_ebb(eq_ebb);
    let bw_inst = pos.ins().Binary(bitwise_opc, ty, x, y).0;
    let bw_result = pos.func.dfg.first_result(bw_inst);
    // This should become a fall-through for this second most common case.
    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[bw_result]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, one_ebb);
    cfg.recompute_ebb(pos.func, uno_ebb);
    cfg.recompute_ebb(pos.func, ueq_ebb);
    cfg.recompute_ebb(pos.func, eq_ebb);
    cfg.recompute_ebb(pos.func, done);
}

/// x86 has no unsigned-to-float conversions. We handle the easy case of zero-extending i32 to
/// i64 with a pattern, the rest needs more code.
fn expand_fcvt_from_uint(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    let x;
    match func.dfg[inst] {
        ir::InstructionData::Unary {
            opcode: ir::Opcode::FcvtFromUint,
            arg,
        } => x = arg,
        _ => panic!("Need fcvt_from_uint: {}", func.dfg.display_inst(inst, None)),
    }
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Conversion from an unsigned int smaller than 64bit is easy on x86-64.
    match xty {
        ir::types::I8 | ir::types::I16 | ir::types::I32 => {
            // TODO: This should be guarded by an ISA check.
            let wide = pos.ins().uextend(ir::types::I64, x);
            pos.func.dfg.replace(inst).fcvt_from_sint(ty, wide);
            return;
        }
        ir::types::I64 => {}
        _ => unimplemented!(),
    }

    let old_ebb = pos.func.layout.pp_ebb(inst);

    // EBB handling the case where x >= 0.
    let poszero_ebb = pos.func.dfg.make_ebb();

    // EBB handling the case where x < 0.
    let neg_ebb = pos.func.dfg.make_ebb();

    // Final EBB with one argument representing the final result value.
    let done = pos.func.dfg.make_ebb();

    // Move the `inst` result value onto the `done` EBB.
    pos.func.dfg.clear_results(inst);
    pos.func.dfg.attach_ebb_param(done, result);

    // If x as a signed int is not negative, we can use the existing `fcvt_from_sint` instruction.
    let is_neg = pos.ins().icmp_imm(IntCC::SignedLessThan, x, 0);
    pos.ins().brnz(is_neg, neg_ebb, &[]);
    pos.ins().jump(poszero_ebb, &[]);

    // Easy case: just use a signed conversion.
    pos.insert_ebb(poszero_ebb);
    let posres = pos.ins().fcvt_from_sint(ty, x);
    pos.ins().jump(done, &[posres]);

    // Now handle the negative case.
    pos.insert_ebb(neg_ebb);

    // Divide x by two to get it in range for the signed conversion, keep the LSB, and scale it
    // back up on the FP side.
    let ihalf = pos.ins().ushr_imm(x, 1);
    let lsb = pos.ins().band_imm(x, 1);
    let ifinal = pos.ins().bor(ihalf, lsb);
    let fhalf = pos.ins().fcvt_from_sint(ty, ifinal);
    let negres = pos.ins().fadd(fhalf, fhalf);

    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[negres]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, poszero_ebb);
    cfg.recompute_ebb(pos.func, neg_ebb);
    cfg.recompute_ebb(pos.func, done);
}

fn expand_fcvt_to_sint(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    use crate::ir::immediates::{Ieee32, Ieee64};

    let x = match func.dfg[inst] {
        ir::InstructionData::Unary {
            opcode: ir::Opcode::FcvtToSint,
            arg,
        } => arg,
        _ => panic!("Need fcvt_to_sint: {}", func.dfg.display_inst(inst, None)),
    };
    let old_ebb = func.layout.pp_ebb(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // Final EBB after the bad value checks.
    let done = func.dfg.make_ebb();

    // EBB for checking failure cases.
    let maybe_trap_ebb = func.dfg.make_ebb();

    // The `x86_cvtt2si` performs the desired conversion, but it doesn't trap on NaN or overflow.
    // It produces an INT_MIN result instead.
    func.dfg.replace(inst).x86_cvtt2si(ty, x);

    let mut pos = FuncCursor::new(func).after_inst(inst);
    pos.use_srcloc(inst);

    let is_done = pos
        .ins()
        .icmp_imm(IntCC::NotEqual, result, 1 << (ty.lane_bits() - 1));
    pos.ins().brnz(is_done, done, &[]);
    pos.ins().jump(maybe_trap_ebb, &[]);

    // We now have the following possibilities:
    //
    // 1. INT_MIN was actually the correct conversion result.
    // 2. The input was NaN -> trap bad_toint
    // 3. The input was out of range -> trap int_ovf
    //
    pos.insert_ebb(maybe_trap_ebb);

    // Check for NaN.
    let is_nan = pos.ins().fcmp(FloatCC::Unordered, x, x);
    pos.ins()
        .trapnz(is_nan, ir::TrapCode::BadConversionToInteger);

    // Check for case 1: INT_MIN is the correct result.
    // Determine the smallest floating point number that would convert to INT_MIN.
    let mut overflow_cc = FloatCC::LessThan;
    let output_bits = ty.lane_bits();
    let flimit = match xty {
        ir::types::F32 =>
        // An f32 can represent `i16::min_value() - 1` exactly with precision to spare, so
        // there are values less than -2^(N-1) that convert correctly to INT_MIN.
        {
            pos.ins().f32const(if output_bits < 32 {
                overflow_cc = FloatCC::LessThanOrEqual;
                Ieee32::fcvt_to_sint_negative_overflow(output_bits)
            } else {
                Ieee32::pow2(output_bits - 1).neg()
            })
        }
        ir::types::F64 =>
        // An f64 can represent `i32::min_value() - 1` exactly with precision to spare, so
        // there are values less than -2^(N-1) that convert correctly to INT_MIN.
        {
            pos.ins().f64const(if output_bits < 64 {
                overflow_cc = FloatCC::LessThanOrEqual;
                Ieee64::fcvt_to_sint_negative_overflow(output_bits)
            } else {
                Ieee64::pow2(output_bits - 1).neg()
            })
        }
        _ => panic!("Can't convert {}", xty),
    };
    let overflow = pos.ins().fcmp(overflow_cc, x, flimit);
    pos.ins().trapnz(overflow, ir::TrapCode::IntegerOverflow);

    // Finally, we could have a positive value that is too large.
    let fzero = match xty {
        ir::types::F32 => pos.ins().f32const(Ieee32::with_bits(0)),
        ir::types::F64 => pos.ins().f64const(Ieee64::with_bits(0)),
        _ => panic!("Can't convert {}", xty),
    };
    let overflow = pos.ins().fcmp(FloatCC::GreaterThanOrEqual, x, fzero);
    pos.ins().trapnz(overflow, ir::TrapCode::IntegerOverflow);

    pos.ins().jump(done, &[]);
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, maybe_trap_ebb);
    cfg.recompute_ebb(pos.func, done);
}

fn expand_fcvt_to_sint_sat(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    use crate::ir::immediates::{Ieee32, Ieee64};

    let x = match func.dfg[inst] {
        ir::InstructionData::Unary {
            opcode: ir::Opcode::FcvtToSintSat,
            arg,
        } => arg,
        _ => panic!(
            "Need fcvt_to_sint_sat: {}",
            func.dfg.display_inst(inst, None)
        ),
    };

    let old_ebb = func.layout.pp_ebb(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // Final EBB after the bad value checks.
    let done_ebb = func.dfg.make_ebb();
    let intmin_ebb = func.dfg.make_ebb();
    let minsat_ebb = func.dfg.make_ebb();
    let maxsat_ebb = func.dfg.make_ebb();
    func.dfg.clear_results(inst);
    func.dfg.attach_ebb_param(done_ebb, result);

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // The `x86_cvtt2si` performs the desired conversion, but it doesn't trap on NaN or
    // overflow. It produces an INT_MIN result instead.
    let cvtt2si = pos.ins().x86_cvtt2si(ty, x);

    let is_done = pos
        .ins()
        .icmp_imm(IntCC::NotEqual, cvtt2si, 1 << (ty.lane_bits() - 1));
    pos.ins().brnz(is_done, done_ebb, &[cvtt2si]);
    pos.ins().jump(intmin_ebb, &[]);

    // We now have the following possibilities:
    //
    // 1. INT_MIN was actually the correct conversion result.
    // 2. The input was NaN -> replace the result value with 0.
    // 3. The input was out of range -> saturate the result to the min/max value.
    pos.insert_ebb(intmin_ebb);

    // Check for NaN, which is truncated to 0.
    let zero = pos.ins().iconst(ty, 0);
    let is_nan = pos.ins().fcmp(FloatCC::Unordered, x, x);
    pos.ins().brnz(is_nan, done_ebb, &[zero]);
    pos.ins().jump(minsat_ebb, &[]);

    // Check for case 1: INT_MIN is the correct result.
    // Determine the smallest floating point number that would convert to INT_MIN.
    pos.insert_ebb(minsat_ebb);
    let mut overflow_cc = FloatCC::LessThan;
    let output_bits = ty.lane_bits();
    let flimit = match xty {
        ir::types::F32 =>
        // An f32 can represent `i16::min_value() - 1` exactly with precision to spare, so
        // there are values less than -2^(N-1) that convert correctly to INT_MIN.
        {
            pos.ins().f32const(if output_bits < 32 {
                overflow_cc = FloatCC::LessThanOrEqual;
                Ieee32::fcvt_to_sint_negative_overflow(output_bits)
            } else {
                Ieee32::pow2(output_bits - 1).neg()
            })
        }
        ir::types::F64 =>
        // An f64 can represent `i32::min_value() - 1` exactly with precision to spare, so
        // there are values less than -2^(N-1) that convert correctly to INT_MIN.
        {
            pos.ins().f64const(if output_bits < 64 {
                overflow_cc = FloatCC::LessThanOrEqual;
                Ieee64::fcvt_to_sint_negative_overflow(output_bits)
            } else {
                Ieee64::pow2(output_bits - 1).neg()
            })
        }
        _ => panic!("Can't convert {}", xty),
    };

    let overflow = pos.ins().fcmp(overflow_cc, x, flimit);
    let min_imm = match ty {
        ir::types::I32 => i32::min_value() as i64,
        ir::types::I64 => i64::min_value(),
        _ => panic!("Don't know the min value for {}", ty),
    };
    let min_value = pos.ins().iconst(ty, min_imm);
    pos.ins().brnz(overflow, done_ebb, &[min_value]);
    pos.ins().jump(maxsat_ebb, &[]);

    // Finally, we could have a positive value that is too large.
    pos.insert_ebb(maxsat_ebb);
    let fzero = match xty {
        ir::types::F32 => pos.ins().f32const(Ieee32::with_bits(0)),
        ir::types::F64 => pos.ins().f64const(Ieee64::with_bits(0)),
        _ => panic!("Can't convert {}", xty),
    };

    let max_imm = match ty {
        ir::types::I32 => i32::max_value() as i64,
        ir::types::I64 => i64::max_value(),
        _ => panic!("Don't know the max value for {}", ty),
    };
    let max_value = pos.ins().iconst(ty, max_imm);

    let overflow = pos.ins().fcmp(FloatCC::GreaterThanOrEqual, x, fzero);
    pos.ins().brnz(overflow, done_ebb, &[max_value]);

    // Recycle the original instruction.
    pos.func.dfg.replace(inst).jump(done_ebb, &[cvtt2si]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_ebb(done_ebb);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, intmin_ebb);
    cfg.recompute_ebb(pos.func, minsat_ebb);
    cfg.recompute_ebb(pos.func, maxsat_ebb);
    cfg.recompute_ebb(pos.func, done_ebb);
}

fn expand_fcvt_to_uint(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    use crate::ir::immediates::{Ieee32, Ieee64};

    let x = match func.dfg[inst] {
        ir::InstructionData::Unary {
            opcode: ir::Opcode::FcvtToUint,
            arg,
        } => arg,
        _ => panic!("Need fcvt_to_uint: {}", func.dfg.display_inst(inst, None)),
    };

    let old_ebb = func.layout.pp_ebb(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // EBB handle numbers < 2^(N-1).
    let below_uint_max_ebb = func.dfg.make_ebb();

    // EBB handle numbers < 0.
    let below_zero_ebb = func.dfg.make_ebb();

    // EBB handling numbers >= 2^(N-1).
    let large = func.dfg.make_ebb();

    // Final EBB after the bad value checks.
    let done = func.dfg.make_ebb();

    // Move the `inst` result value onto the `done` EBB.
    func.dfg.clear_results(inst);
    func.dfg.attach_ebb_param(done, result);

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Start by materializing the floating point constant 2^(N-1) where N is the number of bits in
    // the destination integer type.
    let pow2nm1 = match xty {
        ir::types::F32 => pos.ins().f32const(Ieee32::pow2(ty.lane_bits() - 1)),
        ir::types::F64 => pos.ins().f64const(Ieee64::pow2(ty.lane_bits() - 1)),
        _ => panic!("Can't convert {}", xty),
    };
    let is_large = pos.ins().ffcmp(x, pow2nm1);
    pos.ins()
        .brff(FloatCC::GreaterThanOrEqual, is_large, large, &[]);
    pos.ins().jump(below_uint_max_ebb, &[]);

    // We need to generate a specific trap code when `x` is NaN, so reuse the flags from the
    // previous comparison.
    pos.insert_ebb(below_uint_max_ebb);
    pos.ins().trapff(
        FloatCC::Unordered,
        is_large,
        ir::TrapCode::BadConversionToInteger,
    );

    // Now we know that x < 2^(N-1) and not NaN.
    let sres = pos.ins().x86_cvtt2si(ty, x);
    let is_neg = pos.ins().ifcmp_imm(sres, 0);
    pos.ins()
        .brif(IntCC::SignedGreaterThanOrEqual, is_neg, done, &[sres]);
    pos.ins().jump(below_zero_ebb, &[]);

    pos.insert_ebb(below_zero_ebb);
    pos.ins().trap(ir::TrapCode::IntegerOverflow);

    // Handle the case where x >= 2^(N-1) and not NaN.
    pos.insert_ebb(large);
    let adjx = pos.ins().fsub(x, pow2nm1);
    let lres = pos.ins().x86_cvtt2si(ty, adjx);
    let is_neg = pos.ins().ifcmp_imm(lres, 0);
    pos.ins()
        .trapif(IntCC::SignedLessThan, is_neg, ir::TrapCode::IntegerOverflow);
    let lfinal = pos.ins().iadd_imm(lres, 1 << (ty.lane_bits() - 1));

    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[lfinal]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, below_uint_max_ebb);
    cfg.recompute_ebb(pos.func, below_zero_ebb);
    cfg.recompute_ebb(pos.func, large);
    cfg.recompute_ebb(pos.func, done);
}

fn expand_fcvt_to_uint_sat(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    use crate::ir::immediates::{Ieee32, Ieee64};

    let x = match func.dfg[inst] {
        ir::InstructionData::Unary {
            opcode: ir::Opcode::FcvtToUintSat,
            arg,
        } => arg,
        _ => panic!(
            "Need fcvt_to_uint_sat: {}",
            func.dfg.display_inst(inst, None)
        ),
    };

    let old_ebb = func.layout.pp_ebb(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // EBB handle numbers < 2^(N-1).
    let below_pow2nm1_or_nan_ebb = func.dfg.make_ebb();
    let below_pow2nm1_ebb = func.dfg.make_ebb();

    // EBB handling numbers >= 2^(N-1).
    let large = func.dfg.make_ebb();

    // EBB handling numbers < 2^N.
    let uint_large_ebb = func.dfg.make_ebb();

    // Final EBB after the bad value checks.
    let done = func.dfg.make_ebb();

    // Move the `inst` result value onto the `done` EBB.
    func.dfg.clear_results(inst);
    func.dfg.attach_ebb_param(done, result);

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Start by materializing the floating point constant 2^(N-1) where N is the number of bits in
    // the destination integer type.
    let pow2nm1 = match xty {
        ir::types::F32 => pos.ins().f32const(Ieee32::pow2(ty.lane_bits() - 1)),
        ir::types::F64 => pos.ins().f64const(Ieee64::pow2(ty.lane_bits() - 1)),
        _ => panic!("Can't convert {}", xty),
    };
    let zero = pos.ins().iconst(ty, 0);
    let is_large = pos.ins().ffcmp(x, pow2nm1);
    pos.ins()
        .brff(FloatCC::GreaterThanOrEqual, is_large, large, &[]);
    pos.ins().jump(below_pow2nm1_or_nan_ebb, &[]);

    // We need to generate zero when `x` is NaN, so reuse the flags from the previous comparison.
    pos.insert_ebb(below_pow2nm1_or_nan_ebb);
    pos.ins().brff(FloatCC::Unordered, is_large, done, &[zero]);
    pos.ins().jump(below_pow2nm1_ebb, &[]);

    // Now we know that x < 2^(N-1) and not NaN. If the result of the cvtt2si is positive, we're
    // done; otherwise saturate to the minimum unsigned value, that is 0.
    pos.insert_ebb(below_pow2nm1_ebb);
    let sres = pos.ins().x86_cvtt2si(ty, x);
    let is_neg = pos.ins().ifcmp_imm(sres, 0);
    pos.ins()
        .brif(IntCC::SignedGreaterThanOrEqual, is_neg, done, &[sres]);
    pos.ins().jump(done, &[zero]);

    // Handle the case where x >= 2^(N-1) and not NaN.
    pos.insert_ebb(large);
    let adjx = pos.ins().fsub(x, pow2nm1);
    let lres = pos.ins().x86_cvtt2si(ty, adjx);
    let max_value = pos.ins().iconst(
        ty,
        match ty {
            ir::types::I32 => u32::max_value() as i64,
            ir::types::I64 => u64::max_value() as i64,
            _ => panic!("Can't convert {}", ty),
        },
    );
    let is_neg = pos.ins().ifcmp_imm(lres, 0);
    pos.ins()
        .brif(IntCC::SignedLessThan, is_neg, done, &[max_value]);
    pos.ins().jump(uint_large_ebb, &[]);

    pos.insert_ebb(uint_large_ebb);
    let lfinal = pos.ins().iadd_imm(lres, 1 << (ty.lane_bits() - 1));

    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[lfinal]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, below_pow2nm1_or_nan_ebb);
    cfg.recompute_ebb(pos.func, below_pow2nm1_ebb);
    cfg.recompute_ebb(pos.func, large);
    cfg.recompute_ebb(pos.func, uint_large_ebb);
    cfg.recompute_ebb(pos.func, done);
}

/// Convert shuffle instructions.
fn convert_shuffle(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    if let ir::InstructionData::Shuffle { args, mask, .. } = pos.func.dfg[inst] {
        // A mask-building helper: in 128-bit SIMD, 0-15 indicate which lane to read from and a 1
        // in the most significant position zeroes the lane.
        let zero_unknown_lane_index = |b: u8| if b > 15 { 0b10000000 } else { b };

        // We only have to worry about aliasing here because copies will be introduced later (in
        // regalloc).
        let a = pos.func.dfg.resolve_aliases(args[0]);
        let b = pos.func.dfg.resolve_aliases(args[1]);
        let mask = pos
            .func
            .dfg
            .immediates
            .get(mask)
            .expect("The shuffle immediate should have been recorded before this point")
            .clone();
        if a == b {
            // PSHUFB the first argument (since it is the same as the second).
            let constructed_mask = mask
                .iter()
                // If the mask is greater than 15 it still may be referring to a lane in b.
                .map(|&b| if b > 15 { b.wrapping_sub(16) } else { b })
                .map(zero_unknown_lane_index)
                .collect();
            let handle = pos.func.dfg.constants.insert(constructed_mask);
            // Move the built mask into another XMM register.
            let a_type = pos.func.dfg.value_type(a);
            let mask_value = pos.ins().vconst(a_type, handle);
            // Shuffle the single incoming argument.
            pos.func.dfg.replace(inst).x86_pshufb(a, mask_value);
        } else {
            // PSHUFB the first argument, placing zeroes for unused lanes.
            let constructed_mask = mask.iter().cloned().map(zero_unknown_lane_index).collect();
            let handle = pos.func.dfg.constants.insert(constructed_mask);
            // Move the built mask into another XMM register.
            let a_type = pos.func.dfg.value_type(a);
            let mask_value = pos.ins().vconst(a_type, handle);
            // Shuffle the first argument.
            let shuffled_first_arg = pos.ins().x86_pshufb(a, mask_value);

            // PSHUFB the second argument, placing zeroes for unused lanes.
            let constructed_mask = mask
                .iter()
                .map(|b| b.wrapping_sub(16))
                .map(zero_unknown_lane_index)
                .collect();
            let handle = pos.func.dfg.constants.insert(constructed_mask);
            // Move the built mask into another XMM register.
            let b_type = pos.func.dfg.value_type(b);
            let mask_value = pos.ins().vconst(b_type, handle);
            // Shuffle the second argument.
            let shuffled_second_arg = pos.ins().x86_pshufb(b, mask_value);

            // OR the vectors together to form the final shuffled value.
            pos.func
                .dfg
                .replace(inst)
                .bor(shuffled_first_arg, shuffled_second_arg);

            // TODO when AVX512 is enabled we should replace this sequence with a single VPERMB
        };
    }
}

/// Because floats already exist in XMM registers, we can keep them there when executing a CLIF
/// extractlane instruction
fn convert_extractlane(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    if let ir::InstructionData::ExtractLane {
        opcode: ir::Opcode::Extractlane,
        arg,
        lane,
    } = pos.func.dfg[inst]
    {
        // NOTE: the following legalization assumes that the upper bits of the XMM register do
        // not need to be zeroed during extractlane.
        let value_type = pos.func.dfg.value_type(arg);
        if value_type.lane_type().is_float() {
            // Floats are already in XMM registers and can stay there.
            let shuffled = if lane != 0 {
                // Replace the extractlane with a PSHUFD to get the float in the right place.
                match value_type {
                    F32X4 => {
                        // Move the selected lane to the 0 lane.
                        let shuffle_mask: u8 = 0b00_00_00_00 | lane;
                        pos.ins().x86_pshufd(arg, shuffle_mask)
                    }
                    F64X2 => {
                        assert_eq!(lane, 1);
                        // Because we know the lane == 1, we move the upper 64 bits to the lower
                        // 64 bits, leaving the top 64 bits as-is.
                        let shuffle_mask = 0b11_10_11_10;
                        let bitcast = pos.ins().raw_bitcast(F32X4, arg);
                        pos.ins().x86_pshufd(bitcast, shuffle_mask)
                    }
                    _ => unreachable!(),
                }
            } else {
                // Remove the extractlane instruction, leaving the float where it is.
                arg
            };
            // Then we must bitcast to the right type.
            pos.func
                .dfg
                .replace(inst)
                .raw_bitcast(value_type.lane_type(), shuffled);
        } else {
            // For non-floats, lower with the usual PEXTR* instruction.
            pos.func.dfg.replace(inst).x86_pextr(arg, lane);
        }
    }
}

/// Because floats exist in XMM registers, we can keep them there when executing a CLIF
/// insertlane instruction
fn convert_insertlane(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    if let ir::InstructionData::InsertLane {
        opcode: ir::Opcode::Insertlane,
        args: [vector, replacement],
        lane,
    } = pos.func.dfg[inst]
    {
        let value_type = pos.func.dfg.value_type(vector);
        if value_type.lane_type().is_float() {
            // Floats are already in XMM registers and can stay there.
            match value_type {
                F32X4 => {
                    assert!(lane > 0 && lane <= 3);
                    let immediate = 0b00_00_00_00 | lane << 4;
                    // Insert 32-bits from replacement (at index 00, bits 7:8) to vector (lane
                    // shifted into bits 5:6).
                    pos.func
                        .dfg
                        .replace(inst)
                        .x86_insertps(vector, immediate, replacement)
                }
                F64X2 => {
                    let replacement_as_vector = pos.ins().raw_bitcast(F64X2, replacement); // only necessary due to SSA types
                    if lane == 0 {
                        // Move the lowest quadword in replacement to vector without changing
                        // the upper bits.
                        pos.func
                            .dfg
                            .replace(inst)
                            .x86_movsd(vector, replacement_as_vector)
                    } else {
                        assert_eq!(lane, 1);
                        // Move the low 64 bits of replacement vector to the high 64 bits of the
                        // vector.
                        pos.func
                            .dfg
                            .replace(inst)
                            .x86_movlhps(vector, replacement_as_vector)
                    }
                }
                _ => unreachable!(),
            };
        } else {
            // For non-floats, lower with the usual PINSR* instruction.
            pos.func
                .dfg
                .replace(inst)
                .x86_pinsr(vector, lane, replacement);
        }
    }
}

/// For SIMD negation, convert an `ineg` to a `vconst + isub`.
fn convert_ineg(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    if let ir::InstructionData::Unary {
        opcode: ir::Opcode::Ineg,
        arg,
    } = pos.func.dfg[inst]
    {
        let value_type = pos.func.dfg.value_type(arg);
        if value_type.is_vector() && value_type.lane_type().is_int() {
            let zero_immediate = pos.func.dfg.constants.insert(vec![0; 16].into());
            let zero_value = pos.ins().vconst(value_type, zero_immediate); // this should be legalized to a PXOR
            pos.func.dfg.replace(inst).isub(zero_value, arg);
        }
    }
}
