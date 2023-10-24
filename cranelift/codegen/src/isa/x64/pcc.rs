//! Proof-carrying-code validation for x64 VCode.

use crate::ir::pcc::*;
use crate::ir::types::*;
use crate::ir::Type;
use crate::isa::x64::inst::args::{
    AluRmiROpcode, Amode, GprMem, GprMemImm, Imm8Gpr, Imm8Reg, RegMem, RegMemImm, ShiftKind,
    SyntheticAmode, ToWritableReg, XmmMem, XmmMemAligned, XmmMemAlignedImm, XmmMemImm,
};
use crate::isa::x64::inst::Inst;
use crate::machinst::pcc::*;
use crate::machinst::{InsnIndex, VCode};
use crate::machinst::{Reg, Writable};
use crate::trace;

pub(crate) fn check(
    ctx: &FactContext,
    vcode: &mut VCode<Inst>,
    inst_idx: InsnIndex,
) -> PccResult<()> {
    trace!("Checking facts on inst: {:?}", vcode[inst_idx]);

    match vcode[inst_idx] {
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
            src2: GprMemImm(RegMemImm::Reg { reg: src2 }),
            dst,
        } => {
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

        Inst::AluRmiR {
            size,
            op: AluRmiROpcode::Add,
            src1,
            src2: GprMemImm(RegMemImm::Imm { simm32 }),
            dst,
        } => {
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
        Inst::AluRmiR {
            size,
            op: AluRmiROpcode::Add,
            src1,
            src2: GprMemImm(RegMemImm::Mem { ref addr }),
            dst,
        } => {
            let bits: u16 = size.to_bits().into();
            let loaded = check_load(ctx, None, addr, vcode, size.to_type(), bits)?;
            check_unop(ctx, vcode, 64, dst.to_writable_reg(), src1.into(), |src1| {
                let sum = loaded.and_then(|loaded| ctx.add(src1, &loaded, bits));
                clamp_range(ctx, 64, bits, sum)
            })
        }

        Inst::AluRmiR {
            size,
            op: AluRmiROpcode::Sub,
            src1,
            src2: GprMemImm(RegMemImm::Imm { simm32 }),
            dst,
        } => {
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

        Inst::AluRmiR {
            size,
            src2: GprMemImm(RegMemImm::Mem { ref addr }),
            dst,
            ..
        }
        | Inst::AluRmRVex {
            size,
            src2: GprMem(RegMem::Mem { ref addr }),
            dst,
            ..
        } => {
            let loaded = check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
            check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                clamp_range(ctx, 64, size.to_bits().into(), loaded)
            })
        }
        Inst::CmpRmiR {
            size,
            src: GprMemImm(RegMemImm::Mem { ref addr }),
            ..
        } => {
            check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
            Ok(())
        }

        Inst::Cmove {
            size,
            consequent: GprMem(RegMem::Mem { ref addr, .. }),
            ..
        } => {
            check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
            Ok(())
        }

        Inst::Imm { simm64, dst, .. } => {
            check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                Ok(Fact::constant(64, simm64))
            })
        }

        Inst::AluConstOp {
            op: AluRmiROpcode::Xor,
            dst,
            ..
        } => check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
            Ok(Fact::constant(64, 0))
        }),

        Inst::AluRmiR {
            size,
            dst,
            src2: GprMemImm(RegMemImm::Reg { .. } | RegMemImm::Imm { .. }),
            ..
        }
        | Inst::AluConstOp { size, dst, .. }
        | Inst::MovRR { size, dst, .. }
        | Inst::AluRmRVex {
            size,
            src2: GprMem(RegMem::Reg { .. }),
            dst,
            ..
        } => {
            let bits: u16 = size.to_bits().into();
            trace!("generic case: bits = {}", bits);
            check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                clamp_range(ctx, 64, bits, None)
            })
        }

        Inst::MovzxRmR {
            ref ext_mode,
            src: GprMem(RegMem::Reg { reg: src }),
            dst,
        } => {
            let from_bytes: u16 = ext_mode.src_size().into();
            let to_bytes: u16 = ext_mode.dst_size().into();
            check_unop(ctx, vcode, 64, dst.to_writable_reg(), src, |src| {
                let extended = ctx.uextend(src, from_bytes * 8, to_bytes * 8);
                trace!("src = {:?} extended = {:?}", src, extended);
                clamp_range(ctx, 64, from_bytes * 8, extended)
            })
        }

        Inst::LoadEffectiveAddress {
            ref addr,
            dst,
            size,
        } => {
            let addr = addr.clone();
            let bits: u16 = size.to_bits().into();
            check_output(ctx, vcode, dst.to_writable_reg(), &[], |vcode| {
                trace!("checking output: addr = {:?}", addr);
                let clamped = clamp_range(ctx, 64, bits, compute_addr(ctx, vcode, &addr, bits));
                trace!("clamped = {:?}", clamped);
                clamped
            })
        }

        Inst::MovRM { size, src, ref dst } => {
            check_store(ctx, Some(src.to_reg()), dst, vcode, size.to_type())
        }
        Inst::MovImmM {
            size,
            simm32: _,
            ref dst,
        } => check_store(ctx, None, dst, vcode, size.to_type()),
        Inst::Mov64MR { ref src, dst } => {
            check_load(ctx, Some(dst.to_writable_reg()), src, vcode, I64, 64)?;
            Ok(())
        }
        Inst::MovzxRmR {
            ref ext_mode,
            src: GprMem(RegMem::Mem { ref addr }),
            dst,
        } => {
            check_load(
                ctx,
                Some(dst.to_writable_reg()),
                addr,
                vcode,
                ext_mode.src_type(),
                64,
            )?;
            Ok(())
        }

        Inst::AluRM {
            size,
            op: _,
            ref src1_dst,
            src2: _,
        } => {
            check_load(ctx, None, src1_dst, vcode, size.to_type(), 64)?;
            check_store(ctx, None, src1_dst, vcode, size.to_type())
        }

        Inst::UnaryRmR {
            size,
            src: GprMem(RegMem::Mem { ref addr }),
            dst,
            ..
        }
        | Inst::UnaryRmRVex {
            size,
            src: GprMem(RegMem::Mem { ref addr }),
            dst,
            ..
        }
        | Inst::UnaryRmRImmVex {
            size,
            src: GprMem(RegMem::Mem { ref addr }),
            dst,
            ..
        } => {
            check_load(ctx, None, addr, vcode, size.to_type(), 64)?;
            check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                clamp_range(ctx, 64, size.to_bits().into(), None)
            })
        }

        Inst::Div {
            divisor: GprMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::Div8 {
            divisor: GprMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::MulHi {
            src2: GprMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::UMulLo {
            src2: GprMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::MovsxRmR {
            src: GprMem(RegMem::Mem { ref addr }),
            ..
        } => {
            // Round up on size: some of the above will take 32- or
            // 8-bit mem args, but if we can validate assuming a
            // 64-bit load, we're still (conservatively) safe.
            check_load(ctx, None, addr, vcode, I64, 64)?;
            Ok(())
        }

        Inst::ShiftR {
            size,
            kind: ShiftKind::ShiftLeft,
            src,
            num_bits: Imm8Gpr(Imm8Reg::Imm8 { imm }),
            dst,
        } => check_unop(ctx, vcode, 64, dst.to_writable_reg(), src.to_reg(), |src| {
            clamp_range(
                ctx,
                64,
                size.to_bits().into(),
                ctx.shl(src, size.to_bits().into(), imm.into()),
            )
        }),

        Inst::ShiftR { size, dst, .. } => {
            check_output(ctx, vcode, dst.to_writable_reg(), &[], |_vcode| {
                clamp_range(ctx, 64, size.to_bits().into(), None)
            })
        }

        Inst::Push64 {
            src: GprMemImm(RegMemImm::Mem { ref addr }),
        } => {
            check_load(ctx, None, addr, vcode, I64, 64)?;
            Ok(())
        }

        Inst::XmmMovRMVex { ref dst, .. }
        | Inst::XmmMovRMImmVex { ref dst, .. }
        | Inst::XmmMovRM { ref dst, .. }
        | Inst::XmmMovRMImm { ref dst, .. } => {
            check_store(ctx, None, dst, vcode, I8X16)?;
            Ok(())
        }

        Inst::XmmRmiReg {
            src2: XmmMemAlignedImm(RegMemImm::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmiRVex {
            src2: XmmMemImm(RegMemImm::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmRImmVex {
            src2: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmR {
            src2: XmmMemAligned(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmRUnaligned {
            src2: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmRBlend {
            src2: XmmMemAligned(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmRVex3 {
            src3: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmRBlendVex {
            src2: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmUnaryRmR {
            src: XmmMemAligned(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmUnaryRmRUnaligned {
            src: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmUnaryRmRImm {
            src: XmmMemAligned(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmUnaryRmREvex {
            src: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmUnaryRmRVex {
            src: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmUnaryRmRImmVex {
            src: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmREvex {
            src2: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmUnaryRmRImmEvex {
            src: XmmMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmREvex3 {
            src3: XmmMem(RegMem::Mem { ref addr }),
            ..
        } => {
            check_load(ctx, None, addr, vcode, I8X16, 128)?;
            Ok(())
        }

        Inst::XmmVexPinsr {
            src2: GprMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::GprToXmm {
            src: GprMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::GprToXmmVex {
            src: GprMem(RegMem::Mem { ref addr }),
            ..
        } => {
            check_load(ctx, None, addr, vcode, I64, 64)?;
            Ok(())
        }

        Inst::XmmCmove {
            consequent: XmmMemAligned(RegMem::Mem { ref addr }),
            ..
        } => {
            check_load(ctx, None, addr, vcode, I8X16, 128)?;
            Ok(())
        }

        Inst::XmmCmpRmR {
            src: XmmMemAligned(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::XmmRmRImm {
            src2: RegMem::Mem { ref addr },
            ..
        } => {
            check_load(ctx, None, addr, vcode, I8X16, 128)?;
            Ok(())
        }

        Inst::CvtIntToFloat {
            src2: GprMem(RegMem::Mem { ref addr }),
            ..
        }
        | Inst::CvtIntToFloatVex {
            src2: GprMem(RegMem::Mem { ref addr }),
            ..
        } => {
            check_load(ctx, None, addr, vcode, I64, 64)?;
            Ok(())
        }

        Inst::LockCmpxchg { mem: ref dst, .. } | Inst::AtomicRmwSeq { mem: ref dst, .. } => {
            check_store(ctx, None, dst, vcode, I64)?;
            Ok(())
        }

        Inst::CallUnknown {
            dest: RegMem::Mem { ref addr },
            ..
        }
        | Inst::ReturnCallUnknown {
            callee: RegMem::Mem { ref addr },
            ..
        }
        | Inst::JmpUnknown {
            target: RegMem::Mem { ref addr },
            ..
        } => {
            check_load(ctx, None, addr, vcode, I64, 64)?;
            Ok(())
        }

        _ if vcode.inst_defines_facts(inst_idx) => {
            trace!(
                "Unsupported inst during PCC validation: {:?}",
                vcode[inst_idx]
            );
            Err(PccError::UnsupportedFact)
        }

        _ => Ok(()),
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
    match amode {
        SyntheticAmode::Real(amode) if !amode.get_flags().checked() => return Ok(None),
        SyntheticAmode::NominalSPOffset { .. } | SyntheticAmode::ConstantOffset(_) => {
            return Ok(None)
        }
        _ => {}
    }

    let addr = compute_addr(ctx, vcode, amode, 64).ok_or(PccError::MissingFact)?;

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
            if ctx.subsumes_fact_optionals(Some(&loaded_fact), result_fact) {
                Ok(Some(loaded_fact.clone()))
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

fn compute_addr(
    ctx: &FactContext,
    vcode: &VCode<Inst>,
    amode: &SyntheticAmode,
    bits: u16,
) -> Option<Fact> {
    trace!("compute_addr: {:?}", amode);
    match *amode {
        SyntheticAmode::Real(Amode::ImmReg { simm32, base, .. }) => {
            let base = get_fact_or_default(vcode, base, bits);
            trace!("base = {:?}", base);
            let simm32: i64 = simm32.into();
            let simm32: u64 = simm32 as u64;
            let offset = Fact::constant(bits, simm32);
            let sum = ctx.add(&base, &offset, bits)?;
            trace!("sum = {:?}", sum);
            Some(sum)
        }
        SyntheticAmode::Real(Amode::ImmRegRegShift {
            simm32,
            base,
            index,
            shift,
            ..
        }) => {
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
        SyntheticAmode::Real(Amode::RipRelative { .. })
        | SyntheticAmode::ConstantOffset(_)
        | SyntheticAmode::NominalSPOffset { .. } => None,
    }
}
