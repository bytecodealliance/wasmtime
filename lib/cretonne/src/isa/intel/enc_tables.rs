//! Encoding tables for Intel ISAs.

use bitset::BitSet;
use cursor::{Cursor, FuncCursor};
use flowgraph::ControlFlowGraph;
use ir::{self, InstBuilder};
use isa::constraints::*;
use isa::enc_tables::*;
use isa::encoding::RecipeSizing;
use isa;
use predicates;
use super::registers::*;

include!(concat!(env!("OUT_DIR"), "/encoding-intel.rs"));
include!(concat!(env!("OUT_DIR"), "/legalize-intel.rs"));

/// Expand the `srem` instruction using `x86_sdivmodx`.
fn expand_srem(inst: ir::Inst, func: &mut ir::Function, cfg: &mut ControlFlowGraph) {
    use ir::condcodes::IntCC;

    let (x, y) = match func.dfg[inst] {
        ir::InstructionData::Binary {
            opcode: ir::Opcode::Srem,
            args,
        } => (args[0], args[1]),
        _ => panic!("Need srem: {}", func.dfg.display_inst(inst, None)),
    };
    let old_ebb = func.layout.pp_ebb(inst);

    // EBB handling the -1 divisor case.
    let minus_one = func.dfg.make_ebb();

    // Final EBB with one argument representing the final result value.
    let done = func.dfg.make_ebb();

    // Move the `inst` result value onto the `done` EBB.
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);
    func.dfg.clear_results(inst);
    func.dfg.attach_ebb_arg(done, result);

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Start by checking for a -1 divisor which needs to be handled specially.
    let is_m1 = pos.ins().icmp_imm(IntCC::Equal, y, -1);
    pos.ins().brnz(is_m1, minus_one, &[]);

    // Now it is safe to execute the `x86_sdivmodx` instruction which will still trap on division
    // by zero.
    let xhi = pos.ins().sshr_imm(x, ty.lane_bits() as i64 - 1);
    let (_qout, rem) = pos.ins().x86_sdivmodx(x, xhi, y);
    pos.ins().jump(done, &[rem]);

    // Now deal with the -1 divisor which always yields a 0 remainder.
    pos.insert_ebb(minus_one);
    let zero = pos.ins().iconst(ty, 0);

    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[zero]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, minus_one);
    cfg.recompute_ebb(pos.func, done);
}

/// Expand the `fmin` and `fmax` instructions using the Intel `x86_fmin` and `x86_fmax`
/// instructions.
fn expand_minmax(inst: ir::Inst, func: &mut ir::Function, cfg: &mut ControlFlowGraph) {
    use ir::condcodes::FloatCC;

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

    // EBB handling case 3) where one operand is NaN.
    let uno_ebb = func.dfg.make_ebb();

    // EBB that handles the unordered or equal cases 2) and 3).
    let ueq_ebb = func.dfg.make_ebb();

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
    func.dfg.attach_ebb_arg(done, result);

    // Test for case 1) ordered and not equal.
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);
    let cmp_ueq = pos.ins().fcmp(FloatCC::UnorderedOrEqual, x, y);
    pos.ins().brnz(cmp_ueq, ueq_ebb, &[]);

    // Handle the common ordered, not equal (LT|GT) case.
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

    // We are now in case 2) where x and y compare EQ.
    // We need a bitwise operation to get the sign right.
    let bw_inst = pos.ins().Binary(bitwise_opc, ty, x, y).0;
    let bw_result = pos.func.dfg.first_result(bw_inst);
    // This should become a fall-through for this second most common case.
    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[bw_result]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, ueq_ebb);
    cfg.recompute_ebb(pos.func, uno_ebb);
    cfg.recompute_ebb(pos.func, done);
}

/// Intel has no unsigned-to-float conversions. We handle the easy case of zero-extending i32 to
/// i64 with a pattern, the rest needs more code.
fn expand_fcvt_from_uint(inst: ir::Inst, func: &mut ir::Function, cfg: &mut ControlFlowGraph) {
    use ir::condcodes::IntCC;

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

    // Conversion from unsigned 32-bit is easy on x86-64.
    // TODO: This should be guarded by an ISA check.
    if xty == ir::types::I32 {
        let wide = pos.ins().uextend(ir::types::I64, x);
        pos.func.dfg.replace(inst).fcvt_from_sint(ty, wide);
        return;
    }

    let old_ebb = pos.func.layout.pp_ebb(inst);

    // EBB handling the case where x < 0.
    let neg_ebb = pos.func.dfg.make_ebb();

    // Final EBB with one argument representing the final result value.
    let done = pos.func.dfg.make_ebb();

    // Move the `inst` result value onto the `done` EBB.
    pos.func.dfg.clear_results(inst);
    pos.func.dfg.attach_ebb_arg(done, result);

    // If x as a signed int is not negative, we can use the existing `fcvt_from_sint` instruction.
    let is_neg = pos.ins().icmp_imm(IntCC::SignedLessThan, x, 0);
    pos.ins().brnz(is_neg, neg_ebb, &[]);

    // Easy case: just use a signed conversion.
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
    cfg.recompute_ebb(pos.func, neg_ebb);
    cfg.recompute_ebb(pos.func, done);
}

fn expand_fcvt_to_sint(inst: ir::Inst, func: &mut ir::Function, cfg: &mut ControlFlowGraph) {
    use ir::condcodes::{IntCC, FloatCC};
    use ir::immediates::{Ieee32, Ieee64};

    let x;
    match func.dfg[inst] {
        ir::InstructionData::Unary {
            opcode: ir::Opcode::FcvtToSint,
            arg,
        } => x = arg,
        _ => panic!("Need fcvt_to_sint: {}", func.dfg.display_inst(inst, None)),
    }
    let old_ebb = func.layout.pp_ebb(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // Final EBB after the bad value checks.
    let done = func.dfg.make_ebb();

    // The `x86_cvtt2si` performs the desired conversion, but it doesn't trap on NaN or overflow.
    // It produces an INT_MIN result instead.
    func.dfg.replace(inst).x86_cvtt2si(ty, x);

    let mut pos = FuncCursor::new(func).after_inst(inst);
    pos.use_srcloc(inst);

    let is_done = pos.ins().icmp_imm(
        IntCC::NotEqual,
        result,
        1 << (ty.lane_bits() - 1),
    );
    pos.ins().brnz(is_done, done, &[]);

    // We now have the following possibilities:
    //
    // 1. INT_MIN was actually the correct conversion result.
    // 2. The input was NaN -> trap bad_toint
    // 3. The input was out of range -> trap int_ovf
    //

    // Check for NaN.
    let is_nan = pos.ins().fcmp(FloatCC::Unordered, x, x);
    pos.ins().trapnz(
        is_nan,
        ir::TrapCode::BadConversionToInteger,
    );

    // Check for case 1: INT_MIN is the correct result.
    // Determine the smallest floating point number that would convert to INT_MIN.
    let mut overflow_cc = FloatCC::LessThan;
    let output_bits = ty.lane_bits();
    let flimit = match xty {
        ir::types::F32 => pos.ins().f32const(Ieee32::pow2(output_bits - 1).neg()),
        ir::types::F64 => {
            // An f64 can represent `i32::min_value() - 1` exactly with precision to spare, so
            // there are values less than -2^(N-1) that convert correctly to INT_MIN.
            pos.ins().f64const(if output_bits < 64 {
                overflow_cc = FloatCC::LessThanOrEqual;
                Ieee64::with_float(-((1u64 << (output_bits - 1)) as f64) - 1.0)
            } else {
                Ieee64::pow2(output_bits - 1).neg()
            })
        }
        _ => panic!("Can't convert {}", xty),
    };
    let overflow = pos.ins().fcmp(overflow_cc, x, flimit);
    pos.ins().trapnz(overflow, ir::TrapCode::IntegerOverflow);

    pos.ins().jump(done, &[]);
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, done);
}

fn expand_fcvt_to_uint(inst: ir::Inst, func: &mut ir::Function, cfg: &mut ControlFlowGraph) {
    use ir::condcodes::{IntCC, FloatCC};
    use ir::immediates::{Ieee32, Ieee64};

    let x;
    match func.dfg[inst] {
        ir::InstructionData::Unary {
            opcode: ir::Opcode::FcvtToUint,
            arg,
        } => x = arg,
        _ => panic!("Need fcvt_to_uint: {}", func.dfg.display_inst(inst, None)),
    }
    let old_ebb = func.layout.pp_ebb(inst);
    let xty = func.dfg.value_type(x);
    let result = func.dfg.first_result(inst);
    let ty = func.dfg.value_type(result);

    // EBB handling numbers >= 2^(N-1).
    let large = func.dfg.make_ebb();

    // Final EBB after the bad value checks.
    let done = func.dfg.make_ebb();

    // Move the `inst` result value onto the `done` EBB.
    func.dfg.clear_results(inst);
    func.dfg.attach_ebb_arg(done, result);

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Start by materializing the floating point constant 2^(N-1) where N is the number of bits in
    // the destination integer type.
    let pow2nm1 = match xty {
        ir::types::F32 => pos.ins().f32const(Ieee32::pow2(ty.lane_bits() - 1)),
        ir::types::F64 => pos.ins().f64const(Ieee64::pow2(ty.lane_bits() - 1)),
        _ => panic!("Can't convert {}", xty),
    };
    let is_large = pos.ins().fcmp(FloatCC::GreaterThanOrEqual, x, pow2nm1);
    pos.ins().brnz(is_large, large, &[]);

    // Now we know that x < 2^(N-1) or x is NaN.
    let sres = pos.ins().x86_cvtt2si(ty, x);
    let is_neg = pos.ins().icmp_imm(IntCC::SignedLessThan, sres, 0);
    pos.ins().brz(is_neg, done, &[sres]);
    pos.ins().trap(ir::TrapCode::BadConversionToInteger);

    // Handle the case where x >= 2^(N-1) and not NaN.
    pos.insert_ebb(large);
    let adjx = pos.ins().fsub(x, pow2nm1);
    let lres = pos.ins().x86_cvtt2si(ty, adjx);
    let is_neg = pos.ins().icmp_imm(IntCC::SignedLessThan, lres, 0);
    pos.ins().trapnz(is_neg, ir::TrapCode::IntegerOverflow);
    let lfinal = pos.ins().iadd_imm(lres, 1 << (ty.lane_bits() - 1));

    // Recycle the original instruction as a jump.
    pos.func.dfg.replace(inst).jump(done, &[lfinal]);

    // Finally insert a label for the completion.
    pos.next_inst();
    pos.insert_ebb(done);

    cfg.recompute_ebb(pos.func, old_ebb);
    cfg.recompute_ebb(pos.func, large);
    cfg.recompute_ebb(pos.func, done);
}
