//! This module defines arm32 (AArch32 / A32) machine instruction types.

use crate::binemit::{Addend, CodeOffset, Reloc};
use crate::ir::types::{F32, F64, I8, I16, I32, I64};
use crate::ir::{Type, types};
use crate::isa::FunctionAlignment;
use crate::isa::arm32::abi::Arm32MachineDeps;
use crate::machinst::*;
use crate::{CodegenError, CodegenResult, settings};

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;
use regalloc2::RegClass;

pub mod args;
pub use self::args::*;
pub mod regs;
pub use self::regs::*;
pub mod emit;
pub use self::emit::*;

#[cfg(test)]
mod emit_tests;

// The `Inst` type itself is generated from `inst.isle` and re-exported here so
// that the rest of the backend can refer to `Inst` variants directly.
pub use crate::isa::arm32::lower::isle::generated_code::MInst as Inst;

//=============================================================================
// Operand collection.

fn arm32_get_operands(inst: &mut Inst, collector: &mut impl OperandVisitor) {
    // Only collect virtual (allocatable) registers as operands. Physical
    // registers such as sp/fp/lr are fixed in the encoding and are not tracked
    // by the register allocator.
    fn use_if_virtual(collector: &mut impl OperandVisitor, reg: &mut Reg) {
        if reg.to_real_reg().is_none() {
            collector.reg_use(reg);
        }
    }
    fn def_if_virtual(collector: &mut impl OperandVisitor, reg: &mut Writable<Reg>) {
        if reg.to_reg().to_real_reg().is_none() {
            collector.reg_def(reg);
        }
    }

    match inst {
        Inst::Nop0
        | Inst::Nop4
        | Inst::Ret
        | Inst::AdjustSp { .. }
        | Inst::Jump { .. }
        | Inst::CondBr { .. }
        | Inst::Push { .. }
        | Inst::Pop { .. } => {}

        Inst::BrTable { index, tmp, .. } => {
            use_if_virtual(collector, index);
            if tmp.to_reg().to_real_reg().is_none() {
                collector.reg_early_def(tmp);
            }
        }

        Inst::MovImm { rd, .. }
        | Inst::MovRotImm { rd, .. }
        | Inst::MvnRotImm { rd, .. }
        | Inst::Movw { rd, .. }
        | Inst::Movt { rd, .. } => def_if_virtual(collector, rd),

        Inst::MovReg { rd, rm } | Inst::MvnReg { rd, rm } => {
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::AluRRR { rd, rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::AluRRImm { rd, rn, .. } => {
            use_if_virtual(collector, rn);
            def_if_virtual(collector, rd);
        }
        Inst::AluRRRFlags { rd, rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::ShiftImm { rd, rm, .. } => {
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::ShiftReg { rd, rm, rs, .. } => {
            use_if_virtual(collector, rm);
            use_if_virtual(collector, rs);
            def_if_virtual(collector, rd);
        }
        Inst::Mul { rd, rn, rm } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::DspMul3 { rd, rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::Mla { rd, rn, rm, ra }
        | Inst::Mls { rd, rn, rm, ra }
        | Inst::DspMul4 { rd, rn, rm, ra, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            use_if_virtual(collector, ra);
            def_if_virtual(collector, rd);
        }
        Inst::SDiv { rd, rn, rm } | Inst::UDiv { rd, rn, rm } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::BitRR { rd, rm, .. } | Inst::ExtRR { rd, rm, .. } => {
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::CSel { rd, rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            // `rd` is written before the inputs are all consumed (the default
            // move), so keep it distinct from the operands.
            if rd.to_reg().to_real_reg().is_none() {
                collector.reg_early_def(rd);
            }
        }
        Inst::Bfc { rd, .. } => def_if_virtual(collector, rd),
        Inst::Sat { rd, rm, .. } | Inst::Rrx { rd, rm } => {
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::Bfi { rd, rn, .. } | Inst::Bfx { rd, rn, .. } => {
            use_if_virtual(collector, rn);
            def_if_virtual(collector, rd);
        }
        Inst::QAlu { rd, rm, rn, .. } => {
            use_if_virtual(collector, rm);
            use_if_virtual(collector, rn);
            def_if_virtual(collector, rd);
        }
        Inst::Sel { rd, rn, rm }
        | Inst::Pkh { rd, rn, rm, .. }
        | Inst::ParAlu { rd, rn, rm, .. }
        | Inst::ExtAdd { rd, rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::Udf { .. } | Inst::TrapIf { .. } | Inst::Barrier { .. } => {}

        Inst::AtomicRmw {
            rd,
            addr,
            operand,
            tmp1,
            tmp2,
            ..
        } => {
            use_if_virtual(collector, addr);
            use_if_virtual(collector, operand);
            for d in [rd, tmp1, tmp2] {
                if d.to_reg().to_real_reg().is_none() {
                    collector.reg_early_def(d);
                }
            }
        }
        Inst::AtomicCas {
            rd,
            addr,
            expected,
            new,
            tmp,
            ..
        } => {
            use_if_virtual(collector, addr);
            use_if_virtual(collector, expected);
            use_if_virtual(collector, new);
            for d in [rd, tmp] {
                if d.to_reg().to_real_reg().is_none() {
                    collector.reg_early_def(d);
                }
            }
        }

        // The 64-bit atomics pin every operand to a fixed even/odd register
        // layout: r0/r1 (old value), r2/r3 (new/store value), r4 (addr),
        // r5/r6 (operand/expected), r7 (store-exclusive result). A32's paired
        // exclusive instructions require consecutive even/odd pairs, which
        // regalloc2 cannot express, so the pairs must be fixed physical
        // registers.
        Inst::AtomicRmw64 {
            rd_lo,
            rd_hi,
            addr,
            operand_lo,
            operand_hi,
            tmp_new_lo,
            tmp_new_hi,
            tmp_res,
            ..
        } => {
            collector.reg_fixed_use(addr, xreg(4));
            collector.reg_fixed_use(operand_lo, xreg(5));
            collector.reg_fixed_use(operand_hi, xreg(6));
            collector.reg_fixed_def(rd_lo, xreg(0));
            collector.reg_fixed_def(rd_hi, xreg(1));
            collector.reg_fixed_def(tmp_new_lo, xreg(2));
            collector.reg_fixed_def(tmp_new_hi, xreg(3));
            collector.reg_fixed_def(tmp_res, xreg(7));
        }
        Inst::AtomicCas64 {
            rd_lo,
            rd_hi,
            addr,
            expected_lo,
            expected_hi,
            new_lo,
            new_hi,
            tmp_res,
        } => {
            collector.reg_fixed_use(addr, xreg(4));
            collector.reg_fixed_use(expected_lo, xreg(5));
            collector.reg_fixed_use(expected_hi, xreg(6));
            collector.reg_fixed_use(new_lo, xreg(2));
            collector.reg_fixed_use(new_hi, xreg(3));
            collector.reg_fixed_def(rd_lo, xreg(0));
            collector.reg_fixed_def(rd_hi, xreg(1));
            collector.reg_fixed_def(tmp_res, xreg(7));
        }
        Inst::AtomicLoad64 { rd_lo, rd_hi, addr } => {
            collector.reg_fixed_use(addr, xreg(4));
            collector.reg_fixed_def(rd_lo, xreg(0));
            collector.reg_fixed_def(rd_hi, xreg(1));
        }
        Inst::AtomicStore64 {
            addr,
            src_lo,
            src_hi,
            scratch_lo,
            scratch_hi,
            tmp_res,
        } => {
            collector.reg_fixed_use(addr, xreg(4));
            collector.reg_fixed_use(src_lo, xreg(2));
            collector.reg_fixed_use(src_hi, xreg(3));
            collector.reg_fixed_def(scratch_lo, xreg(0));
            collector.reg_fixed_def(scratch_hi, xreg(1));
            collector.reg_fixed_def(tmp_res, xreg(7));
        }

        Inst::FpuRRR { rd, rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::FpuRR { rd, rm, .. } => {
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::FpuLoad { rd, mem, .. } => {
            mem.get_operands(collector);
            def_if_virtual(collector, rd);
        }
        Inst::FpuStore { rt, mem, .. } => {
            use_if_virtual(collector, rt);
            mem.get_operands(collector);
        }
        Inst::MovToFpu32 { rd, rt } => {
            use_if_virtual(collector, rt);
            def_if_virtual(collector, rd);
        }
        Inst::MovToFpu64 { rd, rt_lo, rt_hi } => {
            use_if_virtual(collector, rt_lo);
            use_if_virtual(collector, rt_hi);
            def_if_virtual(collector, rd);
        }
        Inst::MovFromFpu32 { rt, rm } => {
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rt);
        }
        Inst::MovFromFpu64 { rt_lo, rt_hi, rm } => {
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rt_lo);
            def_if_virtual(collector, rt_hi);
        }
        Inst::VcmpMrs { rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
        }
        Inst::VcvtFF { rd, rm, .. }
        | Inst::VcvtToInt { rd, rm, .. }
        | Inst::VcvtFromInt { rd, rm, .. } => {
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::FpuMinMax { rd, rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::FpuCSel { rd, rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            if rd.to_reg().to_real_reg().is_none() {
                collector.reg_early_def(rd);
            }
        }
        Inst::LdmStm { rn, .. } => use_if_virtual(collector, rn),
        Inst::LoadEx { rt, rn, .. } | Inst::LoadAcq { rt, rn, .. } => {
            use_if_virtual(collector, rn);
            def_if_virtual(collector, rt);
        }
        Inst::StoreEx { rd, rt, rn, .. } => {
            use_if_virtual(collector, rt);
            use_if_virtual(collector, rn);
            def_if_virtual(collector, rd);
        }
        Inst::StoreRel { rt, rn, .. } => {
            use_if_virtual(collector, rt);
            use_if_virtual(collector, rn);
        }
        Inst::Umull {
            rd_lo,
            rd_hi,
            rn,
            rm,
        }
        | Inst::Smull {
            rd_lo,
            rd_hi,
            rn,
            rm,
        }
        | Inst::Umlal {
            rd_lo,
            rd_hi,
            rn,
            rm,
        }
        | Inst::Smlal {
            rd_lo,
            rd_hi,
            rn,
            rm,
        }
        | Inst::DspMulL {
            rd_lo,
            rd_hi,
            rn,
            rm,
            ..
        } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
            // The two destination registers must be distinct from each other
            // and from the inputs, so use early defs. (For the accumulating
            // `umlal`/`smlal` the destinations are also inputs; those are only
            // constructed once i64 support wires them up.)
            if rd_lo.to_reg().to_real_reg().is_none() {
                collector.reg_early_def(rd_lo);
            }
            if rd_hi.to_reg().to_real_reg().is_none() {
                collector.reg_early_def(rd_hi);
            }
        }
        Inst::CmpRR { rn, rm, .. } => {
            use_if_virtual(collector, rn);
            use_if_virtual(collector, rm);
        }
        Inst::CmpRImm { rn, .. } => use_if_virtual(collector, rn),

        Inst::Store { rt, mem, .. } => {
            use_if_virtual(collector, rt);
            mem.get_operands(collector);
        }
        Inst::Load { rt, mem, .. } => {
            mem.get_operands(collector);
            def_if_virtual(collector, rt);
        }

        Inst::Call { info } => {
            let CallInfo { uses, defs, .. } = &mut **info;
            for CallArgPair { vreg, preg } in uses {
                collector.reg_fixed_use(vreg, *preg);
            }
            for CallRetPair { vreg, location } in defs {
                match location {
                    RetLocation::Reg(preg, ..) => collector.reg_fixed_def(vreg, *preg),
                    RetLocation::Stack(..) => collector.any_def(vreg),
                }
            }
            collector.reg_clobbers(info.clobbers);
        }
        Inst::LoadExtName { rd, .. } => {
            collector.reg_def(rd);
        }
        Inst::CallInd { info } => {
            let CallInfo {
                dest, uses, defs, ..
            } = &mut **info;
            collector.reg_use(dest);
            for CallArgPair { vreg, preg } in uses {
                collector.reg_fixed_use(vreg, *preg);
            }
            for CallRetPair { vreg, location } in defs {
                match location {
                    RetLocation::Reg(preg, ..) => collector.reg_fixed_def(vreg, *preg),
                    RetLocation::Stack(..) => collector.any_def(vreg),
                }
            }
            collector.reg_clobbers(info.clobbers);
        }

        Inst::Args { args } => {
            for ArgPair { vreg, preg } in args {
                collector.reg_fixed_def(vreg, *preg);
            }
        }
        Inst::Rets { rets } => {
            for RetPair { vreg, preg } in rets {
                collector.reg_fixed_use(vreg, *preg);
            }
        }
    }
}

impl MachInst for Inst {
    type LabelUse = LabelUse;
    type ABIMachineSpec = Arm32MachineDeps;

    // The undefined instruction `udf`. Used to fill trap-reachable padding.
    const TRAP_OPCODE: &'static [u8] = &0xe7f000f0u32.to_le_bytes();

    fn gen_dummy_use(_reg: Reg) -> Self {
        // Represented as a zero-length nop that "uses" nothing; a dummy use is
        // only needed by targets that must keep a value live artificially.
        Inst::Nop0
    }

    fn canonical_type_for_rc(rc: RegClass) -> Type {
        match rc {
            RegClass::Int => I32,
            RegClass::Float => F64,
            RegClass::Vector => types::I8X16,
        }
    }

    fn is_safepoint(&self) -> bool {
        matches!(self, Inst::Call { .. } | Inst::CallInd { .. })
    }

    fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        arm32_get_operands(self, collector);
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        match self {
            Inst::MovReg { rd, rm } => Some((*rd, *rm)),
            _ => None,
        }
    }

    fn is_included_in_clobbers(&self) -> bool {
        !matches!(self, Inst::Args { .. })
    }

    fn is_trap(&self) -> bool {
        matches!(self, Inst::Udf { .. })
    }

    fn is_args(&self) -> bool {
        matches!(self, Inst::Args { .. })
    }

    fn call_type(&self) -> CallType {
        match self {
            Inst::Call { .. } | Inst::CallInd { .. } => CallType::Regular,
            _ => CallType::None,
        }
    }

    fn is_term(&self) -> MachTerminator {
        match self {
            Inst::Rets { .. } => MachTerminator::Ret,
            Inst::Jump { .. } | Inst::CondBr { .. } | Inst::BrTable { .. } => {
                MachTerminator::Branch
            }
            _ => MachTerminator::None,
        }
    }

    fn is_mem_access(&self) -> bool {
        matches!(self, Inst::Load { .. } | Inst::Store { .. })
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, _ty: Type) -> Inst {
        if to_reg.to_reg().class() == RegClass::Float {
            // A `vmov.f64` copies the whole D register, which is correct for an
            // f32 held in the low half too.
            Inst::FpuRR {
                op: FpuOp2::Vmov,
                size: FpuSize::F64,
                rd: to_reg,
                rm: from_reg,
            }
        } else {
            Inst::MovReg {
                rd: to_reg,
                rm: from_reg,
            }
        }
    }

    fn gen_nop(preferred_size: usize) -> Inst {
        if preferred_size == 0 {
            return Inst::Nop0;
        }
        assert!(preferred_size >= 4);
        Inst::Nop4
    }

    fn gen_nop_units() -> Vec<Vec<u8>> {
        vec![0xe320f000u32.to_le_bytes().to_vec()]
    }

    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> {
        match ty {
            I8 => Ok((&[RegClass::Int], &[I8])),
            I16 => Ok((&[RegClass::Int], &[I16])),
            I32 => Ok((&[RegClass::Int], &[I32])),
            // 64-bit values are held in a pair of 32-bit integer registers.
            I64 => Ok((&[RegClass::Int, RegClass::Int], &[I32, I32])),
            // Floats live in VFP registers (an f32 in the low half of a D reg).
            F32 => Ok((&[RegClass::Float], &[F32])),
            F64 => Ok((&[RegClass::Float], &[F64])),
            _ => Err(CodegenError::Unsupported(alloc::format!(
                "Unsupported type on arm32 (only i8/i16/i32/i64/f32/f64 are implemented so far): {ty}"
            ))),
        }
    }

    fn gen_jump(target: MachLabel) -> Inst {
        Inst::Jump { dest: target }
    }

    fn worst_case_size() -> CodeOffset {
        // Must be an upper bound on the number of bytes any single (non
        // jump-table) instruction emits, since the buffer uses it as the
        // per-instruction island lookahead. The largest such sequence is the
        // 64-bit atomic min/max RMW loop, which expands to 10 words (40 bytes).
        // Inline jump tables manage their own islands and are exempt.
        44
    }

    fn worst_case_island_growth() -> CodeOffset {
        8
    }

    fn ref_type_regclass(_settings: &settings::Flags) -> RegClass {
        RegClass::Int
    }

    fn function_alignment() -> FunctionAlignment {
        FunctionAlignment {
            minimum: 4,
            preferred: 4,
        }
    }
}

//=============================================================================
// Pretty-printing.

impl Inst {
    pub(crate) fn print_with_state(&self, _state: &mut EmitState) -> String {
        let r = |reg: Reg| reg_name(reg);
        let reglist = |mask: u32| -> String {
            let names: Vec<String> = (0..16)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| reg_name(xreg(i)))
                .collect();
            alloc::format!("{{{}}}", names.join(", "))
        };
        match self {
            Inst::Nop0 => "nop-zero-len".to_string(),
            Inst::Nop4 => "nop".to_string(),
            Inst::Ret => "bx lr".to_string(),
            Inst::MovImm { rd, imm } => {
                let rd = r(rd.to_reg());
                if *imm >> 16 == 0 {
                    alloc::format!("movw {rd}, #{imm}")
                } else {
                    alloc::format!("movw {rd}, #{}; movt {rd}, #{}", imm & 0xffff, imm >> 16)
                }
            }
            Inst::MovRotImm { rd, imm12 } => {
                alloc::format!("mov {}, #{}", r(rd.to_reg()), decode_rotated_imm(*imm12))
            }
            Inst::MvnRotImm { rd, imm12 } => {
                alloc::format!("mvn {}, #{}", r(rd.to_reg()), decode_rotated_imm(*imm12))
            }
            Inst::Movw { rd, imm16 } => alloc::format!("movw {}, #{}", r(rd.to_reg()), imm16),
            Inst::Movt { rd, imm16 } => alloc::format!("movt {}, #{}", r(rd.to_reg()), imm16),
            Inst::MovReg { rd, rm } => {
                alloc::format!("mov {}, {}", r(rd.to_reg()), r(*rm))
            }
            Inst::MvnReg { rd, rm } => {
                alloc::format!("mvn {}, {}", r(rd.to_reg()), r(*rm))
            }
            Inst::AluRRR { op, rd, rn, rm } => {
                alloc::format!("{} {}, {}, {}", op.name(), r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::AluRRImm { op, rd, rn, imm12 } => {
                alloc::format!(
                    "{} {}, {}, #{}",
                    op.name(),
                    r(rd.to_reg()),
                    r(*rn),
                    decode_rotated_imm(*imm12)
                )
            }
            Inst::AluRRRFlags { op, rd, rn, rm } => {
                alloc::format!("{}s {}, {}, {}", op.name(), r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::ShiftImm { op, rd, rm, amount } => {
                alloc::format!("{} {}, {}, #{}", op.name(), r(rd.to_reg()), r(*rm), amount)
            }
            Inst::ShiftReg { op, rd, rm, rs } => {
                alloc::format!("{} {}, {}, {}", op.name(), r(rd.to_reg()), r(*rm), r(*rs))
            }
            Inst::Mul { rd, rn, rm } => {
                alloc::format!("mul {}, {}, {}", r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::Mla { rd, rn, rm, ra } => {
                alloc::format!("mla {}, {}, {}, {}", r(rd.to_reg()), r(*rn), r(*rm), r(*ra))
            }
            Inst::Mls { rd, rn, rm, ra } => {
                alloc::format!("mls {}, {}, {}, {}", r(rd.to_reg()), r(*rn), r(*rm), r(*ra))
            }
            Inst::Umull {
                rd_lo,
                rd_hi,
                rn,
                rm,
            }
            | Inst::Smull {
                rd_lo,
                rd_hi,
                rn,
                rm,
            }
            | Inst::Umlal {
                rd_lo,
                rd_hi,
                rn,
                rm,
            }
            | Inst::Smlal {
                rd_lo,
                rd_hi,
                rn,
                rm,
            } => {
                let mnem = match self {
                    Inst::Umull { .. } => "umull",
                    Inst::Smull { .. } => "smull",
                    Inst::Umlal { .. } => "umlal",
                    _ => "smlal",
                };
                alloc::format!(
                    "{mnem} {}, {}, {}, {}",
                    r(rd_lo.to_reg()),
                    r(rd_hi.to_reg()),
                    r(*rn),
                    r(*rm)
                )
            }
            Inst::SDiv { rd, rn, rm } => {
                alloc::format!("sdiv {}, {}, {}", r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::UDiv { rd, rn, rm } => {
                alloc::format!("udiv {}, {}, {}", r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::BitRR { op, rd, rm } => {
                alloc::format!("{} {}, {}", op.name(), r(rd.to_reg()), r(*rm))
            }
            Inst::ExtRR { op, rd, rm } => {
                alloc::format!("{} {}, {}", op.name(), r(rd.to_reg()), r(*rm))
            }
            Inst::CSel { cond, rd, rn, rm } => {
                let rd = r(rd.to_reg());
                alloc::format!("mov {rd}, {}; mov{} {rd}, {}", r(*rm), cond.name(), r(*rn))
            }
            Inst::Bfc { rd, lsb, width } => {
                alloc::format!("bfc {}, #{}, #{}", r(rd.to_reg()), lsb, width)
            }
            Inst::Bfi { rd, rn, lsb, width } => {
                alloc::format!("bfi {}, {}, #{}, #{}", r(rd.to_reg()), r(*rn), lsb, width)
            }
            Inst::Bfx {
                op,
                rd,
                rn,
                lsb,
                width,
            } => alloc::format!(
                "{} {}, {}, #{}, #{}",
                op.name(),
                r(rd.to_reg()),
                r(*rn),
                lsb,
                width
            ),
            Inst::QAlu { op, rd, rm, rn } => {
                alloc::format!("{} {}, {}, {}", op.name(), r(rd.to_reg()), r(*rm), r(*rn))
            }
            Inst::Sat {
                op,
                rd,
                sat_bits,
                rm,
            } => alloc::format!(
                "{} {}, #{}, {}",
                op.name(),
                r(rd.to_reg()),
                sat_bits,
                r(*rm)
            ),
            Inst::ParAlu { op, rd, rn, rm } => {
                alloc::format!("{} {}, {}, {}", op.name(), r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::ExtAdd { op, rd, rn, rm } => {
                alloc::format!("{} {}, {}, {}", op.name(), r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::DspMul3 { op, rd, rn, rm } => {
                alloc::format!("{} {}, {}, {}", op.name(), r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::DspMul4 { op, rd, rn, rm, ra } => alloc::format!(
                "{} {}, {}, {}, {}",
                op.name(),
                r(rd.to_reg()),
                r(*rn),
                r(*rm),
                r(*ra)
            ),
            Inst::DspMulL {
                op,
                rd_lo,
                rd_hi,
                rn,
                rm,
            } => alloc::format!(
                "{} {}, {}, {}, {}",
                op.name(),
                r(rd_lo.to_reg()),
                r(rd_hi.to_reg()),
                r(*rn),
                r(*rm)
            ),
            Inst::Sel { rd, rn, rm } => {
                alloc::format!("sel {}, {}, {}", r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::Pkh { op, rd, rn, rm } => {
                alloc::format!("{} {}, {}, {}", op.name(), r(rd.to_reg()), r(*rn), r(*rm))
            }
            Inst::Rrx { rd, rm } => {
                alloc::format!("rrx {}, {}", r(rd.to_reg()), r(*rm))
            }
            Inst::Udf { code } => alloc::format!("udf ; {code}"),
            Inst::TrapIf { cond, code } => {
                alloc::format!("b{} 1f ; udf ; 1: ; {code}", cond.invert().name())
            }
            Inst::LdmStm {
                load,
                rn,
                writeback,
                reg_list,
            } => {
                let names: Vec<String> = (0..16)
                    .filter(|i| reg_list & (1 << i) != 0)
                    .map(|i| reg_name(xreg(i)))
                    .collect();
                alloc::format!(
                    "{} {}{}, {{{}}}",
                    if *load { "ldmia" } else { "stmia" },
                    r(*rn),
                    if *writeback { "!" } else { "" },
                    names.join(", ")
                )
            }
            Inst::LoadEx {
                acquire,
                size,
                rt,
                rn,
            } => alloc::format!(
                "{}{} {}, [{}]",
                if *acquire { "ldaex" } else { "ldrex" },
                size.suffix(),
                r(rt.to_reg()),
                r(*rn)
            ),
            Inst::StoreEx {
                release,
                size,
                rd,
                rt,
                rn,
            } => alloc::format!(
                "{}{} {}, {}, [{}]",
                if *release { "stlex" } else { "strex" },
                size.suffix(),
                r(rd.to_reg()),
                r(*rt),
                r(*rn)
            ),
            Inst::LoadAcq { size, rt, rn } => {
                alloc::format!("lda{} {}, [{}]", size.suffix(), r(rt.to_reg()), r(*rn))
            }
            Inst::StoreRel { size, rt, rn } => {
                alloc::format!("stl{} {}, [{}]", size.suffix(), r(*rt), r(*rn))
            }
            Inst::Barrier { op } => op.name().to_string(),
            Inst::AtomicRmw {
                op,
                size,
                rd,
                addr,
                operand,
                ..
            } => alloc::format!(
                "atomic_rmw.{op:?}{} {}, [{}], {}",
                size.suffix(),
                r(rd.to_reg()),
                r(*addr),
                r(*operand)
            ),
            Inst::AtomicCas {
                size,
                rd,
                addr,
                expected,
                new,
                ..
            } => alloc::format!(
                "atomic_cas{} {}, [{}], {}, {}",
                size.suffix(),
                r(rd.to_reg()),
                r(*addr),
                r(*expected),
                r(*new)
            ),
            Inst::AtomicRmw64 {
                op,
                rd_lo,
                rd_hi,
                addr,
                operand_lo,
                operand_hi,
                ..
            } => alloc::format!(
                "atomic_rmw.{op:?}.i64 {}, {}, [{}], {}, {}",
                r(rd_lo.to_reg()),
                r(rd_hi.to_reg()),
                r(*addr),
                r(*operand_lo),
                r(*operand_hi)
            ),
            Inst::AtomicCas64 {
                rd_lo,
                rd_hi,
                addr,
                expected_lo,
                expected_hi,
                new_lo,
                new_hi,
                ..
            } => alloc::format!(
                "atomic_cas.i64 {}, {}, [{}], {}, {}, {}, {}",
                r(rd_lo.to_reg()),
                r(rd_hi.to_reg()),
                r(*addr),
                r(*expected_lo),
                r(*expected_hi),
                r(*new_lo),
                r(*new_hi)
            ),
            Inst::AtomicLoad64 { rd_lo, rd_hi, addr } => alloc::format!(
                "atomic_load.i64 {}, {}, [{}]",
                r(rd_lo.to_reg()),
                r(rd_hi.to_reg()),
                r(*addr)
            ),
            Inst::AtomicStore64 {
                addr,
                src_lo,
                src_hi,
                ..
            } => alloc::format!(
                "atomic_store.i64 {}, {}, [{}]",
                r(*src_lo),
                r(*src_hi),
                r(*addr)
            ),

            Inst::FpuRRR {
                op,
                size,
                rd,
                rn,
                rm,
            } => alloc::format!(
                "{}.{} {}, {}, {}",
                op.name(),
                size.suffix(),
                r(rd.to_reg()),
                r(*rn),
                r(*rm)
            ),
            Inst::FpuRR { op, size, rd, rm } => alloc::format!(
                "{}.{} {}, {}",
                op.name(),
                size.suffix(),
                r(rd.to_reg()),
                r(*rm)
            ),
            Inst::FpuLoad { rd, mem, .. } => {
                alloc::format!("vldr {}, {}", r(rd.to_reg()), mem.pretty_print())
            }
            Inst::FpuStore { rt, mem, .. } => {
                alloc::format!("vstr {}, {}", r(*rt), mem.pretty_print())
            }
            Inst::MovToFpu32 { rd, rt } => {
                alloc::format!("vmov {}, {}", r(rd.to_reg()), r(*rt))
            }
            Inst::MovToFpu64 { rd, rt_lo, rt_hi } => {
                alloc::format!("vmov {}, {}, {}", r(rd.to_reg()), r(*rt_lo), r(*rt_hi))
            }
            Inst::MovFromFpu32 { rt, rm } => {
                alloc::format!("vmov {}, {}", r(rt.to_reg()), r(*rm))
            }
            Inst::MovFromFpu64 { rt_lo, rt_hi, rm } => {
                alloc::format!(
                    "vmov {}, {}, {}",
                    r(rt_lo.to_reg()),
                    r(rt_hi.to_reg()),
                    r(*rm)
                )
            }
            Inst::VcmpMrs { size, rn, rm } => {
                alloc::format!("vcmp.{} {}, {}; vmrs", size.suffix(), r(*rn), r(*rm))
            }
            Inst::VcvtFF { to_f64, rd, rm } => {
                let (dst, src) = if *to_f64 {
                    ("f64", "f32")
                } else {
                    ("f32", "f64")
                };
                alloc::format!("vcvt.{dst}.{src} {}, {}", r(rd.to_reg()), r(*rm))
            }
            Inst::VcvtToInt {
                signed,
                size,
                rd,
                rm,
            } => {
                let int = if *signed { "s32" } else { "u32" };
                alloc::format!(
                    "vcvt.{int}.{} {}, {}",
                    size.suffix(),
                    r(rd.to_reg()),
                    r(*rm)
                )
            }
            Inst::VcvtFromInt {
                signed,
                size,
                rd,
                rm,
            } => {
                let int = if *signed { "s32" } else { "u32" };
                alloc::format!(
                    "vcvt.{}.{int} {}, {}",
                    size.suffix(),
                    r(rd.to_reg()),
                    r(*rm)
                )
            }
            Inst::FpuCSel { cond, rd, rn, rm } => {
                let rd = r(rd.to_reg());
                alloc::format!(
                    "vmov {rd}, {}; vmov{} {rd}, {}",
                    r(*rm),
                    cond.name(),
                    r(*rn)
                )
            }
            Inst::FpuMinMax {
                max,
                size,
                rd,
                rn,
                rm,
            } => alloc::format!(
                "{}.{} {}, {}, {}",
                if *max { "vmaxnm" } else { "vminnm" },
                size.suffix(),
                r(rd.to_reg()),
                r(*rn),
                r(*rm)
            ),
            Inst::CmpRR { op, rn, rm } => {
                alloc::format!("{} {}, {}", op.name(), r(*rn), r(*rm))
            }
            Inst::CmpRImm { op, rn, imm12 } => {
                alloc::format!("{} {}, #{}", op.name(), r(*rn), decode_rotated_imm(*imm12))
            }
            Inst::AdjustSp { amount } => {
                if *amount < 0 {
                    alloc::format!("sub sp, sp, #{}", -*amount)
                } else {
                    alloc::format!("add sp, sp, #{amount}")
                }
            }
            Inst::Store { rt, mem, kind } => {
                alloc::format!("{} {}, {}", kind.mnemonic(), r(*rt), mem.pretty_print())
            }
            Inst::Load { rt, mem, kind } => {
                alloc::format!(
                    "{} {}, {}",
                    kind.mnemonic(),
                    r(rt.to_reg()),
                    mem.pretty_print()
                )
            }
            Inst::Push { reg_list } => alloc::format!("push {}", reglist(*reg_list)),
            Inst::Pop { reg_list } => alloc::format!("pop {}", reglist(*reg_list)),
            Inst::Call { info } => alloc::format!("bl {}", info.dest.display(None)),
            Inst::CallInd { info } => alloc::format!("blx {}", r(info.dest)),
            Inst::LoadExtName { rd, name, offset } => {
                alloc::format!("load_ext_name {}, {:?} + {}", r(rd.to_reg()), name, offset)
            }
            Inst::Jump { dest } => alloc::format!("b {}", dest.to_string()),
            Inst::BrTable { index, targets, .. } => {
                let (default, entries) = targets.split_first().unwrap();
                let entries: Vec<String> = entries.iter().map(|l| l.to_string()).collect();
                alloc::format!(
                    "br_table {} [{}] default {}",
                    r(*index),
                    entries.join(", "),
                    default.to_string()
                )
            }
            Inst::CondBr {
                cond,
                taken,
                not_taken,
            } => alloc::format!(
                "b{} {}; b {}",
                cond.name(),
                taken.to_string(),
                not_taken.to_string()
            ),
            Inst::Args { args } => {
                let mut s = "args".to_string();
                for arg in args {
                    write!(&mut s, " {}={}", r(arg.vreg.to_reg()), r(arg.preg)).unwrap();
                }
                s
            }
            Inst::Rets { rets } => {
                let mut s = "rets".to_string();
                for ret in rets {
                    write!(&mut s, " {}={}", r(ret.vreg), r(ret.preg)).unwrap();
                }
                s
            }
        }
    }
}

//=============================================================================
// Label uses (branch fixups).

/// A use of a label by an instruction, for branch fixups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
    /// A 24-bit signed PC-relative branch offset, as used by the A32 `B`/`BL`
    /// instructions. The stored immediate is the offset in units of 4 bytes,
    /// relative to the branch instruction's address plus 8.
    Branch26,
}

impl MachInstLabelUse for LabelUse {
    const ALIGN: CodeOffset = 4;

    fn max_pos_range(self) -> CodeOffset {
        match self {
            // 24-bit signed immediate, scaled by 4.
            LabelUse::Branch26 => ((1 << 23) - 1) * 4,
        }
    }

    fn max_neg_range(self) -> CodeOffset {
        match self {
            LabelUse::Branch26 => (1 << 23) * 4,
        }
    }

    fn patch_size(self) -> CodeOffset {
        4
    }

    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        match self {
            LabelUse::Branch26 => {
                // The ARM PC reads as the instruction address plus 8.
                let pc_base = use_offset as i64 + 8;
                let offset = label_offset as i64 - pc_base;
                debug_assert!(offset & 0b11 == 0);
                let imm24 = ((offset >> 2) as u32) & 0x00ff_ffff;
                let insn = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
                let insn = (insn & 0xff00_0000) | imm24;
                buffer[0..4].copy_from_slice(&insn.to_le_bytes());
            }
        }
    }

    fn supports_veneer(self) -> bool {
        false
    }

    fn veneer_size(self) -> CodeOffset {
        0
    }

    fn worst_case_veneer_size() -> CodeOffset {
        0
    }

    fn generate_veneer(
        self,
        _buffer: &mut [u8],
        _veneer_offset: CodeOffset,
    ) -> (CodeOffset, LabelUse) {
        panic!("arm32 does not support branch veneers yet");
    }

    fn from_reloc(reloc: Reloc, _addend: Addend) -> Option<LabelUse> {
        match reloc {
            Reloc::Arm32Call => Some(LabelUse::Branch26),
            _ => None,
        }
    }
}
