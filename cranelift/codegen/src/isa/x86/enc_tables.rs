//! Encoding tables for x86 ISAs.

use super::registers::*;
use crate::bitset::BitSet;
use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::types::*;
use crate::ir::{self, Function, Inst, InstBuilder, MemFlags};
use crate::isa::constraints::*;
use crate::isa::enc_tables::*;
use crate::isa::encoding::base_size;
use crate::isa::encoding::{Encoding, RecipeSizing};
use crate::isa::RegUnit;
use crate::isa::{self, TargetIsa};
use crate::legalizer::expand_as_libcall;
use crate::predicates;
use crate::regalloc::RegDiversions;

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

/// Calculates the size while inferring if the first and second input registers (inreg0, inreg1)
/// require a dynamic REX prefix and if the second input register (inreg1) requires a SIB or offset.
fn size_plus_maybe_sib_or_offset_inreg1_plus_rex_prefix_for_inreg0_inreg1(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(0, inst, divert, func, is_extended_reg)
        || test_input(1, inst, divert, func, is_extended_reg);
    size_plus_maybe_sib_or_offset_for_inreg_1(sizing, enc, inst, divert, func)
        + if needs_rex { 1 } else { 0 }
}

/// Calculates the size while inferring if the first and second input registers (inreg0, inreg1)
/// require a dynamic REX prefix and if the second input register (inreg1) requires a SIB.
fn size_plus_maybe_sib_inreg1_plus_rex_prefix_for_inreg0_inreg1(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(0, inst, divert, func, is_extended_reg)
        || test_input(1, inst, divert, func, is_extended_reg);
    size_plus_maybe_sib_for_inreg_1(sizing, enc, inst, divert, func) + if needs_rex { 1 } else { 0 }
}

/// Calculates the size while inferring if the first input register (inreg0) and first output
/// register (outreg0) require a dynamic REX and if the first input register (inreg0) requires a
/// SIB or offset.
fn size_plus_maybe_sib_or_offset_for_inreg_0_plus_rex_prefix_for_inreg0_outreg0(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(0, inst, divert, func, is_extended_reg)
        || test_result(0, inst, divert, func, is_extended_reg);
    size_plus_maybe_sib_or_offset_for_inreg_0(sizing, enc, inst, divert, func)
        + if needs_rex { 1 } else { 0 }
}

/// Calculates the size while inferring if the first input register (inreg0) and first output
/// register (outreg0) require a dynamic REX and if the first input register (inreg0) requires a
/// SIB.
fn size_plus_maybe_sib_for_inreg_0_plus_rex_prefix_for_inreg0_outreg0(
    sizing: &RecipeSizing,
    enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(0, inst, divert, func, is_extended_reg)
        || test_result(0, inst, divert, func, is_extended_reg);
    size_plus_maybe_sib_for_inreg_0(sizing, enc, inst, divert, func) + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, for use with one input reg.
///
/// A REX prefix is known to be emitted if either:
///  1. The EncodingBits specify that REX.W is to be set.
///  2. Registers are used that require REX.R or REX.B bits for encoding.
fn size_with_inferred_rex_for_inreg0(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(0, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, based on the second operand.
fn size_with_inferred_rex_for_inreg1(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(1, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, based on the third operand.
fn size_with_inferred_rex_for_inreg2(
    sizing: &RecipeSizing,
    _: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(2, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, for use with two input registers.
///
/// A REX prefix is known to be emitted if either:
///  1. The EncodingBits specify that REX.W is to be set.
///  2. Registers are used that require REX.R or REX.B bits for encoding.
fn size_with_inferred_rex_for_inreg0_inreg1(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(0, inst, divert, func, is_extended_reg)
        || test_input(1, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, based on second and third operand.
fn size_with_inferred_rex_for_inreg1_inreg2(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(1, inst, divert, func, is_extended_reg)
        || test_input(2, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, based on a single
/// input register and a single output register.
fn size_with_inferred_rex_for_inreg0_outreg0(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(0, inst, divert, func, is_extended_reg)
        || test_result(0, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, based on a single output register.
fn size_with_inferred_rex_for_outreg0(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_result(0, inst, divert, func, is_extended_reg);
    sizing.base_size + if needs_rex { 1 } else { 0 }
}

/// Infers whether a dynamic REX prefix will be emitted, for use with CMOV.
///
/// CMOV uses 3 inputs, with the REX is inferred from reg1 and reg2.
fn size_with_inferred_rex_for_cmov(
    sizing: &RecipeSizing,
    _enc: Encoding,
    inst: Inst,
    divert: &RegDiversions,
    func: &Function,
) -> u8 {
    // No need to check for REX.W in `needs_rex` because `infer_rex().w()` is not allowed.
    let needs_rex = test_input(1, inst, divert, func, is_extended_reg)
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

    let old_block = func.layout.pp_block(inst);
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

    // block handling the nominal case.
    let nominal = pos.func.dfg.make_block();

    // block handling the -1 divisor case.
    let minus_one = pos.func.dfg.make_block();

    // Final block with one argument representing the final result value.
    let done = pos.func.dfg.make_block();

    // Move the `inst` result value onto the `done` block.
    pos.func.dfg.attach_block_param(done, result);

    // Start by checking for a -1 divisor which needs to be handled specially.
    let is_m1 = pos.ins().ifcmp_imm(y, -1);
    pos.ins().brif(IntCC::Equal, is_m1, minus_one, &[]);
    pos.ins().jump(nominal, &[]);

    // Now it is safe to execute the `x86_sdivmodx` instruction which will still trap on division
    // by zero.
    pos.insert_block(nominal);
    let xhi = pos.ins().sshr_imm(x, i64::from(ty.lane_bits()) - 1);
    let (quot, rem) = pos.ins().x86_sdivmodx(x, xhi, y);
    let divres = if is_srem { rem } else { quot };
    pos.ins().jump(done, &[divres]);

    // Now deal with the -1 divisor case.
    pos.insert_block(minus_one);
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
    pos.insert_block(done);

    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, nominal);
    cfg.recompute_block(pos.func, minus_one);
    cfg.recompute_block(pos.func, done);
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
    let old_block = func.layout.pp_block(inst);

    // We need to handle the following conditions, depending on how x and y compare:
    //
    // 1. LT or GT: The native `x86_opc` min/max instruction does what we need.
    // 2. EQ: We need to use `bitwise_opc` to make sure that
    //    fmin(0.0, -0.0) -> -0.0 and fmax(0.0, -0.0) -> 0.0.
    // 3. UN: We need to produce a quiet NaN that is canonical if the inputs are canonical.

    // block handling case 1) where operands are ordered but not equal.
    let one_block = func.dfg.make_block();

    // block handling case 3) where one operand is NaN.
    let uno_block = func.dfg.make_block();

    // block that handles the unordered or equal cases 2) and 3).
    let ueq_block = func.dfg.make_block();

    // block handling case 2) where operands are ordered and equal.
    let eq_block = func.dfg.make_block();

    // Final block with one argument representing the final result value.
    let done = func.dfg.make_block();

    // The basic blocks are laid out to minimize branching for the common cases:
    //
    // 1) One branch not taken, one jump.
    // 2) One branch taken.
    // 3) Two branches taken, one jump.

    // Move the `inst` result value onto the `done` block.
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);
    func.dfg.clear_results(inst);
    func.dfg.attach_block_param(done, result);

    // Test for case 1) ordered and not equal.
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);
    let cmp_ueq = pos.ins().fcmp(FloatCC::UnorderedOrEqual, x, y);
    pos.ins().brnz(cmp_ueq, ueq_block, &[]);
    pos.ins().jump(one_block, &[]);

    // Handle the common ordered, not equal (LT|GT) case.
    pos.insert_block(one_block);
    let one_inst = pos.ins().Binary(x86_opc, ty, x, y).0;
    let one_result = pos.func.dfg.first_result(one_inst);
    pos.ins().jump(done, &[one_result]);

    // Case 3) Unordered.
    // We know that at least one operand is a NaN that needs to be propagated. We simply use an
    // `fadd` instruction which has the same NaN propagation semantics.
    pos.insert_block(uno_block);
    let uno_result = pos.ins().fadd(x, y);
    pos.ins().jump(done, &[uno_result]);

    // Case 2) or 3).
    pos.insert_block(ueq_block);
    // Test for case 3) (UN) one value is NaN.
    // TODO: When we get support for flag values, we can reuse the above comparison.
    let cmp_uno = pos.ins().fcmp(FloatCC::Unordered, x, y);
    pos.ins().brnz(cmp_uno, uno_block, &[]);
    pos.ins().jump(eq_block, &[]);

    // We are now in case 2) where x and y compare EQ.
    // We need a bitwise operation to get the sign right.
    pos.insert_block(eq_block);
    let bw_inst = pos.ins().Binary(bitwise_opc, ty, x, y).0;
    let bw_result = pos.func.dfg.first_result(bw_inst);
    // This should become a fall-through for this second most common case.
    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[bw_result]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_block(done);

    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, one_block);
    cfg.recompute_block(pos.func, uno_block);
    cfg.recompute_block(pos.func, ueq_block);
    cfg.recompute_block(pos.func, eq_block);
    cfg.recompute_block(pos.func, done);
}

/// x86 has no unsigned-to-float conversions. We handle the easy case of zero-extending i32 to
/// i64 with a pattern, the rest needs more code.
///
/// Note that this is the scalar implementation; for the vector implemenation see
/// [expand_fcvt_from_uint_vector].
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

    let old_block = pos.func.layout.pp_block(inst);

    // block handling the case where x >= 0.
    let poszero_block = pos.func.dfg.make_block();

    // block handling the case where x < 0.
    let neg_block = pos.func.dfg.make_block();

    // Final block with one argument representing the final result value.
    let done = pos.func.dfg.make_block();

    // Move the `inst` result value onto the `done` block.
    pos.func.dfg.clear_results(inst);
    pos.func.dfg.attach_block_param(done, result);

    // If x as a signed int is not negative, we can use the existing `fcvt_from_sint` instruction.
    let is_neg = pos.ins().icmp_imm(IntCC::SignedLessThan, x, 0);
    pos.ins().brnz(is_neg, neg_block, &[]);
    pos.ins().jump(poszero_block, &[]);

    // Easy case: just use a signed conversion.
    pos.insert_block(poszero_block);
    let posres = pos.ins().fcvt_from_sint(ty, x);
    pos.ins().jump(done, &[posres]);

    // Now handle the negative case.
    pos.insert_block(neg_block);

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
    pos.insert_block(done);

    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, poszero_block);
    cfg.recompute_block(pos.func, neg_block);
    cfg.recompute_block(pos.func, done);
}

/// To convert packed unsigned integers to their float equivalents, we must legalize to a special
/// AVX512 instruction (using MCSR rounding) or use a long sequence of instructions. This logic is
/// separate from [expand_fcvt_from_uint] above (the scalar version), only due to how the transform
/// groups are set up; TODO if we change the SIMD legalization groups, then this logic could be
/// merged into [expand_fcvt_from_uint] (see https://github.com/bytecodealliance/wasmtime/issues/1745).
fn expand_fcvt_from_uint_vector(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    if let ir::InstructionData::Unary {
        opcode: ir::Opcode::FcvtFromUint,
        arg,
    } = pos.func.dfg[inst]
    {
        let controlling_type = pos.func.dfg.ctrl_typevar(inst);
        if controlling_type == F32X4 {
            debug_assert_eq!(pos.func.dfg.value_type(arg), I32X4);
            let x86_isa = isa
                .as_any()
                .downcast_ref::<isa::x86::Isa>()
                .expect("the target ISA must be x86 at this point");
            if x86_isa.isa_flags.use_avx512vl_simd() || x86_isa.isa_flags.use_avx512f_simd() {
                // If we have certain AVX512 features, we can lower this instruction simply.
                pos.func.dfg.replace(inst).x86_vcvtudq2ps(arg);
            } else {
                // Otherwise, we default to a very lengthy SSE4.1-compatible sequence: PXOR,
                // PBLENDW, PSUB, CVTDQ2PS, PSRLD, CVTDQ2PS, ADDPS, ADDPS
                let bitcast_arg = pos.ins().raw_bitcast(I16X8, arg);
                let zero_constant = pos.func.dfg.constants.insert(vec![0; 16].into());
                let zero = pos.ins().vconst(I16X8, zero_constant);
                let low = pos.ins().x86_pblendw(zero, bitcast_arg, 0x55);
                let bitcast_low = pos.ins().raw_bitcast(I32X4, low);
                let high = pos.ins().isub(arg, bitcast_low);
                let convert_low = pos.ins().fcvt_from_sint(F32X4, bitcast_low);
                let shift_high = pos.ins().ushr_imm(high, 1);
                let convert_high = pos.ins().fcvt_from_sint(F32X4, shift_high);
                let double_high = pos.ins().fadd(convert_high, convert_high);
                pos.func.dfg.replace(inst).fadd(double_high, convert_low);
            }
        } else {
            unimplemented!("cannot legalize {}", pos.func.dfg.display_inst(inst, None))
        }
    }
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
    let old_block = func.layout.pp_block(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // Final block after the bad value checks.
    let done = func.dfg.make_block();

    // block for checking failure cases.
    let maybe_trap_block = func.dfg.make_block();

    // The `x86_cvtt2si` performs the desired conversion, but it doesn't trap on NaN or overflow.
    // It produces an INT_MIN result instead.
    func.dfg.replace(inst).x86_cvtt2si(ty, x);

    let mut pos = FuncCursor::new(func).after_inst(inst);
    pos.use_srcloc(inst);

    let is_done = pos
        .ins()
        .icmp_imm(IntCC::NotEqual, result, 1 << (ty.lane_bits() - 1));
    pos.ins().brnz(is_done, done, &[]);
    pos.ins().jump(maybe_trap_block, &[]);

    // We now have the following possibilities:
    //
    // 1. INT_MIN was actually the correct conversion result.
    // 2. The input was NaN -> trap bad_toint
    // 3. The input was out of range -> trap int_ovf
    //
    pos.insert_block(maybe_trap_block);

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
    pos.insert_block(done);

    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, maybe_trap_block);
    cfg.recompute_block(pos.func, done);
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

    let old_block = func.layout.pp_block(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // Final block after the bad value checks.
    let done_block = func.dfg.make_block();
    let intmin_block = func.dfg.make_block();
    let minsat_block = func.dfg.make_block();
    let maxsat_block = func.dfg.make_block();
    func.dfg.clear_results(inst);
    func.dfg.attach_block_param(done_block, result);

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // The `x86_cvtt2si` performs the desired conversion, but it doesn't trap on NaN or
    // overflow. It produces an INT_MIN result instead.
    let cvtt2si = pos.ins().x86_cvtt2si(ty, x);

    let is_done = pos
        .ins()
        .icmp_imm(IntCC::NotEqual, cvtt2si, 1 << (ty.lane_bits() - 1));
    pos.ins().brnz(is_done, done_block, &[cvtt2si]);
    pos.ins().jump(intmin_block, &[]);

    // We now have the following possibilities:
    //
    // 1. INT_MIN was actually the correct conversion result.
    // 2. The input was NaN -> replace the result value with 0.
    // 3. The input was out of range -> saturate the result to the min/max value.
    pos.insert_block(intmin_block);

    // Check for NaN, which is truncated to 0.
    let zero = pos.ins().iconst(ty, 0);
    let is_nan = pos.ins().fcmp(FloatCC::Unordered, x, x);
    pos.ins().brnz(is_nan, done_block, &[zero]);
    pos.ins().jump(minsat_block, &[]);

    // Check for case 1: INT_MIN is the correct result.
    // Determine the smallest floating point number that would convert to INT_MIN.
    pos.insert_block(minsat_block);
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
    pos.ins().brnz(overflow, done_block, &[min_value]);
    pos.ins().jump(maxsat_block, &[]);

    // Finally, we could have a positive value that is too large.
    pos.insert_block(maxsat_block);
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
    pos.ins().brnz(overflow, done_block, &[max_value]);

    // Recycle the original instruction.
    pos.func.dfg.replace(inst).jump(done_block, &[cvtt2si]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_block(done_block);

    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, intmin_block);
    cfg.recompute_block(pos.func, minsat_block);
    cfg.recompute_block(pos.func, maxsat_block);
    cfg.recompute_block(pos.func, done_block);
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

    let old_block = func.layout.pp_block(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // block handle numbers < 2^(N-1).
    let below_uint_max_block = func.dfg.make_block();

    // block handle numbers < 0.
    let below_zero_block = func.dfg.make_block();

    // block handling numbers >= 2^(N-1).
    let large = func.dfg.make_block();

    // Final block after the bad value checks.
    let done = func.dfg.make_block();

    // Move the `inst` result value onto the `done` block.
    func.dfg.clear_results(inst);
    func.dfg.attach_block_param(done, result);

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
    pos.ins().jump(below_uint_max_block, &[]);

    // We need to generate a specific trap code when `x` is NaN, so reuse the flags from the
    // previous comparison.
    pos.insert_block(below_uint_max_block);
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
    pos.ins().jump(below_zero_block, &[]);

    pos.insert_block(below_zero_block);
    pos.ins().trap(ir::TrapCode::IntegerOverflow);

    // Handle the case where x >= 2^(N-1) and not NaN.
    pos.insert_block(large);
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
    pos.insert_block(done);

    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, below_uint_max_block);
    cfg.recompute_block(pos.func, below_zero_block);
    cfg.recompute_block(pos.func, large);
    cfg.recompute_block(pos.func, done);
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

    let old_block = func.layout.pp_block(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // block handle numbers < 2^(N-1).
    let below_pow2nm1_or_nan_block = func.dfg.make_block();
    let below_pow2nm1_block = func.dfg.make_block();

    // block handling numbers >= 2^(N-1).
    let large = func.dfg.make_block();

    // block handling numbers < 2^N.
    let uint_large_block = func.dfg.make_block();

    // Final block after the bad value checks.
    let done = func.dfg.make_block();

    // Move the `inst` result value onto the `done` block.
    func.dfg.clear_results(inst);
    func.dfg.attach_block_param(done, result);

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
    pos.ins().jump(below_pow2nm1_or_nan_block, &[]);

    // We need to generate zero when `x` is NaN, so reuse the flags from the previous comparison.
    pos.insert_block(below_pow2nm1_or_nan_block);
    pos.ins().brff(FloatCC::Unordered, is_large, done, &[zero]);
    pos.ins().jump(below_pow2nm1_block, &[]);

    // Now we know that x < 2^(N-1) and not NaN. If the result of the cvtt2si is positive, we're
    // done; otherwise saturate to the minimum unsigned value, that is 0.
    pos.insert_block(below_pow2nm1_block);
    let sres = pos.ins().x86_cvtt2si(ty, x);
    let is_neg = pos.ins().ifcmp_imm(sres, 0);
    pos.ins()
        .brif(IntCC::SignedGreaterThanOrEqual, is_neg, done, &[sres]);
    pos.ins().jump(done, &[zero]);

    // Handle the case where x >= 2^(N-1) and not NaN.
    pos.insert_block(large);
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
    pos.ins().jump(uint_large_block, &[]);

    pos.insert_block(uint_large_block);
    let lfinal = pos.ins().iadd_imm(lres, 1 << (ty.lane_bits() - 1));

    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[lfinal]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_block(done);

    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, below_pow2nm1_or_nan_block);
    cfg.recompute_block(pos.func, below_pow2nm1_block);
    cfg.recompute_block(pos.func, large);
    cfg.recompute_block(pos.func, uint_large_block);
    cfg.recompute_block(pos.func, done);
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

    if let ir::InstructionData::BinaryImm8 {
        opcode: ir::Opcode::Extractlane,
        arg,
        imm: lane,
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

    if let ir::InstructionData::TernaryImm8 {
        opcode: ir::Opcode::Insertlane,
        args: [vector, replacement],
        imm: lane,
    } = pos.func.dfg[inst]
    {
        let value_type = pos.func.dfg.value_type(vector);
        if value_type.lane_type().is_float() {
            // Floats are already in XMM registers and can stay there.
            match value_type {
                F32X4 => {
                    assert!(lane <= 3);
                    let immediate = 0b00_00_00_00 | lane << 4;
                    // Insert 32-bits from replacement (at index 00, bits 7:8) to vector (lane
                    // shifted into bits 5:6).
                    pos.func
                        .dfg
                        .replace(inst)
                        .x86_insertps(vector, replacement, immediate)
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
                .x86_pinsr(vector, replacement, lane);
        }
    }
}

/// For SIMD or scalar integer negation, convert `ineg` to `vconst + isub` or `iconst + isub`.
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
        let zero_value = if value_type.is_vector() && value_type.lane_type().is_int() {
            let zero_immediate = pos.func.dfg.constants.insert(vec![0; 16].into());
            pos.ins().vconst(value_type, zero_immediate) // this should be legalized to a PXOR
        } else if value_type.is_int() {
            pos.ins().iconst(value_type, 0)
        } else {
            panic!("Can't convert ineg of type {}", value_type)
        };
        pos.func.dfg.replace(inst).isub(zero_value, arg);
    } else {
        unreachable!()
    }
}

fn expand_dword_to_xmm<'f>(
    pos: &mut FuncCursor<'_>,
    arg: ir::Value,
    arg_type: ir::Type,
) -> ir::Value {
    if arg_type == I64 {
        let (arg_lo, arg_hi) = pos.ins().isplit(arg);
        let arg = pos.ins().scalar_to_vector(I32X4, arg_lo);
        let arg = pos.ins().insertlane(arg, arg_hi, 1);
        let arg = pos.ins().raw_bitcast(I64X2, arg);
        arg
    } else {
        pos.ins().bitcast(I64X2, arg)
    }
}

fn contract_dword_from_xmm<'f>(
    pos: &mut FuncCursor<'f>,
    inst: ir::Inst,
    ret: ir::Value,
    ret_type: ir::Type,
) {
    if ret_type == I64 {
        let ret = pos.ins().raw_bitcast(I32X4, ret);
        let ret_lo = pos.ins().extractlane(ret, 0);
        let ret_hi = pos.ins().extractlane(ret, 1);
        pos.func.dfg.replace(inst).iconcat(ret_lo, ret_hi);
    } else {
        let ret = pos.ins().extractlane(ret, 0);
        pos.func.dfg.replace(inst).ireduce(ret_type, ret);
    }
}

// Masks for i8x16 unsigned right shift.
static USHR_MASKS: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f,
    0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f,
    0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f,
    0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
];

// Convert a vector unsigned right shift. x86 has implementations for i16x8 and up (see `x86_pslr`),
// but for i8x16 we translate the shift to a i16x8 shift and mask off the upper bits. This same
// conversion could be provided in the CDSL if we could use varargs there (TODO); i.e. `load_complex`
// has a varargs field that we can't modify with the CDSL in legalize.rs.
fn convert_ushr(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    if let ir::InstructionData::Binary {
        opcode: ir::Opcode::Ushr,
        args: [arg0, arg1],
    } = pos.func.dfg[inst]
    {
        // Note that for Wasm, the bounding of the shift index has happened during translation
        let arg0_type = pos.func.dfg.value_type(arg0);
        let arg1_type = pos.func.dfg.value_type(arg1);
        assert!(!arg1_type.is_vector() && arg1_type.is_int());

        // TODO it may be more clear to use scalar_to_vector here; the current issue is that
        // scalar_to_vector has the restriction that the vector produced has a matching lane size
        // (e.g. i32 -> i32x4) whereas bitcast allows moving any-to-any conversions (e.g. i32 ->
        // i64x2). This matters because for some reason x86_psrl only allows i64x2 as the shift
        // index type--this could be relaxed since it is not really meaningful.
        let shift_index = pos.ins().bitcast(I64X2, arg1);

        if arg0_type == I8X16 {
            // First, shift the vector using an I16X8 shift.
            let bitcasted = pos.ins().raw_bitcast(I16X8, arg0);
            let shifted = pos.ins().x86_psrl(bitcasted, shift_index);
            let shifted = pos.ins().raw_bitcast(I8X16, shifted);

            // Then, fixup the even lanes that have incorrect upper bits. This uses the 128 mask
            // bytes as a table that we index into. It is a substantial code-size increase but
            // reduces the instruction count slightly.
            let masks = pos.func.dfg.constants.insert(USHR_MASKS.as_ref().into());
            let mask_address = pos.ins().const_addr(isa.pointer_type(), masks);
            let mask_offset = pos.ins().ishl_imm(arg1, 4);
            let mask =
                pos.ins()
                    .load_complex(arg0_type, MemFlags::new(), &[mask_address, mask_offset], 0);
            pos.func.dfg.replace(inst).band(shifted, mask);
        } else if arg0_type.is_vector() {
            // x86 has encodings for these shifts.
            pos.func.dfg.replace(inst).x86_psrl(arg0, shift_index);
        } else if arg0_type == I64 {
            // 64 bit shifts need to be legalized on x86_32.
            let x86_isa = isa
                .as_any()
                .downcast_ref::<isa::x86::Isa>()
                .expect("the target ISA must be x86 at this point");
            if x86_isa.isa_flags.has_sse41() {
                // if we have pinstrq/pextrq (SSE 4.1), legalize to that
                let value = expand_dword_to_xmm(&mut pos, arg0, arg0_type);
                let amount = expand_dword_to_xmm(&mut pos, arg1, arg1_type);
                let shifted = pos.ins().x86_psrl(value, amount);
                contract_dword_from_xmm(&mut pos, inst, shifted, arg0_type);
            } else {
                // otherwise legalize to libcall
                expand_as_libcall(inst, func, isa);
            }
        } else {
            // Everything else should be already legal.
            unreachable!()
        }
    }
}

// Masks for i8x16 left shift.
static SHL_MASKS: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe,
    0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc,
    0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8,
    0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0,
    0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0,
    0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0,
    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
];

// Convert a vector left shift. x86 has implementations for i16x8 and up (see `x86_psll`),
// but for i8x16 we translate the shift to a i16x8 shift and mask off the lower bits. This same
// conversion could be provided in the CDSL if we could use varargs there (TODO); i.e. `load_complex`
// has a varargs field that we can't modify with the CDSL in legalize.rs.
fn convert_ishl(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    if let ir::InstructionData::Binary {
        opcode: ir::Opcode::Ishl,
        args: [arg0, arg1],
    } = pos.func.dfg[inst]
    {
        // Note that for Wasm, the bounding of the shift index has happened during translation
        let arg0_type = pos.func.dfg.value_type(arg0);
        let arg1_type = pos.func.dfg.value_type(arg1);
        assert!(!arg1_type.is_vector() && arg1_type.is_int());

        // TODO it may be more clear to use scalar_to_vector here; the current issue is that
        // scalar_to_vector has the restriction that the vector produced has a matching lane size
        // (e.g. i32 -> i32x4) whereas bitcast allows moving any-to-any conversions (e.g. i32 ->
        // i64x2). This matters because for some reason x86_psrl only allows i64x2 as the shift
        // index type--this could be relaxed since it is not really meaningful.
        let shift_index = pos.ins().bitcast(I64X2, arg1);

        if arg0_type == I8X16 {
            // First, shift the vector using an I16X8 shift.
            let bitcasted = pos.ins().raw_bitcast(I16X8, arg0);
            let shifted = pos.ins().x86_psll(bitcasted, shift_index);
            let shifted = pos.ins().raw_bitcast(I8X16, shifted);

            // Then, fixup the even lanes that have incorrect lower bits. This uses the 128 mask
            // bytes as a table that we index into. It is a substantial code-size increase but
            // reduces the instruction count slightly.
            let masks = pos.func.dfg.constants.insert(SHL_MASKS.as_ref().into());
            let mask_address = pos.ins().const_addr(isa.pointer_type(), masks);
            let mask_offset = pos.ins().ishl_imm(arg1, 4);
            let mask =
                pos.ins()
                    .load_complex(arg0_type, MemFlags::new(), &[mask_address, mask_offset], 0);
            pos.func.dfg.replace(inst).band(shifted, mask);
        } else if arg0_type.is_vector() {
            // x86 has encodings for these shifts.
            pos.func.dfg.replace(inst).x86_psll(arg0, shift_index);
        } else if arg0_type == I64 {
            // 64 bit shifts need to be legalized on x86_32.
            let x86_isa = isa
                .as_any()
                .downcast_ref::<isa::x86::Isa>()
                .expect("the target ISA must be x86 at this point");
            if x86_isa.isa_flags.has_sse41() {
                // if we have pinstrq/pextrq (SSE 4.1), legalize to that
                let value = expand_dword_to_xmm(&mut pos, arg0, arg0_type);
                let amount = expand_dword_to_xmm(&mut pos, arg1, arg1_type);
                let shifted = pos.ins().x86_psll(value, amount);
                contract_dword_from_xmm(&mut pos, inst, shifted, arg0_type);
            } else {
                // otherwise legalize to libcall
                expand_as_libcall(inst, func, isa);
            }
        } else {
            // Everything else should be already legal.
            unreachable!()
        }
    }
}

/// Convert an imul.i64x2 to a valid code sequence on x86, first with AVX512 and then with SSE2.
fn convert_i64x2_imul(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    if let ir::InstructionData::Binary {
        opcode: ir::Opcode::Imul,
        args: [arg0, arg1],
    } = pos.func.dfg[inst]
    {
        let ty = pos.func.dfg.ctrl_typevar(inst);
        if ty == I64X2 {
            let x86_isa = isa
                .as_any()
                .downcast_ref::<isa::x86::Isa>()
                .expect("the target ISA must be x86 at this point");
            if x86_isa.isa_flags.use_avx512dq_simd() || x86_isa.isa_flags.use_avx512vl_simd() {
                // If we have certain AVX512 features, we can lower this instruction simply.
                pos.func.dfg.replace(inst).x86_pmullq(arg0, arg1);
            } else {
                // Otherwise, we default to a very lengthy SSE2-compatible sequence. It splits each
                // 64-bit lane into 32-bit high and low sections using shifting and then performs
                // the following arithmetic per lane: with arg0 = concat(high0, low0) and arg1 =
                // concat(high1, low1), calculate (high0 * low1) + (high1 * low0) + (low0 * low1).
                let high0 = pos.ins().ushr_imm(arg0, 32);
                let mul0 = pos.ins().x86_pmuludq(high0, arg1);
                let high1 = pos.ins().ushr_imm(arg1, 32);
                let mul1 = pos.ins().x86_pmuludq(high1, arg0);
                let addhigh = pos.ins().iadd(mul0, mul1);
                let high = pos.ins().ishl_imm(addhigh, 32);
                let low = pos.ins().x86_pmuludq(arg0, arg1);
                pos.func.dfg.replace(inst).iadd(low, high);
            }
        } else {
            unreachable!(
                "{} should be encodable; it cannot be legalized by convert_i64x2_imul",
                pos.func.dfg.display_inst(inst, None)
            );
        }
    }
}

fn expand_tls_value(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    use crate::settings::TlsModel;

    assert!(
        isa.triple().architecture == target_lexicon::Architecture::X86_64,
        "Not yet implemented for {:?}",
        isa.triple(),
    );

    if let ir::InstructionData::UnaryGlobalValue {
        opcode: ir::Opcode::TlsValue,
        global_value,
    } = func.dfg[inst]
    {
        let ctrl_typevar = func.dfg.ctrl_typevar(inst);
        assert_eq!(ctrl_typevar, ir::types::I64);

        match isa.flags().tls_model() {
            TlsModel::None => panic!("tls_model flag is not set."),
            TlsModel::ElfGd => {
                func.dfg.replace(inst).x86_elf_tls_get_addr(global_value);
            }
            TlsModel::Macho => {
                func.dfg.replace(inst).x86_macho_tls_get_addr(global_value);
            }
            model => unimplemented!("tls_value for tls model {:?}", model),
        }
    } else {
        unreachable!();
    }
}
