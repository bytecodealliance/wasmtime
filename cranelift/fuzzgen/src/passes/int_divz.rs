use crate::FuzzGen;
use anyhow::Result;
use cranelift::codegen::cursor::{Cursor, FuncCursor};
use cranelift::codegen::ir::{Function, Inst, Opcode};
use cranelift::prelude::{InstBuilder, IntCC};

pub fn do_int_divz_pass(fuzz: &mut FuzzGen, func: &mut Function) -> Result<()> {
    // Insert this per function, otherwise the actual rate of int_divz doesn't go down that much
    // Experimentally if we decide this per instruction with a 0.1% allow rate, we get 4.4% of runs
    // trapping. Doing this per function decreases the number of runs that trap. It also consumes
    // fewer fuzzer input bytes which is nice.
    let ratio = fuzz.config.allowed_int_divz_ratio;
    let insert_seq = !fuzz.u.ratio(ratio.0, ratio.1)?;
    if !insert_seq {
        return Ok(());
    }

    let mut pos = FuncCursor::new(func);
    while let Some(_block) = pos.next_block() {
        while let Some(inst) = pos.next_inst() {
            if can_int_divz(&pos, inst) {
                insert_int_divz_sequence(&mut pos, inst);
            }
        }
    }
    Ok(())
}

/// Returns true/false if this instruction can cause a `int_divz` trap
fn can_int_divz(pos: &FuncCursor, inst: Inst) -> bool {
    let opcode = pos.func.dfg[inst].opcode();

    matches!(
        opcode,
        Opcode::Sdiv | Opcode::Udiv | Opcode::Srem | Opcode::Urem
    )
}

/// Prepend instructions to inst to avoid `int_divz` traps
fn insert_int_divz_sequence(pos: &mut FuncCursor, inst: Inst) {
    let opcode = pos.func.dfg[inst].opcode();
    let inst_args = pos.func.dfg.inst_args(inst);
    let (lhs, rhs) = (inst_args[0], inst_args[1]);
    assert_eq!(pos.func.dfg.value_type(lhs), pos.func.dfg.value_type(rhs));
    let ty = pos.func.dfg.value_type(lhs);

    // All of these instructions can trap if the denominator is zero
    let zero = pos.ins().iconst(ty, 0);
    let one = pos.ins().iconst(ty, 1);
    let denominator_is_zero = pos.ins().icmp(IntCC::Equal, rhs, zero);

    let replace_denominator = if matches!(opcode, Opcode::Srem | Opcode::Sdiv) {
        // Srem and Sdiv can also trap on INT_MIN / -1. So we need to check for the second one

        // 1 << (ty bits - 1) to get INT_MIN
        let int_min = pos.ins().ishl_imm(one, ty.lane_bits() as i64 - 1);

        // Get a -1 const
        // TODO: A iconst -1 would be clearer, but #2906 makes this impossible for i128
        let neg_one = pos.ins().isub(zero, one);

        let lhs_check = pos.ins().icmp(IntCC::Equal, lhs, int_min);
        let rhs_check = pos.ins().icmp(IntCC::Equal, rhs, neg_one);
        let is_invalid = pos.ins().band(lhs_check, rhs_check);

        // These also crash if the denominator is zero, so we still need to check for that.
        pos.ins().bor(denominator_is_zero, is_invalid)
    } else {
        denominator_is_zero
    };

    // If we have a trap we replace the denominator with a 1
    let new_rhs = pos.ins().select(replace_denominator, one, rhs);

    // Replace the previous rhs with the new one
    let args = pos.func.dfg.inst_args_mut(inst);
    args[1] = new_rhs;
}
