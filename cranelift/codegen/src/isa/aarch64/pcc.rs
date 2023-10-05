//! Proof-carrying code checking for AArch64 VCode.

use crate::facts::*;
use crate::ir::MemFlags;
use crate::isa::aarch64::inst::Inst;
use crate::isa::aarch64::inst::{AMode, ExtendOp};
use crate::machinst::VCode;
use crate::trace;
use crate::{CodegenError, CodegenResult};

pub(crate) fn check(inst: &Inst, vcode: &VCode<Inst>) -> FactResult<()> {
    trace!("Checking facts on inst: {:?}", inst);

    match inst {
        Inst::Args { .. } => {
            // Defs on the args have "axiomatic facts": we trust the
            // ABI code to pass through the values unharmed, so the
            // facts given to us in the CLIF should still be true.
            Ok(())
        }
        Inst::ULoad8 { mem, flags, .. } => check_addr(*flags, mem, vcode, 1),
        i => {
            panic!("Fact on unknown inst: {:?}", i);
        }
    }
}

fn check_addr(flags: MemFlags, addr: &AMode, vcode: &VCode<Inst>, size: u8) -> FactResult<()> {
    if !flags.safe() {
        return Ok(());
    }

    match addr {
        &AMode::RegReg { rn, rm } => {
            panic!("oh no")
        }
        &AMode::RegScaled { rn, rm, ty } => {
            panic!("oh no")
        }
        &AMode::RegScaledExtended {
            rn,
            rm,
            ty,
            extendop,
        } => {
            panic!("oh no")
        }
        &AMode::RegExtended { rn, rm, extendop } => {
            // We need facts on `rn` and `rm`; we add the two.
            let rn = vcode
                .vreg_fact(rn.into())
                .ok_or_else(|| FactError::new("no fact on addr mode source register"))?
                .clone();
            let rm = vcode
                .vreg_fact(rm.into())
                .ok_or_else(|| FactError::new("no fact on addr mode source register"))?
                .clone();

            let rm_extended = match extendop {
                ExtendOp::UXTB => Fact::uextend(rm, 8, 64),
                ExtendOp::UXTH => Fact::uextend(rm, 16, 64),
                ExtendOp::UXTW => Fact::uextend(rm, 32, 64),
                ExtendOp::UXTX => Some(rm),
                ExtendOp::SXTB => Fact::sextend(rm, 8, 64),
                ExtendOp::SXTH => Fact::uextend(rm, 16, 64),
                ExtendOp::SXTW => Fact::uextend(rm, 32, 64),
                ExtendOp::SXTX => None,
            }
            .ok_or_else(|| FactError::new("Missing fact on extended value in amode"))?;

            let accessed = Fact::add(rn, rm_extended, 64, 64).ok_or_else(|| {
                FactError::new("cannot add values with given facts in addressing mode")
            })?;

            Fact::check_address(0, size as u32, accessed)
        }
        &AMode::Unscaled { rn, simm9 } => {
            panic!("oh no")
        }
        &AMode::UnsignedOffset { rn, uimm12 } => {
            panic!("oh no")
        }
        &AMode::Label { .. } | &AMode::Const { .. } => {
            // Always accept: labels and constants must be within the
            // generated code (else they won't be resolved).
            Ok(())
        }
        &AMode::RegOffset { rn, off, ty } => {
            panic!("oh no")
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
