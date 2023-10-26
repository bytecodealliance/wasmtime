//! This module defines zkasm-specific machine instruction types.

// Some variants are not constructed, but we still want them as options in the future.
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(warnings)]

use crate::binemit::{Addend, CodeOffset, Reloc};
pub use crate::ir::condcodes::IntCC;
use crate::ir::types::{self, F32, F64, I128, I16, I32, I64, I8, I8X16, R32, R64};

pub use crate::ir::{ExternalName, MemFlags, Opcode, SourceLoc, Type, ValueLabel};
use crate::isa::{CallConv, FunctionAlignment};
use crate::machinst::*;
use crate::{settings, CodegenError, CodegenResult};

pub use crate::ir::condcodes::FloatCC;

use alloc::vec::Vec;
use regalloc2::{PRegSet, RegClass, VReg};
use smallvec::{smallvec, SmallVec};
use std::boxed::Box;
use std::fmt::Write;
use std::string::{String, ToString};

pub mod regs;
pub use self::regs::*;
pub mod imms;
pub use self::imms::*;
pub mod args;
pub use self::args::*;
pub mod emit;
pub use self::emit::*;
pub mod encode;
pub use self::encode::*;
use regalloc2::PReg;

use crate::isa::zkasm::abi::ZkAsmMachineDeps;

#[cfg(test)]
mod emit_tests;

use std::fmt::{Display, Formatter};

pub(crate) type OptionReg = Option<Reg>;
pub(crate) type VecBranchTarget = Vec<BranchTarget>;
pub(crate) type OptionUimm5 = Option<UImm5>;
pub(crate) type VecU8 = Vec<u8>;
pub(crate) type VecWritableReg = Vec<Writable<Reg>>;
//=============================================================================
// Instructions (top level): definition

use crate::isa::zkasm::lower::isle::generated_code::MInst;
pub use crate::isa::zkasm::lower::isle::generated_code::{
    AluOPRRI, AluOPRRR, FFlagsException, IntSelectOP, LoadOP, MInst as Inst, StoreOP,
};

type BoxCallInfo = Box<CallInfo>;
type BoxCallIndInfo = Box<CallIndInfo>;
type BoxReturnCallInfo = Box<ReturnCallInfo>;

/// Additional information for (direct) Call instructions, left out of line to lower the size of
/// the Inst enum.
#[derive(Clone, Debug)]
pub struct CallInfo {
    pub dest: ExternalName,
    pub uses: CallArgList,
    pub defs: CallRetList,
    pub opcode: Opcode,
    pub caller_callconv: CallConv,
    pub callee_callconv: CallConv,
    pub clobbers: PRegSet,
    pub callee_pop_size: u32,
}

/// Additional information for CallInd instructions, left out of line to lower the size of the Inst
/// enum.
#[derive(Clone, Debug)]
pub struct CallIndInfo {
    pub rn: Reg,
    pub uses: CallArgList,
    pub defs: CallRetList,
    pub opcode: Opcode,
    pub caller_callconv: CallConv,
    pub callee_callconv: CallConv,
    pub clobbers: PRegSet,
    pub callee_pop_size: u32,
}

/// Additional information for `return_call[_ind]` instructions, left out of
/// line to lower the size of the `Inst` enum.
#[derive(Clone, Debug)]
pub struct ReturnCallInfo {
    pub uses: CallArgList,
    pub opcode: Opcode,
    pub old_stack_arg_size: u32,
    pub new_stack_arg_size: u32,
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

impl BranchTarget {
    /// Return the target's label, if it is a label-based target.
    pub(crate) fn as_label(self) -> Option<MachLabel> {
        match self {
            BranchTarget::Label(l) => Some(l),
            _ => None,
        }
    }
    /// offset zero.
    #[inline]
    pub(crate) fn zero() -> Self {
        Self::ResolvedOffset(0)
    }
    #[inline]
    pub(crate) fn offset(off: i32) -> Self {
        Self::ResolvedOffset(off)
    }
    #[inline]
    pub(crate) fn is_zero(self) -> bool {
        match self {
            BranchTarget::Label(_) => false,
            BranchTarget::ResolvedOffset(off) => off == 0,
        }
    }
    #[inline]
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

pub(crate) fn enc_auipc(rd: Writable<Reg>, imm: Imm20) -> u32 {
    let x = 0b0010111 | reg_to_gpr_num(rd.to_reg()) << 7 | imm.as_u32() << 12;
    x
}

/// rd and src must have the same length.
pub(crate) fn gen_moves(rd: &[Writable<Reg>], src: &[Reg]) -> SmallInstVec<Inst> {
    assert!(rd.len() == src.len());
    assert!(rd.len() > 0);
    let mut insts = SmallInstVec::new();
    for (dst, src) in rd.iter().zip(src.iter()) {
        let ty = Inst::canonical_type_for_rc(dst.to_reg().class());
        insts.push(Inst::gen_move(*dst, *src, ty));
    }
    insts
}

impl Inst {
    const INSTRUCTION_SIZE: i32 = 4;

    pub(crate) fn load_constant_u32<F: FnMut(Type) -> Writable<Reg>>(
        rd: Writable<Reg>,
        value: u64,
        alloc_tmp: &mut F,
    ) -> SmallInstVec<Inst> {
        smallvec![Inst::LoadConst32 {
            rd,
            imm: value as u32
        }]
    }

    pub fn load_constant_u64<F: FnMut(Type) -> Writable<Reg>>(
        rd: Writable<Reg>,
        value: u64,
        alloc_tmp: &mut F,
    ) -> SmallInstVec<Inst> {
        smallvec![Inst::LoadConst64 { rd, imm: value }]
    }

    pub(crate) fn construct_auipc_and_jalr(
        _link: Option<Writable<Reg>>,
        _tmp: Writable<Reg>,
        _offset: i64,
    ) -> [Inst; 2] {
        todo!()
    }

    /// Create instructions that load a 32-bit floating-point constant.
    pub fn load_fp_constant32<F: FnMut(Type) -> Writable<Reg>>(
        rd: Writable<Reg>,
        const_data: u32,
        mut alloc_tmp: F,
    ) -> SmallVec<[Inst; 4]> {
        let mut insts = SmallVec::new();
        let tmp = alloc_tmp(I64);
        insts.extend(Self::load_constant_u32(
            tmp,
            const_data as u64,
            &mut alloc_tmp,
        ));
        todo!();
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
        insts.extend(Self::load_constant_u64(tmp, const_data, &mut alloc_tmp));
        todo!();
        insts
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
            flags,
        }
    }
}

//=============================================================================
fn zkasm_get_operands<F: Fn(VReg) -> VReg>(inst: &Inst, collector: &mut OperandCollector<'_, F>) {
    match inst {
        &Inst::Nop0 => {}
        &Inst::Nop4 => {}
        &Inst::Label { .. } => {}
        &Inst::BrTable {
            index, tmp1, tmp2, ..
        } => {
            collector.reg_use(index);
            collector.reg_early_def(tmp1);
            collector.reg_early_def(tmp2);
        }
        &Inst::Auipc { rd, .. } => collector.reg_def(rd),
        &Inst::Lui { rd, .. } => collector.reg_def(rd),
        &Inst::LoadConst32 { rd, .. } => collector.reg_def(rd),
        &Inst::LoadConst64 { rd, .. } => collector.reg_def(rd),
        &Inst::AluRRR { rd, rs1, rs2, .. } => {
            collector.reg_fixed_use(rs1, a0());
            collector.reg_fixed_use(rs2, b0());
            collector.reg_def(rd);
        }
        &Inst::MulArith { rd, rs1, rs2, .. } => {
            collector.reg_fixed_use(rs1, a0());
            collector.reg_fixed_use(rs2, b0());
            let mut clobbered = PRegSet::empty();
            clobbered.add(c0().to_real_reg().unwrap().into());
            clobbered.add(d0().to_real_reg().unwrap().into());
            collector.reg_clobbers(clobbered);
            collector.reg_def(rd);
        }
        &Inst::DivArith { rd, rs1, rs2, .. } => {
            collector.reg_fixed_use(rs1, e0());
            collector.reg_fixed_use(rs2, b0());
            let mut clobbered = PRegSet::empty();
            clobbered.add(c0().to_real_reg().unwrap().into());
            clobbered.add(d0().to_real_reg().unwrap().into());
            collector.reg_clobbers(clobbered);
            collector.reg_fixed_def(rd, a0());
        }
        &Inst::Load { rd, from, .. } => {
            if let Some(r) = from.get_allocatable_register() {
                collector.reg_use(r);
            }
            collector.reg_def(rd);
        }
        &Inst::Store { to, src, .. } => {
            if let Some(r) = to.get_allocatable_register() {
                collector.reg_use(r);
            }
            collector.reg_use(src);
        }
        &Inst::Args { ref args } => {
            for arg in args {
                collector.reg_fixed_def(arg.vreg, arg.preg);
            }
        }
        &Inst::Ret { ref rets, .. } => {
            for ret in rets {
                collector.reg_fixed_use(ret.vreg, ret.preg);
            }
        }
        &Inst::Extend { rd, rn, .. } => {
            collector.reg_use(rn);
            collector.reg_def(rd);
        }

        &Inst::ReserveSp { .. } => {}
        &Inst::ReleaseSp { .. } => {}

        &Inst::Call { ref info } => {
            for u in &info.uses {
                collector.reg_fixed_use(u.vreg, u.preg);
            }
            for d in &info.defs {
                collector.reg_fixed_def(d.vreg, d.preg);
            }
            collector.reg_clobbers(info.clobbers);
        }
        &Inst::CallInd { ref info } => {
            if info.callee_callconv == CallConv::Tail {
                // TODO(https://github.com/bytecodealliance/regalloc2/issues/145):
                // This shouldn't be a fixed register constraint.
                collector.reg_fixed_use(info.rn, x_reg(5));
            } else {
                collector.reg_use(info.rn);
            }

            for u in &info.uses {
                collector.reg_fixed_use(u.vreg, u.preg);
            }
            for d in &info.defs {
                collector.reg_fixed_def(d.vreg, d.preg);
            }
            collector.reg_clobbers(info.clobbers);
        }
        &Inst::ReturnCall {
            callee: _,
            ref info,
        } => {
            for u in &info.uses {
                collector.reg_fixed_use(u.vreg, u.preg);
            }
        }
        &Inst::ReturnCallInd { ref info, callee } => {
            collector.reg_use(callee);
            for u in &info.uses {
                collector.reg_fixed_use(u.vreg, u.preg);
            }
        }
        &Inst::TrapIf { test, .. } => {
            collector.reg_use(test);
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
            if let Some(r) = mem.get_allocatable_register() {
                collector.reg_use(r);
            }
            collector.reg_early_def(rd);
        }

        &Inst::VirtualSPOffsetAdj { .. } => {}
        &Inst::Mov { rd, rm, .. } => {
            collector.reg_use(rm);
            collector.reg_def(rd);
        }
        &Inst::MovFromPReg { rd, rm } => {
            debug_assert!([px_reg(2), px_reg(8)].contains(&rm));
            collector.reg_def(rd);
        }
        &Inst::ECall => {}
        &Inst::EBreak => {}
        &Inst::Udf { .. } => {}

        &Inst::Jalr { rd, base, .. } => {
            collector.reg_use(base);
            collector.reg_def(rd);
        }
        &Inst::Select {
            ref dst,
            condition,
            x,
            y,
            ..
        } => {
            collector.reg_use(condition);
            collector.reg_uses(x.regs());
            collector.reg_uses(y.regs());
            for d in dst.iter() {
                collector.reg_early_def(d.clone());
            }
        }
        &Inst::IntSelect {
            ref dst,
            ref x,
            ref y,
            ..
        } => {
            collector.reg_uses(x.regs());
            collector.reg_uses(y.regs());
            for d in dst.iter() {
                collector.reg_early_def(d.clone());
            }
        }

        &Inst::Icmp { rd, a, b, .. } => {
            // TODO(akashin): Why would Icmp have multiple input registers?
            // collector.reg_uses(a.regs());
            // collector.reg_uses(b.regs());
            collector.reg_fixed_use(
                a.only_reg()
                    .expect("Only support 1 register in comparison now"),
                a0(),
            );
            collector.reg_fixed_use(
                b.only_reg()
                    .expect("Only support 1 register in comparison now"),
                b0(),
            );
            collector.reg_def(rd);
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
        &Inst::RawData { .. } => {}
        &Inst::TrapIfC { rs1, rs2, .. } => {
            collector.reg_use(rs1);
            collector.reg_use(rs2);
        }
        &Inst::Unwind { .. } => {}
        &Inst::DummyUse { reg } => {
            collector.reg_use(reg);
        }
        &Inst::Popcnt {
            sum, step, rs, tmp, ..
        } => {
            collector.reg_use(rs);
            collector.reg_early_def(tmp);
            collector.reg_early_def(step);
            collector.reg_early_def(sum);
        }
        &Inst::Rev8 { rs, rd, tmp, step } => {
            collector.reg_use(rs);
            collector.reg_early_def(tmp);
            collector.reg_early_def(step);
            collector.reg_early_def(rd);
        }
        &Inst::Cltz {
            sum, step, tmp, rs, ..
        } => {
            collector.reg_use(rs);
            collector.reg_early_def(tmp);
            collector.reg_early_def(step);
            collector.reg_early_def(sum);
        }
        &Inst::Brev8 {
            rs,
            rd,
            step,
            tmp,
            tmp2,
            ..
        } => {
            collector.reg_use(rs);
            collector.reg_early_def(step);
            collector.reg_early_def(tmp);
            collector.reg_early_def(tmp2);
            collector.reg_early_def(rd);
        }
        &Inst::StackProbeLoop { .. } => {
            // StackProbeLoop has a tmp register and StackProbeLoop used at gen_prologue.
            // t3 will do the job. (t3 is caller-save register and not used directly by compiler like writable_spilltmp_reg)
            // gen_prologue is called at emit stage.
            // no need let reg alloc know.
        }

        Inst::AddImm32 { rd, src1, src2 } => {
            collector.reg_def(*rd);
        }
    }
}

impl MachInst for Inst {
    type LabelUse = LabelUse;
    type ABIMachineSpec = ZkAsmMachineDeps;

    // TODO:
    const TRAP_OPCODE: &'static [u8] = &[0; 4];

    fn gen_dummy_use(reg: Reg) -> Self {
        Inst::DummyUse { reg }
    }

    fn gen_block_start(
        block_index: usize,
        _is_indirect_branch_target: bool,
        _is_forward_edge_cfi_enabled: bool,
    ) -> Option<Self> {
        Some(Inst::Label { imm: block_index })
    }

    fn canonical_type_for_rc(rc: RegClass) -> Type {
        match rc {
            regalloc2::RegClass::Int => I64,
            regalloc2::RegClass::Float => F64,
            regalloc2::RegClass::Vector => I8X16,
        }
    }

    fn is_safepoint(&self) -> bool {
        match self {
            &Inst::Call { .. }
            | &Inst::CallInd { .. }
            | &Inst::TrapIf { .. }
            | &Inst::Udf { .. } => true,
            _ => false,
        }
    }

    fn get_operands<F: Fn(VReg) -> VReg>(&self, collector: &mut OperandCollector<'_, F>) {
        zkasm_get_operands(self, collector);
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        match self {
            Inst::Mov { rd, rm, .. } => Some((rd.clone(), rm.clone())),
            _ => None,
        }
    }

    fn is_included_in_clobbers(&self) -> bool {
        match self {
            &Inst::Args { .. } => false,
            _ => true,
        }
    }

    fn is_trap(&self) -> bool {
        match self {
            Self::Udf { .. } => true,
            _ => false,
        }
    }

    fn is_args(&self) -> bool {
        match self {
            Self::Args { .. } => true,
            _ => false,
        }
    }

    fn is_term(&self) -> MachTerminator {
        match self {
            &Inst::Jal { .. } => MachTerminator::Uncond,
            &Inst::CondBr { .. } => MachTerminator::Cond,
            &Inst::Jalr { .. } => MachTerminator::Uncond,
            &Inst::Ret { .. } => MachTerminator::Ret,
            &Inst::BrTable { .. } => MachTerminator::Indirect,
            &Inst::ReturnCall { .. } | &Inst::ReturnCallInd { .. } => MachTerminator::RetCall,
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
            R32 => panic!("32-bit reftype pointer should never be seen on zkasm"),
            R64 => Ok((&[RegClass::Int], &[R64])),
            F32 => Ok((&[RegClass::Float], &[F32])),
            F64 => Ok((&[RegClass::Float], &[F64])),
            I128 => Ok((&[RegClass::Int, RegClass::Int], &[I64, I64])),
            _ if ty.is_vector() => {
                debug_assert!(ty.bits() <= 512);

                // Here we only need to return a SIMD type with the same size as `ty`.
                // We use these types for spills and reloads, so prefer types with lanes <= 31
                // since that fits in the immediate field of `vsetivli`.
                const SIMD_TYPES: [[Type; 1]; 6] = [
                    [types::I8X2],
                    [types::I8X4],
                    [types::I8X8],
                    [types::I8X16],
                    [types::I16X16],
                    [types::I32X16],
                ];
                let idx = (ty.bytes().ilog2() - 1) as usize;
                let ty = &SIMD_TYPES[idx][..];

                Ok((&[RegClass::Vector], ty))
            }
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
        // calculate by test function zkasm_worst_case_instruction_size()
        1_000_000
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
// Pretty-printing of instructions.
pub fn reg_name(reg: Reg) -> String {
    match reg.to_real_reg() {
        Some(real) => match real.class() {
            RegClass::Int => match real.hw_enc() {
                0 => "0".into(),
                1 => "RR".into(),
                2 => "SP".into(),
                // TODO(akashin): Do we have a global pointer register in ZK ASM?
                // https://www.five-embeddev.com/quickref/global_pointer.html
                // Supposed to be unallocatable.
                3 => "gp".into(),
                // TODO(akashin): Do we have a thread pointer register in ZK ASM?
                // https://groups.google.com/a/groups.riscv.org/g/sw-dev/c/cov47bNy5gY?pli=1
                // Supposed to be unallocatable.
                4 => "tp".into(),
                // Temporary registers.
                5 => "C".into(),
                6 => "D".into(),
                7 => "E".into(),
                8 => "fp".into(),
                9 => "s1".into(),
                10 => "A".into(),
                11 => "B".into(),
                12 => "CTX".into(),
                13..=17 => format!("a{}", real.hw_enc() - 10),
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
            RegClass::Vector => format!("v{}", real.hw_enc()),
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
        let format_labels = |labels: &[MachLabel]| -> String {
            if labels.len() == 0 {
                return String::from("[_]");
            }
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

        let mut empty_allocs = AllocationConsumer::default();
        match self {
            &Inst::Nop0 => {
                format!("##zero length nop")
            }
            &Inst::Nop4 => {
                format!("##fixed 4-size nop")
            }
            &Inst::Label { imm } => {
                format!("##label=L{imm}")
            }
            &Inst::StackProbeLoop {
                guard_size,
                probe_count,
                tmp,
            } => {
                let tmp = format_reg(tmp.to_reg(), allocs);
                format!(
                    "inline_stack_probe##guard_size={} probe_count={} tmp={}",
                    guard_size, probe_count, tmp
                )
            }
            &Inst::DummyUse { reg } => {
                let reg = format_reg(reg, allocs);
                format!("dummy_use {}", reg)
            }
            &Inst::RawData { ref data } => match data.len() {
                4 => {
                    let mut bytes = [0; 4];
                    for i in 0..bytes.len() {
                        bytes[i] = data[i];
                    }
                    format!(".4byte 0x{:x}", u32::from_le_bytes(bytes))
                }
                8 => {
                    let mut bytes = [0; 8];
                    for i in 0..bytes.len() {
                        bytes[i] = data[i];
                    }
                    format!(".8byte 0x{:x}", u64::from_le_bytes(bytes))
                }
                _ => {
                    format!(".data {:?}", data)
                }
            },
            &Inst::Unwind { ref inst } => {
                todo!()
            }
            &Inst::Brev8 {
                rs,
                ty,
                step,
                tmp,
                tmp2,
                rd,
            } => {
                let rs = format_reg(rs, allocs);
                let step = format_reg(step.to_reg(), allocs);
                let tmp = format_reg(tmp.to_reg(), allocs);
                let tmp2 = format_reg(tmp2.to_reg(), allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!(
                    "brev8 {},{}##tmp={} tmp2={} step={} ty={}",
                    rd, rs, tmp, tmp2, step, ty
                )
            }
            &Inst::Popcnt {
                sum,
                step,
                rs,
                tmp,
                ty,
            } => {
                let rs = format_reg(rs, allocs);
                let tmp = format_reg(tmp.to_reg(), allocs);
                let step = format_reg(step.to_reg(), allocs);
                let sum = format_reg(sum.to_reg(), allocs);
                format!("popcnt {},{}##ty={} tmp={} step={}", sum, rs, ty, tmp, step)
            }
            &Inst::Rev8 { rs, rd, tmp, step } => {
                let rs = format_reg(rs, allocs);
                let tmp = format_reg(tmp.to_reg(), allocs);
                let step = format_reg(step.to_reg(), allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("rev8 {},{}##step={} tmp={}", rd, rs, step, tmp)
            }
            &Inst::Cltz {
                sum,
                step,
                rs,
                tmp,
                ty,
                leading,
            } => {
                let rs = format_reg(rs, allocs);
                let tmp = format_reg(tmp.to_reg(), allocs);
                let step = format_reg(step.to_reg(), allocs);
                let sum = format_reg(sum.to_reg(), allocs);
                format!(
                    "{} {},{}##ty={} tmp={} step={}",
                    if leading { "clz" } else { "ctz" },
                    sum,
                    rs,
                    ty,
                    tmp,
                    step
                )
            }
            &Inst::SelectReg {
                rd,
                rs1,
                rs2,
                ref condition,
            } => {
                let c_rs1 = format_reg(condition.rs1, allocs);
                let c_rs2 = format_reg(condition.rs2, allocs);
                let rs1 = format_reg(rs1, allocs);
                let rs2 = format_reg(rs2, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!(
                    "select_reg {},{},{}##condition={}",
                    rd,
                    rs1,
                    rs2,
                    format!("({} {} {})", c_rs1, condition.kind.to_static_str(), c_rs2),
                )
            }
            &Inst::Icmp { cc, rd, a, b, ty } => {
                let a = format_regs(a.regs(), allocs);
                let b = format_regs(b.regs(), allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("{} {},{},{}##ty={}", cc.to_static_str(), rd, a, b, ty)
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
                format!("{} {},{},{}##ty={}", op.op_name(), dst, x, y, ty,)
            }
            &Inst::BrTable {
                index,
                tmp1,
                tmp2,
                ref targets,
            } => {
                let targets: Vec<_> = targets.iter().map(|x| x.as_label().unwrap()).collect();
                format!(
                    "{} {},{}##tmp1={},tmp2={}",
                    "br_table",
                    format_reg(index, allocs),
                    format_labels(&targets[..]),
                    format_reg(tmp1.to_reg(), allocs),
                    format_reg(tmp2.to_reg(), allocs),
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
            &Inst::Jalr { rd, base, offset } => {
                let base = format_reg(base, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("{} {},{}({})", "jalr", rd, offset.bits, base)
            }
            &Inst::Lui { rd, ref imm } => {
                format!("{} {},{}", "lui", format_reg(rd.to_reg(), allocs), imm.bits)
            }
            &Inst::LoadConst32 { rd, imm } => {
                let rd = format_reg(rd.to_reg(), allocs);
                let mut buf = String::new();
                write!(&mut buf, "auipc {},0; ", rd).unwrap();
                write!(&mut buf, "ld {},12({}); ", rd, rd).unwrap();
                write!(&mut buf, "j {}; ", Inst::INSTRUCTION_SIZE + 4).unwrap();
                write!(&mut buf, ".4byte 0x{:x}", imm).unwrap();
                buf
            }
            &Inst::LoadConst64 { rd, imm } => {
                let rd = format_reg(rd.to_reg(), allocs);
                let mut buf = String::new();
                write!(&mut buf, "auipc {},0; ", rd).unwrap();
                write!(&mut buf, "ld {},12({}); ", rd, rd).unwrap();
                write!(&mut buf, "j {}; ", Inst::INSTRUCTION_SIZE + 8).unwrap();
                write!(&mut buf, ".8byte 0x{:x}", imm).unwrap();
                buf
            }
            &Inst::AluRRR {
                alu_op,
                rd,
                rs1,
                rs2,
            } => {
                let rs1_s = format_reg(rs1, allocs);
                let rs2_s = format_reg(rs2, allocs);
                let rd_s = format_reg(rd.to_reg(), allocs);
                match alu_op {
                    AluOPRRR::Adduw if rs2 == zero_reg() => {
                        format!("zext.w {},{}", rd_s, rs1_s)
                    }
                    _ => {
                        format!("{} {},{},{}", alu_op.op_name(), rd_s, rs1_s, rs2_s)
                    }
                }
            }
            &Inst::MulArith { rd, rs1, rs2 } => {
                let rs1_s = format_reg(rs1, allocs);
                let rs2_s = format_reg(rs2, allocs);
                let rd_s = format_reg(rd.to_reg(), allocs);
                format!("MulArith rd = {}, rs1 = {}, rs2 = {}", rd_s, rs1_s, rs2_s)
            }
            &Inst::DivArith { rd, rs1, rs2 } => {
                let rs1_s = format_reg(rs1, allocs);
                let rs2_s = format_reg(rs2, allocs);
                let rd_s = format_reg(rd.to_reg(), allocs);
                format!("DivArith rd = {}, rs1 = {}, rs2 = {}", rd_s, rs1_s, rs2_s)
            }
            Inst::AddImm32 { rd, src1, src2 } => {
                let rd = format_reg(rd.to_reg(), allocs);
                format!("{src1} + {src2} => {rd};")
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
            &Inst::Args { ref args } => {
                let mut s = "args".to_string();
                let mut empty_allocs = AllocationConsumer::default();
                for arg in args {
                    let preg = format_reg(arg.preg, &mut empty_allocs);
                    let def = format_reg(arg.vreg.to_reg(), allocs);
                    write!(&mut s, " {}={}", def, preg).unwrap();
                }
                s
            }
            &Inst::Ret {
                ref rets,
                stack_bytes_to_pop,
            } => {
                let mut s = if stack_bytes_to_pop != 0 {
                    format!(
                        "  {}\n  :JMP(RR)",
                        Inst::ReleaseSp {
                            amount: stack_bytes_to_pop
                        }
                        .print_with_state(_state, allocs)
                    )
                } else {
                    "  :JMP(RR)".to_string()
                };

                let mut empty_allocs = AllocationConsumer::default();
                for ret in rets {
                    let preg = format_reg(ret.preg, &mut empty_allocs);
                    let vreg = format_reg(ret.vreg, allocs);
                    write!(&mut s, " {vreg}={preg}").unwrap();
                }
                s
            }

            &Inst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                ..
            } => {
                let rn = format_reg(rn, allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                return if signed == false && from_bits == 8 {
                    format!("andi {rd},{rn}")
                } else {
                    let op = if signed { "srai" } else { "srli" };
                    let shift_bits = (64 - from_bits) as i16;
                    format!("slli {rd},{rn},{shift_bits}; {op} {rd},{rd},{shift_bits}")
                };
            }
            &Inst::ReserveSp { amount } => {
                // FIXME: conversion methods
                let amount = amount.checked_div(8).unwrap();
                format!("  SP + {} => SP", amount)
            }
            &Inst::ReleaseSp { amount } => {
                // FIXME: conversion methods
                let amount = amount.checked_div(8).unwrap();
                format!("  SP - {} => SP", amount)
            }
            &Inst::Call { ref info } => format!("call {}", info.dest.display(None)),
            &Inst::CallInd { ref info } => {
                let rd = format_reg(info.rn, allocs);
                format!("callind {}", rd)
            }
            &Inst::ReturnCall {
                ref callee,
                ref info,
            } => {
                let mut s = format!(
                    "return_call {callee:?} old_stack_arg_size:{} new_stack_arg_size:{}",
                    info.old_stack_arg_size, info.new_stack_arg_size
                );
                for ret in &info.uses {
                    let preg = format_reg(ret.preg, &mut empty_allocs);
                    let vreg = format_reg(ret.vreg, allocs);
                    write!(&mut s, " {vreg}={preg}").unwrap();
                }
                s
            }
            &Inst::ReturnCallInd { callee, ref info } => {
                let callee = format_reg(callee, allocs);
                let mut s = format!(
                    "return_call_ind {callee} old_stack_arg_size:{} new_stack_arg_size:{}",
                    info.old_stack_arg_size, info.new_stack_arg_size
                );
                for ret in &info.uses {
                    let preg = format_reg(ret.preg, &mut empty_allocs);
                    let vreg = format_reg(ret.vreg, allocs);
                    write!(&mut s, " {vreg}={preg}").unwrap();
                }
                s
            }
            &Inst::TrapIf { test, trap_code } => {
                format!("trap_if {},{}", format_reg(test, allocs), trap_code,)
            }
            &Inst::TrapIfC {
                rs1,
                rs2,
                cc,
                trap_code,
            } => {
                let rs1 = format_reg(rs1, allocs);
                let rs2 = format_reg(rs2, allocs);
                format!("trap_ifc {}##({} {} {})", trap_code, rs1, cc, rs2)
            }
            &Inst::Jal { dest, .. } => {
                format!("{} {}", "j", dest)
            }
            &Inst::CondBr {
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
            &Inst::LoadExtName {
                rd,
                ref name,
                offset,
            } => {
                let rd = format_reg(rd.to_reg(), allocs);
                format!("load_sym {},{}{:+}", rd, name.display(None), offset)
            }
            &Inst::LoadAddr { ref rd, ref mem } => {
                let rs = mem.to_string_with_alloc(allocs);
                let rd = format_reg(rd.to_reg(), allocs);
                format!("load_addr {},{}", rd, rs)
            }
            &Inst::VirtualSPOffsetAdj { amount } => {
                format!("virtual_sp_offset_adj {:+}", amount)
            }
            &Inst::Mov { rd, rm, ty } => {
                let rd = format_reg(rd.to_reg(), allocs);
                let rm = format_reg(rm, allocs);
                format!("{rm} => {rd}")
            }
            &Inst::MovFromPReg { rd, rm } => {
                let rd = format_reg(rd.to_reg(), allocs);
                debug_assert!([px_reg(2), px_reg(8)].contains(&rm));
                let rm = reg_name(Reg::from(rm));
                format!("mv {},{}", rd, rm)
            }
            &Inst::Select {
                ref dst,
                condition,
                ref x,
                ref y,
                ty,
            } => {
                let condition = format_reg(condition, allocs);
                let x = format_regs(x.regs(), allocs);
                let y = format_regs(y.regs(), allocs);
                let dst: Vec<_> = dst.clone().into_iter().map(|r| r.to_reg()).collect();
                let dst = format_regs(&dst[..], allocs);
                format!("select_{} {},{},{}##condition={}", ty, dst, x, y, condition)
            }
            &Inst::Udf { trap_code } => format!("udf##trap_code={}", trap_code),
            &Inst::EBreak {} => String::from("ebreak"),
            &Inst::ECall {} => String::from("ecall"),
        }
    }
}

/// Different forms of label references for different instruction formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
    /// 20-bit branch offset (unconditional branches). PC-rel, offset is
    /// imm << 1. Immediate is 20 signed bits. Use in Jal instructions.
    Jal20,

    /// The unconditional jump instructions all use PC-relative
    /// addressing to help support position independent code. The JALR
    /// instruction was defined to enable a two-instruction sequence to
    /// jump anywhere in a 32-bit absolute address range. A LUI
    /// instruction can first load rs1 with the upper 20 bits of a
    /// target address, then JALR can add in the lower bits. Similarly,
    /// AUIPC then JALR can jump anywhere in a 32-bit pc-relative
    /// address range.
    PCRel32,

    /// All branch instructions use the B-type instruction format. The
    /// 12-bit B-immediate encodes signed offsets in multiples of 2, and
    /// is added to the current pc to give the target address. The
    /// conditional branch range is ±4 KiB.
    B12,

    /// Equivalent to the `R_RISCV_PCREL_HI20` relocation, Allows setting
    /// the immediate field of an `auipc` instruction.
    PCRelHi20,

    /// Similar to the `R_RISCV_PCREL_LO12_I` relocation but pointing to
    /// the final address, instead of the `PCREL_HI20` label. Allows setting
    /// the immediate field of I Type instructions such as `addi` or `lw`.
    ///
    /// Since we currently don't support offsets in labels, this relocation has
    /// an implicit offset of 4.
    PCRelLo12I,
}

impl MachInstLabelUse for LabelUse {
    const ALIGN: CodeOffset = 1;

    /// Maximum PC-relative range (positive), inclusive.
    fn max_pos_range(self) -> CodeOffset {
        match self {
            LabelUse::Jal20 => ((1 << 19) - 1) * 2,
            LabelUse::PCRelLo12I | LabelUse::PCRelHi20 | LabelUse::PCRel32 => {
                Inst::imm_max() as CodeOffset
            }
            LabelUse::B12 => ((1 << 11) - 1) * 2,
        }
    }

    /// Maximum PC-relative range (negative).
    fn max_neg_range(self) -> CodeOffset {
        CodeOffset::MAX
    }

    /// Size of window into code needed to do the patch.
    fn patch_size(self) -> CodeOffset {
        todo!()
    }

    /// Perform the patch.
    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        todo!()
        // assert!(use_offset % 4 == 0);
        // assert!(label_offset % 4 == 0);
        // let offset = (label_offset as i64) - (use_offset as i64);

        // // re-check range
        // assert!(
        //     offset >= -(self.max_neg_range() as i64) && offset <= (self.max_pos_range() as i64),
        //     "{:?} offset '{}' use_offset:'{}' label_offset:'{}'  must not exceed max range.",
        //     self,
        //     offset,
        //     use_offset,
        //     label_offset,
        // );
        // self.patch_raw_offset(buffer, offset);
    }

    /// Is a veneer supported for this label reference type?
    fn supports_veneer(self) -> bool {
        todo!()
    }

    /// How large is the veneer, if supported?
    fn veneer_size(self) -> CodeOffset {
        todo!()
    }

    /// Generate a veneer into the buffer, given that this veneer is at `veneer_offset`, and return
    /// an offset and label-use for the veneer's use of the original label.
    fn generate_veneer(
        self,
        buffer: &mut [u8],
        veneer_offset: CodeOffset,
    ) -> (CodeOffset, LabelUse) {
        todo!()
        // let base = writable_spilltmp_reg();
        // {
        //     let x = enc_auipc(base, Imm20::from_bits(0)).to_le_bytes();
        //     buffer[0] = x[0];
        //     buffer[1] = x[1];
        //     buffer[2] = x[2];
        //     buffer[3] = x[3];
        // }
        // {
        //     let x = enc_jalr(writable_zero_reg(), base.to_reg(), Imm12::from_bits(0)).to_le_bytes();
        //     buffer[4] = x[0];
        //     buffer[5] = x[1];
        //     buffer[6] = x[2];
        //     buffer[7] = x[3];
        // }
        // (veneer_offset, Self::PCRel32)
    }

    fn from_reloc(reloc: Reloc, addend: Addend) -> Option<LabelUse> {
        match (reloc, addend) {
            _ => None,
        }
    }

    fn worst_case_veneer_size() -> CodeOffset {
        todo!()
    }
}

impl LabelUse {
    fn offset_in_range(self, offset: i64) -> bool {
        true
        // let min = -(self.max_neg_range() as i64);
        // let max = self.max_pos_range() as i64;
        // offset >= min && offset <= max
    }

    fn patch_raw_offset(self, buffer: &mut [u8], offset: i64) {
        todo!()
        // let insn = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        // match self {
        //     LabelUse::Jal20 => {
        //         let offset = offset as u32;
        //         let v = ((offset >> 12 & 0b1111_1111) << 12)
        //             | ((offset >> 11 & 0b1) << 20)
        //             | ((offset >> 1 & 0b11_1111_1111) << 21)
        //             | ((offset >> 20 & 0b1) << 31);
        //         buffer[0..4].clone_from_slice(&u32::to_le_bytes(insn | v));
        //     }
        //     LabelUse::PCRel32 => {
        //         let insn2 = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
        //         Inst::generate_imm(offset as u64, |imm20, imm12| {
        //             let imm20 = imm20.unwrap_or_default();
        //             let imm12 = imm12.unwrap_or_default();
        //             // Encode the OR-ed-in value with zero_reg(). The
        //             // register parameter must be in the original
        //             // encoded instruction and or'ing in zeroes does not
        //             // change it.
        //             buffer[0..4].clone_from_slice(&u32::to_le_bytes(
        //                 insn | enc_auipc(writable_zero_reg(), imm20),
        //             ));
        //             buffer[4..8].clone_from_slice(&u32::to_le_bytes(
        //                 insn2 | enc_jalr(writable_zero_reg(), zero_reg(), imm12),
        //             ));
        //         })
        //         // expect make sure we handled.
        //         .expect("we have check the range before,this is a compiler error.");
        //     }

        //     LabelUse::B12 => {
        //         let offset = offset as u32;
        //         let v = ((offset >> 11 & 0b1) << 7)
        //             | ((offset >> 1 & 0b1111) << 8)
        //             | ((offset >> 5 & 0b11_1111) << 25)
        //             | ((offset >> 12 & 0b1) << 31);
        //         buffer[0..4].clone_from_slice(&u32::to_le_bytes(insn | v));
        //     }

        //     LabelUse::PCRelHi20 => {
        //         // See https://github.com/riscv-non-isa/riscv-elf-psabi-doc/blob/master/riscv-elf.adoc#pc-relative-symbol-addresses
        //         //
        //         // We need to add 0x800 to ensure that we land at the next page as soon as it goes out of range for the
        //         // Lo12 relocation. That relocation is signed and has a maximum range of -2048..2047. So when we get an
        //         // offset of 2048, we need to land at the next page and subtract instead.
        //         let offset = offset as u32;
        //         let hi20 = offset.wrapping_add(0x800) >> 12;
        //         let insn = (insn & 0xFFF) | (hi20 << 12);
        //         buffer[0..4].clone_from_slice(&u32::to_le_bytes(insn));
        //     }

        //     LabelUse::PCRelLo12I => {
        //         // `offset` is the offset from the current instruction to the target address.
        //         //
        //         // However we are trying to compute the offset to the target address from the previous instruction.
        //         // The previous instruction should be the one that contains the PCRelHi20 relocation and
        //         // stores/references the program counter (`auipc` usually).
        //         //
        //         // Since we are trying to compute the offset from the previous instruction, we can
        //         // represent it as offset = target_address - (current_instruction_address - 4)
        //         // which is equivalent to offset = target_address - current_instruction_address + 4.
        //         //
        //         // Thus we need to add 4 to the offset here.
        //         let lo12 = (offset + 4) as u32 & 0xFFF;
        //         let insn = (insn & 0xFFFFF) | (lo12 << 20);
        //         buffer[0..4].clone_from_slice(&u32::to_le_bytes(insn));
        //     }
        // }
    }
}
