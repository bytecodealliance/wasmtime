//! Proof-carrying code checking for AArch64 VCode.

use crate::ir::pcc::*;
use crate::ir::types::*;
use crate::ir::MemFlags;
use crate::isa::aarch64::inst::args::{PairAMode, ShiftOp};
use crate::isa::aarch64::inst::ALUOp;
use crate::isa::aarch64::inst::Inst;
use crate::isa::aarch64::inst::{AMode, ExtendOp};
use crate::machinst::Reg;
use crate::machinst::VCode;
use crate::trace;

fn get_fact(vcode: &VCode<Inst>, reg: Reg) -> PccResult<&Fact> {
    vcode.vreg_fact(reg.into()).ok_or(PccError::MissingFact)
}

fn has_fact(vcode: &VCode<Inst>, reg: Reg) -> bool {
    vcode.vreg_fact(reg.into()).is_some()
}

fn fail_if_missing(fact: Option<Fact>) -> PccResult<Fact> {
    fact.ok_or(PccError::UnsupportedFact)
}

fn check_subsumes(ctx: &FactContext, subsumer: &Fact, subsumee: &Fact) -> PccResult<()> {
    trace!(
        "checking if derived fact {:?} subsumes stated fact {:?}",
        subsumer,
        subsumee
    );

    // For now, allow all `mem` facts to validate.
    if matches!(subsumee, Fact::Mem { .. }) {
        return Ok(());
    }

    if ctx.subsumes(subsumer, subsumee) {
        Ok(())
    } else {
        Err(PccError::UnsupportedFact)
    }
}

fn extend_fact(ctx: &FactContext, value: &Fact, mode: ExtendOp) -> Option<Fact> {
    match mode {
        ExtendOp::UXTB => ctx.uextend(value, 8, 64),
        ExtendOp::UXTH => ctx.uextend(value, 16, 64),
        ExtendOp::UXTW => ctx.uextend(value, 32, 64),
        ExtendOp::UXTX => Some(value.clone()),
        ExtendOp::SXTB => ctx.sextend(value, 8, 64),
        ExtendOp::SXTH => ctx.sextend(value, 16, 64),
        ExtendOp::SXTW => ctx.sextend(value, 32, 64),
        ExtendOp::SXTX => None,
    }
}

fn check_output<F: Fn() -> PccResult<Fact>>(
    ctx: &FactContext,
    vcode: &VCode<Inst>,
    out: Reg,
    f: F,
) -> PccResult<()> {
    if let Some(fact) = vcode.vreg_fact(out.into()) {
        let result = f()?;
        check_subsumes(ctx, &result, fact)
    } else {
        Ok(())
    }
}

pub(crate) fn check(ctx: &FactContext, vcode: &VCode<Inst>, inst: &Inst) -> PccResult<()> {
    trace!("Checking facts on inst: {:?}", inst);

    match inst {
        Inst::Args { .. } => {
            // Defs on the args have "axiomatic facts": we trust the
            // ABI code to pass through the values unharmed, so the
            // facts given to us in the CLIF should still be true.
            Ok(())
        }
        Inst::ULoad8 { rd, mem, flags } | Inst::SLoad8 { rd, mem, flags } => {
            check_load(&ctx, Some(rd.to_reg()), *flags, mem, vcode, I8)
        }
        Inst::ULoad16 { rd, mem, flags } | Inst::SLoad16 { rd, mem, flags } => {
            check_load(&ctx, Some(rd.to_reg()), *flags, mem, vcode, I16)
        }
        Inst::ULoad32 { rd, mem, flags } | Inst::SLoad32 { rd, mem, flags } => {
            check_load(&ctx, Some(rd.to_reg()), *flags, mem, vcode, I32)
        }
        Inst::ULoad64 { rd, mem, flags } => {
            check_load(&ctx, Some(rd.to_reg()), *flags, mem, vcode, I64)
        }
        Inst::FpuLoad32 { mem, flags, .. } => check_load(&ctx, None, *flags, mem, vcode, F32),
        Inst::FpuLoad64 { mem, flags, .. } => check_load(&ctx, None, *flags, mem, vcode, F64),
        Inst::FpuLoad128 { mem, flags, .. } => check_load(&ctx, None, *flags, mem, vcode, I8X16),
        Inst::LoadP64 { mem, flags, .. } => check_load_pair(&ctx, *flags, mem, vcode, 16),
        Inst::FpuLoadP64 { mem, flags, .. } => check_load_pair(&ctx, *flags, mem, vcode, 16),
        Inst::FpuLoadP128 { mem, flags, .. } => check_load_pair(&ctx, *flags, mem, vcode, 32),
        Inst::VecLoadReplicate {
            rn, flags, size, ..
        } => check_load_addr(&ctx, *flags, *rn, vcode, size.lane_size().ty()),
        Inst::LoadAcquire {
            access_ty,
            rn,
            flags,
            ..
        } => check_load_addr(&ctx, *flags, *rn, vcode, *access_ty),

        Inst::Store8 { rd, mem, flags } => check_store(&ctx, Some(*rd), *flags, mem, vcode, I8),
        Inst::Store16 { rd, mem, flags } => check_store(&ctx, Some(*rd), *flags, mem, vcode, I16),
        Inst::Store32 { rd, mem, flags } => check_store(&ctx, Some(*rd), *flags, mem, vcode, I32),
        Inst::Store64 { rd, mem, flags } => check_store(&ctx, Some(*rd), *flags, mem, vcode, I64),
        Inst::FpuStore32 { mem, flags, .. } => check_store(&ctx, None, *flags, mem, vcode, F32),
        Inst::FpuStore64 { mem, flags, .. } => check_store(&ctx, None, *flags, mem, vcode, F64),
        Inst::FpuStore128 { mem, flags, .. } => check_store(&ctx, None, *flags, mem, vcode, I8X16),
        Inst::StoreP64 { mem, flags, .. } => check_store_pair(&ctx, *flags, mem, vcode, 16),
        Inst::FpuStoreP64 { mem, flags, .. } => check_store_pair(&ctx, *flags, mem, vcode, 16),
        Inst::FpuStoreP128 { mem, flags, .. } => check_store_pair(&ctx, *flags, mem, vcode, 32),
        Inst::StoreRelease {
            access_ty,
            rn,
            flags,
            ..
        } => check_store_addr(&ctx, *flags, *rn, vcode, *access_ty),

        Inst::AluRRR {
            alu_op: ALUOp::Add,
            size,
            rd,
            rn,
            rm,
        } if has_fact(vcode, *rn) && has_fact(vcode, *rm) => {
            check_output(&ctx, vcode, rd.to_reg(), || {
                let rn = get_fact(vcode, *rn)?;
                let rm = get_fact(vcode, *rm)?;
                fail_if_missing(ctx.add(rn, rm, size.bits().into()))
            })
        }
        Inst::AluRRImm12 {
            alu_op: ALUOp::Add,
            size,
            rd,
            rn,
            imm12,
        } if has_fact(vcode, *rn) => check_output(&ctx, vcode, rd.to_reg(), || {
            let rn = get_fact(vcode, *rn)?;
            let imm_fact = Fact::ValueMax {
                bit_width: size.bits().into(),
                max: imm12.value(),
            };
            fail_if_missing(ctx.add(&rn, &imm_fact, size.bits().into()))
        }),
        Inst::AluRRRShift {
            alu_op: ALUOp::Add,
            size,
            rd,
            rn,
            rm,
            shiftop,
        } if shiftop.op() == ShiftOp::LSL && has_fact(vcode, *rn) && has_fact(vcode, *rm) => {
            check_output(&ctx, vcode, rd.to_reg(), || {
                let rn = get_fact(vcode, *rn)?;
                let rm = get_fact(vcode, *rm)?;
                let rm_shifted = fail_if_missing(ctx.shl(
                    &rm,
                    size.bits().into(),
                    shiftop.amt().value().into(),
                ))?;
                fail_if_missing(ctx.add(&rn, &rm_shifted, size.bits().into()))
            })
        }
        Inst::AluRRRExtend {
            alu_op: ALUOp::Add,
            size,
            rd,
            rn,
            rm,
            extendop,
        } if has_fact(vcode, *rn) && has_fact(vcode, *rm) => {
            check_output(&ctx, vcode, rd.to_reg(), || {
                let rn = get_fact(vcode, *rn)?;
                let rm = get_fact(vcode, *rm)?;
                let rm_extended = fail_if_missing(extend_fact(&ctx, rm, *extendop))?;
                fail_if_missing(ctx.add(&rn, &rm_extended, size.bits().into()))
            })
        }
        Inst::AluRRImmShift {
            alu_op: ALUOp::Lsl,
            size,
            rd,
            rn,
            immshift,
        } if has_fact(vcode, *rn) && has_fact(vcode, *rn) => {
            check_output(&ctx, vcode, rd.to_reg(), || {
                let rn = get_fact(vcode, *rn)?;
                fail_if_missing(ctx.shl(&rn, size.bits().into(), immshift.value().into()))
            })
        }
        Inst::Extend {
            rd,
            rn,
            signed: false,
            from_bits,
            to_bits,
        } if has_fact(vcode, *rn) => check_output(&ctx, vcode, rd.to_reg(), || {
            let rn = get_fact(vcode, *rn)?;
            fail_if_missing(ctx.uextend(&rn, (*from_bits).into(), (*to_bits).into()))
        }),
        Inst::AluRRR { size, rd, .. }
        | Inst::AluRRImm12 { rd, size, .. }
        | Inst::AluRRRShift { rd, size, .. }
        | Inst::AluRRRExtend { rd, size, .. }
        | Inst::AluRRImmLogic { rd, size, .. }
        | Inst::AluRRImmShift { rd, size, .. } => {
            // Any ALU op can validate a max-value fact where the
            // value is the maximum for its bit-width.
            check_output(&ctx, vcode, rd.to_reg(), || {
                Ok(Fact::ValueMax {
                    bit_width: size.bits().into(),
                    max: size.max_value(),
                })
            })
        }

        i => {
            panic!("Fact on unknown inst: {:?}", i);
        }
    }
}

/// The operation we're checking against an amode: either
///
/// - a *load*, and we need to validate that the field's fact subsumes
///   the load result's fact, OR
///
/// - a *store*, and we need to validate that the stored data's fact
///   subsumes the field's fact.
enum LoadOrStore<'a> {
    Load { result_fact: Option<&'a Fact> },
    Store { stored_fact: Option<&'a Fact> },
}

fn check_load(
    ctx: &FactContext,
    rd: Option<Reg>,
    flags: MemFlags,
    addr: &AMode,
    vcode: &VCode<Inst>,
    ty: Type,
) -> PccResult<()> {
    let result_fact = rd.map(|rd| get_fact(vcode, rd)).transpose()?;
    check_addr(
        ctx,
        flags,
        addr,
        vcode,
        ty,
        LoadOrStore::Load { result_fact },
    )
}

fn check_store(
    ctx: &FactContext,
    rd: Option<Reg>,
    flags: MemFlags,
    addr: &AMode,
    vcode: &VCode<Inst>,
    ty: Type,
) -> PccResult<()> {
    let stored_fact = rd.map(|rd| get_fact(vcode, rd)).transpose()?;
    check_addr(
        ctx,
        flags,
        addr,
        vcode,
        ty,
        LoadOrStore::Store { stored_fact },
    )
}

fn check_addr<'a>(
    ctx: &FactContext,
    flags: MemFlags,
    addr: &AMode,
    vcode: &VCode<Inst>,
    ty: Type,
    op: LoadOrStore<'a>,
) -> PccResult<()> {
    if !flags.checked() {
        return Ok(());
    }

    trace!("check_addr: {:?}", addr);

    let check = |addr: &Fact, ty: Type| -> PccResult<()> {
        match op {
            LoadOrStore::Load { result_fact } => {
                let loaded_fact = ctx.load(addr, ty)?;
                trace!("checking a load: loaded_fact = {loaded_fact:?} result_fact = {result_fact:?}");
                if ctx.subsumes_fact_optionals(loaded_fact, result_fact) {
                    Ok(())
                } else {
                    Err(PccError::UnsupportedFact)
                }
            }
            LoadOrStore::Store { stored_fact } => ctx.store(addr, ty, stored_fact),
        }
    };

    match addr {
        &AMode::RegReg { rn, rm } => {
            let rn = get_fact(vcode, rn)?;
            let rm = get_fact(vcode, rm)?;
            let sum = fail_if_missing(ctx.add(&rn, &rm, 64))?;
            check(&sum, ty)
        }
        &AMode::RegScaled { rn, rm, ty } => {
            let rn = get_fact(vcode, rn)?;
            let rm = get_fact(vcode, rm)?;
            let rm_scaled = fail_if_missing(ctx.scale(&rm, 64, ty.bytes()))?;
            let sum = fail_if_missing(ctx.add(&rn, &rm_scaled, 64))?;
            check(&sum, ty)
        }
        &AMode::RegScaledExtended {
            rn,
            rm,
            ty,
            extendop,
        } => {
            let rn = get_fact(vcode, rn)?;
            let rm = get_fact(vcode, rm)?;
            let rm_extended = fail_if_missing(extend_fact(ctx, rm, extendop))?;
            let rm_scaled = fail_if_missing(ctx.scale(&rm_extended, 64, ty.bytes()))?;
            let sum = fail_if_missing(ctx.add(&rn, &rm_scaled, 64))?;
            check(&sum, ty)
        }
        &AMode::RegExtended { rn, rm, extendop } => {
            let rn = get_fact(vcode, rn)?;
            let rm = get_fact(vcode, rm)?;
            let rm_extended = fail_if_missing(extend_fact(ctx, rm, extendop))?;
            let sum = fail_if_missing(ctx.add(&rn, &rm_extended, 64))?;
            trace!("rn = {rn:?} rm = {rm:?} rm_extended = {rm_extended:?} sum = {sum:?}");
            check(&sum, ty)?;
            trace!(" -> checks out!");
            Ok(())
        }
        &AMode::Unscaled { rn, simm9 } => {
            let rn = get_fact(vcode, rn)?;
            let sum = fail_if_missing(ctx.offset(&rn, 64, simm9.value.into()))?;
            check(&sum, ty)
        }
        &AMode::UnsignedOffset { rn, uimm12 } => {
            let rn = get_fact(vcode, rn)?;
            // Safety: this will not overflow: `size` should be at
            // most 32 or 64 for large vector ops, and the `uimm12`'s
            // value is at most 4095.
            let uimm12: u64 = uimm12.value.into();
            let offset: u64 = uimm12.checked_mul(ty.bytes().into()).unwrap();
            // This `unwrap()` will always succeed because the value
            // will always be positive and much smaller than
            // `i64::MAX` (see above).
            let sum = fail_if_missing(ctx.offset(&rn, 64, i64::try_from(offset).unwrap()))?;
            check(&sum, ty)
        }
        &AMode::Label { .. } | &AMode::Const { .. } => {
            // Always accept: labels and constants must be within the
            // generated code (else they won't be resolved).
            Ok(())
        }
        &AMode::RegOffset { rn, off, .. } => {
            let rn = get_fact(vcode, rn)?;
            let sum = fail_if_missing(ctx.offset(&rn, 64, off))?;
            check(&sum, ty)
        }
        &AMode::SPOffset { .. }
        | &AMode::FPOffset { .. }
        | &AMode::NominalSPOffset { .. }
        | &AMode::SPPostIndexed { .. }
        | &AMode::SPPreIndexed { .. } => {
            // We trust ABI code (for now!) and no lowering rules
            // lower input value accesses directly to these.
            Ok(())
        }
    }
}

fn check_load_pair(
    _ctx: &FactContext,
    _flags: MemFlags,
    _addr: &PairAMode,
    _vcode: &VCode<Inst>,
    _size: u8,
) -> PccResult<()> {
    Err(PccError::UnimplementedInst)
}

fn check_store_pair(
    _ctx: &FactContext,
    _flags: MemFlags,
    _addr: &PairAMode,
    _vcode: &VCode<Inst>,
    _size: u8,
) -> PccResult<()> {
    Err(PccError::UnimplementedInst)
}

fn check_load_addr(
    ctx: &FactContext,
    flags: MemFlags,
    reg: Reg,
    vcode: &VCode<Inst>,
    ty: Type,
) -> PccResult<()> {
    if !flags.checked() {
        return Ok(());
    }
    let fact = get_fact(vcode, reg)?;
    let _output_fact = ctx.load(fact, ty)?;
    Ok(())
}

fn check_store_addr(
    ctx: &FactContext,
    flags: MemFlags,
    reg: Reg,
    vcode: &VCode<Inst>,
    ty: Type,
) -> PccResult<()> {
    if !flags.checked() {
        return Ok(());
    }
    let fact = get_fact(vcode, reg)?;
    let _output_fact = ctx.store(fact, ty, None)?;
    Ok(())
}
