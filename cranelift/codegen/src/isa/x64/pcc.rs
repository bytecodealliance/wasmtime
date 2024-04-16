//! Proof-carrying-code validation for x64 VCode.

use crate::ir::pcc::*;
use crate::ir::types::*;
use crate::isa::x64::args::AvxOpcode;
use crate::isa::x64::inst::args::{
    AluRmiROpcode, Amode, Gpr, Imm8Reg, RegMem, RegMemImm, ShiftKind, SseOpcode, SyntheticAmode,
    ToWritableReg, CC,
};
use crate::isa::x64::inst::Inst;
use crate::machinst::pcc::*;
use crate::machinst::{InsnIndex, VCode, VCodeConstantData};
use crate::machinst::{Reg, Writable};
use crate::trace;

fn undefined_result(
    ctx: &FactContext,
    vcode: &mut VCode<Inst>,
    dst: Writable<Gpr>,
    reg_bits: u16,
    result_bits: u16,
) -> PccResult<()> {
    check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
        clamp_range(ctx, reg_bits, result_bits, None)
    })
}

fn ensure_no_fact(vcode: &VCode<Inst>, reg: Reg) -> PccResult<()> {
    if vcode.vreg_fact(reg.into()).is_some() {
        Err(PccError::UnsupportedFact)
    } else {
        Ok(())
    }
}

/// Flow-state between facts.
#[derive(Clone, Debug, Default)]
pub(crate) struct FactFlowState {
    cmp_flags: Option<(Fact, Fact)>,
}

pub(crate) fn check(
    ctx: &FactContext,
    vcode: &mut VCode<Inst>,
    inst_idx: InsnIndex,
    state: &mut FactFlowState,
) -> PccResult<()> {
    trace!("Checking facts on inst: {:?}", vcode[inst_idx]);

    // We only persist flag state for one instruction, because we
    // can't exhaustively enumerate all flags-effecting ops; so take
    // the `cmp_state` here and perhaps use it below but don't let it
    // remain.
    let cmp_flags = state.cmp_flags.take();

    match vcode[inst_idx] {
        Inst::Nop { .. } => Ok(()),

        Inst::Args { .. } => {
            // Defs on the args have "axiomatic facts": we trust the
            // ABI code to pass through the values unharmed, so the
            // facts given to us in the CLIF should still be true.
            Ok(())
        }

        Inst::AluRmiR {
            size,
            op: AluRmiROpcode::Add,
            src1,
            ref src2,
            dst,
        } => match *<&RegMemImm>::from(src2) {
            RegMemImm::Reg { reg: src2 } => {
                let bits = size.to_bits().into();
                check_binop(
                    ctx,
                    vcode,
                    64,
                    dst.to_writable_reg(),
                    src1.to_reg(),
                    src2,
                    |src1, src2| clamp_range(ctx, 64, bits, ctx.add(src1, src2, bits)),
                )
            }
            RegMemImm::Imm { simm32 } => {
                let bits = size.to_bits().into();
                check_unop(
                    ctx,
                    vcode,
                    64,
                    dst.to_writable_reg(),
                    src1.to_reg(),
                    |src1| {
                        let simm32: i64 = simm32.into();
                        clamp_range(ctx, 64, bits, ctx.offset(src1, bits, simm32))
                    },
                )
            }
            RegMemImm::Mem { ref addr } => {
                let bits: u16 = size.to_bits().into();
                let loaded = check_load(ctx, None, addr, vcode, size.to_type(), bits)?;
                check_unop(ctx, vcode, 64, dst.to_writable_reg(), src1.into(), |src1| {
                    let sum = loaded.and_then(|loaded| ctx.add(src1, &loaded, bits));
                    clamp_range(ctx, 64, bits, sum)
                })
            }
        },

        Inst::AluRmiR {
            size,
            op: AluRmiROpcode::Sub,
            src1,
            ref src2,
            dst,
        } => match *<&RegMemImm>::from(src2) {
            RegMemImm::Imm { simm32 } => {
                let bits = size.to_bits().into();
                check_unop(
                    ctx,
                    vcode,
                    64,
                    dst.to_writable_reg(),
                    src1.to_reg(),
                    |src1| {
                        let simm32: i64 = simm32.into();
                        clamp_range(ctx, 64, bits, ctx.offset(src1, bits, -simm32))
                    },
                )
            }
            RegMemImm::Reg { .. } => {
                let bits: u16 = size.to_bits().into();
                check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                    clamp_range(ctx, 64, bits, None)
                })
            }
            RegMemImm::Mem { ref addr } => {
                let loaded = check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
                check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                    clamp_range(ctx, 64, size.to_bits().into(), loaded)
                })
            }
        },

        Inst::AluRmiR {
            size,
            ref src2,
            dst,
            ..
        } => match <&RegMemImm>::from(src2) {
            RegMemImm::Mem { ref addr } => {
                let loaded = check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
                check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                    clamp_range(ctx, 64, size.to_bits().into(), loaded)
                })
            }
            RegMemImm::Reg { .. } | RegMemImm::Imm { .. } => {
                undefined_result(ctx, vcode, dst, 64, size.to_bits().into())
            }
        },

        Inst::AluRM {
            size,
            op: _,
            ref src1_dst,
            src2: _,
        } => {
            check_load(ctx, None, src1_dst, vcode, size.to_type(), 64)?;
            check_store(ctx, None, src1_dst, vcode, size.to_type())
        }

        Inst::AluRmRVex {
            size,
            ref src2,
            dst,
            ..
        } => match <&RegMem>::from(src2) {
            RegMem::Mem { ref addr } => {
                let loaded = check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
                check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                    clamp_range(ctx, 64, size.to_bits().into(), loaded)
                })
            }
            RegMem::Reg { .. } => undefined_result(ctx, vcode, dst, 64, size.to_bits().into()),
        },

        Inst::AluConstOp {
            op: AluRmiROpcode::Xor,
            dst,
            ..
        } => check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
            Ok(Some(Fact::constant(64, 0)))
        }),

        Inst::AluConstOp { dst, .. } => undefined_result(ctx, vcode, dst, 64, 64),

        Inst::UnaryRmR {
            size, ref src, dst, ..
        }
        | Inst::UnaryRmRVex {
            size, ref src, dst, ..
        }
        | Inst::UnaryRmRImmVex {
            size, ref src, dst, ..
        } => match <&RegMem>::from(src) {
            RegMem::Mem { ref addr } => {
                check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
                check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                    clamp_range(ctx, 64, size.to_bits().into(), None)
                })
            }
            RegMem::Reg { .. } => undefined_result(ctx, vcode, dst, 64, size.to_bits().into()),
        },

        Inst::Not { size, dst, .. } | Inst::Neg { size, dst, .. } => {
            undefined_result(ctx, vcode, dst, 64, size.to_bits().into())
        }

        Inst::Div {
            size,
            ref divisor,
            dst_quotient,
            dst_remainder,
            ..
        } => {
            match <&RegMem>::from(divisor) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            undefined_result(ctx, vcode, dst_quotient, 64, 64)?;
            undefined_result(ctx, vcode, dst_remainder, 64, 64)?;
            Ok(())
        }
        Inst::Div8 {
            dst, ref divisor, ..
        } => {
            match <&RegMem>::from(divisor) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I8, 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            // 64-bit result width because result may be negative
            // hence high bits set.
            undefined_result(ctx, vcode, dst, 64, 64)?;
            Ok(())
        }
        Inst::Mul {
            size,
            dst_lo,
            dst_hi,
            ref src2,
            ..
        } => {
            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            undefined_result(ctx, vcode, dst_lo, 64, size.to_bits().into())?;
            undefined_result(ctx, vcode, dst_hi, 64, size.to_bits().into())?;
            Ok(())
        }
        Inst::Mul8 { dst, ref src2, .. } => {
            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I8, 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            undefined_result(ctx, vcode, dst, 64, 16)?;
            Ok(())
        }
        Inst::IMul {
            size,
            dst,
            ref src2,
            ..
        } => {
            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            undefined_result(ctx, vcode, dst, 64, size.to_bits().into())?;
            Ok(())
        }
        Inst::IMulImm {
            size,
            dst,
            ref src1,
            ..
        } => {
            match <&RegMem>::from(src1) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            undefined_result(ctx, vcode, dst, 64, size.to_bits().into())?;
            Ok(())
        }
        Inst::CheckedSRemSeq {
            dst_quotient,
            dst_remainder,
            ..
        } => {
            undefined_result(ctx, vcode, dst_quotient, 64, 64)?;
            undefined_result(ctx, vcode, dst_remainder, 64, 64)?;
            Ok(())
        }

        Inst::CheckedSRemSeq8 { dst, .. } => undefined_result(ctx, vcode, dst, 64, 64),

        Inst::SignExtendData { dst, .. } => undefined_result(ctx, vcode, dst, 64, 64),

        Inst::Imm { simm64, dst, .. } => {
            check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                Ok(Some(Fact::constant(64, simm64)))
            })
        }

        Inst::MovRR { size, dst, .. } => {
            undefined_result(ctx, vcode, dst, 64, size.to_bits().into())
        }

        Inst::MovFromPReg { dst, .. } => undefined_result(ctx, vcode, dst, 64, 64),
        Inst::MovToPReg { .. } => Ok(()),

        Inst::MovzxRmR {
            ref ext_mode,
            ref src,
            dst,
        } => {
            let from_bytes: u16 = ext_mode.src_size().into();
            let to_bytes: u16 = ext_mode.dst_size().into();
            match <&RegMem>::from(src) {
                RegMem::Reg { reg } => {
                    check_unop(ctx, vcode, 64, dst.to_writable_reg(), *reg, |src| {
                        clamp_range(ctx, 64, from_bytes * 8, Some(src.clone()))
                    })
                }
                RegMem::Mem { ref addr } => {
                    let loaded = check_load(
                        ctx,
                        Some(dst.to_writable_reg()),
                        addr,
                        vcode,
                        ext_mode.src_type(),
                        64,
                    )?;
                    check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                        let extended = loaded
                            .and_then(|loaded| ctx.uextend(&loaded, from_bytes * 8, to_bytes * 8));
                        clamp_range(ctx, 64, from_bytes * 8, extended)
                    })
                }
            }
        }

        Inst::Mov64MR { ref src, dst } => {
            check_load(ctx, Some(dst.to_writable_reg()), src, vcode, I64, 64)?;
            Ok(())
        }

        Inst::LoadEffectiveAddress {
            ref addr,
            dst,
            size,
        } => {
            let addr = addr.clone();
            let bits: u16 = size.to_bits().into();
            check_output(ctx, vcode, dst.to_writable_reg(), &[], |vcode| {
                let fact = if let SyntheticAmode::Real(amode) = &addr {
                    compute_addr(ctx, vcode, amode, bits)
                } else {
                    None
                };
                clamp_range(ctx, 64, bits, fact)
            })
        }

        Inst::MovsxRmR {
            ref ext_mode,
            ref src,
            dst,
        } => {
            match <&RegMem>::from(src) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, ext_mode.src_type(), 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            undefined_result(ctx, vcode, dst, 64, 64)
        }

        Inst::MovImmM { size, ref dst, .. } => check_store(ctx, None, dst, vcode, size.to_type()),

        Inst::MovRM { size, src, ref dst } => {
            check_store(ctx, Some(src.to_reg()), dst, vcode, size.to_type())
        }

        Inst::ShiftR {
            size,
            kind: ShiftKind::ShiftLeft,
            src,
            ref num_bits,
            dst,
        } => match num_bits.clone().to_imm8_reg() {
            Imm8Reg::Imm8 { imm } => {
                check_unop(ctx, vcode, 64, dst.to_writable_reg(), src.to_reg(), |src| {
                    clamp_range(
                        ctx,
                        64,
                        size.to_bits().into(),
                        ctx.shl(src, size.to_bits().into(), imm.into()),
                    )
                })
            }
            Imm8Reg::Reg { .. } => undefined_result(ctx, vcode, dst, 64, size.to_bits().into()),
        },

        Inst::ShiftR { size, dst, .. } => {
            undefined_result(ctx, vcode, dst, 64, size.to_bits().into())
        }

        Inst::XmmRmiReg { dst, ref src2, .. } => {
            match <&RegMemImm>::from(src2) {
                RegMemImm::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I8X16, 128)?;
                }
                _ => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        Inst::CmpRmiR {
            size, dst, ref src, ..
        } => match <&RegMemImm>::from(src) {
            RegMemImm::Mem {
                addr: SyntheticAmode::ConstantOffset(k),
            } => {
                match vcode.constants.get(*k) {
                    VCodeConstantData::U64(bytes) => {
                        let value = u64::from_le_bytes(*bytes);
                        let lhs = get_fact_or_default(vcode, dst.to_reg(), 64);
                        let rhs = Fact::constant(64, value);
                        state.cmp_flags = Some((lhs, rhs));
                    }
                    _ => {}
                }
                Ok(())
            }
            RegMemImm::Mem { ref addr } => {
                if let Some(rhs) = check_load(ctx, None, addr, vcode, size.to_type(), 64)? {
                    let lhs = get_fact_or_default(vcode, dst.to_reg(), 64);
                    state.cmp_flags = Some((lhs, rhs));
                }
                Ok(())
            }
            RegMemImm::Reg { reg } => {
                let rhs = get_fact_or_default(vcode, *reg, 64);
                let lhs = get_fact_or_default(vcode, dst.to_reg(), 64);
                state.cmp_flags = Some((lhs, rhs));
                Ok(())
            }
            RegMemImm::Imm { simm32 } => {
                let lhs = get_fact_or_default(vcode, dst.to_reg(), 64);
                let rhs = Fact::constant(64, (*simm32 as i32) as i64 as u64);
                state.cmp_flags = Some((lhs, rhs));
                Ok(())
            }
        },

        Inst::Setcc { dst, .. } => undefined_result(ctx, vcode, dst, 64, 64),

        Inst::Bswap { dst, .. } => undefined_result(ctx, vcode, dst, 64, 64),

        Inst::Cmove {
            size,
            dst,
            ref consequent,
            alternative,
            cc,
            ..
        } => match <&RegMem>::from(consequent) {
            RegMem::Mem { ref addr } => {
                check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
                Ok(())
            }
            RegMem::Reg { reg } if (cc == CC::NB || cc == CC::NBE) && cmp_flags.is_some() => {
                let (cmp_lhs, cmp_rhs) = cmp_flags.unwrap();
                trace!("lhs = {:?} rhs = {:?}", cmp_lhs, cmp_rhs);
                let reg = *reg;
                check_output(ctx, vcode, dst.to_writable_reg(), &[], |vcode| {
                    // See comments in aarch64::pcc CSel for more details on this.
                    let in_true = get_fact_or_default(vcode, reg, 64);
                    let in_true_kind = match cc {
                        CC::NB => InequalityKind::Loose,
                        CC::NBE => InequalityKind::Strict,
                        _ => unreachable!(),
                    };
                    let in_true = ctx.apply_inequality(&in_true, &cmp_lhs, &cmp_rhs, in_true_kind);
                    let in_false = get_fact_or_default(vcode, alternative.to_reg(), 64);
                    let in_false_kind = match cc {
                        CC::NB => InequalityKind::Strict,
                        CC::NBE => InequalityKind::Loose,
                        _ => unreachable!(),
                    };
                    let in_false =
                        ctx.apply_inequality(&in_false, &cmp_rhs, &cmp_lhs, in_false_kind);
                    let union = ctx.union(&in_true, &in_false);
                    clamp_range(ctx, 64, 64, union)
                })
            }
            _ => undefined_result(ctx, vcode, dst, 64, 64),
        },

        Inst::XmmCmove { dst, .. } => ensure_no_fact(vcode, dst.to_writable_reg().to_reg()),

        Inst::Push64 { ref src } => match <&RegMemImm>::from(src) {
            RegMemImm::Mem { ref addr } => {
                check_load(ctx, None, addr, vcode, I64, 64)?;
                Ok(())
            }
            RegMemImm::Reg { .. } | RegMemImm::Imm { .. } => Ok(()),
        },

        Inst::Pop64 { dst } => undefined_result(ctx, vcode, dst, 64, 64),

        Inst::StackProbeLoop { tmp, .. } => ensure_no_fact(vcode, tmp.to_reg()),

        Inst::XmmRmR { dst, ref src2, .. }
        | Inst::XmmRmRBlend { dst, ref src2, .. }
        | Inst::XmmUnaryRmR {
            dst, src: ref src2, ..
        }
        | Inst::XmmUnaryRmRImm {
            dst, src: ref src2, ..
        } => {
            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I8X16, 128)?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        Inst::XmmUnaryRmRUnaligned {
            dst,
            ref src,
            op: SseOpcode::Movss,
            ..
        } => {
            match <&RegMem>::from(src) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, F32, 32)?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }
        Inst::XmmUnaryRmRUnaligned {
            dst,
            ref src,
            op: SseOpcode::Movsd,
            ..
        } => {
            match <&RegMem>::from(src) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, F64, 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        // NOTE: it's assumed that all of these cases perform 128-bit loads, but this hasn't been
        // verified. The effect of this will be spurious PCC failures when these instructions are
        // involved.
        Inst::XmmRmRUnaligned { dst, ref src2, .. }
        | Inst::XmmRmREvex { dst, ref src2, .. }
        | Inst::XmmUnaryRmRImmEvex {
            dst, src: ref src2, ..
        }
        | Inst::XmmUnaryRmRUnaligned {
            dst, src: ref src2, ..
        }
        | Inst::XmmUnaryRmREvex {
            dst, src: ref src2, ..
        }
        | Inst::XmmRmREvex3 {
            dst,
            src3: ref src2,
            ..
        } => {
            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I8X16, 128)?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        Inst::XmmRmRImmVex {
            op, dst, ref src2, ..
        }
        | Inst::XmmRmRVex3 {
            op,
            dst,
            src3: ref src2,
            ..
        }
        | Inst::XmmRmRBlendVex {
            op, dst, ref src2, ..
        }
        | Inst::XmmUnaryRmRVex {
            op,
            dst,
            src: ref src2,
            ..
        }
        | Inst::XmmUnaryRmRImmVex {
            op,
            dst,
            src: ref src2,
            ..
        } => {
            let (ty, size) = match op {
                AvxOpcode::Vmovss => (F32, 32),
                AvxOpcode::Vmovsd => (F64, 64),
                AvxOpcode::Vpinsrb => (I8, 8),
                AvxOpcode::Vpinsrw => (I16, 16),
                AvxOpcode::Vpinsrd => (I32, 32),
                AvxOpcode::Vpinsrq => (I64, 64),

                // We assume all other operations happen on 128-bit values.
                _ => (I8X16, 128),
            };

            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, ty, size)?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        Inst::XmmRmiRVex { dst, ref src2, .. } => {
            match <&RegMemImm>::from(src2) {
                RegMemImm::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I8X16, 128)?;
                }
                RegMemImm::Reg { .. } | RegMemImm::Imm { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        Inst::XmmVexPinsr { dst, ref src2, .. } => {
            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I64, 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        Inst::XmmMovRMVex { ref dst, .. } | Inst::XmmMovRMImmVex { ref dst, .. } => {
            check_store(ctx, None, dst, vcode, I8X16)
        }

        Inst::XmmToGprImmVex { dst, .. } => ensure_no_fact(vcode, dst.to_writable_reg().to_reg()),

        Inst::GprToXmmVex { dst, ref src, .. } | Inst::GprToXmm { dst, ref src, .. } => {
            match <&RegMem>::from(src) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I64, 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        Inst::XmmToGprVex { dst, .. } => undefined_result(ctx, vcode, dst, 64, 64),

        Inst::XmmMovRM {
            ref dst,
            op: SseOpcode::Movss,
            ..
        } => {
            check_store(ctx, None, dst, vcode, F32)?;
            Ok(())
        }

        Inst::XmmMovRM {
            ref dst,
            op: SseOpcode::Movsd,
            ..
        } => {
            check_store(ctx, None, dst, vcode, F64)?;
            Ok(())
        }

        Inst::XmmMovRM { ref dst, .. } | Inst::XmmMovRMImm { ref dst, .. } => {
            check_store(ctx, None, dst, vcode, I8X16)?;
            Ok(())
        }

        Inst::XmmToGpr { dst, .. } | Inst::XmmToGprImm { dst, .. } => {
            undefined_result(ctx, vcode, dst, 64, 64)
        }

        Inst::CvtIntToFloat { dst, ref src2, .. }
        | Inst::CvtIntToFloatVex { dst, ref src2, .. } => {
            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I64, 64)?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        Inst::CvtUint64ToFloatSeq {
            dst,
            tmp_gpr1,
            tmp_gpr2,
            ..
        } => {
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())?;
            ensure_no_fact(vcode, tmp_gpr1.to_writable_reg().to_reg())?;
            ensure_no_fact(vcode, tmp_gpr2.to_writable_reg().to_reg())?;
            Ok(())
        }

        Inst::CvtFloatToSintSeq {
            dst,
            tmp_gpr,
            tmp_xmm,
            ..
        } => {
            undefined_result(ctx, vcode, dst, 64, 64)?;
            ensure_no_fact(vcode, tmp_gpr.to_writable_reg().to_reg())?;
            ensure_no_fact(vcode, tmp_xmm.to_writable_reg().to_reg())?;
            Ok(())
        }

        Inst::CvtFloatToUintSeq {
            dst,
            tmp_gpr,
            tmp_xmm,
            tmp_xmm2,
            ..
        } => {
            undefined_result(ctx, vcode, dst, 64, 64)?;
            ensure_no_fact(vcode, tmp_gpr.to_writable_reg().to_reg())?;
            ensure_no_fact(vcode, tmp_xmm.to_writable_reg().to_reg())?;
            ensure_no_fact(vcode, tmp_xmm2.to_writable_reg().to_reg())?;
            Ok(())
        }

        Inst::XmmMinMaxSeq { dst, .. } => ensure_no_fact(vcode, dst.to_writable_reg().to_reg()),

        Inst::XmmCmpRmR { ref src, .. } => match <&RegMem>::from(src) {
            RegMem::Mem { ref addr } => {
                check_load(ctx, None, addr, vcode, I8X16, 128)?;
                Ok(())
            }
            RegMem::Reg { .. } => Ok(()),
        },

        Inst::XmmRmRImm {
            dst,
            ref src2,
            size,
            op,
            ..
        } if op.has_scalar_src2() => {
            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(
                        ctx,
                        None,
                        addr,
                        vcode,
                        size.to_type(),
                        size.to_bits().into(),
                    )?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_reg())
        }

        Inst::XmmRmRImm { dst, ref src2, .. } => {
            match <&RegMem>::from(src2) {
                RegMem::Mem { ref addr } => {
                    check_load(ctx, None, addr, vcode, I8X16, 128)?;
                }
                RegMem::Reg { .. } => {}
            }
            ensure_no_fact(vcode, dst.to_reg())
        }

        Inst::CallKnown { .. }
        | Inst::ReturnCallKnown { .. }
        | Inst::JmpKnown { .. }
        | Inst::Ret { .. }
        | Inst::JmpIf { .. }
        | Inst::JmpCond { .. }
        | Inst::TrapIf { .. }
        | Inst::TrapIfAnd { .. }
        | Inst::TrapIfOr { .. }
        | Inst::Hlt {}
        | Inst::Ud2 { .. } => Ok(()),
        Inst::Rets { .. } => Ok(()),

        Inst::CallUnknown { ref dest, .. }
        | Inst::ReturnCallUnknown {
            callee: ref dest, ..
        }
        | Inst::JmpUnknown {
            target: ref dest, ..
        } => match <&RegMem>::from(dest) {
            RegMem::Mem { ref addr } => {
                check_load(ctx, None, addr, vcode, I64, 64)?;
                Ok(())
            }
            RegMem::Reg { .. } => Ok(()),
        },

        Inst::JmpTableSeq { tmp1, tmp2, .. } => {
            ensure_no_fact(vcode, tmp1.to_reg())?;
            ensure_no_fact(vcode, tmp2.to_reg())?;
            Ok(())
        }

        Inst::LoadExtName { dst, .. } => {
            ensure_no_fact(vcode, dst.to_reg())?;
            Ok(())
        }

        Inst::LockCmpxchg {
            ref mem, dst_old, ..
        } => {
            ensure_no_fact(vcode, dst_old.to_reg())?;
            check_store(ctx, None, mem, vcode, I64)?;
            Ok(())
        }

        Inst::AtomicRmwSeq {
            ref mem,
            temp,
            dst_old,
            ..
        } => {
            ensure_no_fact(vcode, dst_old.to_reg())?;
            ensure_no_fact(vcode, temp.to_reg())?;
            check_store(ctx, None, mem, vcode, I64)?;
            Ok(())
        }

        Inst::Fence { .. } | Inst::VirtualSPOffsetAdj { .. } => Ok(()),

        Inst::XmmUninitializedValue { dst } => {
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }

        Inst::ElfTlsGetAddr { dst, .. } | Inst::MachOTlsGetAddr { dst, .. } => {
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())
        }
        Inst::CoffTlsGetAddr { dst, tmp, .. } => {
            ensure_no_fact(vcode, dst.to_writable_reg().to_reg())?;
            ensure_no_fact(vcode, tmp.to_writable_reg().to_reg())?;
            Ok(())
        }

        Inst::Unwind { .. } | Inst::DummyUse { .. } => Ok(()),
    }
}

fn check_load(
    ctx: &FactContext,
    dst: Option<Writable<Reg>>,
    src: &SyntheticAmode,
    vcode: &VCode<Inst>,
    ty: Type,
    to_bits: u16,
) -> PccResult<Option<Fact>> {
    let result_fact = dst.and_then(|dst| vcode.vreg_fact(dst.to_reg().into()));
    let from_bits = u16::try_from(ty.bits()).unwrap();
    check_mem(
        ctx,
        src,
        vcode,
        ty,
        LoadOrStore::Load {
            result_fact,
            from_bits,
            to_bits,
        },
    )
}

fn check_store(
    ctx: &FactContext,
    data: Option<Reg>,
    dst: &SyntheticAmode,
    vcode: &VCode<Inst>,
    ty: Type,
) -> PccResult<()> {
    let stored_fact = data.and_then(|data| vcode.vreg_fact(data.into()));
    check_mem(ctx, dst, vcode, ty, LoadOrStore::Store { stored_fact }).map(|_| ())
}

fn check_mem<'a>(
    ctx: &FactContext,
    amode: &SyntheticAmode,
    vcode: &VCode<Inst>,
    ty: Type,
    op: LoadOrStore<'a>,
) -> PccResult<Option<Fact>> {
    let addr = match amode {
        SyntheticAmode::Real(amode) if amode.get_flags().checked() => {
            compute_addr(ctx, vcode, amode, 64).ok_or(PccError::MissingFact)?
        }
        _ => return Ok(None),
    };

    match op {
        LoadOrStore::Load {
            result_fact,
            from_bits,
            to_bits,
        } => {
            let loaded_fact = clamp_range(ctx, to_bits, from_bits, ctx.load(&addr, ty)?.cloned())?;
            trace!(
                "loaded_fact = {:?} result_fact = {:?}",
                loaded_fact,
                result_fact
            );
            if ctx.subsumes_fact_optionals(loaded_fact.as_ref(), result_fact) {
                Ok(loaded_fact.clone())
            } else {
                Err(PccError::UnsupportedFact)
            }
        }
        LoadOrStore::Store { stored_fact } => {
            ctx.store(&addr, ty, stored_fact)?;
            Ok(None)
        }
    }
}

fn compute_addr(ctx: &FactContext, vcode: &VCode<Inst>, amode: &Amode, bits: u16) -> Option<Fact> {
    trace!("compute_addr: {:?}", amode);
    match *amode {
        Amode::ImmReg { simm32, base, .. } => {
            let base = get_fact_or_default(vcode, base, bits);
            trace!("base = {:?}", base);
            let simm32: i64 = simm32.into();
            let simm32: u64 = simm32 as u64;
            let offset = Fact::constant(bits, simm32);
            let sum = ctx.add(&base, &offset, bits)?;
            trace!("sum = {:?}", sum);
            Some(sum)
        }
        Amode::ImmRegRegShift {
            simm32,
            base,
            index,
            shift,
            ..
        } => {
            let base = get_fact_or_default(vcode, base.into(), bits);
            let index = get_fact_or_default(vcode, index.into(), bits);
            trace!("base = {:?} index = {:?}", base, index);
            let shifted = ctx.shl(&index, bits, shift.into())?;
            let sum = ctx.add(&base, &shifted, bits)?;
            let simm32: i64 = simm32.into();
            let simm32: u64 = simm32 as u64;
            let offset = Fact::constant(bits, simm32);
            let sum = ctx.add(&sum, &offset, bits)?;
            trace!("sum = {:?}", sum);
            Some(sum)
        }
        Amode::RipRelative { .. } => None,
    }
}
