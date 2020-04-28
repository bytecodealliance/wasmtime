//! A pre-legalization rewriting pass.
//!
//! This module provides early-stage optimizations. The optimizations found
//! should be useful for already well-optimized code. More general purpose
//! early-stage optimizations can be found in the preopt crate.

use crate::cursor::{Cursor, FuncCursor};
use crate::divconst_magic_numbers::{magic_s32, magic_s64, magic_u32, magic_u64};
use crate::divconst_magic_numbers::{MS32, MS64, MU32, MU64};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::{
    condcodes::{CondCode, IntCC},
    instructions::Opcode,
    types::{I32, I64},
    Block, DataFlowGraph, Function, Inst, InstBuilder, InstructionData, Type, Value,
};
use crate::isa::TargetIsa;
use crate::peepmatic::ValueOrInst;
use crate::timing;

#[inline]
/// Replaces the unique result of the instruction inst to an alias of the given value, and
/// replaces the instruction with a nop. Can be used only on instructions producing one unique
/// result, otherwise will assert.
fn replace_single_result_with_alias(dfg: &mut DataFlowGraph, inst: Inst, value: Value) {
    // Replace the result value by an alias.
    let results = dfg.detach_results(inst);
    debug_assert!(results.len(&dfg.value_lists) == 1);
    let result = results.get(0, &dfg.value_lists).unwrap();
    dfg.change_to_alias(result, value);

    // Replace instruction by a nop.
    dfg.replace(inst).nop();
}

//----------------------------------------------------------------------
//
// Pattern-match helpers and transformation for div and rem by constants.

// Simple math helpers

/// if `x` is a power of two, or the negation thereof, return the power along
/// with a boolean that indicates whether `x` is negative. Else return None.
#[inline]
fn i32_is_power_of_two(x: i32) -> Option<(bool, u32)> {
    // We have to special-case this because abs(x) isn't representable.
    if x == -0x8000_0000 {
        return Some((true, 31));
    }
    let abs_x = i32::wrapping_abs(x) as u32;
    if abs_x.is_power_of_two() {
        return Some((x < 0, abs_x.trailing_zeros()));
    }
    None
}

/// Same comments as for i32_is_power_of_two apply.
#[inline]
fn i64_is_power_of_two(x: i64) -> Option<(bool, u32)> {
    // We have to special-case this because abs(x) isn't representable.
    if x == -0x8000_0000_0000_0000 {
        return Some((true, 63));
    }
    let abs_x = i64::wrapping_abs(x) as u64;
    if abs_x.is_power_of_two() {
        return Some((x < 0, abs_x.trailing_zeros()));
    }
    None
}

/// Representation of an instruction that can be replaced by a single division/remainder operation
/// between a left Value operand and a right immediate operand.
#[derive(Debug)]
enum DivRemByConstInfo {
    DivU32(Value, u32),
    DivU64(Value, u64),
    DivS32(Value, i32),
    DivS64(Value, i64),
    RemU32(Value, u32),
    RemU64(Value, u64),
    RemS32(Value, i32),
    RemS64(Value, i64),
}

/// Possibly create a DivRemByConstInfo from the given components, by figuring out which, if any,
/// of the 8 cases apply, and also taking care to sanity-check the immediate.
fn package_up_divrem_info(
    value: Value,
    value_type: Type,
    imm_i64: i64,
    is_signed: bool,
    is_rem: bool,
) -> Option<DivRemByConstInfo> {
    let imm_u64 = imm_i64 as u64;

    match (is_signed, value_type) {
        (false, I32) => {
            if imm_u64 < 0x1_0000_0000 {
                if is_rem {
                    Some(DivRemByConstInfo::RemU32(value, imm_u64 as u32))
                } else {
                    Some(DivRemByConstInfo::DivU32(value, imm_u64 as u32))
                }
            } else {
                None
            }
        }

        (false, I64) => {
            // unsigned 64, no range constraint.
            if is_rem {
                Some(DivRemByConstInfo::RemU64(value, imm_u64))
            } else {
                Some(DivRemByConstInfo::DivU64(value, imm_u64))
            }
        }

        (true, I32) => {
            if imm_u64 <= 0x7fff_ffff || imm_u64 >= 0xffff_ffff_8000_0000 {
                if is_rem {
                    Some(DivRemByConstInfo::RemS32(value, imm_u64 as i32))
                } else {
                    Some(DivRemByConstInfo::DivS32(value, imm_u64 as i32))
                }
            } else {
                None
            }
        }

        (true, I64) => {
            // signed 64, no range constraint.
            if is_rem {
                Some(DivRemByConstInfo::RemS64(value, imm_u64 as i64))
            } else {
                Some(DivRemByConstInfo::DivS64(value, imm_u64 as i64))
            }
        }

        _ => None,
    }
}

/// Examine `inst` to see if it is a div or rem by a constant, and if so return the operands,
/// signedness, operation size and div-vs-rem-ness in a handy bundle.
fn get_div_info(inst: Inst, dfg: &DataFlowGraph) -> Option<DivRemByConstInfo> {
    if let InstructionData::BinaryImm { opcode, arg, imm } = dfg[inst] {
        let (is_signed, is_rem) = match opcode {
            Opcode::UdivImm => (false, false),
            Opcode::UremImm => (false, true),
            Opcode::SdivImm => (true, false),
            Opcode::SremImm => (true, true),
            _ => return None,
        };
        return package_up_divrem_info(arg, dfg.value_type(arg), imm.into(), is_signed, is_rem);
    }

    None
}

/// Actually do the transformation given a bundle containing the relevant information.
/// `divrem_info` describes a div or rem by a constant, that `pos` currently points at, and `inst`
/// is the associated instruction.  `inst` is replaced by a sequence of other operations that
/// calculate the same result. Note that there are various `divrem_info` cases where we cannot do
/// any transformation, in which case `inst` is left unchanged.
fn do_divrem_transformation(divrem_info: &DivRemByConstInfo, pos: &mut FuncCursor, inst: Inst) {
    let is_rem = match *divrem_info {
        DivRemByConstInfo::DivU32(_, _)
        | DivRemByConstInfo::DivU64(_, _)
        | DivRemByConstInfo::DivS32(_, _)
        | DivRemByConstInfo::DivS64(_, _) => false,
        DivRemByConstInfo::RemU32(_, _)
        | DivRemByConstInfo::RemU64(_, _)
        | DivRemByConstInfo::RemS32(_, _)
        | DivRemByConstInfo::RemS64(_, _) => true,
    };

    match *divrem_info {
        // -------------------- U32 --------------------

        // U32 div, rem by zero: ignore
        DivRemByConstInfo::DivU32(_n1, 0) | DivRemByConstInfo::RemU32(_n1, 0) => {}

        // U32 div by 1: identity
        // U32 rem by 1: zero
        DivRemByConstInfo::DivU32(_, 1) | DivRemByConstInfo::RemU32(_, 1) => {
            unreachable!("unsigned division and remainder by one is handled in `preopt.peepmatic`");
        }

        // U32 div, rem by a power-of-2
        DivRemByConstInfo::DivU32(n1, d) | DivRemByConstInfo::RemU32(n1, d)
            if d.is_power_of_two() =>
        {
            debug_assert!(d >= 2);
            // compute k where d == 2^k
            let k = d.trailing_zeros();
            debug_assert!(k >= 1 && k <= 31);
            if is_rem {
                let mask = (1u64 << k) - 1;
                pos.func.dfg.replace(inst).band_imm(n1, mask as i64);
            } else {
                unreachable!(
                    "unsigned division by a power of two is handled in \
                     `preopt.peepmatic`"
                );
            }
        }

        // U32 div, rem by non-power-of-2
        DivRemByConstInfo::DivU32(n1, d) | DivRemByConstInfo::RemU32(n1, d) => {
            debug_assert!(d >= 3);
            let MU32 {
                mul_by,
                do_add,
                shift_by,
            } = magic_u32(d);
            let qf; // final quotient
            let q0 = pos.ins().iconst(I32, mul_by as i64);
            let q1 = pos.ins().umulhi(n1, q0);
            if do_add {
                debug_assert!(shift_by >= 1 && shift_by <= 32);
                let t1 = pos.ins().isub(n1, q1);
                let t2 = pos.ins().ushr_imm(t1, 1);
                let t3 = pos.ins().iadd(t2, q1);
                // I never found any case where shift_by == 1 here.
                // So there's no attempt to fold out a zero shift.
                debug_assert_ne!(shift_by, 1);
                qf = pos.ins().ushr_imm(t3, (shift_by - 1) as i64);
            } else {
                debug_assert!(shift_by >= 0 && shift_by <= 31);
                // Whereas there are known cases here for shift_by == 0.
                if shift_by > 0 {
                    qf = pos.ins().ushr_imm(q1, shift_by as i64);
                } else {
                    qf = q1;
                }
            }
            // Now qf holds the final quotient. If necessary calculate the
            // remainder instead.
            if is_rem {
                let tt = pos.ins().imul_imm(qf, d as i64);
                pos.func.dfg.replace(inst).isub(n1, tt);
            } else {
                replace_single_result_with_alias(&mut pos.func.dfg, inst, qf);
            }
        }

        // -------------------- U64 --------------------

        // U64 div, rem by zero: ignore
        DivRemByConstInfo::DivU64(_n1, 0) | DivRemByConstInfo::RemU64(_n1, 0) => {}

        // U64 div by 1: identity
        // U64 rem by 1: zero
        DivRemByConstInfo::DivU64(_, 1) | DivRemByConstInfo::RemU64(_, 1) => {
            unreachable!("unsigned division and remainder by one is handled in `preopt.peepmatic`");
        }

        // U64 div, rem by a power-of-2
        DivRemByConstInfo::DivU64(n1, d) | DivRemByConstInfo::RemU64(n1, d)
            if d.is_power_of_two() =>
        {
            debug_assert!(d >= 2);
            // compute k where d == 2^k
            let k = d.trailing_zeros();
            debug_assert!(k >= 1 && k <= 63);
            if is_rem {
                let mask = (1u64 << k) - 1;
                pos.func.dfg.replace(inst).band_imm(n1, mask as i64);
            } else {
                unreachable!(
                    "unsigned division by a power of two is handled in `preopt.peepmatic`"
                );
            }
        }

        // U64 div, rem by non-power-of-2
        DivRemByConstInfo::DivU64(n1, d) | DivRemByConstInfo::RemU64(n1, d) => {
            debug_assert!(d >= 3);
            let MU64 {
                mul_by,
                do_add,
                shift_by,
            } = magic_u64(d);
            let qf; // final quotient
            let q0 = pos.ins().iconst(I64, mul_by as i64);
            let q1 = pos.ins().umulhi(n1, q0);
            if do_add {
                debug_assert!(shift_by >= 1 && shift_by <= 64);
                let t1 = pos.ins().isub(n1, q1);
                let t2 = pos.ins().ushr_imm(t1, 1);
                let t3 = pos.ins().iadd(t2, q1);
                // I never found any case where shift_by == 1 here.
                // So there's no attempt to fold out a zero shift.
                debug_assert_ne!(shift_by, 1);
                qf = pos.ins().ushr_imm(t3, (shift_by - 1) as i64);
            } else {
                debug_assert!(shift_by >= 0 && shift_by <= 63);
                // Whereas there are known cases here for shift_by == 0.
                if shift_by > 0 {
                    qf = pos.ins().ushr_imm(q1, shift_by as i64);
                } else {
                    qf = q1;
                }
            }
            // Now qf holds the final quotient. If necessary calculate the
            // remainder instead.
            if is_rem {
                let tt = pos.ins().imul_imm(qf, d as i64);
                pos.func.dfg.replace(inst).isub(n1, tt);
            } else {
                replace_single_result_with_alias(&mut pos.func.dfg, inst, qf);
            }
        }

        // -------------------- S32 --------------------

        // S32 div, rem by zero or -1: ignore
        DivRemByConstInfo::DivS32(_n1, -1)
        | DivRemByConstInfo::RemS32(_n1, -1)
        | DivRemByConstInfo::DivS32(_n1, 0)
        | DivRemByConstInfo::RemS32(_n1, 0) => {}

        // S32 div by 1: identity
        // S32 rem by 1: zero
        DivRemByConstInfo::DivS32(_, 1) | DivRemByConstInfo::RemS32(_, 1) => {
            unreachable!("signed division and remainder by one is handled in `preopt.peepmatic`");
        }

        DivRemByConstInfo::DivS32(n1, d) | DivRemByConstInfo::RemS32(n1, d) => {
            if let Some((is_negative, k)) = i32_is_power_of_two(d) {
                // k can be 31 only in the case that d is -2^31.
                debug_assert!(k >= 1 && k <= 31);
                let t1 = if k - 1 == 0 {
                    n1
                } else {
                    pos.ins().sshr_imm(n1, (k - 1) as i64)
                };
                let t2 = pos.ins().ushr_imm(t1, (32 - k) as i64);
                let t3 = pos.ins().iadd(n1, t2);
                if is_rem {
                    // S32 rem by a power-of-2
                    let t4 = pos.ins().band_imm(t3, i32::wrapping_neg(1 << k) as i64);
                    // Curiously, we don't care here what the sign of d is.
                    pos.func.dfg.replace(inst).isub(n1, t4);
                } else {
                    // S32 div by a power-of-2
                    let t4 = pos.ins().sshr_imm(t3, k as i64);
                    if is_negative {
                        pos.func.dfg.replace(inst).irsub_imm(t4, 0);
                    } else {
                        replace_single_result_with_alias(&mut pos.func.dfg, inst, t4);
                    }
                }
            } else {
                // S32 div, rem by a non-power-of-2
                debug_assert!(d < -2 || d > 2);
                let MS32 { mul_by, shift_by } = magic_s32(d);
                let q0 = pos.ins().iconst(I32, mul_by as i64);
                let q1 = pos.ins().smulhi(n1, q0);
                let q2 = if d > 0 && mul_by < 0 {
                    pos.ins().iadd(q1, n1)
                } else if d < 0 && mul_by > 0 {
                    pos.ins().isub(q1, n1)
                } else {
                    q1
                };
                debug_assert!(shift_by >= 0 && shift_by <= 31);
                let q3 = if shift_by == 0 {
                    q2
                } else {
                    pos.ins().sshr_imm(q2, shift_by as i64)
                };
                let t1 = pos.ins().ushr_imm(q3, 31);
                let qf = pos.ins().iadd(q3, t1);
                // Now qf holds the final quotient. If necessary calculate
                // the remainder instead.
                if is_rem {
                    let tt = pos.ins().imul_imm(qf, d as i64);
                    pos.func.dfg.replace(inst).isub(n1, tt);
                } else {
                    replace_single_result_with_alias(&mut pos.func.dfg, inst, qf);
                }
            }
        }

        // -------------------- S64 --------------------

        // S64 div, rem by zero or -1: ignore
        DivRemByConstInfo::DivS64(_n1, -1)
        | DivRemByConstInfo::RemS64(_n1, -1)
        | DivRemByConstInfo::DivS64(_n1, 0)
        | DivRemByConstInfo::RemS64(_n1, 0) => {}

        // S64 div by 1: identity
        // S64 rem by 1: zero
        DivRemByConstInfo::DivS64(_, 1) | DivRemByConstInfo::RemS64(_, 1) => {
            unreachable!("division and remaineder by one are handled in `preopt.peepmatic`");
        }

        DivRemByConstInfo::DivS64(n1, d) | DivRemByConstInfo::RemS64(n1, d) => {
            if let Some((is_negative, k)) = i64_is_power_of_two(d) {
                // k can be 63 only in the case that d is -2^63.
                debug_assert!(k >= 1 && k <= 63);
                let t1 = if k - 1 == 0 {
                    n1
                } else {
                    pos.ins().sshr_imm(n1, (k - 1) as i64)
                };
                let t2 = pos.ins().ushr_imm(t1, (64 - k) as i64);
                let t3 = pos.ins().iadd(n1, t2);
                if is_rem {
                    // S64 rem by a power-of-2
                    let t4 = pos.ins().band_imm(t3, i64::wrapping_neg(1 << k));
                    // Curiously, we don't care here what the sign of d is.
                    pos.func.dfg.replace(inst).isub(n1, t4);
                } else {
                    // S64 div by a power-of-2
                    let t4 = pos.ins().sshr_imm(t3, k as i64);
                    if is_negative {
                        pos.func.dfg.replace(inst).irsub_imm(t4, 0);
                    } else {
                        replace_single_result_with_alias(&mut pos.func.dfg, inst, t4);
                    }
                }
            } else {
                // S64 div, rem by a non-power-of-2
                debug_assert!(d < -2 || d > 2);
                let MS64 { mul_by, shift_by } = magic_s64(d);
                let q0 = pos.ins().iconst(I64, mul_by);
                let q1 = pos.ins().smulhi(n1, q0);
                let q2 = if d > 0 && mul_by < 0 {
                    pos.ins().iadd(q1, n1)
                } else if d < 0 && mul_by > 0 {
                    pos.ins().isub(q1, n1)
                } else {
                    q1
                };
                debug_assert!(shift_by >= 0 && shift_by <= 63);
                let q3 = if shift_by == 0 {
                    q2
                } else {
                    pos.ins().sshr_imm(q2, shift_by as i64)
                };
                let t1 = pos.ins().ushr_imm(q3, 63);
                let qf = pos.ins().iadd(q3, t1);
                // Now qf holds the final quotient. If necessary calculate
                // the remainder instead.
                if is_rem {
                    let tt = pos.ins().imul_imm(qf, d);
                    pos.func.dfg.replace(inst).isub(n1, tt);
                } else {
                    replace_single_result_with_alias(&mut pos.func.dfg, inst, qf);
                }
            }
        }
    }
}

enum BranchOrderKind {
    BrzToBrnz(Value),
    BrnzToBrz(Value),
    InvertIcmpCond(IntCC, Value, Value),
}

/// Reorder branches to encourage fallthroughs.
///
/// When a block ends with a conditional branch followed by an unconditional
/// branch, this will reorder them if one of them is branching to the next Block
/// layout-wise. The unconditional jump can then become a fallthrough.
fn branch_order(pos: &mut FuncCursor, cfg: &mut ControlFlowGraph, block: Block, inst: Inst) {
    let (term_inst, term_inst_args, term_dest, cond_inst, cond_inst_args, cond_dest, kind) =
        match pos.func.dfg[inst] {
            InstructionData::Jump {
                opcode: Opcode::Jump,
                destination,
                ref args,
            } => {
                let next_block = if let Some(next_block) = pos.func.layout.next_block(block) {
                    next_block
                } else {
                    return;
                };

                if destination == next_block {
                    return;
                }

                let prev_inst = if let Some(prev_inst) = pos.func.layout.prev_inst(inst) {
                    prev_inst
                } else {
                    return;
                };

                let prev_inst_data = &pos.func.dfg[prev_inst];

                if let Some(prev_dest) = prev_inst_data.branch_destination() {
                    if prev_dest != next_block {
                        return;
                    }
                } else {
                    return;
                }

                match prev_inst_data {
                    InstructionData::Branch {
                        opcode,
                        args: ref prev_args,
                        destination: cond_dest,
                    } => {
                        let cond_arg = {
                            let args = pos.func.dfg.inst_args(prev_inst);
                            args[0]
                        };

                        let kind = match opcode {
                            Opcode::Brz => BranchOrderKind::BrzToBrnz(cond_arg),
                            Opcode::Brnz => BranchOrderKind::BrnzToBrz(cond_arg),
                            _ => panic!("unexpected opcode"),
                        };

                        (
                            inst,
                            args.clone(),
                            destination,
                            prev_inst,
                            prev_args.clone(),
                            *cond_dest,
                            kind,
                        )
                    }
                    InstructionData::BranchIcmp {
                        opcode: Opcode::BrIcmp,
                        cond,
                        destination: cond_dest,
                        args: ref prev_args,
                    } => {
                        let (x_arg, y_arg) = {
                            let args = pos.func.dfg.inst_args(prev_inst);
                            (args[0], args[1])
                        };

                        (
                            inst,
                            args.clone(),
                            destination,
                            prev_inst,
                            prev_args.clone(),
                            *cond_dest,
                            BranchOrderKind::InvertIcmpCond(*cond, x_arg, y_arg),
                        )
                    }
                    _ => return,
                }
            }

            _ => return,
        };

    let cond_args = cond_inst_args.as_slice(&pos.func.dfg.value_lists).to_vec();
    let term_args = term_inst_args.as_slice(&pos.func.dfg.value_lists).to_vec();

    match kind {
        BranchOrderKind::BrnzToBrz(cond_arg) => {
            pos.func
                .dfg
                .replace(term_inst)
                .jump(cond_dest, &cond_args[1..]);
            pos.func
                .dfg
                .replace(cond_inst)
                .brz(cond_arg, term_dest, &term_args);
        }
        BranchOrderKind::BrzToBrnz(cond_arg) => {
            pos.func
                .dfg
                .replace(term_inst)
                .jump(cond_dest, &cond_args[1..]);
            pos.func
                .dfg
                .replace(cond_inst)
                .brnz(cond_arg, term_dest, &term_args);
        }
        BranchOrderKind::InvertIcmpCond(cond, x_arg, y_arg) => {
            pos.func
                .dfg
                .replace(term_inst)
                .jump(cond_dest, &cond_args[2..]);
            pos.func.dfg.replace(cond_inst).br_icmp(
                cond.inverse(),
                x_arg,
                y_arg,
                term_dest,
                &term_args,
            );
        }
    }

    cfg.recompute_block(pos.func, block);
}

/// The main pre-opt pass.
pub fn do_preopt<'func, 'isa>(
    func: &'func mut Function,
    cfg: &mut ControlFlowGraph,
    isa: &'isa dyn TargetIsa,
) {
    let _tt = timing::preopt();

    let mut pos = FuncCursor::new(func);
    let mut preopt = crate::peepmatic::preopt(isa);

    while let Some(block) = pos.next_block() {
        while let Some(inst) = pos.next_inst() {
            preopt.apply_all(&mut pos, ValueOrInst::Inst(inst));
            let inst = pos.current_inst().unwrap();

            // Try to transform divide-by-constant into simpler operations.
            if let Some(divrem_info) = get_div_info(inst, &pos.func.dfg) {
                do_divrem_transformation(&divrem_info, &mut pos, inst);
                continue;
            }

            branch_order(&mut pos, cfg, block, inst);
        }
    }
}
