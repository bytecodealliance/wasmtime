//! This module defines x86_64-specific machine instruction types.

pub use emit_state::EmitState;

use crate::binemit::{Addend, CodeOffset, Reloc, StackMap};
use crate::ir::{types, ExternalName, LibCall, Opcode, TrapCode, Type};
use crate::isa::x64::abi::X64ABIMachineSpec;
use crate::isa::x64::inst::regs::{pretty_print_reg, show_ireg_sized};
use crate::isa::x64::settings as x64_settings;
use crate::isa::{CallConv, FunctionAlignment};
use crate::{machinst::*, trace};
use crate::{settings, CodegenError, CodegenResult};
use alloc::boxed::Box;
use regalloc2::{Allocation, PRegSet};
use smallvec::{smallvec, SmallVec};
use std::fmt::{self, Write};
use std::string::{String, ToString};

pub mod args;
mod emit;
mod emit_state;
#[cfg(test)]
mod emit_tests;
pub mod regs;
pub mod unwind;

use args::*;

//=============================================================================
// Instructions (top level): definition

// `Inst` is defined inside ISLE as `MInst`. We publicly re-export it here.
pub use super::lower::isle::generated_code::MInst as Inst;

/// Out-of-line data for calls, to keep the size of `Inst` down.
#[derive(Clone, Debug)]
pub struct CallInfo {
    /// Register uses of this call.
    pub uses: CallArgList,
    /// Register defs of this call.
    pub defs: CallRetList,
    /// Registers clobbered by this call, as per its calling convention.
    pub clobbers: PRegSet,
    /// The number of bytes that the callee will pop from the stack for the
    /// caller, if any. (Used for popping stack arguments with the `tail`
    /// calling convention.)
    pub callee_pop_size: u32,
    /// The calling convention of the callee.
    pub callee_conv: CallConv,
}

/// Out-of-line data for return-calls, to keep the size of `Inst` down.
#[derive(Clone, Debug)]
pub struct ReturnCallInfo {
    /// The size of the argument area for this return-call, potentially smaller than that of the
    /// caller, but never larger.
    pub new_stack_arg_size: u32,

    /// The in-register arguments and their constraints.
    pub uses: CallArgList,

    /// A temporary for use when moving the return address.
    pub tmp: WritableGpr,
}

#[test]
#[cfg(target_pointer_width = "64")]
fn inst_size_test() {
    // This test will help with unintentionally growing the size
    // of the Inst enum.
    assert_eq!(40, std::mem::size_of::<Inst>());
}

pub(crate) fn low32_will_sign_extend_to_64(x: u64) -> bool {
    let xs = x as i64;
    xs == ((xs << 32) >> 32)
}

impl Inst {
    /// Retrieve a list of ISA feature sets in which the instruction is available. An empty list
    /// indicates that the instruction is available in the baseline feature set (i.e. SSE2 and
    /// below); more than one `InstructionSet` in the list indicates that the instruction is present
    /// *any* of the included ISA feature sets.
    fn available_in_any_isa(&self) -> SmallVec<[InstructionSet; 2]> {
        match self {
            // These instructions are part of SSE2, which is a basic requirement in Cranelift, and
            // don't have to be checked.
            Inst::AluRmiR { .. }
            | Inst::AluRM { .. }
            | Inst::AtomicRmwSeq { .. }
            | Inst::Bswap { .. }
            | Inst::CallKnown { .. }
            | Inst::CallUnknown { .. }
            | Inst::ReturnCallKnown { .. }
            | Inst::ReturnCallUnknown { .. }
            | Inst::CheckedSRemSeq { .. }
            | Inst::CheckedSRemSeq8 { .. }
            | Inst::Cmove { .. }
            | Inst::CmpRmiR { .. }
            | Inst::CvtFloatToSintSeq { .. }
            | Inst::CvtFloatToUintSeq { .. }
            | Inst::CvtUint64ToFloatSeq { .. }
            | Inst::Div { .. }
            | Inst::Div8 { .. }
            | Inst::Fence { .. }
            | Inst::Hlt
            | Inst::Imm { .. }
            | Inst::JmpCond { .. }
            | Inst::JmpIf { .. }
            | Inst::JmpKnown { .. }
            | Inst::JmpTableSeq { .. }
            | Inst::JmpUnknown { .. }
            | Inst::LoadEffectiveAddress { .. }
            | Inst::LoadExtName { .. }
            | Inst::LockCmpxchg { .. }
            | Inst::Mov64MR { .. }
            | Inst::MovImmM { .. }
            | Inst::MovRM { .. }
            | Inst::MovRR { .. }
            | Inst::MovFromPReg { .. }
            | Inst::MovToPReg { .. }
            | Inst::MovsxRmR { .. }
            | Inst::MovzxRmR { .. }
            | Inst::Mul { .. }
            | Inst::Mul8 { .. }
            | Inst::IMul { .. }
            | Inst::IMulImm { .. }
            | Inst::Neg { .. }
            | Inst::Not { .. }
            | Inst::Nop { .. }
            | Inst::Pop64 { .. }
            | Inst::Push64 { .. }
            | Inst::StackProbeLoop { .. }
            | Inst::Args { .. }
            | Inst::Rets { .. }
            | Inst::Ret { .. }
            | Inst::Setcc { .. }
            | Inst::ShiftR { .. }
            | Inst::SignExtendData { .. }
            | Inst::TrapIf { .. }
            | Inst::TrapIfAnd { .. }
            | Inst::TrapIfOr { .. }
            | Inst::Ud2 { .. }
            | Inst::VirtualSPOffsetAdj { .. }
            | Inst::XmmCmove { .. }
            | Inst::XmmCmpRmR { .. }
            | Inst::XmmMinMaxSeq { .. }
            | Inst::XmmUninitializedValue { .. }
            | Inst::ElfTlsGetAddr { .. }
            | Inst::MachOTlsGetAddr { .. }
            | Inst::CoffTlsGetAddr { .. }
            | Inst::Unwind { .. }
            | Inst::DummyUse { .. }
            | Inst::AluConstOp { .. } => smallvec![],

            Inst::AluRmRVex { op, .. } => op.available_from(),
            Inst::UnaryRmR { op, .. } => op.available_from(),
            Inst::UnaryRmRVex { op, .. } => op.available_from(),
            Inst::UnaryRmRImmVex { op, .. } => op.available_from(),

            // These use dynamic SSE opcodes.
            Inst::GprToXmm { op, .. }
            | Inst::XmmMovRM { op, .. }
            | Inst::XmmMovRMImm { op, .. }
            | Inst::XmmRmiReg { opcode: op, .. }
            | Inst::XmmRmR { op, .. }
            | Inst::XmmRmRUnaligned { op, .. }
            | Inst::XmmRmRBlend { op, .. }
            | Inst::XmmRmRImm { op, .. }
            | Inst::XmmToGpr { op, .. }
            | Inst::XmmToGprImm { op, .. }
            | Inst::XmmUnaryRmRImm { op, .. }
            | Inst::XmmUnaryRmRUnaligned { op, .. }
            | Inst::XmmUnaryRmR { op, .. }
            | Inst::CvtIntToFloat { op, .. } => smallvec![op.available_from()],

            Inst::XmmUnaryRmREvex { op, .. }
            | Inst::XmmRmREvex { op, .. }
            | Inst::XmmRmREvex3 { op, .. }
            | Inst::XmmUnaryRmRImmEvex { op, .. } => op.available_from(),

            Inst::XmmRmiRVex { op, .. }
            | Inst::XmmRmRVex3 { op, .. }
            | Inst::XmmRmRImmVex { op, .. }
            | Inst::XmmRmRBlendVex { op, .. }
            | Inst::XmmVexPinsr { op, .. }
            | Inst::XmmUnaryRmRVex { op, .. }
            | Inst::XmmUnaryRmRImmVex { op, .. }
            | Inst::XmmMovRMVex { op, .. }
            | Inst::XmmMovRMImmVex { op, .. }
            | Inst::XmmToGprImmVex { op, .. }
            | Inst::XmmToGprVex { op, .. }
            | Inst::GprToXmmVex { op, .. }
            | Inst::CvtIntToFloatVex { op, .. }
            | Inst::XmmCmpRmRVex { op, .. } => op.available_from(),
        }
    }
}

// Handy constructors for Insts.

impl Inst {
    pub(crate) fn nop(len: u8) -> Self {
        debug_assert!(len <= 15);
        Self::Nop { len }
    }

    pub(crate) fn alu_rmi_r(
        size: OperandSize,
        op: AluRmiROpcode,
        src: RegMemImm,
        dst: Writable<Reg>,
    ) -> Self {
        src.assert_regclass_is(RegClass::Int);
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        Self::AluRmiR {
            size,
            op,
            src1: Gpr::new(dst.to_reg()).unwrap(),
            src2: GprMemImm::new(src).unwrap(),
            dst: WritableGpr::from_writable_reg(dst).unwrap(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn unary_rm_r(
        size: OperandSize,
        op: UnaryRmROpcode,
        src: RegMem,
        dst: Writable<Reg>,
    ) -> Self {
        src.assert_regclass_is(RegClass::Int);
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        debug_assert!(size.is_one_of(&[
            OperandSize::Size16,
            OperandSize::Size32,
            OperandSize::Size64
        ]));
        Self::UnaryRmR {
            size,
            op,
            src: GprMem::new(src).unwrap(),
            dst: WritableGpr::from_writable_reg(dst).unwrap(),
        }
    }

    pub(crate) fn not(size: OperandSize, src: Writable<Reg>) -> Inst {
        debug_assert_eq!(src.to_reg().class(), RegClass::Int);
        Inst::Not {
            size,
            src: Gpr::new(src.to_reg()).unwrap(),
            dst: WritableGpr::from_writable_reg(src).unwrap(),
        }
    }

    pub(crate) fn div(
        size: OperandSize,
        sign: DivSignedness,
        trap: TrapCode,
        divisor: RegMem,
        dividend_lo: Gpr,
        dividend_hi: Gpr,
        dst_quotient: WritableGpr,
        dst_remainder: WritableGpr,
    ) -> Inst {
        divisor.assert_regclass_is(RegClass::Int);
        Inst::Div {
            size,
            sign,
            trap,
            divisor: GprMem::new(divisor).unwrap(),
            dividend_lo,
            dividend_hi,
            dst_quotient,
            dst_remainder,
        }
    }

    pub(crate) fn div8(
        sign: DivSignedness,
        trap: TrapCode,
        divisor: RegMem,
        dividend: Gpr,
        dst: WritableGpr,
    ) -> Inst {
        divisor.assert_regclass_is(RegClass::Int);
        Inst::Div8 {
            sign,
            trap,
            divisor: GprMem::new(divisor).unwrap(),
            dividend,
            dst,
        }
    }

    pub(crate) fn imm(dst_size: OperandSize, simm64: u64, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        // Try to generate a 32-bit immediate when the upper high bits are zeroed (which matches
        // the semantics of movl).
        let dst_size = match dst_size {
            OperandSize::Size64 if simm64 > u32::max_value() as u64 => OperandSize::Size64,
            _ => OperandSize::Size32,
        };
        Inst::Imm {
            dst_size,
            simm64,
            dst: WritableGpr::from_writable_reg(dst).unwrap(),
        }
    }

    pub(crate) fn mov_r_r(size: OperandSize, src: Reg, dst: Writable<Reg>) -> Inst {
        debug_assert!(size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(src.class() == RegClass::Int);
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        let src = Gpr::new(src).unwrap();
        let dst = WritableGpr::from_writable_reg(dst).unwrap();
        Inst::MovRR { size, src, dst }
    }

    /// Convenient helper for unary float operations.
    pub(crate) fn xmm_unary_rm_r(op: SseOpcode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmUnaryRmR {
            op,
            src: XmmMemAligned::new(src).unwrap(),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    pub(crate) fn xmm_rm_r(op: SseOpcode, src: RegMem, dst: Writable<Reg>) -> Self {
        src.assert_regclass_is(RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmRmR {
            op,
            src1: Xmm::new(dst.to_reg()).unwrap(),
            src2: XmmMemAligned::new(src).unwrap(),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    #[cfg(test)]
    pub(crate) fn xmm_rmr_vex3(op: AvxOpcode, src3: RegMem, src2: Reg, dst: Writable<Reg>) -> Self {
        src3.assert_regclass_is(RegClass::Float);
        debug_assert!(src2.class() == RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmRmRVex3 {
            op,
            src3: XmmMem::new(src3).unwrap(),
            src2: Xmm::new(src2).unwrap(),
            src1: Xmm::new(dst.to_reg()).unwrap(),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    pub(crate) fn xmm_mov_r_m(op: SseOpcode, src: Reg, dst: impl Into<SyntheticAmode>) -> Inst {
        debug_assert!(src.class() == RegClass::Float);
        Inst::XmmMovRM {
            op,
            src: Xmm::new(src).unwrap(),
            dst: dst.into(),
        }
    }

    pub(crate) fn xmm_to_gpr(
        op: SseOpcode,
        src: Reg,
        dst: Writable<Reg>,
        dst_size: OperandSize,
    ) -> Inst {
        debug_assert!(src.class() == RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        debug_assert!(dst_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        Inst::XmmToGpr {
            op,
            src: Xmm::new(src).unwrap(),
            dst: WritableGpr::from_writable_reg(dst).unwrap(),
            dst_size,
        }
    }

    pub(crate) fn gpr_to_xmm(
        op: SseOpcode,
        src: RegMem,
        src_size: OperandSize,
        dst: Writable<Reg>,
    ) -> Inst {
        src.assert_regclass_is(RegClass::Int);
        debug_assert!(src_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::GprToXmm {
            op,
            src: GprMem::new(src).unwrap(),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
            src_size,
        }
    }

    pub(crate) fn xmm_cmp_rm_r(op: SseOpcode, src1: Reg, src2: RegMem) -> Inst {
        src2.assert_regclass_is(RegClass::Float);
        debug_assert!(src1.class() == RegClass::Float);
        let src2 = XmmMemAligned::new(src2).unwrap();
        let src1 = Xmm::new(src1).unwrap();
        Inst::XmmCmpRmR { op, src1, src2 }
    }

    #[allow(dead_code)]
    pub(crate) fn xmm_min_max_seq(
        size: OperandSize,
        is_min: bool,
        lhs: Reg,
        rhs: Reg,
        dst: Writable<Reg>,
    ) -> Inst {
        debug_assert!(size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert_eq!(lhs.class(), RegClass::Float);
        debug_assert_eq!(rhs.class(), RegClass::Float);
        debug_assert_eq!(dst.to_reg().class(), RegClass::Float);
        Inst::XmmMinMaxSeq {
            size,
            is_min,
            lhs: Xmm::new(lhs).unwrap(),
            rhs: Xmm::new(rhs).unwrap(),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    pub(crate) fn movzx_rm_r(ext_mode: ExtMode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::Int);
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        let src = GprMem::new(src).unwrap();
        let dst = WritableGpr::from_writable_reg(dst).unwrap();
        Inst::MovzxRmR { ext_mode, src, dst }
    }

    pub(crate) fn movsx_rm_r(ext_mode: ExtMode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::Int);
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        let src = GprMem::new(src).unwrap();
        let dst = WritableGpr::from_writable_reg(dst).unwrap();
        Inst::MovsxRmR { ext_mode, src, dst }
    }

    pub(crate) fn mov64_m_r(src: impl Into<SyntheticAmode>, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        Inst::Mov64MR {
            src: src.into(),
            dst: WritableGpr::from_writable_reg(dst).unwrap(),
        }
    }

    pub(crate) fn mov_r_m(size: OperandSize, src: Reg, dst: impl Into<SyntheticAmode>) -> Inst {
        debug_assert!(src.class() == RegClass::Int);
        Inst::MovRM {
            size,
            src: Gpr::new(src).unwrap(),
            dst: dst.into(),
        }
    }

    pub(crate) fn lea(addr: impl Into<SyntheticAmode>, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        Inst::LoadEffectiveAddress {
            addr: addr.into(),
            dst: WritableGpr::from_writable_reg(dst).unwrap(),
            size: OperandSize::Size64,
        }
    }

    pub(crate) fn shift_r(
        size: OperandSize,
        kind: ShiftKind,
        num_bits: Imm8Gpr,
        src: Reg,
        dst: Writable<Reg>,
    ) -> Inst {
        if let &Imm8Reg::Imm8 { imm: num_bits } = num_bits.as_imm8_reg() {
            debug_assert!(num_bits < size.to_bits());
        }
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        Inst::ShiftR {
            size,
            kind,
            src: Gpr::new(src).unwrap(),
            num_bits,
            dst: WritableGpr::from_writable_reg(dst).unwrap(),
        }
    }

    /// Does a comparison of dst - src for operands of size `size`, as stated by the machine
    /// instruction semantics. Be careful with the order of parameters!
    pub(crate) fn cmp_rmi_r(size: OperandSize, src1: Reg, src2: RegMemImm) -> Inst {
        src2.assert_regclass_is(RegClass::Int);
        debug_assert_eq!(src1.class(), RegClass::Int);
        Inst::CmpRmiR {
            size,
            src1: Gpr::new(src1).unwrap(),
            src2: GprMemImm::new(src2).unwrap(),
            opcode: CmpOpcode::Cmp,
        }
    }

    pub(crate) fn trap(trap_code: TrapCode) -> Inst {
        Inst::Ud2 { trap_code }
    }

    pub(crate) fn trap_if(cc: CC, trap_code: TrapCode) -> Inst {
        Inst::TrapIf { cc, trap_code }
    }

    pub(crate) fn cmove(size: OperandSize, cc: CC, src: RegMem, dst: Writable<Reg>) -> Inst {
        debug_assert!(size.is_one_of(&[
            OperandSize::Size16,
            OperandSize::Size32,
            OperandSize::Size64
        ]));
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        Inst::Cmove {
            size,
            cc,
            consequent: GprMem::new(src).unwrap(),
            alternative: Gpr::new(dst.to_reg()).unwrap(),
            dst: WritableGpr::from_writable_reg(dst).unwrap(),
        }
    }

    pub(crate) fn push64(src: RegMemImm) -> Inst {
        src.assert_regclass_is(RegClass::Int);
        let src = GprMemImm::new(src).unwrap();
        Inst::Push64 { src }
    }

    pub(crate) fn pop64(dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        let dst = WritableGpr::from_writable_reg(dst).unwrap();
        Inst::Pop64 { dst }
    }

    pub(crate) fn call_known(
        dest: ExternalName,
        uses: CallArgList,
        defs: CallRetList,
        clobbers: PRegSet,
        opcode: Opcode,
        callee_pop_size: u32,
        callee_conv: CallConv,
    ) -> Inst {
        Inst::CallKnown {
            dest,
            opcode,
            info: Some(Box::new(CallInfo {
                uses,
                defs,
                clobbers,
                callee_pop_size,
                callee_conv,
            })),
        }
    }

    pub(crate) fn call_unknown(
        dest: RegMem,
        uses: CallArgList,
        defs: CallRetList,
        clobbers: PRegSet,
        opcode: Opcode,
        callee_pop_size: u32,
        callee_conv: CallConv,
    ) -> Inst {
        dest.assert_regclass_is(RegClass::Int);
        Inst::CallUnknown {
            dest,
            opcode,
            info: Some(Box::new(CallInfo {
                uses,
                defs,
                clobbers,
                callee_pop_size,
                callee_conv,
            })),
        }
    }

    pub(crate) fn ret(stack_bytes_to_pop: u32) -> Inst {
        Inst::Ret { stack_bytes_to_pop }
    }

    pub(crate) fn jmp_known(dst: MachLabel) -> Inst {
        Inst::JmpKnown { dst }
    }

    pub(crate) fn jmp_unknown(target: RegMem) -> Inst {
        target.assert_regclass_is(RegClass::Int);
        Inst::JmpUnknown { target }
    }

    /// Choose which instruction to use for loading a register value from memory. For loads smaller
    /// than 64 bits, this method expects a way to extend the value (i.e. [ExtKind::SignExtend],
    /// [ExtKind::ZeroExtend]); loads with no extension necessary will ignore this.
    pub(crate) fn load(
        ty: Type,
        from_addr: impl Into<SyntheticAmode>,
        to_reg: Writable<Reg>,
        ext_kind: ExtKind,
    ) -> Inst {
        let rc = to_reg.to_reg().class();
        match rc {
            RegClass::Int => {
                let ext_mode = match ty.bytes() {
                    1 => Some(ExtMode::BQ),
                    2 => Some(ExtMode::WQ),
                    4 => Some(ExtMode::LQ),
                    8 => None,
                    _ => unreachable!("the type should never use a scalar load: {}", ty),
                };
                if let Some(ext_mode) = ext_mode {
                    // Values smaller than 64 bits must be extended in some way.
                    match ext_kind {
                        ExtKind::SignExtend => {
                            Inst::movsx_rm_r(ext_mode, RegMem::mem(from_addr), to_reg)
                        }
                        ExtKind::ZeroExtend => {
                            Inst::movzx_rm_r(ext_mode, RegMem::mem(from_addr), to_reg)
                        }
                        ExtKind::None => panic!(
                            "expected an extension kind for extension mode: {:?}",
                            ext_mode
                        ),
                    }
                } else {
                    // 64-bit values can be moved directly.
                    Inst::mov64_m_r(from_addr, to_reg)
                }
            }
            RegClass::Float => {
                let opcode = match ty {
                    types::F32 => SseOpcode::Movss,
                    types::F64 => SseOpcode::Movsd,
                    types::F32X4 => SseOpcode::Movups,
                    types::F64X2 => SseOpcode::Movupd,
                    _ if ty.is_vector() && ty.bits() == 128 => SseOpcode::Movdqu,
                    _ => unimplemented!("unable to load type: {}", ty),
                };
                Inst::xmm_unary_rm_r(opcode, RegMem::mem(from_addr), to_reg)
            }
            RegClass::Vector => unreachable!(),
        }
    }

    /// Choose which instruction to use for storing a register value to memory.
    pub(crate) fn store(ty: Type, from_reg: Reg, to_addr: impl Into<SyntheticAmode>) -> Inst {
        let rc = from_reg.class();
        match rc {
            RegClass::Int => Inst::mov_r_m(OperandSize::from_ty(ty), from_reg, to_addr),
            RegClass::Float => {
                let opcode = match ty {
                    types::F32 => SseOpcode::Movss,
                    types::F64 => SseOpcode::Movsd,
                    types::F32X4 => SseOpcode::Movups,
                    types::F64X2 => SseOpcode::Movupd,
                    _ if ty.is_vector() && ty.bits() == 128 => SseOpcode::Movdqu,
                    _ => unimplemented!("unable to store type: {}", ty),
                };
                Inst::xmm_mov_r_m(opcode, from_reg, to_addr)
            }
            RegClass::Vector => unreachable!(),
        }
    }
}

//=============================================================================
// Instructions: printing

impl PrettyPrint for Inst {
    fn pretty_print(&self, _size: u8, allocs: &mut AllocationConsumer) -> String {
        fn ljustify(s: String) -> String {
            let w = 7;
            if s.len() >= w {
                s
            } else {
                let need = usize::min(w, w - s.len());
                s + &format!("{nil: <width$}", nil = "", width = need)
            }
        }

        fn ljustify2(s1: String, s2: String) -> String {
            ljustify(s1 + &s2)
        }

        fn suffix_lq(size: OperandSize) -> String {
            match size {
                OperandSize::Size32 => "l",
                OperandSize::Size64 => "q",
                _ => unreachable!(),
            }
            .to_string()
        }

        #[allow(dead_code)]
        fn suffix_lqb(size: OperandSize) -> String {
            match size {
                OperandSize::Size32 => "l",
                OperandSize::Size64 => "q",
                _ => unreachable!(),
            }
            .to_string()
        }

        fn suffix_bwlq(size: OperandSize) -> String {
            match size {
                OperandSize::Size8 => "b".to_string(),
                OperandSize::Size16 => "w".to_string(),
                OperandSize::Size32 => "l".to_string(),
                OperandSize::Size64 => "q".to_string(),
            }
        }

        match self {
            Inst::Nop { len } => format!("{} len={}", ljustify("nop".to_string()), len),

            Inst::AluRmiR {
                size,
                op,
                src1,
                src2,
                dst,
            } => {
                let size_bytes = size.to_bytes();
                let src1 = pretty_print_reg(src1.to_reg(), size_bytes, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size_bytes, allocs);
                let src2 = src2.pretty_print(size_bytes, allocs);
                let op = ljustify2(op.to_string(), suffix_bwlq(*size));
                format!("{op} {src1}, {src2}, {dst}")
            }
            Inst::AluConstOp { op, dst, size } => {
                let size_bytes = size.to_bytes();
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size_bytes, allocs);
                let op = ljustify2(op.to_string(), suffix_lqb(*size));
                format!("{op} {dst}, {dst}, {dst}")
            }
            Inst::AluRM {
                size,
                op,
                src1_dst,
                src2,
            } => {
                let size_bytes = size.to_bytes();
                let src2 = pretty_print_reg(src2.to_reg(), size_bytes, allocs);
                let src1_dst = src1_dst.pretty_print(size_bytes, allocs);
                let op = ljustify2(op.to_string(), suffix_bwlq(*size));
                format!("{op} {src2}, {src1_dst}")
            }
            Inst::AluRmRVex {
                size,
                op,
                src1,
                src2,
                dst,
            } => {
                let size_bytes = size.to_bytes();
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let src1 = pretty_print_reg(src1.to_reg(), size_bytes, allocs);
                let src2 = src2.pretty_print(size_bytes, allocs);
                let op = ljustify2(op.to_string(), String::new());
                format!("{op} {src2}, {src1}, {dst}")
            }
            Inst::UnaryRmR { src, dst, op, size } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let src = src.pretty_print(size.to_bytes(), allocs);
                let op = ljustify2(op.to_string(), suffix_bwlq(*size));
                format!("{op} {src}, {dst}")
            }

            Inst::UnaryRmRVex { src, dst, op, size } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let src = src.pretty_print(size.to_bytes(), allocs);
                let op = ljustify2(op.to_string(), suffix_bwlq(*size));
                format!("{op} {src}, {dst}")
            }

            Inst::UnaryRmRImmVex {
                src,
                dst,
                op,
                size,
                imm,
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let src = src.pretty_print(size.to_bytes(), allocs);
                format!(
                    "{} ${imm}, {src}, {dst}",
                    ljustify2(op.to_string(), suffix_bwlq(*size))
                )
            }

            Inst::Not { size, src, dst } => {
                let src = pretty_print_reg(src.to_reg(), size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let op = ljustify2("not".to_string(), suffix_bwlq(*size));
                format!("{op} {src}, {dst}")
            }

            Inst::Neg { size, src, dst } => {
                let src = pretty_print_reg(src.to_reg(), size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let op = ljustify2("neg".to_string(), suffix_bwlq(*size));
                format!("{op} {src}, {dst}")
            }

            Inst::Div {
                size,
                sign,
                trap,
                divisor,
                dividend_lo,
                dividend_hi,
                dst_quotient,
                dst_remainder,
            } => {
                let divisor = divisor.pretty_print(size.to_bytes(), allocs);
                let dividend_lo = pretty_print_reg(dividend_lo.to_reg(), size.to_bytes(), allocs);
                let dividend_hi = pretty_print_reg(dividend_hi.to_reg(), size.to_bytes(), allocs);
                let dst_quotient =
                    pretty_print_reg(dst_quotient.to_reg().to_reg(), size.to_bytes(), allocs);
                let dst_remainder =
                    pretty_print_reg(dst_remainder.to_reg().to_reg(), size.to_bytes(), allocs);
                let op = ljustify(match sign {
                    DivSignedness::Signed => "idiv".to_string(),
                    DivSignedness::Unsigned => "div".to_string(),
                });
                format!(
                    "{op} {dividend_lo}, {dividend_hi}, {divisor}, {dst_quotient}, {dst_remainder} ; trap={trap}"
                )
            }

            Inst::Div8 {
                sign,
                trap,
                divisor,
                dividend,
                dst,
            } => {
                let divisor = divisor.pretty_print(1, allocs);
                let dividend = pretty_print_reg(dividend.to_reg(), 1, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 1, allocs);
                let op = ljustify(match sign {
                    DivSignedness::Signed => "idiv".to_string(),
                    DivSignedness::Unsigned => "div".to_string(),
                });
                format!("{op} {dividend}, {divisor}, {dst} ; trap={trap}")
            }

            Inst::Mul {
                size,
                signed,
                src1,
                src2,
                dst_lo,
                dst_hi,
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), size.to_bytes(), allocs);
                let dst_lo = pretty_print_reg(dst_lo.to_reg().to_reg(), size.to_bytes(), allocs);
                let dst_hi = pretty_print_reg(dst_hi.to_reg().to_reg(), size.to_bytes(), allocs);
                let src2 = src2.pretty_print(size.to_bytes(), allocs);
                let suffix = suffix_bwlq(*size);
                let op = ljustify(if *signed {
                    format!("imul{suffix}")
                } else {
                    format!("mul{suffix}")
                });
                format!("{op} {src1}, {src2}, {dst_lo}, {dst_hi}")
            }

            Inst::Mul8 {
                signed,
                src1,
                src2,
                dst,
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 1, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 1, allocs);
                let src2 = src2.pretty_print(1, allocs);
                let op = ljustify(if *signed {
                    "imulb".to_string()
                } else {
                    "mulb".to_string()
                });
                format!("{op} {src1}, {src2}, {dst}")
            }

            Inst::IMul {
                size,
                src1,
                src2,
                dst,
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let src2 = src2.pretty_print(size.to_bytes(), allocs);
                let suffix = suffix_bwlq(*size);
                let op = ljustify(format!("imul{suffix}"));
                format!("{op} {src1}, {src2}, {dst}")
            }

            Inst::IMulImm {
                size,
                src1,
                src2,
                dst,
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let src1 = src1.pretty_print(size.to_bytes(), allocs);
                let suffix = suffix_bwlq(*size);
                let op = ljustify(format!("imul{suffix}"));
                format!("{op} {src1}, {src2:#x}, {dst}")
            }

            Inst::CheckedSRemSeq {
                size,
                divisor,
                dividend_lo,
                dividend_hi,
                dst_quotient,
                dst_remainder,
            } => {
                let divisor = pretty_print_reg(divisor.to_reg(), size.to_bytes(), allocs);
                let dividend_lo = pretty_print_reg(dividend_lo.to_reg(), size.to_bytes(), allocs);
                let dividend_hi = pretty_print_reg(dividend_hi.to_reg(), size.to_bytes(), allocs);
                let dst_quotient =
                    pretty_print_reg(dst_quotient.to_reg().to_reg(), size.to_bytes(), allocs);
                let dst_remainder =
                    pretty_print_reg(dst_remainder.to_reg().to_reg(), size.to_bytes(), allocs);
                format!(
                    "checked_srem_seq {dividend_lo}, {dividend_hi}, \
                        {divisor}, {dst_quotient}, {dst_remainder}",
                )
            }

            Inst::CheckedSRemSeq8 {
                divisor,
                dividend,
                dst,
            } => {
                let divisor = pretty_print_reg(divisor.to_reg(), 1, allocs);
                let dividend = pretty_print_reg(dividend.to_reg(), 1, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 1, allocs);
                format!("checked_srem_seq {dividend}, {divisor}, {dst}")
            }

            Inst::SignExtendData { size, src, dst } => {
                let src = pretty_print_reg(src.to_reg(), size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let op = match size {
                    OperandSize::Size8 => "cbw",
                    OperandSize::Size16 => "cwd",
                    OperandSize::Size32 => "cdq",
                    OperandSize::Size64 => "cqo",
                };
                format!("{op} {src}, {dst}")
            }

            Inst::XmmUnaryRmR { op, src, dst, .. } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), op.src_size(), allocs);
                let src = src.pretty_print(op.src_size(), allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::XmmUnaryRmRUnaligned { op, src, dst, .. } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), op.src_size(), allocs);
                let src = src.pretty_print(op.src_size(), allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::XmmUnaryRmRImm {
                op, src, dst, imm, ..
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), op.src_size(), allocs);
                let src = src.pretty_print(op.src_size(), allocs);
                let op = ljustify(op.to_string());
                format!("{op} ${imm}, {src}, {dst}")
            }

            Inst::XmmUnaryRmRVex { op, src, dst, .. } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src = src.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::XmmUnaryRmRImmVex {
                op, src, dst, imm, ..
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src = src.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} ${imm}, {src}, {dst}")
            }

            Inst::XmmUnaryRmREvex { op, src, dst, .. } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src = src.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::XmmUnaryRmRImmEvex {
                op, src, dst, imm, ..
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src = src.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} ${imm}, {src}, {dst}")
            }

            Inst::XmmMovRM { op, src, dst, .. } => {
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst = dst.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::XmmMovRMVex { op, src, dst, .. } => {
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst = dst.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::XmmMovRMImm {
                op, src, dst, imm, ..
            } => {
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst = dst.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} ${imm}, {src}, {dst}")
            }

            Inst::XmmMovRMImmVex {
                op, src, dst, imm, ..
            } => {
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst = dst.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} ${imm}, {src}, {dst}")
            }

            Inst::XmmRmR {
                op,
                src1,
                src2,
                dst,
                ..
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src1}, {src2}, {dst}")
            }

            Inst::XmmRmRUnaligned {
                op,
                src1,
                src2,
                dst,
                ..
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src1}, {src2}, {dst}")
            }

            Inst::XmmRmRBlend {
                op,
                src1,
                src2,
                mask,
                dst,
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let mask = allocs.next(mask.to_reg());
                let mask = if mask.is_virtual() {
                    format!(" <{}>", show_ireg_sized(mask, 8))
                } else {
                    debug_assert_eq!(mask, regs::xmm0());
                    String::new()
                };
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src1}, {src2}, {dst}{mask}")
            }

            Inst::XmmRmiRVex {
                op,
                src1,
                src2,
                dst,
                ..
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src1}, {src2}, {dst}")
            }

            Inst::XmmRmRImmVex {
                op,
                src1,
                src2,
                dst,
                imm,
                ..
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} ${imm}, {src1}, {src2}, {dst}")
            }

            Inst::XmmVexPinsr {
                op,
                src1,
                src2,
                dst,
                imm,
                ..
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} ${imm}, {src1}, {src2}, {dst}")
            }

            Inst::XmmRmRVex3 {
                op,
                src1,
                src2,
                src3,
                dst,
                ..
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src2 = pretty_print_reg(src2.to_reg(), 8, allocs);
                let src3 = src3.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src1}, {src2}, {src3}, {dst}")
            }

            Inst::XmmRmRBlendVex {
                op,
                src1,
                src2,
                mask,
                dst,
                ..
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let mask = pretty_print_reg(mask.to_reg(), 8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src1}, {src2}, {mask}, {dst}")
            }

            Inst::XmmRmREvex {
                op,
                src1,
                src2,
                dst,
                ..
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src2}, {src1}, {dst}")
            }

            Inst::XmmRmREvex3 {
                op,
                src1,
                src2,
                src3,
                dst,
                ..
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let src2 = pretty_print_reg(src2.to_reg(), 8, allocs);
                let src3 = src3.pretty_print(8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src3}, {src2}, {src1}, {dst}")
            }

            Inst::XmmMinMaxSeq {
                lhs,
                rhs,
                dst,
                is_min,
                size,
            } => {
                let rhs = pretty_print_reg(rhs.to_reg(), 8, allocs);
                let lhs = pretty_print_reg(lhs.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let op = ljustify2(
                    if *is_min {
                        "xmm min seq ".to_string()
                    } else {
                        "xmm max seq ".to_string()
                    },
                    format!("f{}", size.to_bits()),
                );
                format!("{op} {lhs}, {rhs}, {dst}")
            }

            Inst::XmmRmRImm {
                op,
                src1,
                src2,
                dst,
                imm,
                size,
                ..
            } => {
                let src1 = pretty_print_reg(*src1, 8, allocs);
                let dst = pretty_print_reg(dst.to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let op = ljustify(format!(
                    "{}{}",
                    op.to_string(),
                    if *size == OperandSize::Size64 {
                        ".w"
                    } else {
                        ""
                    }
                ));
                format!("{op} ${imm}, {src1}, {src2}, {dst}")
            }

            Inst::XmmUninitializedValue { dst } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let op = ljustify("uninit".into());
                format!("{op} {dst}")
            }

            Inst::XmmToGpr {
                op,
                src,
                dst,
                dst_size,
            } => {
                let dst_size = dst_size.to_bytes();
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), dst_size, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::XmmToGprVex {
                op,
                src,
                dst,
                dst_size,
            } => {
                let dst_size = dst_size.to_bytes();
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), dst_size, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::XmmToGprImm { op, src, dst, imm } => {
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} ${imm}, {src}, {dst}")
            }

            Inst::XmmToGprImmVex { op, src, dst, imm } => {
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} ${imm}, {src}, {dst}")
            }

            Inst::GprToXmm {
                op,
                src,
                src_size,
                dst,
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src = src.pretty_print(src_size.to_bytes(), allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::GprToXmmVex {
                op,
                src,
                src_size,
                dst,
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src = src.pretty_print(src_size.to_bytes(), allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::XmmCmpRmR { op, src1, src2 } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src2}, {src1}")
            }

            Inst::CvtIntToFloat {
                op,
                src1,
                src2,
                dst,
                src2_size,
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let dst = pretty_print_reg(*dst.to_reg(), 8, allocs);
                let src2 = src2.pretty_print(src2_size.to_bytes(), allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src1}, {src2}, {dst}")
            }

            Inst::CvtIntToFloatVex {
                op,
                src1,
                src2,
                dst,
                src2_size,
            } => {
                let dst = pretty_print_reg(*dst.to_reg(), 8, allocs);
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let src2 = src2.pretty_print(src2_size.to_bytes(), allocs);
                let op = ljustify(op.to_string());
                format!("{op} {src1}, {src2}, {dst}")
            }

            Inst::XmmCmpRmRVex { op, src1, src2 } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                format!("{} {src2}, {src1}", ljustify(op.to_string()))
            }

            Inst::CvtUint64ToFloatSeq {
                src,
                dst,
                dst_size,
                tmp_gpr1,
                tmp_gpr2,
                ..
            } => {
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), dst_size.to_bytes(), allocs);
                let tmp_gpr1 = pretty_print_reg(tmp_gpr1.to_reg().to_reg(), 8, allocs);
                let tmp_gpr2 = pretty_print_reg(tmp_gpr2.to_reg().to_reg(), 8, allocs);
                let op = ljustify(format!(
                    "u64_to_{}_seq",
                    if *dst_size == OperandSize::Size64 {
                        "f64"
                    } else {
                        "f32"
                    }
                ));
                format!("{op} {src}, {dst}, {tmp_gpr1}, {tmp_gpr2}")
            }

            Inst::CvtFloatToSintSeq {
                src,
                dst,
                src_size,
                dst_size,
                tmp_xmm,
                tmp_gpr,
                is_saturating,
            } => {
                let src = pretty_print_reg(src.to_reg(), src_size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), dst_size.to_bytes(), allocs);
                let tmp_gpr = pretty_print_reg(tmp_gpr.to_reg().to_reg(), 8, allocs);
                let tmp_xmm = pretty_print_reg(tmp_xmm.to_reg().to_reg(), 8, allocs);
                let op = ljustify(format!(
                    "cvt_float{}_to_sint{}{}_seq",
                    src_size.to_bits(),
                    dst_size.to_bits(),
                    if *is_saturating { "_sat" } else { "" },
                ));
                format!("{op} {src}, {dst}, {tmp_gpr}, {tmp_xmm}")
            }

            Inst::CvtFloatToUintSeq {
                src,
                dst,
                src_size,
                dst_size,
                tmp_gpr,
                tmp_xmm,
                tmp_xmm2,
                is_saturating,
            } => {
                let src = pretty_print_reg(src.to_reg(), src_size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), dst_size.to_bytes(), allocs);
                let tmp_gpr = pretty_print_reg(tmp_gpr.to_reg().to_reg(), 8, allocs);
                let tmp_xmm = pretty_print_reg(tmp_xmm.to_reg().to_reg(), 8, allocs);
                let tmp_xmm2 = pretty_print_reg(tmp_xmm2.to_reg().to_reg(), 8, allocs);
                let op = ljustify(format!(
                    "cvt_float{}_to_uint{}{}_seq",
                    src_size.to_bits(),
                    dst_size.to_bits(),
                    if *is_saturating { "_sat" } else { "" },
                ));
                format!("{op} {src}, {dst}, {tmp_gpr}, {tmp_xmm}, {tmp_xmm2}")
            }

            Inst::Imm {
                dst_size,
                simm64,
                dst,
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), dst_size.to_bytes(), allocs);
                if *dst_size == OperandSize::Size64 {
                    let op = ljustify("movabsq".to_string());
                    let imm = *simm64 as i64;
                    format!("{op} ${imm}, {dst}")
                } else {
                    let op = ljustify("movl".to_string());
                    let imm = (*simm64 as u32) as i32;
                    format!("{op} ${imm}, {dst}")
                }
            }

            Inst::MovImmM { size, simm32, dst } => {
                let dst = dst.pretty_print(size.to_bytes(), allocs);
                let suffix = suffix_bwlq(*size);
                let imm = match *size {
                    OperandSize::Size8 => ((*simm32 as u8) as i8).to_string(),
                    OperandSize::Size16 => ((*simm32 as u16) as i16).to_string(),
                    OperandSize::Size32 => simm32.to_string(),
                    OperandSize::Size64 => (*simm32 as i64).to_string(),
                };
                let op = ljustify2("mov".to_string(), suffix);
                format!("{op} ${imm}, {dst}")
            }

            Inst::MovRR { size, src, dst } => {
                let src = pretty_print_reg(src.to_reg(), size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let op = ljustify2("mov".to_string(), suffix_lq(*size));
                format!("{op} {src}, {dst}")
            }

            Inst::MovFromPReg { src, dst } => {
                let src: Reg = (*src).into();
                let src = regs::show_ireg_sized(src, 8);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let op = ljustify("movq".to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::MovToPReg { src, dst } => {
                let src = pretty_print_reg(src.to_reg(), 8, allocs);
                let dst: Reg = (*dst).into();
                let dst = regs::show_ireg_sized(dst, 8);
                let op = ljustify("movq".to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::MovzxRmR {
                ext_mode, src, dst, ..
            } => {
                let dst_size = if *ext_mode == ExtMode::LQ {
                    4
                } else {
                    ext_mode.dst_size()
                };
                let dst = pretty_print_reg(dst.to_reg().to_reg(), dst_size, allocs);
                let src = src.pretty_print(ext_mode.src_size(), allocs);

                if *ext_mode == ExtMode::LQ {
                    let op = ljustify("movl".to_string());
                    format!("{op} {src}, {dst}")
                } else {
                    let op = ljustify2("movz".to_string(), ext_mode.to_string());
                    format!("{op} {src}, {dst}")
                }
            }

            Inst::Mov64MR { src, dst, .. } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src = src.pretty_print(8, allocs);
                let op = ljustify("movq".to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::LoadEffectiveAddress { addr, dst, size } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let addr = addr.pretty_print(8, allocs);
                let op = ljustify("lea".to_string());
                format!("{op} {addr}, {dst}")
            }

            Inst::MovsxRmR {
                ext_mode, src, dst, ..
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), ext_mode.dst_size(), allocs);
                let src = src.pretty_print(ext_mode.src_size(), allocs);
                let op = ljustify2("movs".to_string(), ext_mode.to_string());
                format!("{op} {src}, {dst}")
            }

            Inst::MovRM { size, src, dst, .. } => {
                let src = pretty_print_reg(src.to_reg(), size.to_bytes(), allocs);
                let dst = dst.pretty_print(size.to_bytes(), allocs);
                let op = ljustify2("mov".to_string(), suffix_bwlq(*size));
                format!("{op} {src}, {dst}")
            }

            Inst::ShiftR {
                size,
                kind,
                num_bits,
                src,
                dst,
                ..
            } => {
                let src = pretty_print_reg(src.to_reg(), size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                match num_bits.as_imm8_reg() {
                    &Imm8Reg::Reg { reg } => {
                        let reg = pretty_print_reg(reg, 1, allocs);
                        let op = ljustify2(kind.to_string(), suffix_bwlq(*size));
                        format!("{op} {reg}, {src}, {dst}")
                    }

                    &Imm8Reg::Imm8 { imm: num_bits } => {
                        let op = ljustify2(kind.to_string(), suffix_bwlq(*size));
                        format!("{op} ${num_bits}, {src}, {dst}")
                    }
                }
            }

            Inst::XmmRmiReg {
                opcode,
                src1,
                src2,
                dst,
                ..
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), 8, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let src2 = src2.pretty_print(8, allocs);
                let op = ljustify(opcode.to_string());
                format!("{op} {src1}, {src2}, {dst}")
            }

            Inst::CmpRmiR {
                size,
                src1,
                src2,
                opcode,
            } => {
                let src1 = pretty_print_reg(src1.to_reg(), size.to_bytes(), allocs);
                let src2 = src2.pretty_print(size.to_bytes(), allocs);
                let op = match opcode {
                    CmpOpcode::Cmp => "cmp",
                    CmpOpcode::Test => "test",
                };
                let op = ljustify2(op.to_string(), suffix_bwlq(*size));
                format!("{op} {src2}, {src1}")
            }

            Inst::Setcc { cc, dst } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 1, allocs);
                let op = ljustify2("set".to_string(), cc.to_string());
                format!("{op} {dst}")
            }

            Inst::Bswap { size, src, dst } => {
                let src = pretty_print_reg(src.to_reg(), size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let op = ljustify2("bswap".to_string(), suffix_bwlq(*size));
                format!("{op} {src}, {dst}")
            }

            Inst::Cmove {
                size,
                cc,
                consequent,
                alternative,
                dst,
            } => {
                let alternative = pretty_print_reg(alternative.to_reg(), size.to_bytes(), allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size.to_bytes(), allocs);
                let consequent = consequent.pretty_print(size.to_bytes(), allocs);
                let op = ljustify(format!("cmov{}{}", cc.to_string(), suffix_bwlq(*size)));
                format!("{op} {consequent}, {alternative}, {dst}")
            }

            Inst::XmmCmove {
                ty,
                cc,
                consequent,
                alternative,
                dst,
                ..
            } => {
                let size = u8::try_from(ty.bytes()).unwrap();
                let alternative = pretty_print_reg(alternative.to_reg(), size, allocs);
                let dst = pretty_print_reg(dst.to_reg().to_reg(), size, allocs);
                let consequent = pretty_print_reg(consequent.to_reg(), size, allocs);
                let suffix = match *ty {
                    types::F64 => "sd",
                    types::F32 => "ss",
                    types::F32X4 => "aps",
                    types::F64X2 => "apd",
                    _ => "dqa",
                };
                let cc = cc.invert();
                format!(
                    "mov{suffix} {alternative}, {dst}; \
                    j{cc} $next; \
                    mov{suffix} {consequent}, {dst}; \
                    $next:"
                )
            }

            Inst::Push64 { src } => {
                let src = src.pretty_print(8, allocs);
                let op = ljustify("pushq".to_string());
                format!("{op} {src}")
            }

            Inst::StackProbeLoop {
                tmp,
                frame_size,
                guard_size,
            } => {
                let tmp = pretty_print_reg(tmp.to_reg(), 8, allocs);
                let op = ljustify("stack_probe_loop".to_string());
                format!("{op} {tmp}, frame_size={frame_size}, guard_size={guard_size}")
            }

            Inst::Pop64 { dst } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let op = ljustify("popq".to_string());
                format!("{op} {dst}")
            }

            Inst::CallKnown { dest, .. } => {
                let op = ljustify("call".to_string());
                format!("{op} {dest:?}")
            }

            Inst::CallUnknown { dest, .. } => {
                let dest = dest.pretty_print(8, allocs);
                let op = ljustify("call".to_string());
                format!("{op} *{dest}")
            }

            Inst::ReturnCallKnown { callee, info } => {
                let ReturnCallInfo {
                    uses,
                    new_stack_arg_size,
                    tmp,
                } = &**info;
                let tmp = pretty_print_reg(tmp.to_reg().to_reg(), 8, allocs);
                let mut s =
                    format!("return_call_known {callee:?} ({new_stack_arg_size}) tmp={tmp}");
                for ret in uses {
                    let preg = regs::show_reg(ret.preg);
                    let vreg = pretty_print_reg(ret.vreg, 8, allocs);
                    write!(&mut s, " {vreg}={preg}").unwrap();
                }
                s
            }

            Inst::ReturnCallUnknown { callee, info } => {
                let ReturnCallInfo {
                    uses,
                    new_stack_arg_size,
                    tmp,
                } = &**info;
                let callee = pretty_print_reg(*callee, 8, allocs);
                let tmp = pretty_print_reg(tmp.to_reg().to_reg(), 8, allocs);
                let mut s =
                    format!("return_call_unknown {callee} ({new_stack_arg_size}) tmp={tmp}");
                for ret in uses {
                    let preg = regs::show_reg(ret.preg);
                    let vreg = pretty_print_reg(ret.vreg, 8, allocs);
                    write!(&mut s, " {vreg}={preg}").unwrap();
                }
                s
            }

            Inst::Args { args } => {
                let mut s = "args".to_string();
                for arg in args {
                    let preg = regs::show_reg(arg.preg);
                    let def = pretty_print_reg(arg.vreg.to_reg(), 8, allocs);
                    write!(&mut s, " {def}={preg}").unwrap();
                }
                s
            }

            Inst::Rets { rets } => {
                let mut s = "rets".to_string();
                for ret in rets {
                    let preg = regs::show_reg(ret.preg);
                    let vreg = pretty_print_reg(ret.vreg, 8, allocs);
                    write!(&mut s, " {vreg}={preg}").unwrap();
                }
                s
            }

            Inst::Ret { stack_bytes_to_pop } => {
                let mut s = "ret".to_string();
                if *stack_bytes_to_pop != 0 {
                    write!(&mut s, " {stack_bytes_to_pop}").unwrap();
                }
                s
            }

            Inst::JmpKnown { dst } => {
                let op = ljustify("jmp".to_string());
                let dst = dst.to_string();
                format!("{op} {dst}")
            }

            Inst::JmpIf { cc, taken } => {
                let taken = taken.to_string();
                let op = ljustify2("j".to_string(), cc.to_string());
                format!("{op} {taken}")
            }

            Inst::JmpCond {
                cc,
                taken,
                not_taken,
            } => {
                let taken = taken.to_string();
                let not_taken = not_taken.to_string();
                let op = ljustify2("j".to_string(), cc.to_string());
                format!("{op} {taken}; j {not_taken}")
            }

            Inst::JmpTableSeq {
                idx, tmp1, tmp2, ..
            } => {
                let idx = pretty_print_reg(*idx, 8, allocs);
                let tmp1 = pretty_print_reg(tmp1.to_reg(), 8, allocs);
                let tmp2 = pretty_print_reg(tmp2.to_reg(), 8, allocs);
                let op = ljustify("br_table".into());
                format!("{op} {idx}, {tmp1}, {tmp2}")
            }

            Inst::JmpUnknown { target } => {
                let target = target.pretty_print(8, allocs);
                let op = ljustify("jmp".to_string());
                format!("{op} *{target}")
            }

            Inst::TrapIf { cc, trap_code, .. } => {
                format!("j{cc} #trap={trap_code}")
            }

            Inst::TrapIfAnd {
                cc1,
                cc2,
                trap_code,
                ..
            } => {
                let cc1 = cc1.invert();
                let cc2 = cc2.invert();
                format!("trap_if_and {cc1}, {cc2}, {trap_code}")
            }

            Inst::TrapIfOr {
                cc1,
                cc2,
                trap_code,
                ..
            } => {
                let cc2 = cc2.invert();
                format!("trap_if_or {cc1}, {cc2}, {trap_code}")
            }

            Inst::LoadExtName {
                dst, name, offset, ..
            } => {
                let dst = pretty_print_reg(dst.to_reg(), 8, allocs);
                let name = name.display(None);
                let op = ljustify("load_ext_name".into());
                format!("{op} {name}+{offset}, {dst}")
            }

            Inst::LockCmpxchg {
                ty,
                replacement,
                expected,
                mem,
                dst_old,
                ..
            } => {
                let size = ty.bytes() as u8;
                let replacement = pretty_print_reg(*replacement, size, allocs);
                let expected = pretty_print_reg(*expected, size, allocs);
                let dst_old = pretty_print_reg(dst_old.to_reg(), size, allocs);
                let mem = mem.pretty_print(size, allocs);
                let suffix = suffix_bwlq(OperandSize::from_bytes(size as u32));
                format!(
                    "lock cmpxchg{suffix} {replacement}, {mem}, expected={expected}, dst_old={dst_old}"
                )
            }

            Inst::AtomicRmwSeq { ty, op, .. } => {
                let ty = ty.bits();
                format!(
                    "atomically {{ {ty}_bits_at_[%r9]) {op:?}= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }}"
                )
            }

            Inst::Fence { kind } => match kind {
                FenceKind::MFence => "mfence".to_string(),
                FenceKind::LFence => "lfence".to_string(),
                FenceKind::SFence => "sfence".to_string(),
            },

            Inst::VirtualSPOffsetAdj { offset } => format!("virtual_sp_offset_adjust {offset}"),

            Inst::Hlt => "hlt".into(),

            Inst::Ud2 { trap_code } => format!("ud2 {trap_code}"),

            Inst::ElfTlsGetAddr { ref symbol, dst } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                format!("{dst} = elf_tls_get_addr {symbol:?}")
            }

            Inst::MachOTlsGetAddr { ref symbol, dst } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                format!("{dst} = macho_tls_get_addr {symbol:?}")
            }

            Inst::CoffTlsGetAddr {
                ref symbol,
                dst,
                tmp,
            } => {
                let dst = pretty_print_reg(dst.to_reg().to_reg(), 8, allocs);
                let tmp = allocs.next(tmp.to_reg().to_reg());

                let mut s = format!("{dst} = coff_tls_get_addr {symbol:?}");
                if tmp.is_virtual() {
                    let tmp = show_ireg_sized(tmp, 8);
                    write!(&mut s, ", {tmp}").unwrap();
                };

                s
            }

            Inst::Unwind { inst } => format!("unwind {inst:?}"),

            Inst::DummyUse { reg } => {
                let reg = pretty_print_reg(*reg, 8, allocs);
                format!("dummy_use {reg}")
            }
        }
    }
}

impl fmt::Debug for Inst {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "{}",
            self.pretty_print_inst(&[], &mut Default::default())
        )
    }
}

fn x64_get_operands(inst: &mut Inst, collector: &mut impl OperandVisitor) {
    // Note: because we need to statically know the indices of each
    // reg in the operands list in order to fetch its allocation
    // later, we put the variable-operand-count bits (the RegMem,
    // RegMemImm, etc args) last. regalloc2 doesn't care what order
    // the operands come in; they can be freely reordered.

    // N.B.: we MUST keep the below in careful sync with (i) emission,
    // in `emit.rs`, and (ii) pretty-printing, in the `pretty_print`
    // method above.
    match inst {
        Inst::AluRmiR {
            src1, src2, dst, ..
        } => {
            collector.reg_use(src1);
            collector.reg_reuse_def(dst, 0);
            src2.get_operands(collector);
        }
        Inst::AluConstOp { dst, .. } => collector.reg_def(dst),
        Inst::AluRM { src1_dst, src2, .. } => {
            collector.reg_use(src2);
            src1_dst.get_operands(collector);
        }
        Inst::AluRmRVex {
            src1, src2, dst, ..
        } => {
            collector.reg_def(dst);
            collector.reg_use(src1);
            src2.get_operands(collector);
        }
        Inst::Not { src, dst, .. } => {
            collector.reg_use(src);
            collector.reg_reuse_def(dst, 0);
        }
        Inst::Neg { src, dst, .. } => {
            collector.reg_use(src);
            collector.reg_reuse_def(dst, 0);
        }
        Inst::Div {
            divisor,
            dividend_lo,
            dividend_hi,
            dst_quotient,
            dst_remainder,
            ..
        } => {
            divisor.get_operands(collector);
            collector.reg_fixed_use(dividend_lo, regs::rax());
            collector.reg_fixed_use(dividend_hi, regs::rdx());
            collector.reg_fixed_def(dst_quotient, regs::rax());
            collector.reg_fixed_def(dst_remainder, regs::rdx());
        }
        Inst::CheckedSRemSeq {
            divisor,
            dividend_lo,
            dividend_hi,
            dst_quotient,
            dst_remainder,
            ..
        } => {
            collector.reg_use(divisor);
            collector.reg_fixed_use(dividend_lo, regs::rax());
            collector.reg_fixed_use(dividend_hi, regs::rdx());
            collector.reg_fixed_def(dst_quotient, regs::rax());
            collector.reg_fixed_def(dst_remainder, regs::rdx());
        }
        Inst::Div8 {
            divisor,
            dividend,
            dst,
            ..
        } => {
            divisor.get_operands(collector);
            collector.reg_fixed_use(dividend, regs::rax());
            collector.reg_fixed_def(dst, regs::rax());
        }
        Inst::CheckedSRemSeq8 {
            divisor,
            dividend,
            dst,
            ..
        } => {
            collector.reg_use(divisor);
            collector.reg_fixed_use(dividend, regs::rax());
            collector.reg_fixed_def(dst, regs::rax());
        }
        Inst::Mul {
            src1,
            src2,
            dst_lo,
            dst_hi,
            ..
        } => {
            collector.reg_fixed_use(src1, regs::rax());
            collector.reg_fixed_def(dst_lo, regs::rax());
            collector.reg_fixed_def(dst_hi, regs::rdx());
            src2.get_operands(collector);
        }
        Inst::Mul8 {
            src1, src2, dst, ..
        } => {
            collector.reg_fixed_use(src1, regs::rax());
            collector.reg_fixed_def(dst, regs::rax());
            src2.get_operands(collector);
        }
        Inst::IMul {
            src1, src2, dst, ..
        } => {
            collector.reg_use(src1);
            collector.reg_reuse_def(dst, 0);
            src2.get_operands(collector);
        }
        Inst::IMulImm { src1, dst, .. } => {
            collector.reg_def(dst);
            src1.get_operands(collector);
        }
        Inst::SignExtendData { size, src, dst } => {
            match size {
                OperandSize::Size8 => {
                    // Note `rax` on both src and dest: 8->16 extend
                    // does AL -> AX.
                    collector.reg_fixed_use(src, regs::rax());
                    collector.reg_fixed_def(dst, regs::rax());
                }
                _ => {
                    // All other widths do RAX -> RDX (AX -> DX:AX,
                    // EAX -> EDX:EAX).
                    collector.reg_fixed_use(src, regs::rax());
                    collector.reg_fixed_def(dst, regs::rdx());
                }
            }
        }
        Inst::UnaryRmR { src, dst, .. }
        | Inst::UnaryRmRVex { src, dst, .. }
        | Inst::UnaryRmRImmVex { src, dst, .. } => {
            collector.reg_def(dst);
            src.get_operands(collector);
        }
        Inst::XmmUnaryRmR { src, dst, .. } | Inst::XmmUnaryRmRImm { src, dst, .. } => {
            collector.reg_def(dst);
            src.get_operands(collector);
        }
        Inst::XmmUnaryRmREvex { src, dst, .. }
        | Inst::XmmUnaryRmRImmEvex { src, dst, .. }
        | Inst::XmmUnaryRmRUnaligned { src, dst, .. }
        | Inst::XmmUnaryRmRVex { src, dst, .. }
        | Inst::XmmUnaryRmRImmVex { src, dst, .. } => {
            collector.reg_def(dst);
            src.get_operands(collector);
        }
        Inst::XmmRmR {
            src1, src2, dst, ..
        } => {
            collector.reg_use(src1);
            collector.reg_reuse_def(dst, 0);
            src2.get_operands(collector);
        }
        Inst::XmmRmRUnaligned {
            src1, src2, dst, ..
        } => {
            collector.reg_use(src1);
            collector.reg_reuse_def(dst, 0);
            src2.get_operands(collector);
        }
        Inst::XmmRmRBlend {
            src1,
            src2,
            mask,
            dst,
            op,
        } => {
            assert!(matches!(
                op,
                SseOpcode::Blendvpd | SseOpcode::Blendvps | SseOpcode::Pblendvb
            ));
            collector.reg_use(src1);
            collector.reg_fixed_use(mask, regs::xmm0());
            collector.reg_reuse_def(dst, 0);
            src2.get_operands(collector);
        }
        Inst::XmmRmiRVex {
            src1, src2, dst, ..
        } => {
            collector.reg_def(dst);
            collector.reg_use(src1);
            src2.get_operands(collector);
        }
        Inst::XmmRmRImmVex {
            src1, src2, dst, ..
        } => {
            collector.reg_def(dst);
            collector.reg_use(src1);
            src2.get_operands(collector);
        }
        Inst::XmmVexPinsr {
            src1, src2, dst, ..
        } => {
            collector.reg_def(dst);
            collector.reg_use(src1);
            src2.get_operands(collector);
        }
        Inst::XmmRmRVex3 {
            src1,
            src2,
            src3,
            dst,
            ..
        } => {
            collector.reg_use(src1);
            collector.reg_reuse_def(dst, 0);
            collector.reg_use(src2);
            src3.get_operands(collector);
        }
        Inst::XmmRmRBlendVex {
            src1,
            src2,
            mask,
            dst,
            ..
        } => {
            collector.reg_def(dst);
            collector.reg_use(src1);
            src2.get_operands(collector);
            collector.reg_use(mask);
        }
        Inst::XmmRmREvex {
            op,
            src1,
            src2,
            dst,
            ..
        } => {
            assert_ne!(*op, Avx512Opcode::Vpermi2b);
            collector.reg_use(src1);
            src2.get_operands(collector);
            collector.reg_def(dst);
        }
        Inst::XmmRmREvex3 {
            op,
            src1,
            src2,
            src3,
            dst,
            ..
        } => {
            assert_eq!(*op, Avx512Opcode::Vpermi2b);
            collector.reg_use(src1);
            collector.reg_use(src2);
            src3.get_operands(collector);
            collector.reg_reuse_def(dst, 0); // Reuse `src1`.
        }
        Inst::XmmRmRImm {
            src1, src2, dst, ..
        } => {
            collector.reg_use(src1);
            collector.reg_reuse_def(dst, 0);
            src2.get_operands(collector);
        }
        Inst::XmmUninitializedValue { dst } => collector.reg_def(dst),
        Inst::XmmMinMaxSeq { lhs, rhs, dst, .. } => {
            collector.reg_use(rhs);
            collector.reg_use(lhs);
            collector.reg_reuse_def(dst, 0); // Reuse RHS.
        }
        Inst::XmmRmiReg {
            src1, src2, dst, ..
        } => {
            collector.reg_use(src1);
            collector.reg_reuse_def(dst, 0); // Reuse RHS.
            src2.get_operands(collector);
        }
        Inst::XmmMovRM { src, dst, .. }
        | Inst::XmmMovRMVex { src, dst, .. }
        | Inst::XmmMovRMImm { src, dst, .. }
        | Inst::XmmMovRMImmVex { src, dst, .. } => {
            collector.reg_use(src);
            dst.get_operands(collector);
        }
        Inst::XmmCmpRmR { src1, src2, .. } => {
            collector.reg_use(src1);
            src2.get_operands(collector);
        }
        Inst::XmmCmpRmRVex { src1, src2, .. } => {
            collector.reg_use(src1);
            src2.get_operands(collector);
        }
        Inst::Imm { dst, .. } => {
            collector.reg_def(dst);
        }
        Inst::MovRR { src, dst, .. } => {
            collector.reg_use(src);
            collector.reg_def(dst);
        }
        Inst::MovFromPReg { dst, src } => {
            debug_assert!(dst.to_reg().to_reg().is_virtual());
            collector.reg_fixed_nonallocatable(*src);
            collector.reg_def(dst);
        }
        Inst::MovToPReg { dst, src } => {
            debug_assert!(src.to_reg().is_virtual());
            collector.reg_use(src);
            collector.reg_fixed_nonallocatable(*dst);
        }
        Inst::XmmToGpr { src, dst, .. }
        | Inst::XmmToGprVex { src, dst, .. }
        | Inst::XmmToGprImm { src, dst, .. }
        | Inst::XmmToGprImmVex { src, dst, .. } => {
            collector.reg_use(src);
            collector.reg_def(dst);
        }
        Inst::GprToXmm { src, dst, .. } | Inst::GprToXmmVex { src, dst, .. } => {
            collector.reg_def(dst);
            src.get_operands(collector);
        }
        Inst::CvtIntToFloat {
            src1, src2, dst, ..
        } => {
            collector.reg_use(src1);
            collector.reg_reuse_def(dst, 0);
            src2.get_operands(collector);
        }
        Inst::CvtIntToFloatVex {
            src1, src2, dst, ..
        } => {
            collector.reg_def(dst);
            collector.reg_use(src1);
            src2.get_operands(collector);
        }
        Inst::CvtUint64ToFloatSeq {
            src,
            dst,
            tmp_gpr1,
            tmp_gpr2,
            ..
        } => {
            collector.reg_use(src);
            collector.reg_early_def(dst);
            collector.reg_early_def(tmp_gpr1);
            collector.reg_early_def(tmp_gpr2);
        }
        Inst::CvtFloatToSintSeq {
            src,
            dst,
            tmp_xmm,
            tmp_gpr,
            ..
        } => {
            collector.reg_use(src);
            collector.reg_early_def(dst);
            collector.reg_early_def(tmp_gpr);
            collector.reg_early_def(tmp_xmm);
        }
        Inst::CvtFloatToUintSeq {
            src,
            dst,
            tmp_gpr,
            tmp_xmm,
            tmp_xmm2,
            ..
        } => {
            collector.reg_use(src);
            collector.reg_early_def(dst);
            collector.reg_early_def(tmp_gpr);
            collector.reg_early_def(tmp_xmm);
            collector.reg_early_def(tmp_xmm2);
        }

        Inst::MovImmM { dst, .. } => {
            dst.get_operands(collector);
        }

        Inst::MovzxRmR { src, dst, .. } => {
            collector.reg_def(dst);
            src.get_operands(collector);
        }
        Inst::Mov64MR { src, dst, .. } => {
            collector.reg_def(dst);
            src.get_operands(collector);
        }
        Inst::LoadEffectiveAddress { addr: src, dst, .. } => {
            collector.reg_def(dst);
            src.get_operands(collector);
        }
        Inst::MovsxRmR { src, dst, .. } => {
            collector.reg_def(dst);
            src.get_operands(collector);
        }
        Inst::MovRM { src, dst, .. } => {
            collector.reg_use(src);
            dst.get_operands(collector);
        }
        Inst::ShiftR {
            num_bits, src, dst, ..
        } => {
            collector.reg_use(src);
            collector.reg_reuse_def(dst, 0);
            if let Imm8Reg::Reg { reg } = num_bits.as_imm8_reg_mut() {
                collector.reg_fixed_use(reg, regs::rcx());
            }
        }
        Inst::CmpRmiR { src1, src2, .. } => {
            collector.reg_use(src1);
            src2.get_operands(collector);
        }
        Inst::Setcc { dst, .. } => {
            collector.reg_def(dst);
        }
        Inst::Bswap { src, dst, .. } => {
            collector.reg_use(src);
            collector.reg_reuse_def(dst, 0);
        }
        Inst::Cmove {
            consequent,
            alternative,
            dst,
            ..
        } => {
            collector.reg_use(alternative);
            collector.reg_reuse_def(dst, 0);
            consequent.get_operands(collector);
        }
        Inst::XmmCmove {
            consequent,
            alternative,
            dst,
            ..
        } => {
            collector.reg_use(alternative);
            collector.reg_reuse_def(dst, 0);
            collector.reg_use(consequent);
        }
        Inst::Push64 { src } => {
            src.get_operands(collector);
        }
        Inst::Pop64 { dst } => {
            collector.reg_def(dst);
        }
        Inst::StackProbeLoop { tmp, .. } => {
            collector.reg_early_def(tmp);
        }

        Inst::CallKnown { dest, info, .. } => {
            // Probestack is special and is only inserted after
            // regalloc, so we do not need to represent its ABI to the
            // register allocator. Assert that we don't alter that
            // arrangement.
            let CallInfo {
                uses,
                defs,
                clobbers,
                ..
            } = &mut **info.as_mut().expect("CallInfo is expected in this path");
            debug_assert_ne!(*dest, ExternalName::LibCall(LibCall::Probestack));
            for CallArgPair { vreg, preg } in uses {
                collector.reg_fixed_use(vreg, *preg);
            }
            for CallRetPair { vreg, preg } in defs {
                collector.reg_fixed_def(vreg, *preg);
            }
            collector.reg_clobbers(*clobbers);
        }

        Inst::CallUnknown { info, dest, .. } => {
            let CallInfo {
                uses,
                defs,
                clobbers,
                callee_conv,
                ..
            } = &mut **info.as_mut().expect("CallInfo is expected in this path");
            match dest {
                RegMem::Reg { reg } if *callee_conv == CallConv::Winch => {
                    // TODO(https://github.com/bytecodealliance/regalloc2/issues/145):
                    // This shouldn't be a fixed register constraint. r10 is caller-saved, so this
                    // should be safe to use.
                    collector.reg_fixed_use(reg, regs::r10())
                }
                _ => dest.get_operands(collector),
            }
            for CallArgPair { vreg, preg } in uses {
                collector.reg_fixed_use(vreg, *preg);
            }
            for CallRetPair { vreg, preg } in defs {
                collector.reg_fixed_def(vreg, *preg);
            }
            collector.reg_clobbers(*clobbers);
        }

        Inst::ReturnCallKnown { callee, info } => {
            let ReturnCallInfo { uses, tmp, .. } = &mut **info;
            collector.reg_fixed_def(tmp, regs::r11());
            // Same as in the `Inst::CallKnown` branch.
            debug_assert_ne!(*callee, ExternalName::LibCall(LibCall::Probestack));
            for CallArgPair { vreg, preg } in uses {
                collector.reg_fixed_use(vreg, *preg);
            }
        }

        Inst::ReturnCallUnknown { callee, info } => {
            let ReturnCallInfo { uses, tmp, .. } = &mut **info;

            // TODO(https://github.com/bytecodealliance/regalloc2/issues/145):
            // This shouldn't be a fixed register constraint, but it's not clear how to
            // pick a register that won't be clobbered by the callee-save restore code
            // emitted with a return_call_indirect. r10 is caller-saved, so this should be
            // safe to use.
            collector.reg_fixed_use(callee, regs::r10());

            collector.reg_fixed_def(tmp, regs::r11());
            for CallArgPair { vreg, preg } in uses {
                collector.reg_fixed_use(vreg, *preg);
            }
        }

        Inst::JmpTableSeq {
            idx, tmp1, tmp2, ..
        } => {
            collector.reg_use(idx);
            collector.reg_early_def(tmp1);
            // In the sequence emitted for this pseudoinstruction in emit.rs,
            // tmp2 is only written after idx is read, so it doesn't need to be
            // an early def.
            collector.reg_def(tmp2);
        }

        Inst::JmpUnknown { target } => {
            target.get_operands(collector);
        }

        Inst::LoadExtName { dst, .. } => {
            collector.reg_def(dst);
        }

        Inst::LockCmpxchg {
            replacement,
            expected,
            mem,
            dst_old,
            ..
        } => {
            collector.reg_use(replacement);
            collector.reg_fixed_use(expected, regs::rax());
            collector.reg_fixed_def(dst_old, regs::rax());
            mem.get_operands(collector);
        }

        Inst::AtomicRmwSeq {
            operand,
            temp,
            dst_old,
            mem,
            ..
        } => {
            collector.reg_late_use(operand);
            collector.reg_early_def(temp);
            // This `fixed_def` is needed because `CMPXCHG` always uses this
            // register implicitly.
            collector.reg_fixed_def(dst_old, regs::rax());
            mem.get_operands_late(collector)
        }

        Inst::Args { args } => {
            for ArgPair { vreg, preg } in args {
                collector.reg_fixed_def(vreg, *preg);
            }
        }

        Inst::Rets { rets } => {
            // The return value(s) are live-out; we represent this
            // with register uses on the return instruction.
            for RetPair { vreg, preg } in rets {
                collector.reg_fixed_use(vreg, *preg);
            }
        }

        Inst::JmpKnown { .. }
        | Inst::JmpIf { .. }
        | Inst::JmpCond { .. }
        | Inst::Ret { .. }
        | Inst::Nop { .. }
        | Inst::TrapIf { .. }
        | Inst::TrapIfAnd { .. }
        | Inst::TrapIfOr { .. }
        | Inst::VirtualSPOffsetAdj { .. }
        | Inst::Hlt
        | Inst::Ud2 { .. }
        | Inst::Fence { .. } => {
            // No registers are used.
        }

        Inst::ElfTlsGetAddr { dst, .. } | Inst::MachOTlsGetAddr { dst, .. } => {
            collector.reg_fixed_def(dst, regs::rax());
            // All caller-saves are clobbered.
            //
            // We use the SysV calling convention here because the
            // pseudoinstruction (and relocation that it emits) is specific to
            // ELF systems; other x86-64 targets with other conventions (i.e.,
            // Windows) use different TLS strategies.
            let mut clobbers = X64ABIMachineSpec::get_regs_clobbered_by_call(CallConv::SystemV);
            clobbers.remove(regs::gpr_preg(regs::ENC_RAX));
            collector.reg_clobbers(clobbers);
        }

        Inst::CoffTlsGetAddr { dst, tmp, .. } => {
            // We also use the gs register. But that register is not allocatable by the
            // register allocator, so we don't need to mark it as used here.

            // We use %rax to set the address
            collector.reg_fixed_def(dst, regs::rax());

            // We use %rcx as a temporary variable to load the _tls_index
            collector.reg_fixed_def(tmp, regs::rcx());
        }

        Inst::Unwind { .. } => {}

        Inst::DummyUse { reg } => {
            collector.reg_use(reg);
        }
    }
}

//=============================================================================
// Instructions: misc functions and external interface

impl MachInst for Inst {
    type ABIMachineSpec = X64ABIMachineSpec;

    fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        x64_get_operands(self, collector)
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        match self {
            // Note (carefully!) that a 32-bit mov *isn't* a no-op since it zeroes
            // out the upper 32 bits of the destination.  For example, we could
            // conceivably use `movl %reg, %reg` to zero out the top 32 bits of
            // %reg.
            Self::MovRR { size, src, dst, .. } if *size == OperandSize::Size64 => {
                Some((dst.to_writable_reg(), src.to_reg()))
            }
            // Note as well that MOVS[S|D] when used in the `XmmUnaryRmR` context are pure moves of
            // scalar floating-point values (and annotate `dst` as `def`s to the register allocator)
            // whereas the same operation in a packed context, e.g. `XMM_RM_R`, is used to merge a
            // value into the lowest lane of a vector (not a move).
            Self::XmmUnaryRmR { op, src, dst, .. }
                if *op == SseOpcode::Movss
                    || *op == SseOpcode::Movsd
                    || *op == SseOpcode::Movaps
                    || *op == SseOpcode::Movapd
                    || *op == SseOpcode::Movups
                    || *op == SseOpcode::Movupd
                    || *op == SseOpcode::Movdqa
                    || *op == SseOpcode::Movdqu =>
            {
                if let RegMem::Reg { reg } = src.clone().to_reg_mem() {
                    Some((dst.to_writable_reg(), reg))
                } else {
                    None
                }
            }
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
            Self::Ud2 { .. } => true,
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
            // Interesting cases.
            &Self::Rets { .. } => MachTerminator::Ret,
            &Self::ReturnCallKnown { .. } | &Self::ReturnCallUnknown { .. } => {
                MachTerminator::RetCall
            }
            &Self::JmpKnown { .. } => MachTerminator::Uncond,
            &Self::JmpCond { .. } => MachTerminator::Cond,
            &Self::JmpTableSeq { .. } => MachTerminator::Indirect,
            // All other cases are boring.
            _ => MachTerminator::None,
        }
    }

    fn is_mem_access(&self) -> bool {
        panic!("TODO FILL ME OUT")
    }

    fn gen_move(dst_reg: Writable<Reg>, src_reg: Reg, ty: Type) -> Inst {
        trace!(
            "Inst::gen_move {:?} -> {:?} (type: {:?})",
            src_reg,
            dst_reg.to_reg(),
            ty
        );
        let rc_dst = dst_reg.to_reg().class();
        let rc_src = src_reg.class();
        // If this isn't true, we have gone way off the rails.
        debug_assert!(rc_dst == rc_src);
        match rc_dst {
            RegClass::Int => Inst::mov_r_r(OperandSize::Size64, src_reg, dst_reg),
            RegClass::Float => {
                // The Intel optimization manual, in "3.5.1.13 Zero-Latency MOV Instructions",
                // doesn't include MOVSS/MOVSD as instructions with zero-latency. Use movaps for
                // those, which may write more lanes that we need, but are specified to have
                // zero-latency.
                let opcode = match ty {
                    types::F32 | types::F64 | types::F32X4 => SseOpcode::Movaps,
                    types::F64X2 => SseOpcode::Movapd,
                    _ if ty.is_vector() && ty.bits() == 128 => SseOpcode::Movdqa,
                    _ => unimplemented!("unable to move type: {}", ty),
                };
                Inst::xmm_unary_rm_r(opcode, RegMem::reg(src_reg), dst_reg)
            }
            RegClass::Vector => unreachable!(),
        }
    }

    fn gen_nop(preferred_size: usize) -> Inst {
        Inst::nop(std::cmp::min(preferred_size, 15) as u8)
    }

    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> {
        match ty {
            types::I8 => Ok((&[RegClass::Int], &[types::I8])),
            types::I16 => Ok((&[RegClass::Int], &[types::I16])),
            types::I32 => Ok((&[RegClass::Int], &[types::I32])),
            types::I64 => Ok((&[RegClass::Int], &[types::I64])),
            types::R32 => panic!("32-bit reftype pointer should never be seen on x86-64"),
            types::R64 => Ok((&[RegClass::Int], &[types::R64])),
            types::F32 => Ok((&[RegClass::Float], &[types::F32])),
            types::F64 => Ok((&[RegClass::Float], &[types::F64])),
            types::I128 => Ok((&[RegClass::Int, RegClass::Int], &[types::I64, types::I64])),
            _ if ty.is_vector() => {
                assert!(ty.bits() <= 128);
                Ok((&[RegClass::Float], &[types::I8X16]))
            }
            _ => Err(CodegenError::Unsupported(format!(
                "Unexpected SSA-value type: {}",
                ty
            ))),
        }
    }

    fn canonical_type_for_rc(rc: RegClass) -> Type {
        match rc {
            RegClass::Float => types::I8X16,
            RegClass::Int => types::I64,
            RegClass::Vector => unreachable!(),
        }
    }

    fn gen_jump(label: MachLabel) -> Inst {
        Inst::jmp_known(label)
    }

    fn gen_imm_u64(value: u64, dst: Writable<Reg>) -> Option<Self> {
        Some(Inst::imm(OperandSize::Size64, value, dst))
    }

    fn gen_imm_f64(value: f64, tmp: Writable<Reg>, dst: Writable<Reg>) -> SmallVec<[Self; 2]> {
        let imm_to_gpr = Inst::imm(OperandSize::Size64, value.to_bits(), tmp);
        let gpr_to_xmm = Self::gpr_to_xmm(
            SseOpcode::Movd,
            tmp.to_reg().into(),
            OperandSize::Size64,
            dst,
        );
        smallvec![imm_to_gpr, gpr_to_xmm]
    }

    fn gen_dummy_use(reg: Reg) -> Self {
        Inst::DummyUse { reg }
    }

    fn worst_case_size() -> CodeOffset {
        15
    }

    fn ref_type_regclass(_: &settings::Flags) -> RegClass {
        RegClass::Int
    }

    fn is_safepoint(&self) -> bool {
        match self {
            Inst::CallKnown { .. }
            | Inst::CallUnknown { .. }
            | Inst::TrapIf { .. }
            | Inst::Ud2 { .. } => true,
            _ => false,
        }
    }

    fn function_alignment() -> FunctionAlignment {
        FunctionAlignment {
            minimum: 1,
            // Prefer an alignment of 16-bytes to hypothetically get the whole
            // function into a minimum number of lines.
            preferred: 16,
        }
    }

    type LabelUse = LabelUse;

    const TRAP_OPCODE: &'static [u8] = &[0x0f, 0x0b];
}

/// Constant state used during emissions of a sequence of instructions.
pub struct EmitInfo {
    pub(super) flags: settings::Flags,
    isa_flags: x64_settings::Flags,
}

impl EmitInfo {
    /// Create a constant state for emission of instructions.
    pub fn new(flags: settings::Flags, isa_flags: x64_settings::Flags) -> Self {
        Self { flags, isa_flags }
    }
}

impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(
        &self,
        allocs: &[Allocation],
        sink: &mut MachBuffer<Inst>,
        info: &Self::Info,
        state: &mut Self::State,
    ) {
        let mut allocs = AllocationConsumer::new(allocs);
        emit::emit(self, &mut allocs, sink, info, state);
    }

    fn pretty_print_inst(&self, allocs: &[Allocation], _: &mut Self::State) -> String {
        PrettyPrint::pretty_print(self, 0, &mut AllocationConsumer::new(allocs))
    }
}

/// A label-use (internal relocation) in generated code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
    /// A 32-bit offset from location of relocation itself, added to the existing value at that
    /// location. Used for control flow instructions which consider an offset from the start of the
    /// next instruction (so the size of the payload -- 4 bytes -- is subtracted from the payload).
    JmpRel32,

    /// A 32-bit offset from location of relocation itself, added to the existing value at that
    /// location.
    PCRel32,
}

impl MachInstLabelUse for LabelUse {
    const ALIGN: CodeOffset = 1;

    fn max_pos_range(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => 0x7fff_ffff,
        }
    }

    fn max_neg_range(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => 0x8000_0000,
        }
    }

    fn patch_size(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => 4,
        }
    }

    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        let pc_rel = (label_offset as i64) - (use_offset as i64);
        debug_assert!(pc_rel <= self.max_pos_range() as i64);
        debug_assert!(pc_rel >= -(self.max_neg_range() as i64));
        let pc_rel = pc_rel as u32;
        match self {
            LabelUse::JmpRel32 => {
                let addend = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                let value = pc_rel.wrapping_add(addend).wrapping_sub(4);
                buffer.copy_from_slice(&value.to_le_bytes()[..]);
            }
            LabelUse::PCRel32 => {
                let addend = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                let value = pc_rel.wrapping_add(addend);
                buffer.copy_from_slice(&value.to_le_bytes()[..]);
            }
        }
    }

    fn supports_veneer(self) -> bool {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => false,
        }
    }

    fn veneer_size(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => 0,
        }
    }

    fn worst_case_veneer_size() -> CodeOffset {
        0
    }

    fn generate_veneer(self, _: &mut [u8], _: CodeOffset) -> (CodeOffset, LabelUse) {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => {
                panic!("Veneer not supported for JumpRel32 label-use.");
            }
        }
    }

    fn from_reloc(reloc: Reloc, addend: Addend) -> Option<Self> {
        match (reloc, addend) {
            (Reloc::X86CallPCRel4, -4) => Some(LabelUse::JmpRel32),
            _ => None,
        }
    }
}
