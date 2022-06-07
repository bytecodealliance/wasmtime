//! This module defines riscv64-specific machine instruction types.

// Some variants are not constructed, but we still want them as options in the future.
#![allow(dead_code)]
#![allow(non_camel_case_types)]

use crate::binemit::{Addend, CodeOffset, Reloc};
pub use crate::ir::condcodes::IntCC;
use crate::ir::types::{
    B1, B128, B16, B32, B64, B8, F32, F64, FFLAGS, I128, I16, I32, I64, I8, IFLAGS, R32, R64,
};

pub use crate::ir::{ExternalName, MemFlags, Opcode, SourceLoc, Type, ValueLabel};
use crate::isa::CallConv;
use crate::machinst::*;
use crate::{settings, CodegenError, CodegenResult};

pub use crate::ir::condcodes::FloatCC;

use alloc::vec::Vec;
use regalloc2::VReg;
use smallvec::{smallvec, SmallVec};
use std::boxed::Box;
use std::string::String;

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

use std::fmt::{Display, Formatter};

pub(crate) type OptionReg = Option<Reg>;
pub(crate) type OptionImm12 = Option<Imm12>;
pub(crate) type VecBranchTarget = Vec<BranchTarget>;
pub(crate) type OptionUimm5 = Option<Uimm5>;
pub(crate) type OptionFloatRoundingMode = Option<FRM>;
//=============================================================================
// Instructions (top level): definition

use crate::isa::riscv64::lower::isle::generated_code::MInst;
pub use crate::isa::riscv64::lower::isle::generated_code::{
    AluOPRRI, AluOPRRR, AtomicOP, CsrOP, FClassResult, FFlagsException, FpuOPRR, FpuOPRRR,
    FpuOPRRRR, IntSelectOP, LoadOP, MInst as Inst, ReferenceCheckOP, StoreOP, FRM, I128OP, OPFPFMT,
};

type BoxCallInfo = Box<CallInfo>;
type BoxCallIndInfo = Box<CallIndInfo>;

/// Additional information for (direct) Call instructions, left out of line to lower the size of
/// the Inst enum.
#[derive(Clone, Debug)]
pub struct CallInfo {
    pub dest: ExternalName,
    pub uses: Vec<Reg>,
    pub defs: Vec<Writable<Reg>>,
    pub opcode: Opcode,
    pub caller_callconv: CallConv,
    pub callee_callconv: CallConv,
}

/// Additional information for CallInd instructions, left out of line to lower the size of the Inst
/// enum.
#[derive(Clone, Debug)]
pub struct CallIndInfo {
    pub rn: Reg,
    pub uses: Vec<Reg>,
    pub defs: Vec<Writable<Reg>>,
    pub opcode: Opcode,
    pub caller_callconv: CallConv,
    pub callee_callconv: CallConv,
}

/// A branch target. Either unresolved (basic-block index) or resolved (offset
/// from end of current instruction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchTarget {
    /// An unresolved reference to a Label, as passed into
    /// `lower_branch_group()`.
    Label(MachLabel),
    /// A fixed PC offset.
    ResolvedOffset(i32),
}

pub(crate) fn enc_auipc(rd: Writable<Reg>, imm: Imm20) -> u32 {
    let x = 0b0010111 | reg_to_gpr_num(rd.to_reg()) << 7 | imm.as_u32() << 12;
    x
}

pub(crate) fn enc_jalr(rd: Writable<Reg>, base: Reg, offset: Imm12) -> u32 {
    let x = 0b1100111 |  reg_to_gpr_num(rd.to_reg() )  << 7 |  0b000 << 12 /* funct3 */  | reg_to_gpr_num(base) << 15 |  offset.as_u32() << 20;
    x
}

impl BranchTarget {
    /// Return the target's label, if it is a label-based target.
    pub(crate) fn as_label(self) -> Option<MachLabel> {
        match self {
            BranchTarget::Label(l) => Some(l),
            _ => None,
        }
    }
    #[inline(always)]
    pub(crate) fn zero() -> Self {
        Self::ResolvedOffset(0)
    }
    #[inline(always)]
    pub(crate) fn offset(off: i32) -> Self {
        Self::ResolvedOffset(off)
    }
    #[inline(always)]
    pub(crate) fn is_zero(self) -> bool {
        match self {
            BranchTarget::Label(_) => false,
            BranchTarget::ResolvedOffset(off) => off == 0,
        }
    }

    #[inline(always)]
    pub(crate) fn as_offset(self) -> Option<i32> {
        match self {
            BranchTarget::Label(_) => None,
            BranchTarget::ResolvedOffset(off) => Some(off),
        }
    }
}

impl Display for BranchTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchTarget::Label(l) => write!(f, "{}", l.to_string()),
            BranchTarget::ResolvedOffset(off) => write!(f, "{}", off),
        }
    }
}

/*
    rd and src must have the same length.
*/
fn gen_moves(rd: &[Writable<Reg>], src: &[Reg]) -> SmallInstVec<Inst> {
    assert!(rd.len() == src.len());
    assert!(rd.len() > 0);
    let out_ty = Inst::canonical_type_for_rc(rd[0].to_reg().class());
    let in_ty = Inst::canonical_type_for_rc(src[0].class());
    let mut insts = SmallInstVec::new();
    for (dst, src) in rd.iter().zip(src.iter()) {
        insts.push(gen_move(*dst, out_ty, *src, in_ty));
    }
    insts
}

/*
    if input or output is float,
    you should use special instruction.
*/
pub(crate) fn gen_move(rd: Writable<Reg>, oty: Type, rm: Reg, ity: Type) -> Inst {
    match (ity.is_float(), oty.is_float()) {
        (false, false) => Inst::gen_move(rd, rm, oty),
        (true, true) => Inst::gen_move(rd, rm, oty),
        (false, true) => Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::move_f_to_x_op(ity),
            rd: rd,
            rs: rm,
        },
        (true, false) => Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::move_x_to_f_op(ity),
            rd: rd,
            rs: rm,
        },
    }
}

impl Inst {
    #[inline(always)]
    fn in_i32_range(value: u64) -> bool {
        let value = value as i64;
        value >= (i32::MIN as i64) && value <= (i32::MAX as i64)
    }

    pub(crate) const fn instruction_size() -> i32 /* less type cast  */ {
        4
    }

    #[inline(always)]
    pub(crate) fn load_constant_imm12(rd: Writable<Reg>, imm: Imm12) -> Inst {
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Ori,
            rd: rd,
            rs: zero_reg(),
            imm12: imm,
        }
    }

    pub(crate) fn load_constant_u32(rd: Writable<Reg>, value: u64) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        if let Some(imm) = Imm12::maybe_from_u64(value) {
            insts.push(Inst::load_constant_imm12(rd, imm));
        } else {
            /*
             https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md
             The following example shows the li pseudo instruction which is used to load immediate values:

                 .equ	CONSTANT, 0xdeadbeef
                 li	a0, CONSTANT

            Which, for RV32I, generates the following assembler output, as seen by objdump:

                00000000 <.text>:
                0:	deadc537          	lui	a0,0xdeadc
                4:	eef50513          	addi	a0,a0,-273 # deadbeef <CONSTANT+0x0>
                 */
            insts.push(Inst::Lui {
                rd: rd,
                imm: Imm20::from_bits((value as i32) >> 12),
            });
            insts.push(Inst::AluRRImm12 {
                alu_op: AluOPRRI::Addi,
                rd: rd,
                rs: rd.to_reg(),
                imm12: Imm12::from_bits(value as i16),
            });
        }
        insts
    }

    /*
        this will discard ra register.
    */
    pub(crate) fn construct_auipc_and_jalr(rd: Writable<Reg>, offset: i32) -> [Inst; 2] {
        let a = Inst::Auipc {
            rd,
            imm: Imm20::from_bits(offset >> 12),
        };
        let b = Inst::Jalr {
            rd: writable_zero_reg(),
            base: rd.to_reg(),
            offset: Imm12::from_bits((offset & 0xfff) as i16),
        };
        [a, b]
    }

    /*
        todo:: load 64-bit constant must need two register.
        this is annoying
        https://www.reddit.com/r/RISCV/comments/63e55h/load_a_large_immediate_constant_in_asm/
    */
    pub fn load_constant_u64(rd: Writable<Reg>, value: u64) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        if Inst::in_i32_range(value) {
            insts.extend(Inst::load_constant_u32(rd, value));
        } else {
            let tmp = writable_spilltmp_reg();
            assert!(tmp != rd);
            // high part
            insts.extend(Inst::load_constant_u32(rd, value >> 32));
            // low part

            insts.extend(Inst::load_constant_u32(tmp, value & 0xffff_ffff));
            // rd = rd << 32
            insts.push(Inst::AluRRImm12 {
                alu_op: AluOPRRI::Slli,
                rd: rd,
                rs: rd.to_reg(),
                imm12: Imm12::from_bits(32),
            });
            // tmp = tmp >> 32
            insts.push(Inst::AluRRImm12 {
                alu_op: AluOPRRI::Srli,
                rd: tmp,
                rs: tmp.to_reg(),
                imm12: Imm12::from_bits(32),
            });
            // rd = rd | tmp
            insts.push(Inst::AluRRR {
                alu_op: AluOPRRR::Or,
                rd: rd,
                rs1: rd.to_reg(),
                rs2: tmp.to_reg(),
            });
        }
        insts
    }

    /// Create instructions that load a 32-bit floating-point constant.
    pub fn load_fp_constant32(rd: Writable<Reg>, const_data: u32) -> SmallVec<[Inst; 4]> {
        let tmp = writable_spilltmp_reg();
        let mut insts = SmallVec::new();
        insts.extend(Self::load_constant_u32(tmp, const_data as u64));
        insts.push(Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::move_x_to_f_op(F32),
            rd,
            rs: tmp.to_reg(),
        });
        insts
    }

    /// Create instructions that load a 64-bit floating-point constant.
    pub fn load_fp_constant64<F: FnMut(Type) -> Writable<Reg>>(
        rd: Writable<Reg>,
        const_data: u64,
        mut alloc_tmp: F,
    ) -> SmallVec<[Inst; 4]> {
        let mut insts = SmallInstVec::new();
        let tmp = alloc_tmp(I64);
        insts.extend(Self::load_constant_u64(tmp, const_data));
        insts.push(Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::move_x_to_f_op(F64),
            rd,
            rs: tmp.to_reg(),
        });
        insts
    }

    /// Create instructions that load a 128-bit vector constant.
    pub fn load_fp_constant128<F: FnMut(Type) -> Writable<Reg>>(
        _rd: Writable<Reg>,
        _const_data: u128,
        _alloc_tmp: F,
    ) -> SmallVec<[Inst; 5]> {
        todo!()
    }

    /// Generic constructor for a load (zero-extending where appropriate).
    pub fn gen_load(into_reg: Writable<Reg>, mem: AMode, ty: Type, flags: MemFlags) -> Inst {
        Inst::Load {
            rd: into_reg,
            op: LoadOP::from_type(ty),
            from: mem,
            flags,
        }
    }

    /// Generic constructor for a store.
    pub fn gen_store(mem: AMode, from_reg: Reg, ty: Type, _flags: MemFlags) -> Inst {
        Inst::Store {
            src: from_reg,
            op: StoreOP::from_type(ty),
            to: mem,
            flags: MemFlags::new(),
        }
    }
}

//=============================================================================
// Instructions: get_regs
fn riscv64_get_operands<F: Fn(VReg) -> VReg>(inst: &Inst, collector: &mut OperandCollector<'_, F>) {
    match inst {
        &Inst::Nop0 => {}
        &Inst::Nop4 => {}
        &Inst::BrTable { index, tmp1, .. } => {
            collector.reg_use(index);
            collector.reg_early_def(tmp1);
        }
        &Inst::Auipc { rd, .. } => collector.reg_def(rd),
        &Inst::Lui { rd, .. } => collector.reg_def(rd),
        &Inst::AluRRR { rd, rs1, rs2, .. } => {
            collector.reg_use(rs1);
            collector.reg_use(rs2);
            collector.reg_def(rd);
        }
        &Inst::FpuRRR { rd, rs1, rs2, .. } => {
            collector.reg_use(rs1);
            collector.reg_use(rs2);
            collector.reg_def(rd);
        }
        &Inst::AluRRImm12 { rd, rs, .. } => {
            collector.reg_use(rs);
            collector.reg_def(rd);
        }
        &Inst::Load { rd, from, .. } => {
            collector.reg_use(from.get_base_register());
            collector.reg_def(rd);
        }
        &Inst::Store { to, src, .. } => {
            collector.reg_use(to.get_base_register());
            collector.reg_use(src);
        }

        &Inst::EpiloguePlaceholder => {}
        &Inst::Ret { .. } => {}
        &Inst::Extend { rd, rn, .. } => {
            collector.reg_use(rn);
            collector.reg_early_def(rd);
        }
        &Inst::AjustSp { .. } => {}
        &Inst::Call { ref info } => {
            collector.reg_uses(&info.uses[..]);
            collector.reg_defs(&info.defs[..]);
        }
        &Inst::CallInd { ref info } => {
            collector.reg_use(info.rn);
            collector.reg_uses(&info.uses[..]);
            collector.reg_defs(&info.defs[..]);
        }
        &Inst::TrapIf { ref x, ref y, .. } => {
            collector.reg_uses(x.regs());
            collector.reg_uses(y.regs());
        }
        &Inst::TrapFf {
            x, y, tmp, tmp2, ..
        } => {
            collector.reg_use(x);
            collector.reg_use(y);
            collector.reg_early_def(tmp);
            collector.reg_early_def(tmp2);
        }

        &Inst::Jal { .. } => {}
        &Inst::CondBr { kind, .. } => {
            collector.reg_use(kind.rs1);
            collector.reg_use(kind.rs2);
        }
        &Inst::LoadExtName { rd, .. } => {
            collector.reg_def(rd);
        }
        &Inst::LoadAddr { rd, mem } => {
            collector.reg_use(mem.get_base_register());
            collector.reg_def(rd);
        }

        &Inst::VirtualSPOffsetAdj { .. } => {}
        &Inst::Mov { rd, rm, .. } => {
            collector.reg_use(rm);
            collector.reg_def(rd);
        }
        &Inst::Fence => {}
        &Inst::FenceI => {}
        &Inst::ECall => {}
        &Inst::EBreak => {}
        &Inst::Udf { .. } => {}
        &Inst::FpuRR { rd, rs, .. } => {
            collector.reg_use(rs);
            collector.reg_def(rd);
        }
        &Inst::FpuRRRR {
            rd, rs1, rs2, rs3, ..
        } => {
            collector.reg_uses(&[rs1, rs2, rs3]);
            collector.reg_def(rd);
        }

        &Inst::Jalr { rd, base, .. } => {
            collector.reg_use(base);
            collector.reg_def(rd);
        }
        &Inst::Atomic { rd, addr, src, .. } => {
            collector.reg_use(addr);
            collector.reg_use(src);
            collector.reg_def(rd);
        }
        &Inst::Fcmp {
            rd, rs1, rs2, tmp, ..
        } => {
            collector.reg_use(rs1);
            collector.reg_use(rs2);
            collector.reg_early_def(rd);
            collector.reg_early_def(tmp);
        }
        &Inst::Select {
            ref dst,
            conditon,
            x,
            y,
            ..
        } => {
            collector.reg_use(conditon);
            collector.reg_uses(x.regs());
            collector.reg_uses(y.regs());
            collector.reg_defs(&dst[..]);
        }
        &Inst::ReferenceCheck { rd, x, .. } => {
            collector.reg_use(x);
            collector.reg_def(rd);
        }
        &Inst::AtomicCas {
            t0,
            dst,
            e,
            addr,
            v,
            ..
        } => {
            collector.reg_uses(&[e, addr, v]);
            collector.reg_early_def(t0);
            collector.reg_early_def(dst);
        }
        &Inst::IntSelect {
            ref dst,
            ref x,
            ref y,
            ..
        } => {
            collector.reg_uses(x.regs());
            collector.reg_uses(y.regs());
            collector.reg_defs(&dst[..]);
        }

        &Inst::Csr { rd, rs, .. } => {
            if let Some(rs) = rs {
                collector.reg_use(rs);
            }
            collector.reg_def(rd);
        }
        &Inst::Icmp { rd, a, b, .. } => {
            collector.reg_uses(a.regs());
            collector.reg_uses(b.regs());
            collector.reg_def(rd);
        }
        &Inst::Cls { rs, rd, .. } => {
            collector.reg_use(rs);
            collector.reg_early_def(rd);
        }
        &Inst::SelectReg {
            rd,
            rs1,
            rs2,
            condition,
        } => {
            collector.reg_use(condition.rs1);
            collector.reg_use(condition.rs2);
            collector.reg_use(rs1);
            collector.reg_use(rs2);
            collector.reg_def(rd);
        }
        &Inst::FcvtToIntSat { rd, rs, tmp, .. } => {
            collector.reg_use(rs);
            collector.reg_def(rd);
            collector.reg_def(tmp);
        }
        &Inst::SelectIf {
            ref rd,
            ref cmp_x,
            ref cmp_y,
            ref x,
            ref y,
            ..
        } => {
            collector.reg_uses(cmp_x.regs());
            collector.reg_uses(cmp_y.regs());
            collector.reg_uses(x.regs());
            collector.reg_uses(y.regs());
            rd.iter().for_each(|r| collector.reg_def(*r));
        }
    }
}

impl MachInst for Inst {
    type LabelUse = LabelUse;

    fn gen_dummy_use(reg: Reg) -> Self {
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Ori,
            rd: Writable::from_reg(reg),
            rs: reg,
            imm12: Imm12::zero(),
        }
    }

    fn canonical_type_for_rc(rc: RegClass) -> Type {
        if rc == RegClass::Float {
            F64
        } else {
            I64
        }
    }

    fn is_safepoint(&self) -> bool {
        match self {
            &Inst::Call { .. } => true,
            &Inst::CallInd { .. } => true,
            _ => false,
        }
    }

    fn get_operands<F: Fn(VReg) -> VReg>(&self, collector: &mut OperandCollector<'_, F>) {
        riscv64_get_operands(self, collector);
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        match self {
            Inst::Mov { rd, rm, .. } => Some((rd.clone(), rm.clone())),
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

    fn is_included_in_clobbers(&self) -> bool {
        true
    }

    fn is_term(&self) -> MachTerminator {
        match self {
            &Inst::Jal { .. } => MachTerminator::Uncond,
            &Inst::CondBr { .. } => MachTerminator::Cond,
            &Inst::Jalr { .. } => MachTerminator::Uncond,
            &Inst::Ret { .. } => MachTerminator::Ret,
            &Inst::BrTable { .. } => MachTerminator::Indirect,
            _ => MachTerminator::None,
        }
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Inst {
        let x = Inst::Mov {
            rd: to_reg,
            rm: from_reg,
            ty,
        };
        x
    }

    fn gen_constant<F: FnMut(Type) -> Writable<Reg>>(
        to_regs: ValueRegs<Writable<Reg>>,
        mut value: u128,
        ty: Type,
        alloc_tmp: F,
    ) -> SmallVec<[Inst; 4]> {
        if ty.is_bool() && value != 0 {
            value = !0;
        }
        if (ty.bits() <= 64 && (ty.is_bool() || ty.is_int())) || ty == R32 || ty == R64 {
            return Inst::load_constant_u64(to_regs.only_reg().unwrap(), value as u64);
        };
        match ty {
            F32 => Inst::load_fp_constant32(to_regs.only_reg().unwrap(), value as u32),
            F64 => Inst::load_fp_constant64(to_regs.only_reg().unwrap(), value as u64, alloc_tmp),
            I128 | B128 => {
                let mut insts = SmallInstVec::new();
                insts.extend(Inst::load_constant_u64(
                    to_regs.regs()[0],
                    (value >> 64) as u64,
                ));
                insts.extend(Inst::load_constant_u64(to_regs.regs()[1], value as u64));
                return insts;
            }
            _ => todo!(),
        }
    }

    fn gen_nop(preferred_size: usize) -> Inst {
        if preferred_size == 0 {
            return Inst::Nop0;
        }
        // We can't give a NOP (or any insn) < 4 bytes.
        assert!(preferred_size >= 4);
        Inst::Nop4
    }

    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> {
        match ty {
            I8 => Ok((&[RegClass::Int], &[I8])),
            I16 => Ok((&[RegClass::Int], &[I16])),
            I32 => Ok((&[RegClass::Int], &[I32])),
            I64 => Ok((&[RegClass::Int], &[I64])),
            B1 => Ok((&[RegClass::Int], &[B1])),
            B8 => Ok((&[RegClass::Int], &[B8])),
            B16 => Ok((&[RegClass::Int], &[B16])),
            B32 => Ok((&[RegClass::Int], &[B32])),
            B64 => Ok((&[RegClass::Int], &[B64])),
            R32 => panic!("32-bit reftype pointer should never be seen on riscv64"),
            R64 => Ok((&[RegClass::Int], &[R64])),
            F32 => Ok((&[RegClass::Float], &[F32])),
            F64 => Ok((&[RegClass::Float], &[F64])),
            I128 => Ok((&[RegClass::Int, RegClass::Int], &[I64, I64])),
            B128 => Ok((&[RegClass::Int, RegClass::Int], &[B64, B64])),
            IFLAGS => Ok((&[RegClass::Int], &[IFLAGS])),
            FFLAGS => Ok((&[RegClass::Int], &[FFLAGS])),
            _ => Err(CodegenError::Unsupported(format!(
                "Unexpected SSA-value type: {}",
                ty
            ))),
        }
    }

    fn gen_jump(target: MachLabel) -> Inst {
        Inst::Jal {
            dest: BranchTarget::Label(target),
        }
    }

    fn worst_case_size() -> CodeOffset {
        //caculate by test function riscv64_worst_case_size_instrcution_size()
        64
    }

    fn ref_type_regclass(_: &settings::Flags) -> RegClass {
        RegClass::Int
    }
}

//=============================================================================
// Pretty-printing of instructions.
pub fn reg_name(reg: Reg) -> String {
    match reg.to_real_reg() {
        Some(real) => match real.class() {
            RegClass::Int => match real.hw_enc() {
                0 => "zero".into(),
                1 => "ra".into(),
                2 => "sp".into(),
                3 => "gp".into(),
                4 => "tp".into(),
                5 => "t0".into(),
                6..=7 => format!("t{}", real.hw_enc() - 5),
                8 => "fp".into(),
                9 => "s1".into(),
                10..=17 => format!("a{}", real.hw_enc() - 10),
                18..=27 => format!("s{}", real.hw_enc() - 16),
                28..=31 => format!("t{}", real.hw_enc() - 25),
                _ => unreachable!(),
            },
            RegClass::Float => match real.hw_enc() {
                0..=7 => format!("ft{}", real.hw_enc() - 0),
                8..=9 => format!("fs{}", real.hw_enc() - 8),
                10..=17 => format!("fa{}", real.hw_enc() - 10),
                18..=27 => format!("fs{}", real.hw_enc() - 16),
                28..=31 => format!("ft{}", real.hw_enc() - 20),
                _ => unreachable!(),
            },
        },
        None => {
            format!("{:?}", reg)
        }
    }
}

impl Inst {
    fn print_with_state(
        &self,
        _state: &mut EmitState,
        allocs: &mut AllocationConsumer<'_>,
    ) -> String {
        let format_reg = |reg: Reg, allocs: &mut AllocationConsumer<'_>| -> String {
            let reg = allocs.next(reg);
            reg_name(reg)
        };

        let format_regs = |regs: &[Reg], allocs: &mut AllocationConsumer<'_>| -> String {
            let mut x = if regs.len() > 1 {
                String::from("[")
            } else {
                String::default()
            };
            regs.iter().for_each(|i| {
                x.push_str(format_reg(i.clone(), allocs).as_str());
                if *i != *regs.last().unwrap() {
                    x.push_str(",");
                }
            });
            if regs.len() > 1 {
                x.push_str("]");
            }
            x
        };
        let format_lables = |labels: &[MachLabel]| -> String {
            assert!(labels.len() > 0);
            let mut x = String::from("[");
            labels.iter().for_each(|l| {
                x.push_str(
                    format!(
                        "{:?}{}",
                        l,
                        if l != labels.last().unwrap() { "," } else { "" },
                    )
                    .as_str(),
                );
            });
            x.push_str("]");
            x
        };

        fn format_extend_op(signed: bool, from_bits: u8, _to_bits: u8) -> String {
            let type_name = match from_bits {
                1 => "b1",
                8 => "b",
                16 => "h",
                32 => "w",
                _ => unreachable!(),
            };
            format!("{}ext.{}", if signed { "s" } else { "u" }, type_name)
        }
        fn format_frm(rounding_mode: Option<FRM>) -> String {
            if FRM::is_none_or_using_fcsr(rounding_mode) {
                "".into()
            } else {
                format!(",{}", rounding_mode.unwrap().to_static_str())
            }
        }

        match self {
            &Inst::Nop0 => {
                format!(";;zero length nop")
            }
            &Inst::Nop4 => {
                format!(";;fixed 4-size nop")
            }
            &Inst::Cls { rd, rs, ty } => {
                let rs = format_reg(rs, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("cls {},{};;ty={}", rd, rs, ty)
            }
            &Inst::SelectIf {
                if_spectre_guard,
                ref rd,
                ref cmp_x,
                ref cmp_y,
                cc,
                ref x,
                ref y,
                cmp_ty,
            } => {
                let cmp_x = format_regs(cmp_x.regs(), allocs);
                let cmp_y = format_regs(cmp_y.regs(), allocs);
                let x = format_regs(x.regs(), allocs);
                let y = format_regs(y.regs(), allocs);
                let rd: Vec<_> = rd.iter().map(|r| r.to_reg()).collect();
                let rd = format_regs(&rd[..], allocs);
                format!(
                    "selectif{} {},{},{};;{} {} {} ty={}",
                    if if_spectre_guard {
                        "_spectre_guard"
                    } else {
                        ""
                    },
                    rd,
                    x,
                    y,
                    cmp_x,
                    cc,
                    cmp_y,
                    cmp_ty,
                )
            }
            &Inst::FcvtToIntSat {
                rd,
                rs,
                is_signed,
                in_type,
                out_type,
                tmp,
            } => {
                let rs = format_reg(rs, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                let tmp = format_reg(tmp.to_reg(), allocs);
                format!(
                    "fcvt_to_{}int_sat {},{};;in_ty={} out_ty={} tmp={}",
                    if is_signed { "s" } else { "u" },
                    rd,
                    rs,
                    in_type,
                    out_type,
                    tmp
                )
            }
            &Inst::SelectReg {
                rd,
                rs1,
                rs2,
                condition,
            } => {
                let c_rs1 = format_reg(condition.rs1, allocs);
                let c_rs2 = format_reg(condition.rs2, allocs);
                let rs1 = format_reg(rs1, allocs);
                let rs2 = format_reg(rs2, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!(
                    "select_reg {},{},{};;condition={}",
                    rd,
                    rs1,
                    rs2,
                    format!("({} {} {})", c_rs1, condition.kind.to_static_str(), c_rs2),
                )
            }
            &Inst::AtomicCas {
                t0,
                dst,
                e,
                addr,
                v,
                ty,
            } => {
                let e = format_reg(e, allocs);
                let addr = format_reg(addr, allocs);
                let v = format_reg(v, allocs);
                let t0 = format_reg(t0.to_reg(), allocs);
                let dst = format_reg(dst.to_reg(), allocs);
                format!(
                    "{} {},{},{},({});;t0={} ty={}",
                    "atomic_cas", dst, e, v, addr, t0, ty
                )
            }
            &Inst::Icmp { cc, rd, a, b, ty } => {
                let a = format_regs(a.regs(), allocs);
                let b = format_regs(b.regs(), allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("{} {},{},{};;ty={}", cc.to_static_str(), rd, a, b, ty)
            }
            &Inst::IntSelect {
                op,
                ref dst,
                x,
                y,
                ty,
            } => {
                let x = format_regs(x.regs(), allocs);
                let y = format_regs(y.regs(), allocs);

                let dst: Vec<_> = dst.iter().map(|r| r.to_reg()).collect();
                let dst = format_regs(&dst[..], allocs);
                format!("{} {},{},{};;ty={}", op.op_name(), dst, x, y, ty,)
            }
            &Inst::BrTable {
                index,
                tmp1,
                default_,
                ref targets,
            } => {
                let targets: Vec<_> = targets.iter().map(|x| x.as_label().unwrap()).collect();
                format!(
                    "{} {},{},{};;tmp1={}",
                    "br_table",
                    format_reg(index, allocs),
                    default_,
                    format_lables(&targets[..]),
                    format_reg(tmp1.to_reg(), allocs),
                )
            }
            &Inst::Auipc { rd, imm } => {
                format!(
                    "{} {},{}",
                    "auipc",
                    format_reg(rd.to_reg(), allocs),
                    imm.bits
                )
            }

            &Inst::ReferenceCheck { rd, op, x } => {
                let x = format_reg(x, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("{} {},{}", op.op_name(), rd, x)
            }
            &Inst::Jalr { rd, base, offset } => {
                let base = format_reg(base, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("{} {},{}({})", "jalr", rd, offset.bits, base,)
            }
            &Inst::Lui { rd, ref imm } => {
                format!("{} {},{}", "lui", format_reg(rd.to_reg(), allocs), imm.bits)
            }

            &Inst::AluRRR {
                alu_op,
                rd,
                rs1,
                rs2,
            } => {
                let rs1 = format_reg(rs1, allocs);
                let rs2 = format_reg(rs2, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("{} {},{},{}", alu_op.op_name(), rd, rs1, rs2,)
            }
            &Inst::FpuRR {
                frm,
                alu_op,
                rd,
                rs,
            } => {
                let rs = format_reg(rs, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("{} {},{}{}", alu_op.op_name(), rd, rs, format_frm(frm))
            }
            &Inst::FpuRRR {
                alu_op,
                rd,
                rs1,
                rs2,
                frm,
            } => {
                let rs1 = format_reg(rs1, allocs);
                let rs2 = format_reg(rs2, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!(
                    "{} {},{},{}{}",
                    alu_op.op_name(),
                    rd,
                    rs1,
                    rs2,
                    format_frm(frm)
                )
            }
            &Inst::Csr {
                csr_op,
                rd,
                rs,
                imm,
                csr,
            } => {
                let rs = rs.map_or("".into(), |r| format_reg(r, allocs));
                let rd = format_reg(rd.to_reg(), allocs);
                if csr_op.need_rs() {
                    format!("{} {},{},{}", csr_op.op_name(), rd, csr, rs)
                } else {
                    format!("{} {},{},{}", csr_op.op_name(), rd, csr, imm.unwrap())
                }
            }

            &Inst::FpuRRRR {
                alu_op,
                rd,
                rs1,
                rs2,
                rs3,
                frm,
            } => {
                let rs1 = format_reg(rs1, allocs);
                let rs2 = format_reg(rs2, allocs);
                let rs3 = format_reg(rs3, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!(
                    "{} {},{},{},{}{}",
                    alu_op.op_name(),
                    rd,
                    rs1,
                    rs2,
                    rs3,
                    format_frm(frm)
                )
            }
            &Inst::AluRRImm12 {
                alu_op,
                rd,
                rs,
                ref imm12,
            } => {
                let rs = format_reg(rs, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                if alu_op.is_bit_manip() {
                    if let Some(_) = alu_op.need_shamt() {
                        let shamt = (imm12.as_i16() as u8) & alu_op.shamt_mask();
                        format!("{} {},{},{}", alu_op.op_name(), rd, rs, shamt)
                    } else {
                        format!("{} {},{}", alu_op.op_name(), rd, rs)
                    }
                } else {
                    format!("{} {},{},{}", alu_op.op_name(), rd, rs, imm12.as_i16())
                }
            }
            &Inst::Load {
                rd,
                op,
                from,
                flags: _flags,
            } => {
                let base = from.to_string_with_alloc(allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("{} {},{}", op.op_name(), rd, base,)
            }
            &Inst::Fcmp {
                rd,
                tmp,
                cc,
                ty,
                rs1,
                rs2,
            } => {
                let rs1 = format_reg(rs1, allocs);
                let rs2 = format_reg(rs2, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                let tmp = format_reg(tmp.to_reg(), allocs);
                format!(
                    "{}.{} {},{},{};;tmp={}",
                    if ty == F32 { "f" } else { "d" },
                    cc,
                    rd,
                    rs1,
                    rs2,
                    tmp
                )
            }
            &Inst::Store {
                to,
                src,
                op,
                flags: _flags,
            } => {
                let base = to.to_string_with_alloc(allocs);
                let src = format_reg(src, allocs);
                format!("{} {},{}", op.op_name(), src, base,)
            }
            &Inst::EpiloguePlaceholder => {
                format!("epilogue place holder")
            }
            &Inst::Ret { .. } => {
                format!("ret")
            }

            &MInst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                to_bits,
            } => {
                let rn = format_reg(rn, allocs);
                let rm = format_reg(rd.to_reg(), allocs);
                format!(
                    "{} {},{}",
                    format_extend_op(signed, from_bits, to_bits),
                    rm,
                    rn
                )
            }
            &MInst::AjustSp { amount } => {
                format!("{} sp,{:+}", "addi", amount)
            }
            &MInst::Call { ref info } => format!("call {}", info.dest),
            &MInst::CallInd { ref info } => {
                let rd = format_reg(info.rn, allocs);
                format!("callind {}", rd)
            }
            &MInst::TrapIf {
                cc,
                x,
                y,
                ty,
                trap_code,
            } => format!(
                "trap_if_{} {} {},{};;ty={}",
                cc.to_static_str(),
                trap_code,
                format_regs(x.regs(), allocs),
                format_regs(y.regs(), allocs),
                ty,
            ),
            &MInst::TrapFf {
                cc,
                x,
                y,
                ty,
                trap_code,
                tmp,
                tmp2,
            } => format!(
                "trap_ff_{} {} {},{};;tmp={} tmp2={} ty={}",
                cc,
                trap_code,
                format_reg(x, allocs),
                format_reg(y, allocs),
                format_reg(tmp.to_reg(), allocs),
                format_reg(tmp2.to_reg(), allocs),
                ty,
            ),

            &MInst::Jal { dest, .. } => {
                format!("{} {}", "j", dest)
            }
            &MInst::CondBr {
                taken,
                not_taken,
                kind,
                ..
            } => {
                let rs1 = format_reg(kind.rs1, allocs);
                let rs2 = format_reg(kind.rs2, allocs);
                if not_taken.is_zero() && taken.as_label().is_none() {
                    let off = taken.as_offset().unwrap();
                    format!("{} {},{},{}", kind.op_name(), rs1, rs2, off)
                } else {
                    let x = format!(
                        "{} {},{},taken({}),not_taken({})",
                        kind.op_name(),
                        rs1,
                        rs2,
                        taken,
                        not_taken
                    );
                    x
                }
            }
            &MInst::Atomic {
                op,
                rd,
                addr,
                src,
                aq,
                rl,
            } => {
                let mut op_name = String::from(op.op_name());
                if aq && rl {
                    unreachable!("aq and rl can not both be true.")
                }
                if aq {
                    op_name.push_str(".aq");
                }
                if rl {
                    op_name.push_str(".rl");
                }
                let addr = format_reg(addr, allocs);
                let src = format_reg(src, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                if op.is_load() {
                    format!("{} {},({})", op_name, rd, addr,)
                } else {
                    format!("{} {},{},({})", op_name, rd, src, addr)
                }
            }
            &MInst::LoadExtName {
                rd,
                ref name,
                offset,
            } => {
                let rd = format_reg(rd.to_reg(), allocs);
                format!("load_sym {},{}{:+}", rd, name, offset)
            }
            &MInst::LoadAddr { ref rd, ref mem } => {
                let mem = mem.to_string_with_alloc(allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("load_addr {},{}", rd, mem)
            }
            &MInst::VirtualSPOffsetAdj { amount } => {
                format!("virtual_sp_offset_adj {:+}", amount)
            }
            &MInst::Mov { rd, rm, ty } => {
                let rd = format_reg(rd.to_reg(), allocs);
                let rm = format_reg(rm, allocs);
                let v = if ty == F32 {
                    "fmv.s"
                } else if ty == F64 {
                    "fmv.d"
                } else {
                    "mov"
                };
                format!("{} {},{}", v, rd, rm)
            }
            &MInst::Fence => "fence".into(),
            &MInst::FenceI => "fence.i".into(),
            &MInst::Select {
                ref dst,
                conditon,
                ref x,
                ref y,
                ty,
            } => {
                let condition = format_reg(conditon, allocs);
                let x = format_regs(x.regs(), allocs);
                let y = format_regs(y.regs(), allocs);
                let dst: Vec<_> = dst.clone().into_iter().map(|r| r.to_reg()).collect();
                let dst = format_regs(&dst[..], allocs);
                format!("select_{} {},{},{};;condition={}", ty, dst, x, y, condition)
            }

            &MInst::Udf { trap_code } => format!("udf;;trap_code={}", trap_code),
            &MInst::EBreak {} => String::from("ebreak"),
            &MInst::ECall {} => String::from("ecall"),
        }
    }
}

/// Different forms of label references for different instruction formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
    /// 20-bit branch offset (unconditional branches). PC-rel, offset is imm << 1. Immediate is 20
    /// signed bits. use in Jal
    Jal20,

    /*
            The unconditional jump instructions all use PC-relative addressing to help support position independent code. The JALR instruction was defined to enable a two-instruction sequence to
    jump anywhere in a 32-bit absolute address range. A LUI instruction can first load rs1 with the
    upper 20 bits of a target address, then JALR can add in the lower bits. Similarly, AUIPC then
    JALR can jump anywhere in a 32-bit pc-relative address range.
        */
    PCRel32,

    /*
        All branch instructions use the B-type instruction format. The 12-bit B-immediate encodes signed
    offsets in multiples of 2, and is added to the current pc to give the target address. The conditional
    branch range is ±4 KiB.
        */
    B12,
}

impl MachInstLabelUse for LabelUse {
    /// Alignment for veneer code. Every Riscv64 instruction must be 4-byte-aligned.
    const ALIGN: CodeOffset = 4;

    /// Maximum PC-relative range (positive), inclusive.
    fn max_pos_range(self) -> CodeOffset {
        match self {
            LabelUse::Jal20 => ((1 << 20) - 1) * 2,
            LabelUse::PCRel32 => i32::MAX as CodeOffset,
            LabelUse::B12 => ((1 << 12) - 1) * 2,
        }
    }

    /// Maximum PC-relative range (negative).
    fn max_neg_range(self) -> CodeOffset {
        match self {
            LabelUse::PCRel32 => self.max_pos_range() + 1,
            _ => self.max_pos_range() + 2,
        }
    }

    /// Size of window into code needed to do the patch.
    fn patch_size(self) -> CodeOffset {
        match self {
            LabelUse::Jal20 => 4,
            LabelUse::PCRel32 => 8,
            LabelUse::B12 => 4,
        }
    }

    /// Perform the patch.
    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        assert!(use_offset % 4 == 0);
        assert!(label_offset % 4 == 0);
        let offset = (label_offset as i64) - (use_offset as i64); /* todo::verify this*/
        //check range
        assert!(
            offset >= -(self.max_neg_range() as i64) && offset <= (self.max_pos_range() as i64),
            "offset must not exceed max range."
        );
        // safe to convert long range to short range.
        self.patch_raw_offset(buffer, offset as i32);
    }

    /// Is a veneer supported for this label reference type?
    fn supports_veneer(self) -> bool {
        match self {
            Self::B12 => true,
            Self::Jal20 => true,
            _ => false,
        }
    }

    /// How large is the veneer, if supported?
    fn veneer_size(self) -> CodeOffset {
        match self {
            Self::B12 => 8,
            Self::Jal20 => 8,
            _ => 0,
        }
    }

    /// Generate a veneer into the buffer, given that this veneer is at `veneer_offset`, and return
    /// an offset and label-use for the veneer's use of the original label.
    fn generate_veneer(
        self,
        buffer: &mut [u8],
        veneer_offset: CodeOffset,
    ) -> (CodeOffset, LabelUse) {
        // unimplemented!("generate_veneer:{:?} {:?}", buffer, veneer_offset);
        let base = writable_spilltmp_reg();
        {
            let x = enc_auipc(base, Imm20::from_bits(0)).to_le_bytes();
            buffer[0] = x[0];
            buffer[1] = x[1];
            buffer[2] = x[2];
            buffer[3] = x[3];
        }
        {
            let x = enc_jalr(writable_zero_reg(), base.to_reg(), Imm12::from_bits(0)).to_le_bytes();
            buffer[4] = x[0];
            buffer[5] = x[1];
            buffer[6] = x[2];
            buffer[7] = x[3];
        }

        (veneer_offset, Self::PCRel32)
    }

    fn from_reloc(_reloc: Reloc, _addend: Addend) -> Option<LabelUse> {
        None
    }
}

impl LabelUse {
    fn offset_in_range(self, offset: i32) -> bool {
        let offset = offset as i64;
        let min = -(self.max_neg_range() as i64);
        let max = self.max_pos_range() as i64;
        offset >= min && offset <= max
    }
    /*

    */
    fn patch_raw_offset(self, buffer: &mut [u8], offset: i32) {
        // safe to convert long range to short range.
        let offset = offset as u32;
        match self {
            LabelUse::Jal20 => {
                let raw = { &mut buffer[0] as *mut u8 as *mut u32 };
                let v = ((offset >> 12 & 0b1111_1111) << 12)
                    | ((offset >> 11 & 0b1) << 20)
                    | ((offset >> 1 & 0b11_1111_1111) << 21)
                    | ((offset >> 20 & 0b1) << 31);
                unsafe {
                    *raw |= v;
                }
            }
            LabelUse::PCRel32 => {
                // auipc part
                {
                    let raw = { &mut buffer[0] as *mut u8 as *mut u32 };
                    let v = offset & (!0xfff);
                    unsafe {
                        *raw |= v;
                    }
                }
                {
                    let raw = { &mut buffer[4] as *mut u8 as *mut u32 };
                    let v = (offset & 0xfff) << 20;
                    unsafe {
                        *raw |= v;
                    }
                }
            }
            LabelUse::B12 => {
                let raw = &mut buffer[0] as *mut u8 as *mut u32;
                let v = ((offset >> 11 & 0b1) << 7)
                    | ((offset >> 1 & 0b1111) << 8)
                    | ((offset >> 5 & 0b11_1111) << 25)
                    | ((offset >> 12 & 0b1) << 31);
                unsafe {
                    *raw |= v;
                }
            }
        }
    }
}
