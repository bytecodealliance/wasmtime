//! This module defines s390x-specific machine instruction types.

// Some variants are not constructed, but we still want them as options in the future.
#![allow(dead_code)]

use crate::binemit::{Addend, CodeOffset, Reloc};
use crate::ir::{types, ExternalName, Opcode, Type};
use crate::machinst::*;
use crate::{settings, CodegenError, CodegenResult};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::convert::TryFrom;
use regalloc2::VReg;
use smallvec::{smallvec, SmallVec};
use std::string::{String, ToString};
pub mod regs;
pub use self::regs::*;
pub mod imms;
pub use self::imms::*;
pub mod args;
pub use self::args::*;
pub mod emit;
pub use self::emit::*;
pub mod unwind;

#[cfg(test)]
mod emit_tests;

//=============================================================================
// Instructions (top level): definition

pub use crate::isa::s390x::lower::isle::generated_code::{
    ALUOp, CmpOp, FPUOp1, FPUOp2, FPUOp3, FpuRoundMode, FpuToIntOp, IntToFpuOp, MInst as Inst,
    RxSBGOp, ShiftOp, UnaryOp,
};

/// Additional information for (direct) Call instructions, left out of line to lower the size of
/// the Inst enum.
#[derive(Clone, Debug)]
pub struct CallInfo {
    pub dest: ExternalName,
    pub uses: Vec<Reg>,
    pub defs: Vec<Writable<Reg>>,
    pub opcode: Opcode,
}

/// Additional information for CallInd instructions, left out of line to lower the size of the Inst
/// enum.
#[derive(Clone, Debug)]
pub struct CallIndInfo {
    pub rn: Reg,
    pub uses: Vec<Reg>,
    pub defs: Vec<Writable<Reg>>,
    pub opcode: Opcode,
}

#[test]
fn inst_size_test() {
    // This test will help with unintentionally growing the size
    // of the Inst enum.
    assert_eq!(32, std::mem::size_of::<Inst>());
}

/// Supported instruction sets
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub(crate) enum InstructionSet {
    /// Baseline ISA for cranelift is z14.
    Base,
    /// Miscellaneous-Instruction-Extensions Facility 2 (z15)
    MIE2,
    /// Vector-Enhancements Facility 2 (z15)
    VXRS_EXT2,
}

impl Inst {
    /// Retrieve the ISA feature set in which the instruction is available.
    fn available_in_isa(&self) -> InstructionSet {
        match self {
            // These instructions are part of the baseline ISA for cranelift (z14)
            Inst::Nop0
            | Inst::Nop2
            | Inst::AluRRSImm16 { .. }
            | Inst::AluRR { .. }
            | Inst::AluRX { .. }
            | Inst::AluRSImm16 { .. }
            | Inst::AluRSImm32 { .. }
            | Inst::AluRUImm32 { .. }
            | Inst::AluRUImm16Shifted { .. }
            | Inst::AluRUImm32Shifted { .. }
            | Inst::ShiftRR { .. }
            | Inst::RxSBG { .. }
            | Inst::RxSBGTest { .. }
            | Inst::SMulWide { .. }
            | Inst::UMulWide { .. }
            | Inst::SDivMod32 { .. }
            | Inst::SDivMod64 { .. }
            | Inst::UDivMod32 { .. }
            | Inst::UDivMod64 { .. }
            | Inst::Flogr { .. }
            | Inst::CmpRR { .. }
            | Inst::CmpRX { .. }
            | Inst::CmpRSImm16 { .. }
            | Inst::CmpRSImm32 { .. }
            | Inst::CmpRUImm32 { .. }
            | Inst::CmpTrapRR { .. }
            | Inst::CmpTrapRSImm16 { .. }
            | Inst::CmpTrapRUImm16 { .. }
            | Inst::AtomicRmw { .. }
            | Inst::AtomicCas32 { .. }
            | Inst::AtomicCas64 { .. }
            | Inst::Fence
            | Inst::Load32 { .. }
            | Inst::Load32ZExt8 { .. }
            | Inst::Load32SExt8 { .. }
            | Inst::Load32ZExt16 { .. }
            | Inst::Load32SExt16 { .. }
            | Inst::Load64 { .. }
            | Inst::Load64ZExt8 { .. }
            | Inst::Load64SExt8 { .. }
            | Inst::Load64ZExt16 { .. }
            | Inst::Load64SExt16 { .. }
            | Inst::Load64ZExt32 { .. }
            | Inst::Load64SExt32 { .. }
            | Inst::LoadRev16 { .. }
            | Inst::LoadRev32 { .. }
            | Inst::LoadRev64 { .. }
            | Inst::Store8 { .. }
            | Inst::Store16 { .. }
            | Inst::Store32 { .. }
            | Inst::Store64 { .. }
            | Inst::StoreImm8 { .. }
            | Inst::StoreImm16 { .. }
            | Inst::StoreImm32SExt16 { .. }
            | Inst::StoreImm64SExt16 { .. }
            | Inst::StoreRev16 { .. }
            | Inst::StoreRev32 { .. }
            | Inst::StoreRev64 { .. }
            | Inst::LoadMultiple64 { .. }
            | Inst::StoreMultiple64 { .. }
            | Inst::Mov32 { .. }
            | Inst::Mov64 { .. }
            | Inst::Mov32Imm { .. }
            | Inst::Mov32SImm16 { .. }
            | Inst::Mov64SImm16 { .. }
            | Inst::Mov64SImm32 { .. }
            | Inst::Mov64UImm16Shifted { .. }
            | Inst::Mov64UImm32Shifted { .. }
            | Inst::Insert64UImm16Shifted { .. }
            | Inst::Insert64UImm32Shifted { .. }
            | Inst::Extend { .. }
            | Inst::CMov32 { .. }
            | Inst::CMov64 { .. }
            | Inst::CMov32SImm16 { .. }
            | Inst::CMov64SImm16 { .. }
            | Inst::FpuMove32 { .. }
            | Inst::FpuMove64 { .. }
            | Inst::FpuCMov32 { .. }
            | Inst::FpuCMov64 { .. }
            | Inst::MovToFpr { .. }
            | Inst::MovFromFpr { .. }
            | Inst::FpuRR { .. }
            | Inst::FpuRRR { .. }
            | Inst::FpuRRRR { .. }
            | Inst::FpuCopysign { .. }
            | Inst::FpuCmp32 { .. }
            | Inst::FpuCmp64 { .. }
            | Inst::FpuLoad32 { .. }
            | Inst::FpuStore32 { .. }
            | Inst::FpuLoad64 { .. }
            | Inst::FpuStore64 { .. }
            | Inst::LoadFpuConst32 { .. }
            | Inst::LoadFpuConst64 { .. }
            | Inst::FpuToInt { .. }
            | Inst::IntToFpu { .. }
            | Inst::FpuRound { .. }
            | Inst::FpuVecRRR { .. }
            | Inst::Call { .. }
            | Inst::CallInd { .. }
            | Inst::Ret { .. }
            | Inst::EpiloguePlaceholder
            | Inst::Jump { .. }
            | Inst::CondBr { .. }
            | Inst::TrapIf { .. }
            | Inst::OneWayCondBr { .. }
            | Inst::IndirectBr { .. }
            | Inst::Debugtrap
            | Inst::Trap { .. }
            | Inst::JTSequence { .. }
            | Inst::LoadExtNameFar { .. }
            | Inst::LoadAddr { .. }
            | Inst::Loop { .. }
            | Inst::CondBreak { .. }
            | Inst::VirtualSPOffsetAdj { .. }
            | Inst::Unwind { .. } => InstructionSet::Base,

            // These depend on the opcode
            Inst::AluRRR { alu_op, .. } => match alu_op {
                ALUOp::AndNot32 | ALUOp::AndNot64 => InstructionSet::MIE2,
                ALUOp::OrrNot32 | ALUOp::OrrNot64 => InstructionSet::MIE2,
                ALUOp::XorNot32 | ALUOp::XorNot64 => InstructionSet::MIE2,
                _ => InstructionSet::Base,
            },
            Inst::UnaryRR { op, .. } => match op {
                UnaryOp::PopcntReg => InstructionSet::MIE2,
                _ => InstructionSet::Base,
            },

            // These are all part of VXRS_EXT2
            Inst::FpuLoadRev32 { .. }
            | Inst::FpuStoreRev32 { .. }
            | Inst::FpuLoadRev64 { .. }
            | Inst::FpuStoreRev64 { .. } => InstructionSet::VXRS_EXT2,

            Inst::DummyUse { .. } => InstructionSet::Base,
        }
    }

    /// Create a 64-bit move instruction.
    pub fn mov64(to_reg: Writable<Reg>, from_reg: Reg) -> Inst {
        assert!(to_reg.to_reg().class() == from_reg.class());
        if from_reg.class() == RegClass::Int {
            Inst::Mov64 {
                rd: to_reg,
                rm: from_reg,
            }
        } else {
            Inst::FpuMove64 {
                rd: to_reg,
                rn: from_reg,
            }
        }
    }

    /// Create a 32-bit move instruction.
    pub fn mov32(to_reg: Writable<Reg>, from_reg: Reg) -> Inst {
        if from_reg.class() == RegClass::Int {
            Inst::Mov32 {
                rd: to_reg,
                rm: from_reg,
            }
        } else {
            Inst::FpuMove32 {
                rd: to_reg,
                rn: from_reg,
            }
        }
    }

    /// Create an instruction that loads a 64-bit integer constant.
    pub fn load_constant64(rd: Writable<Reg>, value: u64) -> SmallVec<[Inst; 4]> {
        if let Ok(imm) = i16::try_from(value as i64) {
            // 16-bit signed immediate
            smallvec![Inst::Mov64SImm16 { rd, imm }]
        } else if let Ok(imm) = i32::try_from(value as i64) {
            // 32-bit signed immediate
            smallvec![Inst::Mov64SImm32 { rd, imm }]
        } else if let Some(imm) = UImm16Shifted::maybe_from_u64(value) {
            // 16-bit shifted immediate
            smallvec![Inst::Mov64UImm16Shifted { rd, imm }]
        } else if let Some(imm) = UImm32Shifted::maybe_from_u64(value) {
            // 32-bit shifted immediate
            smallvec![Inst::Mov64UImm32Shifted { rd, imm }]
        } else {
            let mut insts = smallvec![];
            let hi = value & 0xffff_ffff_0000_0000u64;
            let lo = value & 0x0000_0000_ffff_ffffu64;

            if let Some(imm) = UImm16Shifted::maybe_from_u64(hi) {
                // 16-bit shifted immediate
                insts.push(Inst::Mov64UImm16Shifted { rd, imm });
            } else if let Some(imm) = UImm32Shifted::maybe_from_u64(hi) {
                // 32-bit shifted immediate
                insts.push(Inst::Mov64UImm32Shifted { rd, imm });
            } else {
                unreachable!();
            }

            if let Some(imm) = UImm16Shifted::maybe_from_u64(lo) {
                // 16-bit shifted immediate
                insts.push(Inst::Insert64UImm16Shifted { rd, imm });
            } else if let Some(imm) = UImm32Shifted::maybe_from_u64(lo) {
                // 32-bit shifted immediate
                insts.push(Inst::Insert64UImm32Shifted { rd, imm });
            } else {
                unreachable!();
            }

            insts
        }
    }

    /// Create an instruction that loads a 32-bit integer constant.
    pub fn load_constant32(rd: Writable<Reg>, value: u32) -> SmallVec<[Inst; 4]> {
        if let Ok(imm) = i16::try_from(value as i32) {
            // 16-bit signed immediate
            smallvec![Inst::Mov32SImm16 { rd, imm }]
        } else {
            // 32-bit full immediate
            smallvec![Inst::Mov32Imm { rd, imm: value }]
        }
    }

    /// Create an instruction that loads a 32-bit floating-point constant.
    pub fn load_fp_constant32(rd: Writable<Reg>, value: f32) -> Inst {
        // TODO: use LZER to load 0.0
        Inst::LoadFpuConst32 {
            rd,
            const_data: value.to_bits(),
        }
    }

    /// Create an instruction that loads a 64-bit floating-point constant.
    pub fn load_fp_constant64(rd: Writable<Reg>, value: f64) -> Inst {
        // TODO: use LZDR to load 0.0
        Inst::LoadFpuConst64 {
            rd,
            const_data: value.to_bits(),
        }
    }

    /// Generic constructor for a load (zero-extending where appropriate).
    pub fn gen_load(into_reg: Writable<Reg>, mem: MemArg, ty: Type) -> Inst {
        match ty {
            types::B1 | types::B8 | types::I8 => Inst::Load64ZExt8 { rd: into_reg, mem },
            types::B16 | types::I16 => Inst::Load64ZExt16 { rd: into_reg, mem },
            types::B32 | types::I32 => Inst::Load64ZExt32 { rd: into_reg, mem },
            types::B64 | types::I64 | types::R64 => Inst::Load64 { rd: into_reg, mem },
            types::F32 => Inst::FpuLoad32 { rd: into_reg, mem },
            types::F64 => Inst::FpuLoad64 { rd: into_reg, mem },
            _ => unimplemented!("gen_load({})", ty),
        }
    }

    /// Generic constructor for a store.
    pub fn gen_store(mem: MemArg, from_reg: Reg, ty: Type) -> Inst {
        match ty {
            types::B1 | types::B8 | types::I8 => Inst::Store8 { rd: from_reg, mem },
            types::B16 | types::I16 => Inst::Store16 { rd: from_reg, mem },
            types::B32 | types::I32 => Inst::Store32 { rd: from_reg, mem },
            types::B64 | types::I64 | types::R64 => Inst::Store64 { rd: from_reg, mem },
            types::F32 => Inst::FpuStore32 { rd: from_reg, mem },
            types::F64 => Inst::FpuStore64 { rd: from_reg, mem },
            _ => unimplemented!("gen_store({})", ty),
        }
    }
}

//=============================================================================
// Instructions: get_regs

fn memarg_operands<F: Fn(VReg) -> VReg>(memarg: &MemArg, collector: &mut OperandCollector<'_, F>) {
    match memarg {
        &MemArg::BXD12 { base, index, .. } | &MemArg::BXD20 { base, index, .. } => {
            collector.reg_use(base);
            collector.reg_use(index);
        }
        &MemArg::Label { .. } | &MemArg::Symbol { .. } => {}
        &MemArg::RegOffset { reg, .. } => {
            collector.reg_use(reg);
        }
        &MemArg::InitialSPOffset { .. } | &MemArg::NominalSPOffset { .. } => {}
    }
}

fn s390x_get_operands<F: Fn(VReg) -> VReg>(inst: &Inst, collector: &mut OperandCollector<'_, F>) {
    match inst {
        &Inst::AluRRR { rd, rn, rm, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
            collector.reg_use(rm);
        }
        &Inst::AluRRSImm16 { rd, rn, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::AluRR { rd, rm, .. } => {
            collector.reg_mod(rd);
            collector.reg_use(rm);
        }
        &Inst::AluRX { rd, ref mem, .. } => {
            collector.reg_mod(rd);
            memarg_operands(mem, collector);
        }
        &Inst::AluRSImm16 { rd, .. } => {
            collector.reg_mod(rd);
        }
        &Inst::AluRSImm32 { rd, .. } => {
            collector.reg_mod(rd);
        }
        &Inst::AluRUImm32 { rd, .. } => {
            collector.reg_mod(rd);
        }
        &Inst::AluRUImm16Shifted { rd, .. } => {
            collector.reg_mod(rd);
        }
        &Inst::AluRUImm32Shifted { rd, .. } => {
            collector.reg_mod(rd);
        }
        &Inst::SMulWide { rn, rm, .. } => {
            collector.reg_use(rn);
            collector.reg_use(rm);
            collector.reg_def(writable_gpr(0));
            collector.reg_def(writable_gpr(1));
        }
        &Inst::UMulWide { rn, .. } => {
            collector.reg_use(rn);
            collector.reg_def(writable_gpr(0));
            collector.reg_mod(writable_gpr(1));
        }
        &Inst::SDivMod32 { rn, .. } | &Inst::SDivMod64 { rn, .. } => {
            collector.reg_use(rn);
            collector.reg_def(writable_gpr(0));
            collector.reg_mod(writable_gpr(1));
        }
        &Inst::UDivMod32 { rn, .. } | &Inst::UDivMod64 { rn, .. } => {
            collector.reg_use(rn);
            collector.reg_mod(writable_gpr(0));
            collector.reg_mod(writable_gpr(1));
        }
        &Inst::Flogr { rn, .. } => {
            collector.reg_use(rn);
            collector.reg_def(writable_gpr(0));
            collector.reg_def(writable_gpr(1));
        }
        &Inst::ShiftRR {
            rd, rn, shift_reg, ..
        } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
            collector.reg_use(shift_reg);
        }
        &Inst::RxSBG { rd, rn, .. } => {
            collector.reg_mod(rd);
            collector.reg_use(rn);
        }
        &Inst::RxSBGTest { rd, rn, .. } => {
            collector.reg_use(rd);
            collector.reg_use(rn);
        }
        &Inst::UnaryRR { rd, rn, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::CmpRR { rn, rm, .. } => {
            collector.reg_use(rn);
            collector.reg_use(rm);
        }
        &Inst::CmpRX { rn, ref mem, .. } => {
            collector.reg_use(rn);
            memarg_operands(mem, collector);
        }
        &Inst::CmpRSImm16 { rn, .. } => {
            collector.reg_use(rn);
        }
        &Inst::CmpRSImm32 { rn, .. } => {
            collector.reg_use(rn);
        }
        &Inst::CmpRUImm32 { rn, .. } => {
            collector.reg_use(rn);
        }
        &Inst::CmpTrapRR { rn, rm, .. } => {
            collector.reg_use(rn);
            collector.reg_use(rm);
        }
        &Inst::CmpTrapRSImm16 { rn, .. } => {
            collector.reg_use(rn);
        }
        &Inst::CmpTrapRUImm16 { rn, .. } => {
            collector.reg_use(rn);
        }
        &Inst::AtomicRmw {
            rd, rn, ref mem, ..
        } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
            memarg_operands(mem, collector);
        }
        &Inst::AtomicCas32 {
            rd, rn, ref mem, ..
        }
        | &Inst::AtomicCas64 {
            rd, rn, ref mem, ..
        } => {
            collector.reg_mod(rd);
            collector.reg_use(rn);
            memarg_operands(mem, collector);
        }
        &Inst::Fence => {}
        &Inst::Load32 { rd, ref mem, .. }
        | &Inst::Load32ZExt8 { rd, ref mem, .. }
        | &Inst::Load32SExt8 { rd, ref mem, .. }
        | &Inst::Load32ZExt16 { rd, ref mem, .. }
        | &Inst::Load32SExt16 { rd, ref mem, .. }
        | &Inst::Load64 { rd, ref mem, .. }
        | &Inst::Load64ZExt8 { rd, ref mem, .. }
        | &Inst::Load64SExt8 { rd, ref mem, .. }
        | &Inst::Load64ZExt16 { rd, ref mem, .. }
        | &Inst::Load64SExt16 { rd, ref mem, .. }
        | &Inst::Load64ZExt32 { rd, ref mem, .. }
        | &Inst::Load64SExt32 { rd, ref mem, .. }
        | &Inst::LoadRev16 { rd, ref mem, .. }
        | &Inst::LoadRev32 { rd, ref mem, .. }
        | &Inst::LoadRev64 { rd, ref mem, .. } => {
            collector.reg_def(rd);
            memarg_operands(mem, collector);
        }
        &Inst::Store8 { rd, ref mem, .. }
        | &Inst::Store16 { rd, ref mem, .. }
        | &Inst::Store32 { rd, ref mem, .. }
        | &Inst::Store64 { rd, ref mem, .. }
        | &Inst::StoreRev16 { rd, ref mem, .. }
        | &Inst::StoreRev32 { rd, ref mem, .. }
        | &Inst::StoreRev64 { rd, ref mem, .. } => {
            collector.reg_use(rd);
            memarg_operands(mem, collector);
        }
        &Inst::StoreImm8 { ref mem, .. }
        | &Inst::StoreImm16 { ref mem, .. }
        | &Inst::StoreImm32SExt16 { ref mem, .. }
        | &Inst::StoreImm64SExt16 { ref mem, .. } => {
            memarg_operands(mem, collector);
        }
        &Inst::LoadMultiple64 {
            rt, rt2, ref mem, ..
        } => {
            memarg_operands(mem, collector);
            let first_regnum = rt.to_reg().to_real_reg().unwrap().hw_enc();
            let last_regnum = rt2.to_reg().to_real_reg().unwrap().hw_enc();
            for regnum in first_regnum..last_regnum + 1 {
                collector.reg_def(writable_gpr(regnum));
            }
        }
        &Inst::StoreMultiple64 {
            rt, rt2, ref mem, ..
        } => {
            memarg_operands(mem, collector);
            let first_regnum = rt.to_real_reg().unwrap().hw_enc();
            let last_regnum = rt2.to_real_reg().unwrap().hw_enc();
            for regnum in first_regnum..last_regnum + 1 {
                collector.reg_use(gpr(regnum));
            }
        }
        &Inst::Mov64 { rd, rm } => {
            collector.reg_def(rd);
            collector.reg_use(rm);
        }
        &Inst::Mov32 { rd, rm } => {
            collector.reg_def(rd);
            collector.reg_use(rm);
        }
        &Inst::Mov32Imm { rd, .. }
        | &Inst::Mov32SImm16 { rd, .. }
        | &Inst::Mov64SImm16 { rd, .. }
        | &Inst::Mov64SImm32 { rd, .. }
        | &Inst::Mov64UImm16Shifted { rd, .. }
        | &Inst::Mov64UImm32Shifted { rd, .. } => {
            collector.reg_def(rd);
        }
        &Inst::CMov32 { rd, rm, .. } | &Inst::CMov64 { rd, rm, .. } => {
            collector.reg_mod(rd);
            collector.reg_use(rm);
        }
        &Inst::CMov32SImm16 { rd, .. } | &Inst::CMov64SImm16 { rd, .. } => {
            collector.reg_mod(rd);
        }
        &Inst::Insert64UImm16Shifted { rd, .. } | &Inst::Insert64UImm32Shifted { rd, .. } => {
            collector.reg_mod(rd);
        }
        &Inst::FpuMove32 { rd, rn } | &Inst::FpuMove64 { rd, rn } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::FpuCMov32 { rd, rm, .. } | &Inst::FpuCMov64 { rd, rm, .. } => {
            collector.reg_mod(rd);
            collector.reg_use(rm);
        }
        &Inst::MovToFpr { rd, rn } | &Inst::MovFromFpr { rd, rn } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::FpuRR { rd, rn, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::FpuRRR { rd, rm, .. } => {
            collector.reg_mod(rd);
            collector.reg_use(rm);
        }
        &Inst::FpuRRRR { rd, rn, rm, .. } => {
            collector.reg_mod(rd);
            collector.reg_use(rn);
            collector.reg_use(rm);
        }
        &Inst::FpuCopysign { rd, rn, rm, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
            collector.reg_use(rm);
        }
        &Inst::FpuCmp32 { rn, rm } | &Inst::FpuCmp64 { rn, rm } => {
            collector.reg_use(rn);
            collector.reg_use(rm);
        }
        &Inst::FpuLoad32 { rd, ref mem, .. } => {
            collector.reg_def(rd);
            memarg_operands(mem, collector);
        }
        &Inst::FpuLoad64 { rd, ref mem, .. } => {
            collector.reg_def(rd);
            memarg_operands(mem, collector);
        }
        &Inst::FpuStore32 { rd, ref mem, .. } => {
            collector.reg_use(rd);
            memarg_operands(mem, collector);
        }
        &Inst::FpuStore64 { rd, ref mem, .. } => {
            collector.reg_use(rd);
            memarg_operands(mem, collector);
        }
        &Inst::FpuLoadRev32 { rd, ref mem, .. } => {
            collector.reg_def(rd);
            memarg_operands(mem, collector);
        }
        &Inst::FpuLoadRev64 { rd, ref mem, .. } => {
            collector.reg_def(rd);
            memarg_operands(mem, collector);
        }
        &Inst::FpuStoreRev32 { rd, ref mem, .. } => {
            collector.reg_use(rd);
            memarg_operands(mem, collector);
        }
        &Inst::FpuStoreRev64 { rd, ref mem, .. } => {
            collector.reg_use(rd);
            memarg_operands(mem, collector);
        }
        &Inst::LoadFpuConst32 { rd, .. } | &Inst::LoadFpuConst64 { rd, .. } => {
            collector.reg_def(rd);
        }
        &Inst::FpuToInt { rd, rn, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::IntToFpu { rd, rn, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::FpuRound { rd, rn, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::FpuVecRRR { rd, rn, rm, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
            collector.reg_use(rm);
        }
        &Inst::Extend { rd, rn, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::Call { link, ref info } => {
            collector.reg_def(link);
            collector.reg_uses(&*info.uses);
            collector.reg_defs(&*info.defs);
        }
        &Inst::CallInd { link, ref info } => {
            collector.reg_def(link);
            collector.reg_use(info.rn);
            collector.reg_uses(&*info.uses);
            collector.reg_defs(&*info.defs);
        }
        &Inst::Ret { link, ref rets } => {
            collector.reg_use(link);
            collector.reg_uses(&rets[..]);
        }
        &Inst::Jump { .. } | &Inst::EpiloguePlaceholder => {}
        &Inst::IndirectBr { rn, .. } => {
            collector.reg_use(rn);
        }
        &Inst::CondBr { .. } | &Inst::OneWayCondBr { .. } => {}
        &Inst::Nop0 | Inst::Nop2 => {}
        &Inst::Debugtrap => {}
        &Inst::Trap { .. } => {}
        &Inst::TrapIf { .. } => {}
        &Inst::JTSequence { ridx, .. } => {
            collector.reg_use(ridx);
        }
        &Inst::LoadExtNameFar { rd, .. } => {
            collector.reg_def(rd);
        }
        &Inst::LoadAddr { rd, ref mem } => {
            collector.reg_def(rd);
            memarg_operands(mem, collector);
        }
        &Inst::Loop { ref body, .. } => {
            for inst in body.iter() {
                s390x_get_operands(inst, collector);
            }
        }
        &Inst::CondBreak { .. } => {}
        &Inst::VirtualSPOffsetAdj { .. } => {}
        &Inst::Unwind { .. } => {}
        &Inst::DummyUse { reg } => {
            collector.reg_use(reg);
        }
    }
}

//=============================================================================
// Instructions: misc functions and external interface

impl MachInst for Inst {
    type LabelUse = LabelUse;

    fn get_operands<F: Fn(VReg) -> VReg>(&self, collector: &mut OperandCollector<'_, F>) {
        s390x_get_operands(self, collector);
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        match self {
            &Inst::Mov32 { rd, rm } => Some((rd, rm)),
            &Inst::Mov64 { rd, rm } => Some((rd, rm)),
            &Inst::FpuMove32 { rd, rn } => Some((rd, rn)),
            &Inst::FpuMove64 { rd, rn } => Some((rd, rn)),
            _ => None,
        }
    }

    fn is_epilogue_placeholder(&self) -> bool {
        if let Inst::EpiloguePlaceholder = self {
            true
        } else {
            false
        }
    }

    fn is_term<'a>(&'a self) -> MachTerminator<'a> {
        match self {
            &Inst::Ret { .. } | &Inst::EpiloguePlaceholder => MachTerminator::Ret,
            &Inst::Jump { dest } => MachTerminator::Uncond(dest),
            &Inst::CondBr {
                taken, not_taken, ..
            } => MachTerminator::Cond(taken, not_taken),
            &Inst::OneWayCondBr { .. } => {
                // Explicitly invisible to CFG processing.
                MachTerminator::None
            }
            &Inst::IndirectBr { ref targets, .. } => MachTerminator::Indirect(&targets[..]),
            &Inst::JTSequence { ref targets, .. } => MachTerminator::Indirect(&targets[..]),
            _ => MachTerminator::None,
        }
    }

    fn is_safepoint(&self) -> bool {
        match self {
            &Inst::Call { .. }
            | &Inst::CallInd { .. }
            | &Inst::Trap { .. }
            | Inst::TrapIf { .. }
            | &Inst::CmpTrapRR { .. }
            | &Inst::CmpTrapRSImm16 { .. }
            | &Inst::CmpTrapRUImm16 { .. } => true,
            _ => false,
        }
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Inst {
        assert!(ty.bits() <= 64);
        if ty.bits() <= 32 {
            Inst::mov32(to_reg, from_reg)
        } else {
            Inst::mov64(to_reg, from_reg)
        }
    }

    fn gen_constant<F: FnMut(Type) -> Writable<Reg>>(
        to_regs: ValueRegs<Writable<Reg>>,
        value: u128,
        ty: Type,
        _alloc_tmp: F,
    ) -> SmallVec<[Inst; 4]> {
        let to_reg = to_regs
            .only_reg()
            .expect("multi-reg values not supported yet");
        let value = value as u64;
        match ty {
            types::F64 => {
                let mut ret = SmallVec::new();
                ret.push(Inst::load_fp_constant64(to_reg, f64::from_bits(value)));
                ret
            }
            types::F32 => {
                let mut ret = SmallVec::new();
                ret.push(Inst::load_fp_constant32(
                    to_reg,
                    f32::from_bits(value as u32),
                ));
                ret
            }
            types::I64 | types::B64 | types::R64 => Inst::load_constant64(to_reg, value),
            types::B1
            | types::I8
            | types::B8
            | types::I16
            | types::B16
            | types::I32
            | types::B32 => Inst::load_constant32(to_reg, value as u32),
            _ => unreachable!(),
        }
    }

    fn gen_nop(preferred_size: usize) -> Inst {
        if preferred_size == 0 {
            Inst::Nop0
        } else {
            // We can't give a NOP (or any insn) < 2 bytes.
            assert!(preferred_size >= 2);
            Inst::Nop2
        }
    }

    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> {
        match ty {
            types::I8 => Ok((&[RegClass::Int], &[types::I8])),
            types::I16 => Ok((&[RegClass::Int], &[types::I16])),
            types::I32 => Ok((&[RegClass::Int], &[types::I32])),
            types::I64 => Ok((&[RegClass::Int], &[types::I64])),
            types::B1 => Ok((&[RegClass::Int], &[types::B1])),
            types::B8 => Ok((&[RegClass::Int], &[types::B8])),
            types::B16 => Ok((&[RegClass::Int], &[types::B16])),
            types::B32 => Ok((&[RegClass::Int], &[types::B32])),
            types::B64 => Ok((&[RegClass::Int], &[types::B64])),
            types::R32 => panic!("32-bit reftype pointer should never be seen on s390x"),
            types::R64 => Ok((&[RegClass::Int], &[types::R64])),
            types::F32 => Ok((&[RegClass::Float], &[types::F32])),
            types::F64 => Ok((&[RegClass::Float], &[types::F64])),
            types::I128 => Ok((&[RegClass::Int, RegClass::Int], &[types::I64, types::I64])),
            types::B128 => Ok((&[RegClass::Int, RegClass::Int], &[types::B64, types::B64])),
            // FIXME: We don't really have IFLAGS, but need to allow it here
            // for now to support the SelectifSpectreGuard instruction.
            types::IFLAGS => Ok((&[RegClass::Int], &[types::I64])),
            _ => Err(CodegenError::Unsupported(format!(
                "Unexpected SSA-value type: {}",
                ty
            ))),
        }
    }

    fn canonical_type_for_rc(rc: RegClass) -> Type {
        match rc {
            RegClass::Int => types::I64,
            RegClass::Float => types::F64,
        }
    }

    fn gen_jump(target: MachLabel) -> Inst {
        Inst::Jump { dest: target }
    }

    fn worst_case_size() -> CodeOffset {
        // The maximum size, in bytes, of any `Inst`'s emitted code. We have at least one case of
        // an 8-instruction sequence (saturating int-to-float conversions) with three embedded
        // 64-bit f64 constants.
        //
        // Note that inline jump-tables handle island/pool insertion separately, so we do not need
        // to account for them here (otherwise the worst case would be 2^31 * 4, clearly not
        // feasible for other reasons).
        44
    }

    fn ref_type_regclass(_: &settings::Flags) -> RegClass {
        RegClass::Int
    }

    fn gen_dummy_use(reg: Reg) -> Inst {
        Inst::DummyUse { reg }
    }
}

//=============================================================================
// Pretty-printing of instructions.

fn mem_finalize_for_show(
    mem: &MemArg,
    state: &EmitState,
    have_d12: bool,
    have_d20: bool,
    have_pcrel: bool,
    have_index: bool,
) -> (String, MemArg) {
    let (mem_insts, mem) = mem_finalize(mem, state, have_d12, have_d20, have_pcrel, have_index);
    let mut mem_str = mem_insts
        .into_iter()
        .map(|inst| {
            inst.print_with_state(&mut EmitState::default(), &mut AllocationConsumer::new(&[]))
        })
        .collect::<Vec<_>>()
        .join(" ; ");
    if !mem_str.is_empty() {
        mem_str += " ; ";
    }

    (mem_str, mem)
}

impl Inst {
    fn print_with_state(
        &self,
        state: &mut EmitState,
        allocs: &mut AllocationConsumer<'_>,
    ) -> String {
        // N.B.: order of consumption of `allocs` must match the order
        // in `s390x_get_operands()`.

        let mut empty_allocs = AllocationConsumer::new(&[]);

        match self {
            &Inst::Nop0 => "nop-zero-len".to_string(),
            &Inst::Nop2 => "nop".to_string(),
            &Inst::AluRRR { alu_op, rd, rn, rm } => {
                let rd = allocs.next_writable(rd);
                let rn = allocs.next(rn);
                let rm = allocs.next(rm);

                let (op, have_rr) = match alu_op {
                    ALUOp::Add32 => ("ark", true),
                    ALUOp::Add64 => ("agrk", true),
                    ALUOp::AddLogical32 => ("alrk", true),
                    ALUOp::AddLogical64 => ("algrk", true),
                    ALUOp::Sub32 => ("srk", true),
                    ALUOp::Sub64 => ("sgrk", true),
                    ALUOp::SubLogical32 => ("slrk", true),
                    ALUOp::SubLogical64 => ("slgrk", true),
                    ALUOp::Mul32 => ("msrkc", true),
                    ALUOp::Mul64 => ("msgrkc", true),
                    ALUOp::And32 => ("nrk", true),
                    ALUOp::And64 => ("ngrk", true),
                    ALUOp::Orr32 => ("ork", true),
                    ALUOp::Orr64 => ("ogrk", true),
                    ALUOp::Xor32 => ("xrk", true),
                    ALUOp::Xor64 => ("xgrk", true),
                    ALUOp::AndNot32 => ("nnrk", false),
                    ALUOp::AndNot64 => ("nngrk", false),
                    ALUOp::OrrNot32 => ("nork", false),
                    ALUOp::OrrNot64 => ("nogrk", false),
                    ALUOp::XorNot32 => ("nxrk", false),
                    ALUOp::XorNot64 => ("nxgrk", false),
                    _ => unreachable!(),
                };
                if have_rr && rd.to_reg() == rn {
                    let inst = Inst::AluRR { alu_op, rd, rm };
                    return inst.print_with_state(state, &mut empty_allocs);
                }
                let rd = pretty_print_reg(rd.to_reg(), &mut empty_allocs);
                let rn = pretty_print_reg(rn, &mut empty_allocs);
                let rm = pretty_print_reg(rm, &mut empty_allocs);
                format!("{} {}, {}, {}", op, rd, rn, rm)
            }
            &Inst::AluRRSImm16 {
                alu_op,
                rd,
                rn,
                imm,
            } => {
                let rd = allocs.next_writable(rd);
                let rn = allocs.next(rn);

                if rd.to_reg() == rn {
                    let inst = Inst::AluRSImm16 { alu_op, rd, imm };
                    return inst.print_with_state(state, &mut empty_allocs);
                }
                let op = match alu_op {
                    ALUOp::Add32 => "ahik",
                    ALUOp::Add64 => "aghik",
                    _ => unreachable!(),
                };
                let rd = pretty_print_reg(rd.to_reg(), &mut empty_allocs);
                let rn = pretty_print_reg(rn, &mut empty_allocs);
                format!("{} {}, {}, {}", op, rd, rn, imm)
            }
            &Inst::AluRR { alu_op, rd, rm } => {
                let op = match alu_op {
                    ALUOp::Add32 => "ar",
                    ALUOp::Add64 => "agr",
                    ALUOp::Add64Ext32 => "agfr",
                    ALUOp::AddLogical32 => "alr",
                    ALUOp::AddLogical64 => "algr",
                    ALUOp::AddLogical64Ext32 => "algfr",
                    ALUOp::Sub32 => "sr",
                    ALUOp::Sub64 => "sgr",
                    ALUOp::Sub64Ext32 => "sgfr",
                    ALUOp::SubLogical32 => "slr",
                    ALUOp::SubLogical64 => "slgr",
                    ALUOp::SubLogical64Ext32 => "slgfr",
                    ALUOp::Mul32 => "msr",
                    ALUOp::Mul64 => "msgr",
                    ALUOp::Mul64Ext32 => "msgfr",
                    ALUOp::And32 => "nr",
                    ALUOp::And64 => "ngr",
                    ALUOp::Orr32 => "or",
                    ALUOp::Orr64 => "ogr",
                    ALUOp::Xor32 => "xr",
                    ALUOp::Xor64 => "xgr",
                    _ => unreachable!(),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("{} {}, {}", op, rd, rm)
            }
            &Inst::AluRX {
                alu_op,
                rd,
                ref mem,
            } => {
                let (opcode_rx, opcode_rxy) = match alu_op {
                    ALUOp::Add32 => (Some("a"), Some("ay")),
                    ALUOp::Add32Ext16 => (Some("ah"), Some("ahy")),
                    ALUOp::Add64 => (None, Some("ag")),
                    ALUOp::Add64Ext16 => (None, Some("agh")),
                    ALUOp::Add64Ext32 => (None, Some("agf")),
                    ALUOp::AddLogical32 => (Some("al"), Some("aly")),
                    ALUOp::AddLogical64 => (None, Some("alg")),
                    ALUOp::AddLogical64Ext32 => (None, Some("algf")),
                    ALUOp::Sub32 => (Some("s"), Some("sy")),
                    ALUOp::Sub32Ext16 => (Some("sh"), Some("shy")),
                    ALUOp::Sub64 => (None, Some("sg")),
                    ALUOp::Sub64Ext16 => (None, Some("sgh")),
                    ALUOp::Sub64Ext32 => (None, Some("sgf")),
                    ALUOp::SubLogical32 => (Some("sl"), Some("sly")),
                    ALUOp::SubLogical64 => (None, Some("slg")),
                    ALUOp::SubLogical64Ext32 => (None, Some("slgf")),
                    ALUOp::Mul32 => (Some("ms"), Some("msy")),
                    ALUOp::Mul32Ext16 => (Some("mh"), Some("mhy")),
                    ALUOp::Mul64 => (None, Some("msg")),
                    ALUOp::Mul64Ext16 => (None, Some("mgh")),
                    ALUOp::Mul64Ext32 => (None, Some("msgf")),
                    ALUOp::And32 => (Some("n"), Some("ny")),
                    ALUOp::And64 => (None, Some("ng")),
                    ALUOp::Orr32 => (Some("o"), Some("oy")),
                    ALUOp::Orr64 => (None, Some("og")),
                    ALUOp::Xor32 => (Some("x"), Some("xy")),
                    ALUOp::Xor64 => (None, Some("xg")),
                    _ => unreachable!(),
                };

                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(
                    &mem,
                    state,
                    opcode_rx.is_some(),
                    opcode_rxy.is_some(),
                    false,
                    true,
                );
                let op = match &mem {
                    &MemArg::BXD12 { .. } => opcode_rx,
                    &MemArg::BXD20 { .. } => opcode_rxy,
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();

                format!("{}{} {}, {}", mem_str, op.unwrap(), rd, mem)
            }
            &Inst::AluRSImm16 { alu_op, rd, imm } => {
                let op = match alu_op {
                    ALUOp::Add32 => "ahi",
                    ALUOp::Add64 => "aghi",
                    ALUOp::Mul32 => "mhi",
                    ALUOp::Mul64 => "mghi",
                    _ => unreachable!(),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                format!("{} {}, {}", op, rd, imm)
            }
            &Inst::AluRSImm32 { alu_op, rd, imm } => {
                let op = match alu_op {
                    ALUOp::Add32 => "afi",
                    ALUOp::Add64 => "agfi",
                    ALUOp::Mul32 => "msfi",
                    ALUOp::Mul64 => "msgfi",
                    _ => unreachable!(),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                format!("{} {}, {}", op, rd, imm)
            }
            &Inst::AluRUImm32 { alu_op, rd, imm } => {
                let op = match alu_op {
                    ALUOp::AddLogical32 => "alfi",
                    ALUOp::AddLogical64 => "algfi",
                    ALUOp::SubLogical32 => "slfi",
                    ALUOp::SubLogical64 => "slgfi",
                    _ => unreachable!(),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                format!("{} {}, {}", op, rd, imm)
            }
            &Inst::AluRUImm16Shifted { alu_op, rd, imm } => {
                let op = match (alu_op, imm.shift) {
                    (ALUOp::And32, 0) => "nill",
                    (ALUOp::And32, 1) => "nilh",
                    (ALUOp::And64, 0) => "nill",
                    (ALUOp::And64, 1) => "nilh",
                    (ALUOp::And64, 2) => "nihl",
                    (ALUOp::And64, 3) => "nihh",
                    (ALUOp::Orr32, 0) => "oill",
                    (ALUOp::Orr32, 1) => "oilh",
                    (ALUOp::Orr64, 0) => "oill",
                    (ALUOp::Orr64, 1) => "oilh",
                    (ALUOp::Orr64, 2) => "oihl",
                    (ALUOp::Orr64, 3) => "oihh",
                    _ => unreachable!(),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                format!("{} {}, {}", op, rd, imm.bits)
            }
            &Inst::AluRUImm32Shifted { alu_op, rd, imm } => {
                let op = match (alu_op, imm.shift) {
                    (ALUOp::And32, 0) => "nilf",
                    (ALUOp::And64, 0) => "nilf",
                    (ALUOp::And64, 1) => "nihf",
                    (ALUOp::Orr32, 0) => "oilf",
                    (ALUOp::Orr64, 0) => "oilf",
                    (ALUOp::Orr64, 1) => "oihf",
                    (ALUOp::Xor32, 0) => "xilf",
                    (ALUOp::Xor64, 0) => "xilf",
                    (ALUOp::Xor64, 1) => "xihf",
                    _ => unreachable!(),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                format!("{} {}, {}", op, rd, imm.bits)
            }
            &Inst::SMulWide { rn, rm } => {
                let op = "mgrk";
                let rn = pretty_print_reg(rn, allocs);
                let rm = pretty_print_reg(rm, allocs);
                let rd = pretty_print_reg(gpr(0), allocs);
                let _r1 = allocs.next(gpr(1));
                format!("{} {}, {}, {}", op, rd, rn, rm)
            }
            &Inst::UMulWide { rn } => {
                let op = "mlgr";
                let rn = pretty_print_reg(rn, allocs);
                let rd = pretty_print_reg(gpr(0), allocs);
                let _r1 = allocs.next(gpr(1));
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::SDivMod32 { rn, .. } => {
                let op = "dsgfr";
                let rn = pretty_print_reg(rn, allocs);
                let rd = pretty_print_reg(gpr(0), allocs);
                let _r1 = allocs.next(gpr(1));
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::SDivMod64 { rn, .. } => {
                let op = "dsgr";
                let rn = pretty_print_reg(rn, allocs);
                let rd = pretty_print_reg(gpr(0), allocs);
                let _r1 = allocs.next(gpr(1));
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::UDivMod32 { rn, .. } => {
                let op = "dlr";
                let rn = pretty_print_reg(rn, allocs);
                let rd = pretty_print_reg(gpr(0), allocs);
                let _r1 = allocs.next(gpr(1));
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::UDivMod64 { rn, .. } => {
                let op = "dlgr";
                let rn = pretty_print_reg(rn, allocs);
                let rd = pretty_print_reg(gpr(0), allocs);
                let _r1 = allocs.next(gpr(1));
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::Flogr { rn } => {
                let op = "flogr";
                let rn = pretty_print_reg(rn, allocs);
                let rd = pretty_print_reg(gpr(0), allocs);
                let _r1 = allocs.next(gpr(1));
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::ShiftRR {
                shift_op,
                rd,
                rn,
                shift_imm,
                shift_reg,
            } => {
                let op = match shift_op {
                    ShiftOp::RotL32 => "rll",
                    ShiftOp::RotL64 => "rllg",
                    ShiftOp::LShL32 => "sllk",
                    ShiftOp::LShL64 => "sllg",
                    ShiftOp::LShR32 => "srlk",
                    ShiftOp::LShR64 => "srlg",
                    ShiftOp::AShR32 => "srak",
                    ShiftOp::AShR64 => "srag",
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                let shift_reg = if shift_reg != zero_reg() {
                    format!("({})", pretty_print_reg(shift_reg, allocs))
                } else {
                    "".to_string()
                };
                format!("{} {}, {}, {}{}", op, rd, rn, shift_imm, shift_reg)
            }
            &Inst::RxSBG {
                op,
                rd,
                rn,
                start_bit,
                end_bit,
                rotate_amt,
            } => {
                let op = match op {
                    RxSBGOp::Insert => "risbgn",
                    RxSBGOp::And => "rnsbg",
                    RxSBGOp::Or => "rosbg",
                    RxSBGOp::Xor => "rxsbg",
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!(
                    "{} {}, {}, {}, {}, {}",
                    op,
                    rd,
                    rn,
                    start_bit,
                    end_bit,
                    (rotate_amt as u8) & 63
                )
            }
            &Inst::RxSBGTest {
                op,
                rd,
                rn,
                start_bit,
                end_bit,
                rotate_amt,
            } => {
                let op = match op {
                    RxSBGOp::And => "rnsbg",
                    RxSBGOp::Or => "rosbg",
                    RxSBGOp::Xor => "rxsbg",
                    _ => unreachable!(),
                };
                let rd = pretty_print_reg(rd, allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!(
                    "{} {}, {}, {}, {}, {}",
                    op,
                    rd,
                    rn,
                    start_bit | 0x80,
                    end_bit,
                    (rotate_amt as u8) & 63
                )
            }
            &Inst::UnaryRR { op, rd, rn } => {
                let (op, extra) = match op {
                    UnaryOp::Abs32 => ("lpr", ""),
                    UnaryOp::Abs64 => ("lpgr", ""),
                    UnaryOp::Abs64Ext32 => ("lpgfr", ""),
                    UnaryOp::Neg32 => ("lcr", ""),
                    UnaryOp::Neg64 => ("lcgr", ""),
                    UnaryOp::Neg64Ext32 => ("lcgfr", ""),
                    UnaryOp::PopcntByte => ("popcnt", ""),
                    UnaryOp::PopcntReg => ("popcnt", ", 8"),
                    UnaryOp::BSwap32 => ("lrvr", ""),
                    UnaryOp::BSwap64 => ("lrvgr", ""),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!("{} {}, {}{}", op, rd, rn, extra)
            }
            &Inst::CmpRR { op, rn, rm } => {
                let op = match op {
                    CmpOp::CmpS32 => "cr",
                    CmpOp::CmpS64 => "cgr",
                    CmpOp::CmpS64Ext32 => "cgfr",
                    CmpOp::CmpL32 => "clr",
                    CmpOp::CmpL64 => "clgr",
                    CmpOp::CmpL64Ext32 => "clgfr",
                    _ => unreachable!(),
                };
                let rn = pretty_print_reg(rn, allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("{} {}, {}", op, rn, rm)
            }
            &Inst::CmpRX { op, rn, ref mem } => {
                let (opcode_rx, opcode_rxy, opcode_ril) = match op {
                    CmpOp::CmpS32 => (Some("c"), Some("cy"), Some("crl")),
                    CmpOp::CmpS32Ext16 => (Some("ch"), Some("chy"), Some("chrl")),
                    CmpOp::CmpS64 => (None, Some("cg"), Some("cgrl")),
                    CmpOp::CmpS64Ext16 => (None, Some("cgh"), Some("cghrl")),
                    CmpOp::CmpS64Ext32 => (None, Some("cgf"), Some("cgfrl")),
                    CmpOp::CmpL32 => (Some("cl"), Some("cly"), Some("clrl")),
                    CmpOp::CmpL32Ext16 => (None, None, Some("clhrl")),
                    CmpOp::CmpL64 => (None, Some("clg"), Some("clgrl")),
                    CmpOp::CmpL64Ext16 => (None, None, Some("clghrl")),
                    CmpOp::CmpL64Ext32 => (None, Some("clgf"), Some("clgfrl")),
                };

                let rn = pretty_print_reg(rn, allocs);
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(
                    &mem,
                    state,
                    opcode_rx.is_some(),
                    opcode_rxy.is_some(),
                    opcode_ril.is_some(),
                    true,
                );
                let op = match &mem {
                    &MemArg::BXD12 { .. } => opcode_rx,
                    &MemArg::BXD20 { .. } => opcode_rxy,
                    &MemArg::Label { .. } | &MemArg::Symbol { .. } => opcode_ril,
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();

                format!("{}{} {}, {}", mem_str, op.unwrap(), rn, mem)
            }
            &Inst::CmpRSImm16 { op, rn, imm } => {
                let op = match op {
                    CmpOp::CmpS32 => "chi",
                    CmpOp::CmpS64 => "cghi",
                    _ => unreachable!(),
                };
                let rn = pretty_print_reg(rn, allocs);
                format!("{} {}, {}", op, rn, imm)
            }
            &Inst::CmpRSImm32 { op, rn, imm } => {
                let op = match op {
                    CmpOp::CmpS32 => "cfi",
                    CmpOp::CmpS64 => "cgfi",
                    _ => unreachable!(),
                };
                let rn = pretty_print_reg(rn, allocs);
                format!("{} {}, {}", op, rn, imm)
            }
            &Inst::CmpRUImm32 { op, rn, imm } => {
                let op = match op {
                    CmpOp::CmpL32 => "clfi",
                    CmpOp::CmpL64 => "clgfi",
                    _ => unreachable!(),
                };
                let rn = pretty_print_reg(rn, allocs);
                format!("{} {}, {}", op, rn, imm)
            }
            &Inst::CmpTrapRR {
                op, rn, rm, cond, ..
            } => {
                let op = match op {
                    CmpOp::CmpS32 => "crt",
                    CmpOp::CmpS64 => "cgrt",
                    CmpOp::CmpL32 => "clrt",
                    CmpOp::CmpL64 => "clgrt",
                    _ => unreachable!(),
                };
                let rn = pretty_print_reg(rn, allocs);
                let rm = pretty_print_reg(rm, allocs);
                let cond = cond.pretty_print_default();
                format!("{}{} {}, {}", op, cond, rn, rm)
            }
            &Inst::CmpTrapRSImm16 {
                op, rn, imm, cond, ..
            } => {
                let op = match op {
                    CmpOp::CmpS32 => "cit",
                    CmpOp::CmpS64 => "cgit",
                    _ => unreachable!(),
                };
                let rn = pretty_print_reg(rn, allocs);
                let cond = cond.pretty_print_default();
                format!("{}{} {}, {}", op, cond, rn, imm)
            }
            &Inst::CmpTrapRUImm16 {
                op, rn, imm, cond, ..
            } => {
                let op = match op {
                    CmpOp::CmpL32 => "clfit",
                    CmpOp::CmpL64 => "clgit",
                    _ => unreachable!(),
                };
                let rn = pretty_print_reg(rn, allocs);
                let cond = cond.pretty_print_default();
                format!("{}{} {}, {}", op, cond, rn, imm)
            }
            &Inst::AtomicRmw {
                alu_op,
                rd,
                rn,
                ref mem,
            } => {
                let op = match alu_op {
                    ALUOp::Add32 => "laa",
                    ALUOp::Add64 => "laag",
                    ALUOp::AddLogical32 => "laal",
                    ALUOp::AddLogical64 => "laalg",
                    ALUOp::And32 => "lan",
                    ALUOp::And64 => "lang",
                    ALUOp::Orr32 => "lao",
                    ALUOp::Orr64 => "laog",
                    ALUOp::Xor32 => "lax",
                    ALUOp::Xor64 => "laxg",
                    _ => unreachable!(),
                };

                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(&mem, state, false, true, false, false);
                let mem = mem.pretty_print_default();
                format!("{}{} {}, {}, {}", mem_str, op, rd, rn, mem)
            }
            &Inst::AtomicCas32 { rd, rn, ref mem } | &Inst::AtomicCas64 { rd, rn, ref mem } => {
                let (opcode_rs, opcode_rsy) = match self {
                    &Inst::AtomicCas32 { .. } => (Some("cs"), Some("csy")),
                    &Inst::AtomicCas64 { .. } => (None, Some("csg")),
                    _ => unreachable!(),
                };

                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(
                    &mem,
                    state,
                    opcode_rs.is_some(),
                    opcode_rsy.is_some(),
                    false,
                    false,
                );
                let op = match &mem {
                    &MemArg::BXD12 { .. } => opcode_rs,
                    &MemArg::BXD20 { .. } => opcode_rsy,
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();

                format!("{}{} {}, {}, {}", mem_str, op.unwrap(), rd, rn, mem)
            }
            &Inst::Fence => "bcr 14, 0".to_string(),
            &Inst::Load32 { rd, ref mem }
            | &Inst::Load32ZExt8 { rd, ref mem }
            | &Inst::Load32SExt8 { rd, ref mem }
            | &Inst::Load32ZExt16 { rd, ref mem }
            | &Inst::Load32SExt16 { rd, ref mem }
            | &Inst::Load64 { rd, ref mem }
            | &Inst::Load64ZExt8 { rd, ref mem }
            | &Inst::Load64SExt8 { rd, ref mem }
            | &Inst::Load64ZExt16 { rd, ref mem }
            | &Inst::Load64SExt16 { rd, ref mem }
            | &Inst::Load64ZExt32 { rd, ref mem }
            | &Inst::Load64SExt32 { rd, ref mem }
            | &Inst::LoadRev16 { rd, ref mem }
            | &Inst::LoadRev32 { rd, ref mem }
            | &Inst::LoadRev64 { rd, ref mem }
            | &Inst::FpuLoad32 { rd, ref mem }
            | &Inst::FpuLoad64 { rd, ref mem } => {
                let (opcode_rx, opcode_rxy, opcode_ril) = match self {
                    &Inst::Load32 { .. } => (Some("l"), Some("ly"), Some("lrl")),
                    &Inst::Load32ZExt8 { .. } => (None, Some("llc"), None),
                    &Inst::Load32SExt8 { .. } => (None, Some("lb"), None),
                    &Inst::Load32ZExt16 { .. } => (None, Some("llh"), Some("llhrl")),
                    &Inst::Load32SExt16 { .. } => (Some("lh"), Some("lhy"), Some("lhrl")),
                    &Inst::Load64 { .. } => (None, Some("lg"), Some("lgrl")),
                    &Inst::Load64ZExt8 { .. } => (None, Some("llgc"), None),
                    &Inst::Load64SExt8 { .. } => (None, Some("lgb"), None),
                    &Inst::Load64ZExt16 { .. } => (None, Some("llgh"), Some("llghrl")),
                    &Inst::Load64SExt16 { .. } => (None, Some("lgh"), Some("lghrl")),
                    &Inst::Load64ZExt32 { .. } => (None, Some("llgf"), Some("llgfrl")),
                    &Inst::Load64SExt32 { .. } => (None, Some("lgf"), Some("lgfrl")),
                    &Inst::LoadRev16 { .. } => (None, Some("lrvh"), None),
                    &Inst::LoadRev32 { .. } => (None, Some("lrv"), None),
                    &Inst::LoadRev64 { .. } => (None, Some("lrvg"), None),
                    &Inst::FpuLoad32 { .. } => (Some("le"), Some("ley"), None),
                    &Inst::FpuLoad64 { .. } => (Some("ld"), Some("ldy"), None),
                    _ => unreachable!(),
                };

                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(
                    &mem,
                    state,
                    opcode_rx.is_some(),
                    opcode_rxy.is_some(),
                    opcode_ril.is_some(),
                    true,
                );
                let op = match &mem {
                    &MemArg::BXD12 { .. } => opcode_rx,
                    &MemArg::BXD20 { .. } => opcode_rxy,
                    &MemArg::Label { .. } | &MemArg::Symbol { .. } => opcode_ril,
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();
                format!("{}{} {}, {}", mem_str, op.unwrap(), rd, mem)
            }
            &Inst::FpuLoadRev32 { rd, ref mem } | &Inst::FpuLoadRev64 { rd, ref mem } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(&mem, state, true, false, false, true);
                let op = match self {
                    &Inst::FpuLoadRev32 { .. } => "vlebrf",
                    &Inst::FpuLoadRev64 { .. } => "vlebrg",
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();
                format!("{}{} {}, {}, 0", mem_str, op, rd, mem)
            }
            &Inst::Store8 { rd, ref mem }
            | &Inst::Store16 { rd, ref mem }
            | &Inst::Store32 { rd, ref mem }
            | &Inst::Store64 { rd, ref mem }
            | &Inst::StoreRev16 { rd, ref mem }
            | &Inst::StoreRev32 { rd, ref mem }
            | &Inst::StoreRev64 { rd, ref mem }
            | &Inst::FpuStore32 { rd, ref mem }
            | &Inst::FpuStore64 { rd, ref mem } => {
                let (opcode_rx, opcode_rxy, opcode_ril) = match self {
                    &Inst::Store8 { .. } => (Some("stc"), Some("stcy"), None),
                    &Inst::Store16 { .. } => (Some("sth"), Some("sthy"), Some("sthrl")),
                    &Inst::Store32 { .. } => (Some("st"), Some("sty"), Some("strl")),
                    &Inst::Store64 { .. } => (None, Some("stg"), Some("stgrl")),
                    &Inst::StoreRev16 { .. } => (None, Some("strvh"), None),
                    &Inst::StoreRev32 { .. } => (None, Some("strv"), None),
                    &Inst::StoreRev64 { .. } => (None, Some("strvg"), None),
                    &Inst::FpuStore32 { .. } => (Some("ste"), Some("stey"), None),
                    &Inst::FpuStore64 { .. } => (Some("std"), Some("stdy"), None),
                    _ => unreachable!(),
                };

                let rd = pretty_print_reg(rd, allocs);
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(
                    &mem,
                    state,
                    opcode_rx.is_some(),
                    opcode_rxy.is_some(),
                    opcode_ril.is_some(),
                    true,
                );
                let op = match &mem {
                    &MemArg::BXD12 { .. } => opcode_rx,
                    &MemArg::BXD20 { .. } => opcode_rxy,
                    &MemArg::Label { .. } | &MemArg::Symbol { .. } => opcode_ril,
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();

                format!("{}{} {}, {}", mem_str, op.unwrap(), rd, mem)
            }
            &Inst::StoreImm8 { imm, ref mem } => {
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(&mem, state, true, true, false, false);
                let op = match &mem {
                    &MemArg::BXD12 { .. } => "mvi",
                    &MemArg::BXD20 { .. } => "mviy",
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();

                format!("{}{} {}, {}", mem_str, op, mem, imm)
            }
            &Inst::StoreImm16 { imm, ref mem }
            | &Inst::StoreImm32SExt16 { imm, ref mem }
            | &Inst::StoreImm64SExt16 { imm, ref mem } => {
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(&mem, state, false, true, false, false);
                let op = match self {
                    &Inst::StoreImm16 { .. } => "mvhhi",
                    &Inst::StoreImm32SExt16 { .. } => "mvhi",
                    &Inst::StoreImm64SExt16 { .. } => "mvghi",
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();

                format!("{}{} {}, {}", mem_str, op, mem, imm)
            }
            &Inst::FpuStoreRev32 { rd, ref mem } | &Inst::FpuStoreRev64 { rd, ref mem } => {
                let rd = pretty_print_reg(rd, allocs);
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(&mem, state, true, false, false, true);
                let op = match self {
                    &Inst::FpuStoreRev32 { .. } => "vstebrf",
                    &Inst::FpuStoreRev64 { .. } => "vstebrg",
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();

                format!("{}{} {}, {}, 0", mem_str, op, rd, mem)
            }
            &Inst::LoadMultiple64 { rt, rt2, ref mem } => {
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(&mem, state, false, true, false, false);
                let rt = pretty_print_reg(rt.to_reg(), &mut empty_allocs);
                let rt2 = pretty_print_reg(rt2.to_reg(), &mut empty_allocs);
                let mem = mem.pretty_print_default();
                format!("{}lmg {}, {}, {}", mem_str, rt, rt2, mem)
            }
            &Inst::StoreMultiple64 { rt, rt2, ref mem } => {
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(&mem, state, false, true, false, false);
                let rt = pretty_print_reg(rt, &mut empty_allocs);
                let rt2 = pretty_print_reg(rt2, &mut empty_allocs);
                let mem = mem.pretty_print_default();
                format!("{}stmg {}, {}, {}", mem_str, rt, rt2, mem)
            }
            &Inst::Mov64 { rd, rm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("lgr {}, {}", rd, rm)
            }
            &Inst::Mov32 { rd, rm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("lr {}, {}", rd, rm)
            }
            &Inst::Mov32Imm { rd, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                format!("iilf {}, {}", rd, imm)
            }
            &Inst::Mov32SImm16 { rd, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                format!("lhi {}, {}", rd, imm)
            }
            &Inst::Mov64SImm16 { rd, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                format!("lghi {}, {}", rd, imm)
            }
            &Inst::Mov64SImm32 { rd, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                format!("lgfi {}, {}", rd, imm)
            }
            &Inst::Mov64UImm16Shifted { rd, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let op = match imm.shift {
                    0 => "llill",
                    1 => "llilh",
                    2 => "llihl",
                    3 => "llihh",
                    _ => unreachable!(),
                };
                format!("{} {}, {}", op, rd, imm.bits)
            }
            &Inst::Mov64UImm32Shifted { rd, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let op = match imm.shift {
                    0 => "llilf",
                    1 => "llihf",
                    _ => unreachable!(),
                };
                format!("{} {}, {}", op, rd, imm.bits)
            }
            &Inst::Insert64UImm16Shifted { rd, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let op = match imm.shift {
                    0 => "iill",
                    1 => "iilh",
                    2 => "iihl",
                    3 => "iihh",
                    _ => unreachable!(),
                };
                format!("{} {}, {}", op, rd, imm.bits)
            }
            &Inst::Insert64UImm32Shifted { rd, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let op = match imm.shift {
                    0 => "iilf",
                    1 => "iihf",
                    _ => unreachable!(),
                };
                format!("{} {}, {}", op, rd, imm.bits)
            }
            &Inst::CMov32 { rd, cond, rm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rm = pretty_print_reg(rm, allocs);
                let cond = cond.pretty_print_default();
                format!("locr{} {}, {}", cond, rd, rm)
            }
            &Inst::CMov64 { rd, cond, rm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rm = pretty_print_reg(rm, allocs);
                let cond = cond.pretty_print_default();
                format!("locgr{} {}, {}", cond, rd, rm)
            }
            &Inst::CMov32SImm16 { rd, cond, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let cond = cond.pretty_print_default();
                format!("lochi{} {}, {}", cond, rd, imm)
            }
            &Inst::CMov64SImm16 { rd, cond, ref imm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let cond = cond.pretty_print_default();
                format!("locghi{} {}, {}", cond, rd, imm)
            }
            &Inst::FpuMove32 { rd, rn } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!("ler {}, {}", rd, rn)
            }
            &Inst::FpuMove64 { rd, rn } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!("ldr {}, {}", rd, rn)
            }
            &Inst::FpuCMov32 { rd, cond, rm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rm = pretty_print_reg(rm, allocs);
                let cond = cond.invert().pretty_print_default();
                format!("j{} 6 ; ler {}, {}", cond, rd, rm)
            }
            &Inst::FpuCMov64 { rd, cond, rm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rm = pretty_print_reg(rm, allocs);
                let cond = cond.invert().pretty_print_default();
                format!("j{} 6 ; ldr {}, {}", cond, rd, rm)
            }
            &Inst::MovToFpr { rd, rn } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!("ldgr {}, {}", rd, rn)
            }
            &Inst::MovFromFpr { rd, rn } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!("lgdr {}, {}", rd, rn)
            }
            &Inst::FpuRR { fpu_op, rd, rn } => {
                let op = match fpu_op {
                    FPUOp1::Abs32 => "lpebr",
                    FPUOp1::Abs64 => "lpdbr",
                    FPUOp1::Neg32 => "lcebr",
                    FPUOp1::Neg64 => "lcdbr",
                    FPUOp1::NegAbs32 => "lnebr",
                    FPUOp1::NegAbs64 => "lndbr",
                    FPUOp1::Sqrt32 => "sqebr",
                    FPUOp1::Sqrt64 => "sqdbr",
                    FPUOp1::Cvt32To64 => "ldebr",
                    FPUOp1::Cvt64To32 => "ledbr",
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::FpuRRR { fpu_op, rd, rm } => {
                let op = match fpu_op {
                    FPUOp2::Add32 => "aebr",
                    FPUOp2::Add64 => "adbr",
                    FPUOp2::Sub32 => "sebr",
                    FPUOp2::Sub64 => "sdbr",
                    FPUOp2::Mul32 => "meebr",
                    FPUOp2::Mul64 => "mdbr",
                    FPUOp2::Div32 => "debr",
                    FPUOp2::Div64 => "ddbr",
                    _ => unimplemented!(),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("{} {}, {}", op, rd, rm)
            }
            &Inst::FpuRRRR { fpu_op, rd, rn, rm } => {
                let op = match fpu_op {
                    FPUOp3::MAdd32 => "maebr",
                    FPUOp3::MAdd64 => "madbr",
                    FPUOp3::MSub32 => "msebr",
                    FPUOp3::MSub64 => "msdbr",
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("{} {}, {}, {}", op, rd, rn, rm)
            }
            &Inst::FpuCopysign { rd, rn, rm } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("cpsdr {}, {}, {}", rd, rm, rn)
            }
            &Inst::FpuCmp32 { rn, rm } => {
                let rn = pretty_print_reg(rn, allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("cebr {}, {}", rn, rm)
            }
            &Inst::FpuCmp64 { rn, rm } => {
                let rn = pretty_print_reg(rn, allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("cdbr {}, {}", rn, rm)
            }
            &Inst::LoadFpuConst32 { rd, const_data } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let tmp = pretty_print_reg(writable_spilltmp_reg().to_reg(), &mut empty_allocs);
                format!(
                    "bras {}, 8 ; data.f32 {} ; le {}, 0({})",
                    tmp,
                    f32::from_bits(const_data),
                    rd,
                    tmp
                )
            }
            &Inst::LoadFpuConst64 { rd, const_data } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let tmp = pretty_print_reg(writable_spilltmp_reg().to_reg(), &mut empty_allocs);
                format!(
                    "bras {}, 12 ; data.f64 {} ; ld {}, 0({})",
                    tmp,
                    f64::from_bits(const_data),
                    rd,
                    tmp
                )
            }
            &Inst::FpuToInt { op, rd, rn } => {
                let op = match op {
                    FpuToIntOp::F32ToI32 => "cfebra",
                    FpuToIntOp::F32ToU32 => "clfebr",
                    FpuToIntOp::F32ToI64 => "cgebra",
                    FpuToIntOp::F32ToU64 => "clgebr",
                    FpuToIntOp::F64ToI32 => "cfdbra",
                    FpuToIntOp::F64ToU32 => "clfdbr",
                    FpuToIntOp::F64ToI64 => "cgdbra",
                    FpuToIntOp::F64ToU64 => "clgdbr",
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!("{} {}, 5, {}, 0", op, rd, rn)
            }
            &Inst::IntToFpu { op, rd, rn } => {
                let op = match op {
                    IntToFpuOp::I32ToF32 => "cefbra",
                    IntToFpuOp::U32ToF32 => "celfbr",
                    IntToFpuOp::I64ToF32 => "cegbra",
                    IntToFpuOp::U64ToF32 => "celgbr",
                    IntToFpuOp::I32ToF64 => "cdfbra",
                    IntToFpuOp::U32ToF64 => "cdlfbr",
                    IntToFpuOp::I64ToF64 => "cdgbra",
                    IntToFpuOp::U64ToF64 => "cdlgbr",
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!("{} {}, 0, {}, 0", op, rd, rn)
            }
            &Inst::FpuRound { op, rd, rn } => {
                let (op, m3) = match op {
                    FpuRoundMode::Minus32 => ("fiebr", 7),
                    FpuRoundMode::Minus64 => ("fidbr", 7),
                    FpuRoundMode::Plus32 => ("fiebr", 6),
                    FpuRoundMode::Plus64 => ("fidbr", 6),
                    FpuRoundMode::Zero32 => ("fiebr", 5),
                    FpuRoundMode::Zero64 => ("fidbr", 5),
                    FpuRoundMode::Nearest32 => ("fiebr", 4),
                    FpuRoundMode::Nearest64 => ("fidbr", 4),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                format!("{} {}, {}, {}", op, rd, rn, m3)
            }
            &Inst::FpuVecRRR { fpu_op, rd, rn, rm } => {
                let op = match fpu_op {
                    FPUOp2::Max32 => "wfmaxsb",
                    FPUOp2::Max64 => "wfmaxdb",
                    FPUOp2::Min32 => "wfminsb",
                    FPUOp2::Min64 => "wfmindb",
                    _ => unimplemented!(),
                };
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                let rm = pretty_print_reg(rm, allocs);
                format!("{} {}, {}, {}, 1", op, rd, rn, rm)
            }
            &Inst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                to_bits,
            } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let rn = pretty_print_reg(rn, allocs);
                let op = match (signed, from_bits, to_bits) {
                    (_, 1, 32) => "llcr",
                    (_, 1, 64) => "llgcr",
                    (false, 8, 32) => "llcr",
                    (false, 8, 64) => "llgcr",
                    (true, 8, 32) => "lbr",
                    (true, 8, 64) => "lgbr",
                    (false, 16, 32) => "llhr",
                    (false, 16, 64) => "llghr",
                    (true, 16, 32) => "lhr",
                    (true, 16, 64) => "lghr",
                    (false, 32, 64) => "llgfr",
                    (true, 32, 64) => "lgfr",
                    _ => panic!("Unsupported Extend case: {:?}", self),
                };
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::Call { link, ref info, .. } => {
                let link = pretty_print_reg(link.to_reg(), allocs);
                format!("brasl {}, {}", link, info.dest)
            }
            &Inst::CallInd { link, ref info, .. } => {
                let link = pretty_print_reg(link.to_reg(), allocs);
                let rn = pretty_print_reg(info.rn, allocs);
                format!("basr {}, {}", link, rn)
            }
            &Inst::Ret { link, .. } => {
                let link = pretty_print_reg(link, allocs);
                format!("br {}", link)
            }
            &Inst::EpiloguePlaceholder => "epilogue placeholder".to_string(),
            &Inst::Jump { dest } => {
                let dest = dest.to_string();
                format!("jg {}", dest)
            }
            &Inst::IndirectBr { rn, .. } => {
                let rn = pretty_print_reg(rn, allocs);
                format!("br {}", rn)
            }
            &Inst::CondBr {
                taken,
                not_taken,
                cond,
            } => {
                let taken = taken.to_string();
                let not_taken = not_taken.to_string();
                let cond = cond.pretty_print_default();
                format!("jg{} {} ; jg {}", cond, taken, not_taken)
            }
            &Inst::OneWayCondBr { target, cond } => {
                let target = target.to_string();
                let cond = cond.pretty_print_default();
                format!("jg{} {}", cond, target)
            }
            &Inst::Debugtrap => "debugtrap".to_string(),
            &Inst::Trap { .. } => "trap".to_string(),
            &Inst::TrapIf { cond, .. } => {
                let cond = cond.invert().pretty_print_default();
                format!("j{} 6 ; trap", cond)
            }
            &Inst::JTSequence { ridx, ref targets } => {
                let ridx = pretty_print_reg(ridx, allocs);
                let rtmp = pretty_print_reg(writable_spilltmp_reg().to_reg(), &mut empty_allocs);
                // The first entry is the default target, which is not emitted
                // into the jump table, so we skip it here.  It is only in the
                // list so MachTerminator will see the potential target.
                let jt_entries: String = targets
                    .iter()
                    .skip(1)
                    .map(|label| format!(" {}", label.to_string()))
                    .collect();
                format!(
                    concat!(
                        "larl {}, 14 ; ",
                        "agf {}, 0({}, {}) ; ",
                        "br {} ; ",
                        "jt_entries{}"
                    ),
                    rtmp, rtmp, rtmp, ridx, rtmp, jt_entries,
                )
            }
            &Inst::LoadExtNameFar {
                rd,
                ref name,
                offset,
            } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let tmp = pretty_print_reg(writable_spilltmp_reg().to_reg(), &mut empty_allocs);
                format!(
                    "bras {}, 12 ; data {} + {} ; lg {}, 0({})",
                    tmp, name, offset, rd, tmp
                )
            }
            &Inst::LoadAddr { rd, ref mem } => {
                let rd = pretty_print_reg(rd.to_reg(), allocs);
                let mem = mem.with_allocs(allocs);
                let (mem_str, mem) = mem_finalize_for_show(&mem, state, true, true, true, true);
                let op = match &mem {
                    &MemArg::BXD12 { .. } => "la",
                    &MemArg::BXD20 { .. } => "lay",
                    &MemArg::Label { .. } | &MemArg::Symbol { .. } => "larl",
                    _ => unreachable!(),
                };
                let mem = mem.pretty_print_default();

                format!("{}{} {}, {}", mem_str, op, rd, mem)
            }
            &Inst::Loop { ref body, cond } => {
                let body = body
                    .into_iter()
                    .map(|inst| inst.print_with_state(state, allocs))
                    .collect::<Vec<_>>()
                    .join(" ; ");
                let cond = cond.pretty_print_default();
                format!("0: {} ; jg{} 0b ; 1:", body, cond)
            }
            &Inst::CondBreak { cond } => {
                let cond = cond.pretty_print_default();
                format!("jg{} 1f", cond)
            }
            &Inst::VirtualSPOffsetAdj { offset } => {
                state.virtual_sp_offset += offset;
                format!("virtual_sp_offset_adjust {}", offset)
            }
            &Inst::Unwind { ref inst } => {
                format!("unwind {:?}", inst)
            }
            &Inst::DummyUse { reg } => {
                let reg = pretty_print_reg(reg, allocs);
                format!("dummy_use {}", reg)
            }
        }
    }
}

//=============================================================================
// Label fixups and jump veneers.

/// Different forms of label references for different instruction formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
    /// RI-format branch.  16-bit signed offset.  PC-relative, offset is imm << 1.
    BranchRI,
    /// RIL-format branch.  32-bit signed offset.  PC-relative, offset is imm << 1.
    BranchRIL,
    /// 32-bit PC relative constant offset (from address of constant itself),
    /// signed. Used in jump tables.
    PCRel32,
    /// 32-bit PC relative constant offset (from address of call instruction),
    /// signed. Offset is imm << 1.  Used for call relocations.
    PCRel32Dbl,
}

impl MachInstLabelUse for LabelUse {
    /// Alignment for veneer code.
    const ALIGN: CodeOffset = 2;

    /// Maximum PC-relative range (positive), inclusive.
    fn max_pos_range(self) -> CodeOffset {
        match self {
            // 16-bit signed immediate, left-shifted by 1.
            LabelUse::BranchRI => ((1 << 15) - 1) << 1,
            // 32-bit signed immediate, left-shifted by 1.
            LabelUse::BranchRIL => 0xffff_fffe,
            // 32-bit signed immediate.
            LabelUse::PCRel32 => 0x7fff_ffff,
            // 32-bit signed immediate, left-shifted by 1, offset by 2.
            LabelUse::PCRel32Dbl => 0xffff_fffc,
        }
    }

    /// Maximum PC-relative range (negative).
    fn max_neg_range(self) -> CodeOffset {
        match self {
            // 16-bit signed immediate, left-shifted by 1.
            LabelUse::BranchRI => (1 << 15) << 1,
            // 32-bit signed immediate, left-shifted by 1.
            // NOTE: This should be 4GB, but CodeOffset is only u32.
            LabelUse::BranchRIL => 0xffff_ffff,
            // 32-bit signed immediate.
            LabelUse::PCRel32 => 0x8000_0000,
            // 32-bit signed immediate, left-shifted by 1, offset by 2.
            // NOTE: This should be 4GB + 2, but CodeOffset is only u32.
            LabelUse::PCRel32Dbl => 0xffff_ffff,
        }
    }

    /// Size of window into code needed to do the patch.
    fn patch_size(self) -> CodeOffset {
        match self {
            LabelUse::BranchRI => 4,
            LabelUse::BranchRIL => 6,
            LabelUse::PCRel32 => 4,
            LabelUse::PCRel32Dbl => 4,
        }
    }

    /// Perform the patch.
    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        let pc_rel = (label_offset as i64) - (use_offset as i64);
        debug_assert!(pc_rel <= self.max_pos_range() as i64);
        debug_assert!(pc_rel >= -(self.max_neg_range() as i64));
        debug_assert!(pc_rel & 1 == 0);
        let pc_rel_shifted = pc_rel >> 1;

        match self {
            LabelUse::BranchRI => {
                buffer[2..4].clone_from_slice(&u16::to_be_bytes(pc_rel_shifted as u16));
            }
            LabelUse::BranchRIL => {
                buffer[2..6].clone_from_slice(&u32::to_be_bytes(pc_rel_shifted as u32));
            }
            LabelUse::PCRel32 => {
                let insn_word = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                let insn_word = insn_word.wrapping_add(pc_rel as u32);
                buffer[0..4].clone_from_slice(&u32::to_be_bytes(insn_word));
            }
            LabelUse::PCRel32Dbl => {
                let insn_word = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                let insn_word = insn_word.wrapping_add((pc_rel_shifted + 1) as u32);
                buffer[0..4].clone_from_slice(&u32::to_be_bytes(insn_word));
            }
        }
    }

    /// Is a veneer supported for this label reference type?
    fn supports_veneer(self) -> bool {
        false
    }

    /// How large is the veneer, if supported?
    fn veneer_size(self) -> CodeOffset {
        0
    }

    /// Generate a veneer into the buffer, given that this veneer is at `veneer_offset`, and return
    /// an offset and label-use for the veneer's use of the original label.
    fn generate_veneer(
        self,
        _buffer: &mut [u8],
        _veneer_offset: CodeOffset,
    ) -> (CodeOffset, LabelUse) {
        unreachable!();
    }

    fn from_reloc(reloc: Reloc, addend: Addend) -> Option<Self> {
        match (reloc, addend) {
            (Reloc::S390xPCRel32Dbl, 2) => Some(LabelUse::PCRel32Dbl),
            _ => None,
        }
    }
}
