//! A pre-legalization rewriting pass.

#![allow(non_snake_case)]

use cursor::{Cursor, FuncCursor};
use divconst_magic_numbers::{magicS32, magicS64, magicU32, magicU64};
use divconst_magic_numbers::{MS32, MS64, MU32, MU64};
use ir::dfg::ValueDef;
use ir::instructions::Opcode;
use ir::types::{I32, I64};
use ir::Inst;
use ir::{DataFlowGraph, Function, InstBuilder, InstructionData, Type, Value};
use timing;

//----------------------------------------------------------------------
//
// Pattern-match helpers and transformation for div and rem by constants.

// Simple math helpers

/// if `x` is a power of two, or the negation thereof, return the power along
/// with a boolean that indicates whether `x` is negative. Else return None.
#[inline]
fn isPowerOf2_S32(x: i32) -> Option<(bool, u32)> {
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

/// Same comments as for isPowerOf2_S64 apply.
#[inline]
fn isPowerOf2_S64(x: i64) -> Option<(bool, u32)> {
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

#[derive(Debug)]
enum DivRemByConstInfo {
    DivU32(Value, u32), // In all cases, the arguments are:
    DivU64(Value, u64), // left operand, right operand
    DivS32(Value, i32),
    DivS64(Value, i64),
    RemU32(Value, u32),
    RemU64(Value, u64),
    RemS32(Value, i32),
    RemS64(Value, i64),
}

/// Possibly create a DivRemByConstInfo from the given components, by
/// figuring out which, if any, of the 8 cases apply, and also taking care to
/// sanity-check the immediate.
fn package_up_divrem_info(
    argL: Value,
    argL_ty: Type,
    argRs: i64,
    isSigned: bool,
    isRem: bool,
) -> Option<DivRemByConstInfo> {
    let argRu: u64 = argRs as u64;
    if !isSigned && argL_ty == I32 && argRu < 0x1_0000_0000 {
        let con = if isRem {
            DivRemByConstInfo::RemU32
        } else {
            DivRemByConstInfo::DivU32
        };
        return Some(con(argL, argRu as u32));
    }
    if !isSigned && argL_ty == I64 {
        // unsigned 64, no range constraint
        let con = if isRem {
            DivRemByConstInfo::RemU64
        } else {
            DivRemByConstInfo::DivU64
        };
        return Some(con(argL, argRu));
    }
    if isSigned && argL_ty == I32 && (argRu <= 0x7fff_ffff || argRu >= 0xffff_ffff_8000_0000) {
        let con = if isRem {
            DivRemByConstInfo::RemS32
        } else {
            DivRemByConstInfo::DivS32
        };
        return Some(con(argL, argRu as i32));
    }
    if isSigned && argL_ty == I64 {
        // signed 64, no range constraint
        let con = if isRem {
            DivRemByConstInfo::RemS64
        } else {
            DivRemByConstInfo::DivS64
        };
        return Some(con(argL, argRu as i64));
    }
    None
}

/// Examine `idata` to see if it is a div or rem by a constant, and if so
/// return the operands, signedness, operation size and div-vs-rem-ness in a
/// handy bundle.
fn get_div_info(inst: Inst, dfg: &DataFlowGraph) -> Option<DivRemByConstInfo> {
    let idata: &InstructionData = &dfg[inst];

    if let InstructionData::BinaryImm { opcode, arg, imm } = *idata {
        let (isSigned, isRem) = match opcode {
            Opcode::UdivImm => (false, false),
            Opcode::UremImm => (false, true),
            Opcode::SdivImm => (true, false),
            Opcode::SremImm => (true, true),
            _other => return None,
        };
        // Pull the operation size (type) from the left arg
        let argL_ty = dfg.value_type(arg);
        return package_up_divrem_info(arg, argL_ty, imm.into(), isSigned, isRem);
    }

    None
}

/// Actually do the transformation given a bundle containing the relevant
/// information. `divrem_info` describes a div or rem by a constant, that
/// `pos` currently points at, and `inst` is the associated instruction.
/// `inst` is replaced by a sequence of other operations that calculate the
/// same result. Note that there are various `divrem_info` cases where we
/// cannot do any transformation, in which case `inst` is left unchanged.
fn do_divrem_transformation(divrem_info: &DivRemByConstInfo, pos: &mut FuncCursor, inst: Inst) {
    let isRem = match *divrem_info {
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
        DivRemByConstInfo::DivU32(n1, 1) | DivRemByConstInfo::RemU32(n1, 1) => {
            if isRem {
                pos.func.dfg.replace(inst).iconst(I32, 0);
            } else {
                pos.func.dfg.replace(inst).copy(n1);
            }
        }

        // U32 div, rem by a power-of-2
        DivRemByConstInfo::DivU32(n1, d) | DivRemByConstInfo::RemU32(n1, d)
            if d.is_power_of_two() =>
        {
            debug_assert!(d >= 2);
            // compute k where d == 2^k
            let k = d.trailing_zeros();
            debug_assert!(k >= 1 && k <= 31);
            if isRem {
                let mask = (1u64 << k) - 1;
                pos.func.dfg.replace(inst).band_imm(n1, mask as i64);
            } else {
                pos.func.dfg.replace(inst).ushr_imm(n1, k as i64);
            }
        }

        // U32 div, rem by non-power-of-2
        DivRemByConstInfo::DivU32(n1, d) | DivRemByConstInfo::RemU32(n1, d) => {
            debug_assert!(d >= 3);
            let MU32 {
                mulBy,
                doAdd,
                shiftBy,
            } = magicU32(d);
            let qf; // final quotient
            let q0 = pos.ins().iconst(I32, mulBy as i64);
            let q1 = pos.ins().umulhi(n1, q0);
            if doAdd {
                debug_assert!(shiftBy >= 1 && shiftBy <= 32);
                let t1 = pos.ins().isub(n1, q1);
                let t2 = pos.ins().ushr_imm(t1, 1);
                let t3 = pos.ins().iadd(t2, q1);
                // I never found any case where shiftBy == 1 here.
                // So there's no attempt to fold out a zero shift.
                debug_assert_ne!(shiftBy, 1);
                qf = pos.ins().ushr_imm(t3, (shiftBy - 1) as i64);
            } else {
                debug_assert!(shiftBy >= 0 && shiftBy <= 31);
                // Whereas there are known cases here for shiftBy == 0.
                if shiftBy > 0 {
                    qf = pos.ins().ushr_imm(q1, shiftBy as i64);
                } else {
                    qf = q1;
                }
            }
            // Now qf holds the final quotient. If necessary calculate the
            // remainder instead.
            if isRem {
                let tt = pos.ins().imul_imm(qf, d as i64);
                pos.func.dfg.replace(inst).isub(n1, tt);
            } else {
                pos.func.dfg.replace(inst).copy(qf);
            }
        }

        // -------------------- U64 --------------------

        // U64 div, rem by zero: ignore
        DivRemByConstInfo::DivU64(_n1, 0) | DivRemByConstInfo::RemU64(_n1, 0) => {}

        // U64 div by 1: identity
        // U64 rem by 1: zero
        DivRemByConstInfo::DivU64(n1, 1) | DivRemByConstInfo::RemU64(n1, 1) => {
            if isRem {
                pos.func.dfg.replace(inst).iconst(I64, 0);
            } else {
                pos.func.dfg.replace(inst).copy(n1);
            }
        }

        // U64 div, rem by a power-of-2
        DivRemByConstInfo::DivU64(n1, d) | DivRemByConstInfo::RemU64(n1, d)
            if d.is_power_of_two() =>
        {
            debug_assert!(d >= 2);
            // compute k where d == 2^k
            let k = d.trailing_zeros();
            debug_assert!(k >= 1 && k <= 63);
            if isRem {
                let mask = (1u64 << k) - 1;
                pos.func.dfg.replace(inst).band_imm(n1, mask as i64);
            } else {
                pos.func.dfg.replace(inst).ushr_imm(n1, k as i64);
            }
        }

        // U64 div, rem by non-power-of-2
        DivRemByConstInfo::DivU64(n1, d) | DivRemByConstInfo::RemU64(n1, d) => {
            debug_assert!(d >= 3);
            let MU64 {
                mulBy,
                doAdd,
                shiftBy,
            } = magicU64(d);
            let qf; // final quotient
            let q0 = pos.ins().iconst(I64, mulBy as i64);
            let q1 = pos.ins().umulhi(n1, q0);
            if doAdd {
                debug_assert!(shiftBy >= 1 && shiftBy <= 64);
                let t1 = pos.ins().isub(n1, q1);
                let t2 = pos.ins().ushr_imm(t1, 1);
                let t3 = pos.ins().iadd(t2, q1);
                // I never found any case where shiftBy == 1 here.
                // So there's no attempt to fold out a zero shift.
                debug_assert_ne!(shiftBy, 1);
                qf = pos.ins().ushr_imm(t3, (shiftBy - 1) as i64);
            } else {
                debug_assert!(shiftBy >= 0 && shiftBy <= 63);
                // Whereas there are known cases here for shiftBy == 0.
                if shiftBy > 0 {
                    qf = pos.ins().ushr_imm(q1, shiftBy as i64);
                } else {
                    qf = q1;
                }
            }
            // Now qf holds the final quotient. If necessary calculate the
            // remainder instead.
            if isRem {
                let tt = pos.ins().imul_imm(qf, d as i64);
                pos.func.dfg.replace(inst).isub(n1, tt);
            } else {
                pos.func.dfg.replace(inst).copy(qf);
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
        DivRemByConstInfo::DivS32(n1, 1) | DivRemByConstInfo::RemS32(n1, 1) => {
            if isRem {
                pos.func.dfg.replace(inst).iconst(I32, 0);
            } else {
                pos.func.dfg.replace(inst).copy(n1);
            }
        }

        DivRemByConstInfo::DivS32(n1, d) | DivRemByConstInfo::RemS32(n1, d) => {
            if let Some((isNeg, k)) = isPowerOf2_S32(d) {
                // k can be 31 only in the case that d is -2^31.
                debug_assert!(k >= 1 && k <= 31);
                let t1 = if k - 1 == 0 {
                    n1
                } else {
                    pos.ins().sshr_imm(n1, (k - 1) as i64)
                };
                let t2 = pos.ins().ushr_imm(t1, (32 - k) as i64);
                let t3 = pos.ins().iadd(n1, t2);
                if isRem {
                    // S32 rem by a power-of-2
                    let t4 = pos.ins().band_imm(t3, i32::wrapping_neg(1 << k) as i64);
                    // Curiously, we don't care here what the sign of d is.
                    pos.func.dfg.replace(inst).isub(n1, t4);
                } else {
                    // S32 div by a power-of-2
                    let t4 = pos.ins().sshr_imm(t3, k as i64);
                    if isNeg {
                        pos.func.dfg.replace(inst).irsub_imm(t4, 0);
                    } else {
                        pos.func.dfg.replace(inst).copy(t4);
                    }
                }
            } else {
                // S32 div, rem by a non-power-of-2
                debug_assert!(d < -2 || d > 2);
                let MS32 { mulBy, shiftBy } = magicS32(d);
                let q0 = pos.ins().iconst(I32, mulBy as i64);
                let q1 = pos.ins().smulhi(n1, q0);
                let q2 = if d > 0 && mulBy < 0 {
                    pos.ins().iadd(q1, n1)
                } else if d < 0 && mulBy > 0 {
                    pos.ins().isub(q1, n1)
                } else {
                    q1
                };
                debug_assert!(shiftBy >= 0 && shiftBy <= 31);
                let q3 = if shiftBy == 0 {
                    q2
                } else {
                    pos.ins().sshr_imm(q2, shiftBy as i64)
                };
                let t1 = pos.ins().ushr_imm(q3, 31);
                let qf = pos.ins().iadd(q3, t1);
                // Now qf holds the final quotient. If necessary calculate
                // the remainder instead.
                if isRem {
                    let tt = pos.ins().imul_imm(qf, d as i64);
                    pos.func.dfg.replace(inst).isub(n1, tt);
                } else {
                    pos.func.dfg.replace(inst).copy(qf);
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
        DivRemByConstInfo::DivS64(n1, 1) | DivRemByConstInfo::RemS64(n1, 1) => {
            if isRem {
                pos.func.dfg.replace(inst).iconst(I64, 0);
            } else {
                pos.func.dfg.replace(inst).copy(n1);
            }
        }

        DivRemByConstInfo::DivS64(n1, d) | DivRemByConstInfo::RemS64(n1, d) => {
            if let Some((isNeg, k)) = isPowerOf2_S64(d) {
                // k can be 63 only in the case that d is -2^63.
                debug_assert!(k >= 1 && k <= 63);
                let t1 = if k - 1 == 0 {
                    n1
                } else {
                    pos.ins().sshr_imm(n1, (k - 1) as i64)
                };
                let t2 = pos.ins().ushr_imm(t1, (64 - k) as i64);
                let t3 = pos.ins().iadd(n1, t2);
                if isRem {
                    // S64 rem by a power-of-2
                    let t4 = pos.ins().band_imm(t3, i64::wrapping_neg(1 << k));
                    // Curiously, we don't care here what the sign of d is.
                    pos.func.dfg.replace(inst).isub(n1, t4);
                } else {
                    // S64 div by a power-of-2
                    let t4 = pos.ins().sshr_imm(t3, k as i64);
                    if isNeg {
                        pos.func.dfg.replace(inst).irsub_imm(t4, 0);
                    } else {
                        pos.func.dfg.replace(inst).copy(t4);
                    }
                }
            } else {
                // S64 div, rem by a non-power-of-2
                debug_assert!(d < -2 || d > 2);
                let MS64 { mulBy, shiftBy } = magicS64(d);
                let q0 = pos.ins().iconst(I64, mulBy);
                let q1 = pos.ins().smulhi(n1, q0);
                let q2 = if d > 0 && mulBy < 0 {
                    pos.ins().iadd(q1, n1)
                } else if d < 0 && mulBy > 0 {
                    pos.ins().isub(q1, n1)
                } else {
                    q1
                };
                debug_assert!(shiftBy >= 0 && shiftBy <= 63);
                let q3 = if shiftBy == 0 {
                    q2
                } else {
                    pos.ins().sshr_imm(q2, shiftBy as i64)
                };
                let t1 = pos.ins().ushr_imm(q3, 63);
                let qf = pos.ins().iadd(q3, t1);
                // Now qf holds the final quotient. If necessary calculate
                // the remainder instead.
                if isRem {
                    let tt = pos.ins().imul_imm(qf, d);
                    pos.func.dfg.replace(inst).isub(n1, tt);
                } else {
                    pos.func.dfg.replace(inst).copy(qf);
                }
            }
        }
    }
}

/// Apply basic simplifications.
///
/// This folds constants with arithmetic to form `_imm` instructions, and other
/// minor simplifications.
fn simplify(pos: &mut FuncCursor, inst: Inst) {
    match pos.func.dfg[inst] {
        InstructionData::Binary { opcode, args } => {
            if let ValueDef::Result(iconst_inst, _) = pos.func.dfg.value_def(args[1]) {
                if let InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    mut imm,
                } = pos.func.dfg[iconst_inst]
                {
                    let new_opcode = match opcode {
                        Opcode::Iadd => Opcode::IaddImm,
                        Opcode::Imul => Opcode::ImulImm,
                        Opcode::Sdiv => Opcode::SdivImm,
                        Opcode::Udiv => Opcode::UdivImm,
                        Opcode::Srem => Opcode::SremImm,
                        Opcode::Urem => Opcode::UremImm,
                        Opcode::Band => Opcode::BandImm,
                        Opcode::Bor => Opcode::BorImm,
                        Opcode::Bxor => Opcode::BxorImm,
                        Opcode::Rotl => Opcode::RotlImm,
                        Opcode::Rotr => Opcode::RotrImm,
                        Opcode::Ishl => Opcode::IshlImm,
                        Opcode::Ushr => Opcode::UshrImm,
                        Opcode::Sshr => Opcode::SshrImm,
                        Opcode::Isub => {
                            imm = imm.wrapping_neg();
                            Opcode::IaddImm
                        }
                        _ => return,
                    };
                    let ty = pos.func.dfg.ctrl_typevar(inst);
                    pos.func
                        .dfg
                        .replace(inst)
                        .BinaryImm(new_opcode, ty, imm, args[0]);
                }
            } else if let ValueDef::Result(iconst_inst, _) = pos.func.dfg.value_def(args[0]) {
                if let InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                } = pos.func.dfg[iconst_inst]
                {
                    let new_opcode = match opcode {
                        Opcode::Isub => Opcode::IrsubImm,
                        _ => return,
                    };
                    let ty = pos.func.dfg.ctrl_typevar(inst);
                    pos.func
                        .dfg
                        .replace(inst)
                        .BinaryImm(new_opcode, ty, imm, args[1]);
                }
            }
        }
        InstructionData::IntCompare { opcode, cond, args } => {
            debug_assert_eq!(opcode, Opcode::Icmp);
            if let ValueDef::Result(iconst_inst, _) = pos.func.dfg.value_def(args[1]) {
                if let InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                } = pos.func.dfg[iconst_inst]
                {
                    pos.func.dfg.replace(inst).icmp_imm(cond, args[0], imm);
                }
            }
        }
        InstructionData::CondTrap { .. }
        | InstructionData::Branch { .. }
        | InstructionData::Ternary {
            opcode: Opcode::Select,
            ..
        } => {
            // Fold away a redundant `bint`.
            let condition_def = {
                let args = pos.func.dfg.inst_args(inst);
                pos.func.dfg.value_def(args[0])
            };
            if let ValueDef::Result(def_inst, _) = condition_def {
                if let InstructionData::Unary {
                    opcode: Opcode::Bint,
                    arg: bool_val,
                } = pos.func.dfg[def_inst]
                {
                    let args = pos.func.dfg.inst_args_mut(inst);
                    args[0] = bool_val;
                }
            }
        }
        _ => {}
    }
}

/// The main pre-opt pass.
pub fn do_preopt(func: &mut Function) {
    let _tt = timing::preopt();
    let mut pos = FuncCursor::new(func);
    while let Some(_ebb) = pos.next_ebb() {
        while let Some(inst) = pos.next_inst() {
            // Apply basic simplifications.
            simplify(&mut pos, inst);

            //-- BEGIN -- division by constants ----------------

            let mb_dri = get_div_info(inst, &pos.func.dfg);
            if let Some(divrem_info) = mb_dri {
                do_divrem_transformation(&divrem_info, &mut pos, inst);
                continue;
            }

            //-- END -- division by constants ------------------
        }
    }
}
