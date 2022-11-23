use crate::{FuzzGen, Type};
use anyhow::Result;
use cranelift::codegen::cursor::{Cursor, FuncCursor};
use cranelift::codegen::ir::{Function, Inst, Opcode};
use cranelift::prelude::{types::*, *};

pub fn do_fcvt_trap_pass(fuzz: &mut FuzzGen, func: &mut Function) -> Result<()> {
    let ratio = fuzz.config.allowed_fcvt_traps_ratio;
    let insert_seq = !fuzz.u.ratio(ratio.0, ratio.1)?;
    if !insert_seq {
        return Ok(());
    }

    let mut pos = FuncCursor::new(func);
    while let Some(_block) = pos.next_block() {
        while let Some(inst) = pos.next_inst() {
            if can_fcvt_trap(&pos, inst) {
                insert_fcvt_sequence(&mut pos, inst);
            }
        }
    }
    Ok(())
}

/// Returns true/false if this instruction can trap
fn can_fcvt_trap(pos: &FuncCursor, inst: Inst) -> bool {
    let opcode = pos.func.dfg[inst].opcode();

    matches!(opcode, Opcode::FcvtToUint | Opcode::FcvtToSint)
}

/// Gets the max and min float values for this integer type
/// Inserts fconst instructions with these values.
//
// When converting to integers, floats are truncated. This means that the maximum float value
// that can be converted into an i8 is 127.99999. And surprisingly the minimum float for an
// u8 is -0.99999! So get the limits of this type as a float value by adding or subtracting
// 1.0 from its min and max integer values.
fn float_limits(
    pos: &mut FuncCursor,
    float_ty: Type,
    int_ty: Type,
    is_signed: bool,
) -> (Value, Value) {
    let (min_int, max_int) = int_ty.bounds(is_signed);

    if float_ty == F32 {
        let (min, max) = if is_signed {
            ((min_int as i128) as f32, (max_int as i128) as f32)
        } else {
            (min_int as f32, max_int as f32)
        };

        (pos.ins().f32const(min - 1.0), pos.ins().f32const(max + 1.0))
    } else {
        let (min, max) = if is_signed {
            ((min_int as i128) as f64, (max_int as i128) as f64)
        } else {
            (min_int as f64, max_int as f64)
        };

        (pos.ins().f64const(min - 1.0), pos.ins().f64const(max + 1.0))
    }
}

/// Prepend instructions to inst to avoid traps
fn insert_fcvt_sequence(pos: &mut FuncCursor, inst: Inst) {
    let dfg = &pos.func.dfg;
    let opcode = dfg[inst].opcode();
    let arg = dfg.inst_args(inst)[0];
    let float_ty = dfg.value_type(arg);
    let int_ty = dfg.value_type(dfg.first_result(inst));

    // These instructions trap on NaN
    let is_nan = pos.ins().fcmp(FloatCC::NotEqual, arg, arg);

    // They also trap if the value is larger or smaller than what the integer type can represent. So
    // we generate the maximum and minimum float value that would make this trap, and compare against
    // those limits.
    let is_signed = opcode == Opcode::FcvtToSint;
    let (min, max) = float_limits(pos, float_ty, int_ty, is_signed);
    let underflows = pos.ins().fcmp(FloatCC::LessThanOrEqual, arg, min);
    let overflows = pos.ins().fcmp(FloatCC::GreaterThanOrEqual, arg, max);

    // Check the previous conditions and replace with a 1.0 if this instruction would trap
    let overflows_int = pos.ins().bor(underflows, overflows);
    let is_invalid = pos.ins().bor(is_nan, overflows_int);

    let one = if float_ty == F32 {
        pos.ins().f32const(1.0)
    } else {
        pos.ins().f64const(1.0)
    };
    let new_arg = pos.ins().select(is_invalid, one, arg);

    // Replace the previous arg with the new one
    pos.func.dfg.inst_args_mut(inst)[0] = new_arg;
}
