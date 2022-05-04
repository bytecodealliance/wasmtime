//! This module defines risc_v-specific machine instruction types.

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
use crate::machinst::*;
use regalloc2::Allocation;

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

pub type OptionReg = Option<Reg>;
pub type OptionImm12 = Option<Imm12>;

//=============================================================================
// Instructions (top level): definition

use crate::isa::risc_v::lower::isle::generated_code::MInst;
pub use crate::isa::risc_v::lower::isle::generated_code::{
    AluOPRR, AluOPRRI, AluOPRRR, AluOPRRRR, AtomicOP, ExtendOp, FClassResult, FloatException,
    FloatFlagOp, FloatRoundingMode, LoadOP, MInst as Inst, StoreOP, OPFPFMT,
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
    /// todo when to use this??
    ResolvedOffset(i32),
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
}

impl Display for BranchTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchTarget::Label(l) => write!(f, "{}", l.to_string()),
            BranchTarget::ResolvedOffset(off) => write!(f, "{:+}", off),
        }
    }
}

impl Inst {
    fn in_i32_range(value: u64) -> bool {
        let value = value as i64;
        value >= (i32::MIN as i64) && value <= (i32::MAX as i64)
    }

    pub(crate) fn instruction_size() -> i32 /* less type cast  */ {
        4
    }
    pub(crate) fn load_constant_imm12(rd: Writable<Reg>, imm: Imm12) -> Inst {
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Ori,
            rd: rd,
            rs: zero_reg(),
            imm12: imm,
        }
    }

    pub(crate) fn load_constant_u32(rd: Writable<Reg>, value: u32) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        if let Some(imm) = Imm12::maybe_from_u64(value as u64) {
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
                imm: Imm20::from_bits((value as i32) << 12),
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

    pub(crate) fn construct_auipc_and_jalr(offset: i32) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        insts.push(Inst::Auipc {
            rd: writable_spilltmp_reg(),
            imm: Imm20::from_bits(offset >> 12),
        });
        insts.push(Inst::Jalr {
            rd: writable_zero_reg(),
            base: spilltmp_reg(),
            offset: Imm12::from_bits((offset & 0xfff) as i16),
        });
        insts
    }

    /*
        todo:: load 64-bit constant must need two register.
        this is annoying
        https://www.reddit.com/r/RISCV/comments/63e55h/load_a_large_immediate_constant_in_asm/
    */
    pub fn load_constant_u64(rd: Writable<Reg>, value: u64) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        if Imm12::maybe_from_u64(value).is_some() || Inst::in_i32_range(value) {
            insts.extend(Inst::load_constant_u32(rd, value as u32));
        } else {
            let tmp = writable_spilltmp_reg();
            assert!(tmp != rd);
            // high part
            insts.extend(Inst::load_constant_u64(rd, value >> 32));
            // low part

            insts.extend(Inst::load_constant_u64(tmp, value & 0xffff_ffff));
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
        insts.extend(Self::load_constant_u32(tmp, const_data));
        insts.push(Inst::AluRR {
            alu_op: AluOPRR::FmvWX,
            rd,
            rs: tmp.to_reg(),
        });
        insts
    }

    /// Create instructions that load a 64-bit floating-point constant.
    pub fn load_fp_constant64(rd: Writable<Reg>, const_data: u64) -> SmallVec<[Inst; 4]> {
        Inst::do_something_with_registers(1, |regs, insts| {
            insts.extend(Self::load_constant_u64(regs[0], const_data));
            insts.push(Inst::AluRR {
                alu_op: AluOPRR::FmvDX,
                rd,
                rs: regs[0].to_reg(),
            });
        })
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
    pub fn gen_store(mem: AMode, from_reg: Reg, ty: Type, flags: MemFlags) -> Inst {
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
// todo如果add_mod 好像是这种指令 x1 = x1 + 1 引用的
fn riscv64_get_operands<F: Fn(VReg) -> VReg>(inst: &Inst, collector: &mut OperandCollector<'_, F>) {
    match inst {
        &Inst::Nop0 => {
            //todo do nothing ok
        }
        &Inst::Nop4 => {
            //todo do nothing ok
        }
        &Inst::Auipc { rd, .. } => collector.reg_def(rd),
        &Inst::Lui { rd, .. } => collector.reg_def(rd),
        &Inst::AluRRR { rd, rs1, rs2, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rs1);
            collector.reg_use(rs2);
        }
        &Inst::AluRRImm12 { rd, rs, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rs);
        }
        &Inst::Load { rd, from, .. } => {
            collector.reg_def(rd);
            collector.reg_use(from.get_base_register());
        }
        &Inst::Store { src, to, .. } => {
            collector.reg_use(to.get_base_register());
            collector.reg_use(src);
        }

        &Inst::EpiloguePlaceholder => {}
        &Inst::Ret => {}
        &Inst::Extend { rd, rn, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rn);
        }
        &Inst::AjustSp { .. } => {}
        &Inst::Call { ref info } => todo!(),
        &Inst::CallInd { ref info } => todo!(),
        &Inst::TrapIf { rs1, rs2, .. } => {
            collector.reg_use(rs1);
            collector.reg_use(rs2);
        }
        &Inst::Trap { .. } => {}
        &Inst::Jal { .. } => {}
        &Inst::CondBr { kind, .. } => {
            collector.reg_use(kind.rs1);
            collector.reg_use(kind.rs2);
        }
        &Inst::LoadExtName { rd, .. } => todo!(),
        &Inst::LoadAddr { rd, mem } => todo!(),
        &Inst::VirtualSPOffsetAdj { .. } => {}
        &Inst::Mov { rd, rm, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rm);
        }
        &Inst::Fence => todo!(),
        &Inst::FenceI => todo!(),
        &Inst::ECall => todo!(),
        &Inst::EBreak => todo!(),
        &Inst::Udf { .. } => todo!(),
        &Inst::AluRR { rd, rs, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rs);
        }
        &Inst::AluRRRR {
            rd, rs1, rs2, rs3, ..
        } => {
            collector.reg_def(rd);
            collector.reg_uses(&[rs1, rs2, rs3]);
        }
        &Inst::FloatFlagOperation { rs, rd, .. } => {
            collector.reg_def(rd);
            if let Some(r) = rs {
                collector.reg_use(r);
            }
        }
        &Inst::Jalr { rd, base, .. } => {
            collector.reg_def(rd);
            collector.reg_use(base);
        }
        &Inst::Atomic { rd, addr, src, .. } => {
            collector.reg_def(rd);
            collector.reg_use(addr);
            collector.reg_use(src);
        }
        &Inst::Ffcmp { rd, rs1, rs2, .. } => {
            collector.reg_def(rd);
            collector.reg_use(rs1);
            collector.reg_use(rs2);
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
        I64
    }

    fn is_safepoint(&self) -> bool {
        /*
            todo
        */
        false
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
        /*
            default is true , why ???????
        */
        true
    }

    fn is_term(&self) -> MachTerminator {
        /*
            todo more
        */
        match self {
            &Inst::Jal { dest } => {
                let dest = dest.as_label();
                if dest.is_some() {
                    MachTerminator::Uncond
                } else {
                    MachTerminator::None
                }
            }
            &Inst::CondBr {
                taken, not_taken, ..
            } => {
                let taken = taken.as_label();
                let not_taken = not_taken.as_label();
                if taken.is_some() && not_taken.is_some() {
                    MachTerminator::Cond
                } else {
                    MachTerminator::None
                }
            }
            &Inst::Ret => MachTerminator::Ret,

            _ => MachTerminator::None,
        }
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Inst {
        Inst::Mov {
            rd: to_reg,
            rm: from_reg,
            ty,
        }
    }

    fn gen_constant<F: FnMut(Type) -> Writable<Reg>>(
        to_regs: ValueRegs<Writable<Reg>>,
        value: u128,
        ty: Type,
        alloc_tmp: F,
    ) -> SmallVec<[Inst; 4]> {
        if (ty.bits() <= 64 && (ty.is_bool() || ty.is_int())) || ty == R32 || ty == R64 {
            return Inst::load_constant_u64(to_regs.only_reg().unwrap(), value as u64);
        };
        match ty {
            F32 => Inst::load_fp_constant32(to_regs.only_reg().unwrap(), value as u32),
            F64 => Inst::load_fp_constant64(to_regs.only_reg().unwrap(), value as u64),
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
            R32 => panic!("32-bit reftype pointer should never be seen on risc-v64"),
            R64 => Ok((&[RegClass::Int], &[R64])),
            F32 => Ok((&[RegClass::Float], &[F32])),
            F64 => Ok((&[RegClass::Float], &[F64])),
            I128 => Ok((&[RegClass::Int, RegClass::Int], &[I64, I64])),
            B128 => Ok((&[RegClass::Int, RegClass::Int], &[B64, B64])),

            IFLAGS | FFLAGS => Ok((&[RegClass::Int], &[I64])),
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
        // The maximum size, in bytes, of any `Inst`'s emitted code. We have at least one case of
        // an 8-instruction sequence (saturating int-to-float conversions) with three embedded
        // 64-bit f64 constants.
        //
        // Note that inline jump-tables handle island/pool insertion separately, so we do not need
        // to account for them here (otherwise the worst case would be 2^31 * 4, clearly not
        // feasible for other reasons).

        //todo I don't know yet
        100
    }

    fn ref_type_regclass(_: &settings::Flags) -> RegClass {
        RegClass::Int
    }
}

//=============================================================================
// Pretty-printing of instructions.

impl PrettyPrint for Inst {
    fn pretty_print(&self, size_bytes: u8, allocs: &mut AllocationConsumer<'_>) -> String {
        unimplemented!()
    }
}

fn reg_name(reg: Reg) -> String {
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
        state: &mut EmitState,
        allocs: &mut AllocationConsumer<'_>,
    ) -> String {
        let mut register_name = |reg: Reg| -> String {
            let reg = allocs.next(reg);
            reg_name(reg)
        };

        match self {
            &Inst::Nop0 => {
                format!(";;zero length nop")
            }
            &Inst::Nop4 => {
                format!(";;fixed 4-size nop")
            }
            &Inst::Auipc { rd, imm } => {
                format!("{} {},{}", "auipc", register_name(rd.to_reg()), imm.bits)
            }
            &Inst::Jalr { rd, base, offset } => {
                format!(
                    "{} {},{},{}",
                    "jalr",
                    register_name(rd.to_reg()),
                    register_name(base),
                    offset.bits
                )
            }
            &Inst::Lui { rd, ref imm } => {
                format!("{} {},{}", "lui", register_name(rd.to_reg()), imm.bits)
            }
            &Inst::AluRRR {
                alu_op,
                rd,
                rs1,
                rs2,
            } => {
                format!(
                    "{} {},{},{}",
                    alu_op.op_name(),
                    register_name(rd.to_reg()),
                    register_name(rs1),
                    register_name(rs2)
                )
            }
            &Inst::AluRR { alu_op, rd, rs } => {
                format!(
                    "{} {},{}",
                    alu_op.op_name(),
                    register_name(rd.to_reg()),
                    register_name(rs),
                )
            }
            &Inst::AluRRRR {
                alu_op,
                rd,
                rs1,
                rs2,
                rs3,
            } => {
                format!(
                    "{} {},{},{},{}",
                    alu_op.op_name(),
                    register_name(rd.to_reg()),
                    register_name(rs1),
                    register_name(rs2),
                    register_name(rs3),
                )
            }
            &Inst::AluRRImm12 {
                alu_op,
                rd,
                rs,
                ref imm12,
            } => {
                format!(
                    "{} {},{},{}",
                    alu_op.op_name(),
                    register_name(rd.to_reg()),
                    register_name(rs),
                    imm12.as_i16()
                )
            }
            &Inst::Load {
                rd,
                op,
                from,
                flags,
            } => {
                format!(
                    "{} {},{}",
                    op.op_name(),
                    register_name(rd.to_reg()),
                    from.to_string_may_be_alloc(allocs)
                )
            }
            &Inst::Ffcmp {
                rd,
                cc,
                ty,
                rs1,
                rs2,
            } => {
                format!(
                    "{}{} {},{},{}",
                    if ty == F32 { "f" } else { "d" },
                    cc,
                    register_name(rd.to_reg()),
                    register_name(rs1),
                    register_name(rs2),
                )
            }
            &Inst::Store { src, op, to, flags } => {
                format!(
                    "{} {},{}",
                    op.op_name(),
                    register_name(src),
                    to.to_string_may_be_alloc(allocs)
                )
            }
            &Inst::EpiloguePlaceholder => {
                format!("epilogue place holder")
            }
            &Inst::Ret => {
                format!("ret")
            }
            &MInst::Extend { rd, rn, op } => {
                format!(
                    "{} {},{}",
                    op.op_name(),
                    register_name(rd.to_reg()),
                    register_name(rn)
                )
            }
            &MInst::AjustSp { amount } => {
                format!("{} sp,{}", "addi", amount)
            }
            &MInst::Call { .. } => todo!(),
            &MInst::CallInd { .. } => todo!(),
            &MInst::TrapIf { .. } => todo!(),
            &MInst::Trap { .. } => todo!(),
            &MInst::Jal { dest } => {
                format!("{} {}", "Jal", format!("{:?}", dest))
            }
            &MInst::CondBr {
                taken,
                not_taken,
                kind,
                ..
            } => {
                let (rs1, rs2) = kind.rs1_rs2();
                format!(
                    "{} {},{},{},{}",
                    kind.kind_name(),
                    register_name(rs1),
                    register_name(rs2),
                    taken,
                    not_taken,
                )
            }
            &MInst::Atomic {
                op,
                rd,
                addr,
                src,
                aq,
                rl,
            } => {
                if op.is_load() {
                    format!(
                        "{} {},{}",
                        op.op_name(),
                        register_name(rd.to_reg()),
                        register_name(addr)
                    )
                } else {
                    format!(
                        "{} {},{},{}",
                        op.op_name(),
                        register_name(rd.to_reg()),
                        register_name(addr),
                        register_name(src),
                    )
                }
            }
            &MInst::LoadExtName { .. } => todo!(),
            &MInst::LoadAddr { .. } => todo!(),
            &MInst::VirtualSPOffsetAdj { .. } => todo!(),
            &MInst::Mov { rd, rm, ty } => {
                let v = if ty == F32 {
                    "fmv.s"
                } else if ty == F64 {
                    "fmv.d"
                } else {
                    "mov"
                };
                format!("{} {},{}", v, register_name(rd.to_reg()), register_name(rm))
            }
            &MInst::Fence => "fence".into(),
            &MInst::FenceI => "fence.i".into(),

            &MInst::Udf { .. } => todo!(),
            &MInst::EBreak {} => todo!(),
            &MInst::ECall {} => todo!(),
            &MInst::FloatFlagOperation { op, rs, rd, imm } => {
                if op.use_imm12() {
                    format!(
                        "{} {},{}",
                        op.op_name(),
                        register_name(rd.to_reg()),
                        imm.unwrap().as_i16()
                    )
                } else if let Some(r) = rs {
                    format!(
                        "{} {},{}",
                        op.op_name(),
                        register_name(rd.to_reg()),
                        register_name(r),
                    )
                } else {
                    format!("{} {}", op.op_name(), register_name(rd.to_reg()))
                }
            }
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
            The indirect jump instruction JALR (jump and link register) uses the I-type encoding. The target
    address is obtained by adding the 12-bit signed I-immediate to the register rs1, then setting the
    least-significant bit of the result to zero. The address of the instruction following the jump (pc+4)
    is written to register rd. Register x0 can be used as the destination if the result is not required.
        */
    Jalr12,

    /*
        All branch instructions use the B-type instruction format. The 12-bit B-immediate encodes signed
    offsets in multiples of 2, and is added to the current pc to give the target address. The conditional
    branch range is ±4 KiB.
        */
    B12,
}

impl MachInstLabelUse for LabelUse {
    /// Alignment for veneer code. Every AArch64 instruction must be 4-byte-aligned.
    const ALIGN: CodeOffset = 4;

    /// Maximum PC-relative range (positive), inclusive.
    fn max_pos_range(self) -> CodeOffset {
        match self {
            LabelUse::Jal20 => ((1 << 20) - 1) * 2,
            LabelUse::PCRel32 => i32::MAX as CodeOffset,
            LabelUse::Jalr12 => (1 << 12) - 1,
            LabelUse::B12 => ((1 << 12) - 1) * 2,
        }
    }

    /// Maximum PC-relative range (negative).
    fn max_neg_range(self) -> CodeOffset {
        match self {
            LabelUse::PCRel32 | LabelUse::Jalr12 => self.max_pos_range() + 1,
            _ => self.max_pos_range() + 2,
        }
    }

    /// Size of window into code needed to do the patch.
    fn patch_size(self) -> CodeOffset {
        match self {
            LabelUse::Jal20 => 4,
            LabelUse::PCRel32 => 8,
            LabelUse::Jalr12 => 4,
            LabelUse::B12 => 4,
        }
    }

    /// Perform the patch.
    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        assert!(use_offset % 4 == 0);
        assert!(label_offset % 4 == 0);
        let offset = (label_offset as i64) - (use_offset as i64);
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
        buffer: &mut [u8],
        veneer_offset: CodeOffset,
    ) -> (CodeOffset, LabelUse) {
        unimplemented!("don't support");
    }

    fn from_reloc(reloc: Reloc, addend: Addend) -> Option<LabelUse> {
        unimplemented!("don't support");
    }
}

impl LabelUse {
    fn offset_in_range(self, offset: i32) -> bool {
        let offset = offset as i64;
        let min = -(self.max_neg_range() as i64);
        let max = self.max_pos_range() as i64;
        offset >= min && offset <= max
    }
    fn patch_raw_offset(self, buffer: &mut [u8], offset: i32) {
        // safe to convert long range to short range.
        let offset = offset as u32;
        match self {
            LabelUse::Jal20 => {
                // this is certainly safe
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
                // this is certainly safe
                // auipc part
                {
                    let raw = { &mut buffer[0] as *mut u8 as *mut u32 };
                    let v = offset & (!0xfff);
                    unsafe {
                        *raw |= v;
                    }
                }
                {
                    // this is certainly safe
                    let raw = { &mut buffer[4] as *mut u8 as *mut u32 };
                    let v = (offset & 0xfff) << 20;
                    unsafe {
                        *raw |= v;
                    }
                }
            }

            LabelUse::Jalr12 => {
                // this is certainly safe
                let raw = { &mut buffer[0] as *mut u8 as *mut u32 };
                let v = (offset & 0xfff) << 20;
                unsafe {
                    *raw |= v;
                }
            }

            LabelUse::B12 => {
                // this is certainly safe
                let raw = &mut buffer[0] as *mut u8 as *mut u32;
                let v = ((offset >> 11 & 0b1) << 6)
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
