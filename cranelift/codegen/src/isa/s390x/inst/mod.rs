//! This module defines s390x-specific machine instruction types.

// Some variants are not constructed, but we still want them as options in the future.
#![allow(dead_code)]

use crate::binemit::{Addend, CodeOffset, Reloc};
use crate::ir::{types, ExternalName, Opcode, TrapCode, Type, ValueLabel};
use crate::isa::unwind::UnwindInst;
use crate::machinst::*;
use crate::{settings, CodegenError, CodegenResult};

use regalloc::{PrettyPrint, RegUsageCollector, RegUsageMapper};
use regalloc::{RealRegUniverse, Reg, RegClass, SpillSlot, VirtualReg, Writable};

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::convert::TryFrom;
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

/// An ALU operation. This can be paired with several instruction formats
/// below (see `Inst`) in any combination.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ALUOp {
    Add32,
    Add32Ext16,
    Add64,
    Add64Ext16,
    Add64Ext32,
    AddLogical32,
    AddLogical64,
    AddLogical64Ext32,
    Sub32,
    Sub32Ext16,
    Sub64,
    Sub64Ext16,
    Sub64Ext32,
    SubLogical32,
    SubLogical64,
    SubLogical64Ext32,
    Mul32,
    Mul32Ext16,
    Mul64,
    Mul64Ext16,
    Mul64Ext32,
    And32,
    And64,
    Orr32,
    Orr64,
    Xor32,
    Xor64,
    /// NAND
    AndNot32,
    AndNot64,
    /// NOR
    OrrNot32,
    OrrNot64,
    /// XNOR
    XorNot32,
    XorNot64,
}

impl ALUOp {
    pub(crate) fn available_from(&self) -> InstructionSet {
        match self {
            ALUOp::AndNot32 | ALUOp::AndNot64 => InstructionSet::MIE2,
            ALUOp::OrrNot32 | ALUOp::OrrNot64 => InstructionSet::MIE2,
            ALUOp::XorNot32 | ALUOp::XorNot64 => InstructionSet::MIE2,
            _ => InstructionSet::Base,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Abs32,
    Abs64,
    Abs64Ext32,
    Neg32,
    Neg64,
    Neg64Ext32,
    PopcntByte,
    PopcntReg,
}

impl UnaryOp {
    pub(crate) fn available_from(&self) -> InstructionSet {
        match self {
            UnaryOp::PopcntReg => InstructionSet::MIE2,
            _ => InstructionSet::Base,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShiftOp {
    RotL32,
    RotL64,
    LShL32,
    LShL64,
    LShR32,
    LShR64,
    AShR32,
    AShR64,
}

/// An integer comparison operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CmpOp {
    CmpS32,
    CmpS32Ext16,
    CmpS64,
    CmpS64Ext16,
    CmpS64Ext32,
    CmpL32,
    CmpL32Ext16,
    CmpL64,
    CmpL64Ext16,
    CmpL64Ext32,
}

/// A floating-point unit (FPU) operation with one arg.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FPUOp1 {
    Abs32,
    Abs64,
    Neg32,
    Neg64,
    NegAbs32,
    NegAbs64,
    Sqrt32,
    Sqrt64,
    Cvt32To64,
    Cvt64To32,
}

/// A floating-point unit (FPU) operation with two args.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FPUOp2 {
    Add32,
    Add64,
    Sub32,
    Sub64,
    Mul32,
    Mul64,
    Div32,
    Div64,
    Max32,
    Max64,
    Min32,
    Min64,
}

/// A floating-point unit (FPU) operation with three args.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FPUOp3 {
    MAdd32,
    MAdd64,
    MSub32,
    MSub64,
}

/// A conversion from an FP to an integer value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FpuToIntOp {
    F32ToU32,
    F32ToI32,
    F32ToU64,
    F32ToI64,
    F64ToU32,
    F64ToI32,
    F64ToU64,
    F64ToI64,
}

/// A conversion from an integer to an FP value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IntToFpuOp {
    U32ToF32,
    I32ToF32,
    U32ToF64,
    I32ToF64,
    U64ToF32,
    I64ToF32,
    U64ToF64,
    I64ToF64,
}

/// Modes for FP rounding ops: round down (floor) or up (ceil), or toward zero (trunc), or to
/// nearest, and for 32- or 64-bit FP values.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FpuRoundMode {
    Minus32,
    Minus64,
    Plus32,
    Plus64,
    Zero32,
    Zero64,
    Nearest32,
    Nearest64,
}

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

/// Additional information for JTSequence instructions, left out of line to lower the size of the Inst
/// enum.
#[derive(Clone, Debug)]
pub struct JTSequenceInfo {
    pub default_target: BranchTarget,
    pub targets: Vec<BranchTarget>,
    pub targets_for_term: Vec<MachLabel>, // needed for MachTerminator.
}

/// Instruction formats.
#[derive(Clone, Debug)]
pub enum Inst {
    /// A no-op of zero size.
    Nop0,

    /// A no-op of size two bytes.
    Nop2,

    /// An ALU operation with two register sources and a register destination.
    AluRRR {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        rn: Reg,
        rm: Reg,
    },
    /// An ALU operation with a register source and a signed 16-bit
    /// immediate source, and a separate register destination.
    AluRRSImm16 {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        rn: Reg,
        imm: i16,
    },
    /// An ALU operation with a register in-/out operand and
    /// a second register source.
    AluRR {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        rm: Reg,
    },
    /// An ALU operation with a register in-/out operand and
    /// a memory source.
    AluRX {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// An ALU operation with a register in-/out operand and a signed 16-bit
    /// immediate source.
    AluRSImm16 {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        imm: i16,
    },
    /// An ALU operation with a register in-/out operand and a signed 32-bit
    /// immediate source.
    AluRSImm32 {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        imm: i32,
    },
    /// An ALU operation with a register in-/out operand and an unsigned 32-bit
    /// immediate source.
    AluRUImm32 {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        imm: u32,
    },
    /// An ALU operation with a register in-/out operand and a shifted 16-bit
    /// immediate source.
    AluRUImm16Shifted {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        imm: UImm16Shifted,
    },
    /// An ALU operation with a register in-/out operand and a shifted 32-bit
    /// immediate source.
    AluRUImm32Shifted {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        imm: UImm32Shifted,
    },
    /// A multiply operation with two register sources and a register pair destination.
    /// FIXME: The pair is hard-coded as %r0/%r1 because regalloc cannot handle pairs.
    SMulWide {
        rn: Reg,
        rm: Reg,
    },
    /// A multiply operation with an in/out register pair, and an extra register source.
    /// Only the lower half of the register pair is used as input.
    /// FIXME: The pair is hard-coded as %r0/%r1 because regalloc cannot handle pairs.
    UMulWide {
        rn: Reg,
    },
    /// A divide operation with an in/out register pair, and an extra register source.
    /// Only the lower half of the register pair is used as input.
    /// FIXME: The pair is hard-coded as %r0/%r1 because regalloc cannot handle pairs.
    SDivMod32 {
        rn: Reg,
    },
    SDivMod64 {
        rn: Reg,
    },
    /// A divide operation with an in/out register pair, and an extra register source.
    /// FIXME: The pair is hard-coded as %r0/%r1 because regalloc cannot handle pairs.
    UDivMod32 {
        rn: Reg,
    },
    UDivMod64 {
        rn: Reg,
    },
    /// A FLOGR operation with a register source and a register pair destination.
    /// FIXME: The pair is hard-coded as %r0/%r1 because regalloc cannot handle pairs.
    Flogr {
        rn: Reg,
    },

    /// A shift instruction with a register source, a register destination,
    /// and an immediate plus an optional register as shift count.
    ShiftRR {
        shift_op: ShiftOp,
        rd: Writable<Reg>,
        rn: Reg,
        shift_imm: u8,
        shift_reg: Reg,
    },

    /// An unary operation with a register source and a register destination.
    UnaryRR {
        op: UnaryOp,
        rd: Writable<Reg>,
        rn: Reg,
    },

    /// A compare operation with two register sources.
    CmpRR {
        op: CmpOp,
        rn: Reg,
        rm: Reg,
    },
    /// A compare operation with a register source and a memory source.
    CmpRX {
        op: CmpOp,
        rn: Reg,
        mem: MemArg,
    },
    /// A compare operation with a register source and a signed 16-bit
    /// immediate source.
    CmpRSImm16 {
        op: CmpOp,
        rn: Reg,
        imm: i16,
    },
    /// A compare operation with a register source and a signed 32-bit
    /// immediate source.
    CmpRSImm32 {
        op: CmpOp,
        rn: Reg,
        imm: i32,
    },
    /// A compare operation with a register source and a unsigned 32-bit
    /// immediate source.
    CmpRUImm32 {
        op: CmpOp,
        rn: Reg,
        imm: u32,
    },
    /// A compare-and-trap instruction with two register sources.
    CmpTrapRR {
        op: CmpOp,
        rn: Reg,
        rm: Reg,
        cond: Cond,
        trap_code: TrapCode,
    },
    /// A compare-and-trap operation with a register source and a signed 16-bit
    /// immediate source.
    CmpTrapRSImm16 {
        op: CmpOp,
        rn: Reg,
        imm: i16,
        cond: Cond,
        trap_code: TrapCode,
    },
    /// A compare-and-trap operation with a register source and an unsigned 16-bit
    /// immediate source.
    CmpTrapRUImm16 {
        op: CmpOp,
        rn: Reg,
        imm: u16,
        cond: Cond,
        trap_code: TrapCode,
    },

    /// An atomic read-modify-write operation with a memory in-/out operand,
    /// a register destination, and a register source.
    /// a memory source.
    AtomicRmw {
        alu_op: ALUOp,
        rd: Writable<Reg>,
        rn: Reg,
        mem: MemArg,
    },
    /// A 32-bit atomic compare-and-swap operation.
    AtomicCas32 {
        rd: Writable<Reg>,
        rn: Reg,
        mem: MemArg,
    },
    /// A 64-bit atomic compare-and-swap operation.
    AtomicCas64 {
        rd: Writable<Reg>,
        rn: Reg,
        mem: MemArg,
    },
    /// A memory fence operation.
    Fence,

    /// A 32-bit load.
    Load32 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// An unsigned (zero-extending) 8-bit to 32-bit load.
    Load32ZExt8 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// A signed (sign-extending) 8-bit to 32-bit load.
    Load32SExt8 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// An unsigned (zero-extending) 16-bit to 32-bit load.
    Load32ZExt16 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// A signed (sign-extending) 16-bit to 32-bit load.
    Load32SExt16 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// A 64-bit load.
    Load64 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// An unsigned (zero-extending) 8-bit to 64-bit load.
    Load64ZExt8 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// A signed (sign-extending) 8-bit to 64-bit load.
    Load64SExt8 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// An unsigned (zero-extending) 16-bit to 64-bit load.
    Load64ZExt16 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// A signed (sign-extending) 16-bit to 64-bit load.
    Load64SExt16 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// An unsigned (zero-extending) 32-bit to 64-bit load.
    Load64ZExt32 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// A signed (sign-extending) 32-bit to 64-bit load.
    Load64SExt32 {
        rd: Writable<Reg>,
        mem: MemArg,
    },

    /// A 16-bit byte-reversed load.
    LoadRev16 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// A 32-bit byte-reversed load.
    LoadRev32 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// A 64-bit byte-reversed load.
    LoadRev64 {
        rd: Writable<Reg>,
        mem: MemArg,
    },

    /// An 8-bit store.
    Store8 {
        rd: Reg,
        mem: MemArg,
    },
    /// A 16-bit store.
    Store16 {
        rd: Reg,
        mem: MemArg,
    },
    /// A 32-bit store.
    Store32 {
        rd: Reg,
        mem: MemArg,
    },
    /// A 64-bit store.
    Store64 {
        rd: Reg,
        mem: MemArg,
    },
    /// An 8-bit store of an immediate.
    StoreImm8 {
        imm: u8,
        mem: MemArg,
    },
    /// A 16-bit store of an immediate.
    StoreImm16 {
        imm: i16,
        mem: MemArg,
    },
    /// A 32-bit store of a sign-extended 16-bit immediate.
    StoreImm32SExt16 {
        imm: i16,
        mem: MemArg,
    },
    /// A 64-bit store of a sign-extended 16-bit immediate.
    StoreImm64SExt16 {
        imm: i16,
        mem: MemArg,
    },

    /// A 16-bit byte-reversed store.
    StoreRev16 {
        rd: Reg,
        mem: MemArg,
    },
    /// A 32-bit byte-reversed store.
    StoreRev32 {
        rd: Reg,
        mem: MemArg,
    },
    /// A 64-bit byte-reversed store.
    StoreRev64 {
        rd: Reg,
        mem: MemArg,
    },

    /// A load-multiple instruction.
    LoadMultiple64 {
        rt: Writable<Reg>,
        rt2: Writable<Reg>,
        mem: MemArg,
    },
    /// A store-multiple instruction.
    StoreMultiple64 {
        rt: Reg,
        rt2: Reg,
        mem: MemArg,
    },

    /// A 32-bit move instruction.
    Mov32 {
        rd: Writable<Reg>,
        rm: Reg,
    },
    /// A 64-bit move instruction.
    Mov64 {
        rd: Writable<Reg>,
        rm: Reg,
    },
    /// A 32-bit move instruction with a full 32-bit immediate.
    Mov32Imm {
        rd: Writable<Reg>,
        imm: u32,
    },
    /// A 32-bit move instruction with a 16-bit signed immediate.
    Mov32SImm16 {
        rd: Writable<Reg>,
        imm: i16,
    },
    /// A 64-bit move instruction with a 16-bit signed immediate.
    Mov64SImm16 {
        rd: Writable<Reg>,
        imm: i16,
    },
    /// A 64-bit move instruction with a 32-bit signed immediate.
    Mov64SImm32 {
        rd: Writable<Reg>,
        imm: i32,
    },
    /// A 64-bit move instruction with a shifted 16-bit immediate.
    Mov64UImm16Shifted {
        rd: Writable<Reg>,
        imm: UImm16Shifted,
    },
    /// A 64-bit move instruction with a shifted 32-bit immediate.
    Mov64UImm32Shifted {
        rd: Writable<Reg>,
        imm: UImm32Shifted,
    },

    /// A 64-bit insert instruction with a shifted 16-bit immediate.
    Insert64UImm16Shifted {
        rd: Writable<Reg>,
        imm: UImm16Shifted,
    },
    /// A 64-bit insert instruction with a shifted 32-bit immediate.
    Insert64UImm32Shifted {
        rd: Writable<Reg>,
        imm: UImm32Shifted,
    },

    /// A sign- or zero-extend operation.
    Extend {
        rd: Writable<Reg>,
        rn: Reg,
        signed: bool,
        from_bits: u8,
        to_bits: u8,
    },

    /// A 32-bit conditional move instruction.
    CMov32 {
        rd: Writable<Reg>,
        cond: Cond,
        rm: Reg,
    },
    /// A 64-bit conditional move instruction.
    CMov64 {
        rd: Writable<Reg>,
        cond: Cond,
        rm: Reg,
    },
    /// A 32-bit conditional move instruction with a 16-bit signed immediate.
    CMov32SImm16 {
        rd: Writable<Reg>,
        cond: Cond,
        imm: i16,
    },
    /// A 64-bit conditional move instruction with a 16-bit signed immediate.
    CMov64SImm16 {
        rd: Writable<Reg>,
        cond: Cond,
        imm: i16,
    },

    /// 32-bit FPU move.
    FpuMove32 {
        rd: Writable<Reg>,
        rn: Reg,
    },
    /// 64-bit FPU move.
    FpuMove64 {
        rd: Writable<Reg>,
        rn: Reg,
    },

    /// A 32-bit conditional move FPU instruction.
    FpuCMov32 {
        rd: Writable<Reg>,
        cond: Cond,
        rm: Reg,
    },
    /// A 64-bit conditional move FPU instruction.
    FpuCMov64 {
        rd: Writable<Reg>,
        cond: Cond,
        rm: Reg,
    },

    /// A 64-bit move instruction from GPR to FPR.
    MovToFpr {
        rd: Writable<Reg>,
        rn: Reg,
    },
    /// A 64-bit move instruction from FPR to GPR.
    MovFromFpr {
        rd: Writable<Reg>,
        rn: Reg,
    },

    /// 1-op FPU instruction.
    FpuRR {
        fpu_op: FPUOp1,
        rd: Writable<Reg>,
        rn: Reg,
    },

    /// 2-op FPU instruction.
    FpuRRR {
        fpu_op: FPUOp2,
        rd: Writable<Reg>,
        rm: Reg,
    },

    /// 3-op FPU instruction.
    FpuRRRR {
        fpu_op: FPUOp3,
        rd: Writable<Reg>,
        rn: Reg,
        rm: Reg,
    },

    /// FPU copy sign instruction.
    FpuCopysign {
        rd: Writable<Reg>,
        rn: Reg,
        rm: Reg,
    },

    /// FPU comparison, single-precision (32 bit).
    FpuCmp32 {
        rn: Reg,
        rm: Reg,
    },

    /// FPU comparison, double-precision (64 bit).
    FpuCmp64 {
        rn: Reg,
        rm: Reg,
    },

    /// Floating-point load, single-precision (32 bit).
    FpuLoad32 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// Floating-point store, single-precision (32 bit).
    FpuStore32 {
        rd: Reg,
        mem: MemArg,
    },
    /// Floating-point load, double-precision (64 bit).
    FpuLoad64 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// Floating-point store, double-precision (64 bit).
    FpuStore64 {
        rd: Reg,
        mem: MemArg,
    },
    /// Floating-point byte-reversed load, single-precision (32 bit).
    FpuLoadRev32 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// Floating-point byte-reversed store, single-precision (32 bit).
    FpuStoreRev32 {
        rd: Reg,
        mem: MemArg,
    },
    /// Floating-point byte-reversed load, double-precision (64 bit).
    FpuLoadRev64 {
        rd: Writable<Reg>,
        mem: MemArg,
    },
    /// Floating-point byte-reversed store, double-precision (64 bit).
    FpuStoreRev64 {
        rd: Reg,
        mem: MemArg,
    },

    LoadFpuConst32 {
        rd: Writable<Reg>,
        const_data: u32,
    },

    LoadFpuConst64 {
        rd: Writable<Reg>,
        const_data: u64,
    },

    /// Conversion: FP -> integer.
    FpuToInt {
        op: FpuToIntOp,
        rd: Writable<Reg>,
        rn: Reg,
    },

    /// Conversion: integer -> FP.
    IntToFpu {
        op: IntToFpuOp,
        rd: Writable<Reg>,
        rn: Reg,
    },

    /// Round to integer.
    FpuRound {
        op: FpuRoundMode,
        rd: Writable<Reg>,
        rn: Reg,
    },

    /// 2-op FPU instruction implemented as vector instruction with the W bit.
    FpuVecRRR {
        fpu_op: FPUOp2,
        rd: Writable<Reg>,
        rn: Reg,
        rm: Reg,
    },

    /// A machine call instruction.
    Call {
        link: Writable<Reg>,
        info: Box<CallInfo>,
    },
    /// A machine indirect-call instruction.
    CallInd {
        link: Writable<Reg>,
        info: Box<CallIndInfo>,
    },

    // ---- branches (exactly one must appear at end of BB) ----
    /// A machine return instruction.
    Ret {
        link: Reg,
    },

    /// A placeholder instruction, generating no code, meaning that a function epilogue must be
    /// inserted there.
    EpiloguePlaceholder,

    /// An unconditional branch.
    Jump {
        dest: BranchTarget,
    },

    /// A conditional branch. Contains two targets; at emission time, both are emitted, but
    /// the MachBuffer knows to truncate the trailing branch if fallthrough. We optimize the
    /// choice of taken/not_taken (inverting the branch polarity as needed) based on the
    /// fallthrough at the time of lowering.
    CondBr {
        taken: BranchTarget,
        not_taken: BranchTarget,
        cond: Cond,
    },

    /// A conditional trap: execute a `Trap` if the condition is true. This is
    /// one VCode instruction because it uses embedded control flow; it is
    /// logically a single-in, single-out region, but needs to appear as one
    /// unit to the register allocator.
    ///
    /// The `Cond` gives the conditional-branch condition that will
    /// *execute* the embedded `Trap`. (In the emitted code, we use the inverse
    /// of this condition in a branch that skips the trap instruction.)
    TrapIf {
        cond: Cond,
        trap_code: TrapCode,
    },

    /// A one-way conditional branch, invisible to the CFG processing; used *only* as part of
    /// straight-line sequences in code to be emitted.
    ///
    /// In more detail:
    /// - This branch is lowered to a branch at the machine-code level, but does not end a basic
    ///   block, and does not create edges in the CFG seen by regalloc.
    /// - Thus, it is *only* valid to use as part of a single-in, single-out sequence that is
    ///   lowered from a single CLIF instruction. For example, certain arithmetic operations may
    ///   use these branches to handle certain conditions, such as overflows, traps, etc.
    ///
    /// See, e.g., the lowering of `trapif` (conditional trap) for an example.
    OneWayCondBr {
        target: BranchTarget,
        cond: Cond,
    },

    /// An indirect branch through a register, augmented with set of all
    /// possible successors.
    IndirectBr {
        rn: Reg,
        targets: Vec<MachLabel>,
    },

    /// A "debugtrap" instruction, used for e.g. traps and debug breakpoints.
    Debugtrap,

    /// An instruction guaranteed to always be undefined and to trigger an illegal instruction at
    /// runtime.
    Trap {
        trap_code: TrapCode,
    },

    /// Jump-table sequence, as one compound instruction (see note in lower.rs
    /// for rationale).
    JTSequence {
        info: Box<JTSequenceInfo>,
        ridx: Reg,
        rtmp1: Writable<Reg>,
        rtmp2: Writable<Reg>,
    },

    /// Load an inline symbol reference with RelocDistance::Far.
    LoadExtNameFar {
        rd: Writable<Reg>,
        name: Box<ExternalName>,
        offset: i64,
    },

    /// Load address referenced by `mem` into `rd`.
    LoadAddr {
        rd: Writable<Reg>,
        mem: MemArg,
    },

    /// Marker, no-op in generated code: SP "virtual offset" is adjusted. This
    /// controls how MemArg::NominalSPOffset args are lowered.
    VirtualSPOffsetAdj {
        offset: i64,
    },

    /// A definition of a value label.
    ValueLabelMarker {
        reg: Reg,
        label: ValueLabel,
    },

    /// An unwind pseudoinstruction describing the state of the
    /// machine at this program point.
    Unwind {
        inst: UnwindInst,
    },
}

#[test]
fn inst_size_test() {
    // This test will help with unintentionally growing the size
    // of the Inst enum.
    assert_eq!(32, std::mem::size_of::<Inst>());
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
            | Inst::VirtualSPOffsetAdj { .. }
            | Inst::ValueLabelMarker { .. }
            | Inst::Unwind { .. } => InstructionSet::Base,

            // These depend on the opcode
            Inst::AluRRR { alu_op, .. } => alu_op.available_from(),
            Inst::UnaryRR { op, .. } => op.available_from(),

            // These are all part of VXRS_EXT2
            Inst::FpuLoadRev32 { .. }
            | Inst::FpuStoreRev32 { .. }
            | Inst::FpuLoadRev64 { .. }
            | Inst::FpuStoreRev64 { .. } => InstructionSet::VXRS_EXT2,
        }
    }

    /// Create a 64-bit move instruction.
    pub fn mov64(to_reg: Writable<Reg>, from_reg: Reg) -> Inst {
        assert!(to_reg.to_reg().get_class() == from_reg.get_class());
        if from_reg.get_class() == RegClass::I64 {
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
        if from_reg.get_class() == RegClass::I64 {
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

fn memarg_regs(memarg: &MemArg, collector: &mut RegUsageCollector) {
    match memarg {
        &MemArg::BXD12 { base, index, .. } | &MemArg::BXD20 { base, index, .. } => {
            if base != zero_reg() {
                collector.add_use(base);
            }
            if index != zero_reg() {
                collector.add_use(index);
            }
        }
        &MemArg::Label { .. } | &MemArg::Symbol { .. } => {}
        &MemArg::RegOffset { reg, .. } => {
            collector.add_use(reg);
        }
        &MemArg::InitialSPOffset { .. } | &MemArg::NominalSPOffset { .. } => {
            collector.add_use(stack_reg());
        }
    }
}

fn s390x_get_regs(inst: &Inst, collector: &mut RegUsageCollector) {
    match inst {
        &Inst::AluRRR { rd, rn, rm, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
            collector.add_use(rm);
        }
        &Inst::AluRRSImm16 { rd, rn, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
        }
        &Inst::AluRR { rd, rm, .. } => {
            collector.add_mod(rd);
            collector.add_use(rm);
        }
        &Inst::AluRX { rd, ref mem, .. } => {
            collector.add_mod(rd);
            memarg_regs(mem, collector);
        }
        &Inst::AluRSImm16 { rd, .. } => {
            collector.add_mod(rd);
        }
        &Inst::AluRSImm32 { rd, .. } => {
            collector.add_mod(rd);
        }
        &Inst::AluRUImm32 { rd, .. } => {
            collector.add_mod(rd);
        }
        &Inst::AluRUImm16Shifted { rd, .. } => {
            collector.add_mod(rd);
        }
        &Inst::AluRUImm32Shifted { rd, .. } => {
            collector.add_mod(rd);
        }
        &Inst::SMulWide { rn, rm, .. } => {
            collector.add_def(writable_gpr(0));
            collector.add_def(writable_gpr(1));
            collector.add_use(rn);
            collector.add_use(rm);
        }
        &Inst::UMulWide { rn, .. } => {
            collector.add_def(writable_gpr(0));
            collector.add_mod(writable_gpr(1));
            collector.add_use(rn);
        }
        &Inst::SDivMod32 { rn, .. } | &Inst::SDivMod64 { rn, .. } => {
            collector.add_def(writable_gpr(0));
            collector.add_mod(writable_gpr(1));
            collector.add_use(rn);
        }
        &Inst::UDivMod32 { rn, .. } | &Inst::UDivMod64 { rn, .. } => {
            collector.add_mod(writable_gpr(0));
            collector.add_mod(writable_gpr(1));
            collector.add_use(rn);
        }
        &Inst::Flogr { rn, .. } => {
            collector.add_def(writable_gpr(0));
            collector.add_def(writable_gpr(1));
            collector.add_use(rn);
        }
        &Inst::ShiftRR {
            rd, rn, shift_reg, ..
        } => {
            collector.add_def(rd);
            collector.add_use(rn);
            if shift_reg != zero_reg() {
                collector.add_use(shift_reg);
            }
        }
        &Inst::UnaryRR { rd, rn, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
        }
        &Inst::CmpRR { rn, rm, .. } => {
            collector.add_use(rn);
            collector.add_use(rm);
        }
        &Inst::CmpRX { rn, ref mem, .. } => {
            collector.add_use(rn);
            memarg_regs(mem, collector);
        }
        &Inst::CmpRSImm16 { rn, .. } => {
            collector.add_use(rn);
        }
        &Inst::CmpRSImm32 { rn, .. } => {
            collector.add_use(rn);
        }
        &Inst::CmpRUImm32 { rn, .. } => {
            collector.add_use(rn);
        }
        &Inst::CmpTrapRR { rn, rm, .. } => {
            collector.add_use(rn);
            collector.add_use(rm);
        }
        &Inst::CmpTrapRSImm16 { rn, .. } => {
            collector.add_use(rn);
        }
        &Inst::CmpTrapRUImm16 { rn, .. } => {
            collector.add_use(rn);
        }
        &Inst::AtomicRmw {
            rd, rn, ref mem, ..
        } => {
            collector.add_def(rd);
            collector.add_use(rn);
            memarg_regs(mem, collector);
        }
        &Inst::AtomicCas32 {
            rd, rn, ref mem, ..
        }
        | &Inst::AtomicCas64 {
            rd, rn, ref mem, ..
        } => {
            collector.add_mod(rd);
            collector.add_use(rn);
            memarg_regs(mem, collector);
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
            collector.add_def(rd);
            memarg_regs(mem, collector);
        }
        &Inst::Store8 { rd, ref mem, .. }
        | &Inst::Store16 { rd, ref mem, .. }
        | &Inst::Store32 { rd, ref mem, .. }
        | &Inst::Store64 { rd, ref mem, .. }
        | &Inst::StoreRev16 { rd, ref mem, .. }
        | &Inst::StoreRev32 { rd, ref mem, .. }
        | &Inst::StoreRev64 { rd, ref mem, .. } => {
            collector.add_use(rd);
            memarg_regs(mem, collector);
        }
        &Inst::StoreImm8 { ref mem, .. }
        | &Inst::StoreImm16 { ref mem, .. }
        | &Inst::StoreImm32SExt16 { ref mem, .. }
        | &Inst::StoreImm64SExt16 { ref mem, .. } => {
            memarg_regs(mem, collector);
        }
        &Inst::LoadMultiple64 {
            rt, rt2, ref mem, ..
        } => {
            let first_regnum = rt.to_reg().get_hw_encoding();
            let last_regnum = rt2.to_reg().get_hw_encoding();
            for regnum in first_regnum..last_regnum + 1 {
                collector.add_def(writable_gpr(regnum));
            }
            memarg_regs(mem, collector);
        }
        &Inst::StoreMultiple64 {
            rt, rt2, ref mem, ..
        } => {
            let first_regnum = rt.get_hw_encoding();
            let last_regnum = rt2.get_hw_encoding();
            for regnum in first_regnum..last_regnum + 1 {
                collector.add_use(gpr(regnum));
            }
            memarg_regs(mem, collector);
        }
        &Inst::Mov64 { rd, rm } => {
            collector.add_def(rd);
            collector.add_use(rm);
        }
        &Inst::Mov32 { rd, rm } => {
            collector.add_def(rd);
            collector.add_use(rm);
        }
        &Inst::Mov32Imm { rd, .. }
        | &Inst::Mov32SImm16 { rd, .. }
        | &Inst::Mov64SImm16 { rd, .. }
        | &Inst::Mov64SImm32 { rd, .. }
        | &Inst::Mov64UImm16Shifted { rd, .. }
        | &Inst::Mov64UImm32Shifted { rd, .. } => {
            collector.add_def(rd);
        }
        &Inst::CMov32 { rd, rm, .. } | &Inst::CMov64 { rd, rm, .. } => {
            collector.add_mod(rd);
            collector.add_use(rm);
        }
        &Inst::CMov32SImm16 { rd, .. } | &Inst::CMov64SImm16 { rd, .. } => {
            collector.add_mod(rd);
        }
        &Inst::Insert64UImm16Shifted { rd, .. } | &Inst::Insert64UImm32Shifted { rd, .. } => {
            collector.add_mod(rd);
        }
        &Inst::FpuMove32 { rd, rn } | &Inst::FpuMove64 { rd, rn } => {
            collector.add_def(rd);
            collector.add_use(rn);
        }
        &Inst::FpuCMov32 { rd, rm, .. } | &Inst::FpuCMov64 { rd, rm, .. } => {
            collector.add_mod(rd);
            collector.add_use(rm);
        }
        &Inst::MovToFpr { rd, rn } | &Inst::MovFromFpr { rd, rn } => {
            collector.add_def(rd);
            collector.add_use(rn);
        }
        &Inst::FpuRR { rd, rn, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
        }
        &Inst::FpuRRR { rd, rm, .. } => {
            collector.add_mod(rd);
            collector.add_use(rm);
        }
        &Inst::FpuRRRR { rd, rn, rm, .. } => {
            collector.add_mod(rd);
            collector.add_use(rn);
            collector.add_use(rm);
        }
        &Inst::FpuCopysign { rd, rn, rm, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
            collector.add_use(rm);
        }
        &Inst::FpuCmp32 { rn, rm } | &Inst::FpuCmp64 { rn, rm } => {
            collector.add_use(rn);
            collector.add_use(rm);
        }
        &Inst::FpuLoad32 { rd, ref mem, .. } => {
            collector.add_def(rd);
            memarg_regs(mem, collector);
        }
        &Inst::FpuLoad64 { rd, ref mem, .. } => {
            collector.add_def(rd);
            memarg_regs(mem, collector);
        }
        &Inst::FpuStore32 { rd, ref mem, .. } => {
            collector.add_use(rd);
            memarg_regs(mem, collector);
        }
        &Inst::FpuStore64 { rd, ref mem, .. } => {
            collector.add_use(rd);
            memarg_regs(mem, collector);
        }
        &Inst::FpuLoadRev32 { rd, ref mem, .. } => {
            collector.add_def(rd);
            memarg_regs(mem, collector);
        }
        &Inst::FpuLoadRev64 { rd, ref mem, .. } => {
            collector.add_def(rd);
            memarg_regs(mem, collector);
        }
        &Inst::FpuStoreRev32 { rd, ref mem, .. } => {
            collector.add_use(rd);
            memarg_regs(mem, collector);
        }
        &Inst::FpuStoreRev64 { rd, ref mem, .. } => {
            collector.add_use(rd);
            memarg_regs(mem, collector);
        }
        &Inst::LoadFpuConst32 { rd, .. } | &Inst::LoadFpuConst64 { rd, .. } => {
            collector.add_def(rd);
        }
        &Inst::FpuToInt { rd, rn, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
        }
        &Inst::IntToFpu { rd, rn, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
        }
        &Inst::FpuRound { rd, rn, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
        }
        &Inst::FpuVecRRR { rd, rn, rm, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
            collector.add_use(rm);
        }
        &Inst::Extend { rd, rn, .. } => {
            collector.add_def(rd);
            collector.add_use(rn);
        }
        &Inst::Call { link, ref info } => {
            collector.add_def(link);
            collector.add_uses(&*info.uses);
            collector.add_defs(&*info.defs);
        }
        &Inst::CallInd { link, ref info } => {
            collector.add_def(link);
            collector.add_uses(&*info.uses);
            collector.add_defs(&*info.defs);
            collector.add_use(info.rn);
        }
        &Inst::Ret { .. } => {}
        &Inst::Jump { .. } | &Inst::EpiloguePlaceholder => {}
        &Inst::IndirectBr { rn, .. } => {
            collector.add_use(rn);
        }
        &Inst::CondBr { .. } | &Inst::OneWayCondBr { .. } => {}
        &Inst::Nop0 | Inst::Nop2 => {}
        &Inst::Debugtrap => {}
        &Inst::Trap { .. } => {}
        &Inst::TrapIf { .. } => {}
        &Inst::JTSequence {
            ridx, rtmp1, rtmp2, ..
        } => {
            collector.add_use(ridx);
            collector.add_def(rtmp1);
            collector.add_def(rtmp2);
        }
        &Inst::LoadExtNameFar { rd, .. } => {
            collector.add_def(rd);
        }
        &Inst::LoadAddr { rd, ref mem } => {
            collector.add_def(rd);
            memarg_regs(mem, collector);
        }
        &Inst::VirtualSPOffsetAdj { .. } => {}
        &Inst::ValueLabelMarker { reg, .. } => {
            collector.add_use(reg);
        }
        &Inst::Unwind { .. } => {}
    }
}

//=============================================================================
// Instructions: map_regs

fn s390x_map_regs<RUM: RegUsageMapper>(inst: &mut Inst, mapper: &RUM) {
    fn map_use<RUM: RegUsageMapper>(m: &RUM, r: &mut Reg) {
        if r.is_virtual() {
            let new = m.get_use(r.to_virtual_reg()).unwrap().to_reg();
            *r = new;
        }
    }

    fn map_def<RUM: RegUsageMapper>(m: &RUM, r: &mut Writable<Reg>) {
        if r.to_reg().is_virtual() {
            let new = m.get_def(r.to_reg().to_virtual_reg()).unwrap().to_reg();
            *r = Writable::from_reg(new);
        }
    }

    fn map_mod<RUM: RegUsageMapper>(m: &RUM, r: &mut Writable<Reg>) {
        if r.to_reg().is_virtual() {
            let new = m.get_mod(r.to_reg().to_virtual_reg()).unwrap().to_reg();
            *r = Writable::from_reg(new);
        }
    }

    fn map_mem<RUM: RegUsageMapper>(m: &RUM, mem: &mut MemArg) {
        match mem {
            &mut MemArg::BXD12 {
                ref mut base,
                ref mut index,
                ..
            }
            | &mut MemArg::BXD20 {
                ref mut base,
                ref mut index,
                ..
            } => {
                if *base != zero_reg() {
                    map_use(m, base);
                }
                if *index != zero_reg() {
                    map_use(m, index);
                }
            }
            &mut MemArg::Label { .. } | &mut MemArg::Symbol { .. } => {}
            &mut MemArg::RegOffset { ref mut reg, .. } => map_use(m, reg),
            &mut MemArg::InitialSPOffset { .. } | &mut MemArg::NominalSPOffset { .. } => {}
        };
    }

    match inst {
        &mut Inst::AluRRR {
            ref mut rd,
            ref mut rn,
            ref mut rm,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
            map_use(mapper, rm);
        }
        &mut Inst::AluRRSImm16 {
            ref mut rd,
            ref mut rn,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::AluRX {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_mod(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::AluRR {
            ref mut rd,
            ref mut rm,
            ..
        } => {
            map_mod(mapper, rd);
            map_use(mapper, rm);
        }
        &mut Inst::AluRSImm16 { ref mut rd, .. } => {
            map_mod(mapper, rd);
        }
        &mut Inst::AluRSImm32 { ref mut rd, .. } => {
            map_mod(mapper, rd);
        }
        &mut Inst::AluRUImm32 { ref mut rd, .. } => {
            map_mod(mapper, rd);
        }
        &mut Inst::AluRUImm16Shifted { ref mut rd, .. } => {
            map_mod(mapper, rd);
        }
        &mut Inst::AluRUImm32Shifted { ref mut rd, .. } => {
            map_mod(mapper, rd);
        }
        &mut Inst::SMulWide {
            ref mut rn,
            ref mut rm,
            ..
        } => {
            map_use(mapper, rn);
            map_use(mapper, rm);
        }
        &mut Inst::UMulWide { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::SDivMod32 { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::SDivMod64 { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::UDivMod32 { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::UDivMod64 { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::Flogr { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::ShiftRR {
            ref mut rd,
            ref mut rn,
            ref mut shift_reg,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
            if *shift_reg != zero_reg() {
                map_use(mapper, shift_reg);
            }
        }
        &mut Inst::UnaryRR {
            ref mut rd,
            ref mut rn,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::CmpRR {
            ref mut rn,
            ref mut rm,
            ..
        } => {
            map_use(mapper, rn);
            map_use(mapper, rm);
        }
        &mut Inst::CmpRX {
            ref mut rn,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rn);
            map_mem(mapper, mem);
        }
        &mut Inst::CmpRSImm16 { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::CmpRSImm32 { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::CmpRUImm32 { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::CmpTrapRR {
            ref mut rn,
            ref mut rm,
            ..
        } => {
            map_use(mapper, rn);
            map_use(mapper, rm);
        }
        &mut Inst::CmpTrapRSImm16 { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::CmpTrapRUImm16 { ref mut rn, .. } => {
            map_use(mapper, rn);
        }

        &mut Inst::AtomicRmw {
            ref mut rd,
            ref mut rn,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
            map_mem(mapper, mem);
        }
        &mut Inst::AtomicCas32 {
            ref mut rd,
            ref mut rn,
            ref mut mem,
            ..
        } => {
            map_mod(mapper, rd);
            map_use(mapper, rn);
            map_mem(mapper, mem);
        }
        &mut Inst::AtomicCas64 {
            ref mut rd,
            ref mut rn,
            ref mut mem,
            ..
        } => {
            map_mod(mapper, rd);
            map_use(mapper, rn);
            map_mem(mapper, mem);
        }
        &mut Inst::Fence => {}

        &mut Inst::Load32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load32ZExt8 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load32SExt8 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load32ZExt16 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load32SExt16 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load64 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load64ZExt8 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load64SExt8 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load64ZExt16 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load64SExt16 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load64ZExt32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Load64SExt32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::LoadRev16 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::LoadRev32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::LoadRev64 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }

        &mut Inst::Store8 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Store16 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Store32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::Store64 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::StoreImm8 { ref mut mem, .. } => {
            map_mem(mapper, mem);
        }
        &mut Inst::StoreImm16 { ref mut mem, .. } => {
            map_mem(mapper, mem);
        }
        &mut Inst::StoreImm32SExt16 { ref mut mem, .. } => {
            map_mem(mapper, mem);
        }
        &mut Inst::StoreImm64SExt16 { ref mut mem, .. } => {
            map_mem(mapper, mem);
        }
        &mut Inst::StoreRev16 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::StoreRev32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::StoreRev64 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::LoadMultiple64 { .. } => {
            // This instruction accesses all registers between rt and rt2,
            // so it cannot be remapped.  But this does not matter since
            // the instruction is only ever used after register allocation.
            unreachable!();
        }
        &mut Inst::StoreMultiple64 { .. } => {
            // This instruction accesses all registers between rt and rt2,
            // so it cannot be remapped.  But this does not matter since
            // the instruction is only ever used after register allocation.
            unreachable!();
        }

        &mut Inst::Mov64 {
            ref mut rd,
            ref mut rm,
        } => {
            map_def(mapper, rd);
            map_use(mapper, rm);
        }
        &mut Inst::Mov32 {
            ref mut rd,
            ref mut rm,
        } => {
            map_def(mapper, rd);
            map_use(mapper, rm);
        }
        &mut Inst::Mov32Imm { ref mut rd, .. } => {
            map_def(mapper, rd);
        }
        &mut Inst::Mov32SImm16 { ref mut rd, .. } => {
            map_def(mapper, rd);
        }
        &mut Inst::Mov64SImm16 { ref mut rd, .. } => {
            map_def(mapper, rd);
        }
        &mut Inst::Mov64SImm32 { ref mut rd, .. } => {
            map_def(mapper, rd);
        }
        &mut Inst::Mov64UImm16Shifted { ref mut rd, .. } => {
            map_def(mapper, rd);
        }
        &mut Inst::Mov64UImm32Shifted { ref mut rd, .. } => {
            map_def(mapper, rd);
        }
        &mut Inst::Insert64UImm16Shifted { ref mut rd, .. } => {
            map_mod(mapper, rd);
        }
        &mut Inst::Insert64UImm32Shifted { ref mut rd, .. } => {
            map_mod(mapper, rd);
        }
        &mut Inst::CMov64 {
            ref mut rd,
            ref mut rm,
            ..
        } => {
            map_mod(mapper, rd);
            map_use(mapper, rm);
        }
        &mut Inst::CMov32 {
            ref mut rd,
            ref mut rm,
            ..
        } => {
            map_mod(mapper, rd);
            map_use(mapper, rm);
        }
        &mut Inst::CMov32SImm16 { ref mut rd, .. } => {
            map_mod(mapper, rd);
        }
        &mut Inst::CMov64SImm16 { ref mut rd, .. } => {
            map_mod(mapper, rd);
        }
        &mut Inst::FpuMove32 {
            ref mut rd,
            ref mut rn,
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::FpuMove64 {
            ref mut rd,
            ref mut rn,
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::FpuCMov64 {
            ref mut rd,
            ref mut rm,
            ..
        } => {
            map_mod(mapper, rd);
            map_use(mapper, rm);
        }
        &mut Inst::FpuCMov32 {
            ref mut rd,
            ref mut rm,
            ..
        } => {
            map_mod(mapper, rd);
            map_use(mapper, rm);
        }
        &mut Inst::MovToFpr {
            ref mut rd,
            ref mut rn,
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::MovFromFpr {
            ref mut rd,
            ref mut rn,
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::FpuRR {
            ref mut rd,
            ref mut rn,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::FpuRRR {
            ref mut rd,
            ref mut rm,
            ..
        } => {
            map_mod(mapper, rd);
            map_use(mapper, rm);
        }
        &mut Inst::FpuRRRR {
            ref mut rd,
            ref mut rn,
            ref mut rm,
            ..
        } => {
            map_mod(mapper, rd);
            map_use(mapper, rn);
            map_use(mapper, rm);
        }
        &mut Inst::FpuCopysign {
            ref mut rd,
            ref mut rn,
            ref mut rm,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
            map_use(mapper, rm);
        }
        &mut Inst::FpuCmp32 {
            ref mut rn,
            ref mut rm,
        } => {
            map_use(mapper, rn);
            map_use(mapper, rm);
        }
        &mut Inst::FpuCmp64 {
            ref mut rn,
            ref mut rm,
        } => {
            map_use(mapper, rn);
            map_use(mapper, rm);
        }
        &mut Inst::FpuLoad32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::FpuLoad64 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::FpuStore32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::FpuStore64 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::FpuLoadRev32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::FpuLoadRev64 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::FpuStoreRev32 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::FpuStoreRev64 {
            ref mut rd,
            ref mut mem,
            ..
        } => {
            map_use(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::LoadFpuConst32 { ref mut rd, .. } => {
            map_def(mapper, rd);
        }
        &mut Inst::LoadFpuConst64 { ref mut rd, .. } => {
            map_def(mapper, rd);
        }
        &mut Inst::FpuToInt {
            ref mut rd,
            ref mut rn,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::IntToFpu {
            ref mut rd,
            ref mut rn,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::FpuRound {
            ref mut rd,
            ref mut rn,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::FpuVecRRR {
            ref mut rd,
            ref mut rn,
            ref mut rm,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
            map_use(mapper, rm);
        }
        &mut Inst::Extend {
            ref mut rd,
            ref mut rn,
            ..
        } => {
            map_def(mapper, rd);
            map_use(mapper, rn);
        }
        &mut Inst::Call {
            ref mut link,
            ref mut info,
        } => {
            map_def(mapper, link);
            for r in info.uses.iter_mut() {
                map_use(mapper, r);
            }
            for r in info.defs.iter_mut() {
                map_def(mapper, r);
            }
        }
        &mut Inst::CallInd {
            ref mut link,
            ref mut info,
            ..
        } => {
            map_def(mapper, link);
            for r in info.uses.iter_mut() {
                map_use(mapper, r);
            }
            for r in info.defs.iter_mut() {
                map_def(mapper, r);
            }
            map_use(mapper, &mut info.rn);
        }
        &mut Inst::Ret { .. } => {}
        &mut Inst::EpiloguePlaceholder => {}
        &mut Inst::Jump { .. } => {}
        &mut Inst::IndirectBr { ref mut rn, .. } => {
            map_use(mapper, rn);
        }
        &mut Inst::CondBr { .. } | &mut Inst::OneWayCondBr { .. } => {}
        &mut Inst::Debugtrap | &mut Inst::Trap { .. } | &mut Inst::TrapIf { .. } => {}
        &mut Inst::Nop0 | &mut Inst::Nop2 => {}
        &mut Inst::JTSequence {
            ref mut ridx,
            ref mut rtmp1,
            ref mut rtmp2,
            ..
        } => {
            map_use(mapper, ridx);
            map_def(mapper, rtmp1);
            map_def(mapper, rtmp2);
        }
        &mut Inst::LoadExtNameFar { ref mut rd, .. } => {
            map_def(mapper, rd);
        }
        &mut Inst::LoadAddr {
            ref mut rd,
            ref mut mem,
        } => {
            map_def(mapper, rd);
            map_mem(mapper, mem);
        }
        &mut Inst::VirtualSPOffsetAdj { .. } => {}
        &mut Inst::ValueLabelMarker { ref mut reg, .. } => {
            map_use(mapper, reg);
        }
        &mut Inst::Unwind { .. } => {}
    }
}

//=============================================================================
// Instructions: misc functions and external interface

impl MachInst for Inst {
    type LabelUse = LabelUse;

    fn get_regs(&self, collector: &mut RegUsageCollector) {
        s390x_get_regs(self, collector)
    }

    fn map_regs<RUM: RegUsageMapper>(&mut self, mapper: &RUM) {
        s390x_map_regs(self, mapper);
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
            &Inst::Jump { dest } => MachTerminator::Uncond(dest.as_label().unwrap()),
            &Inst::CondBr {
                taken, not_taken, ..
            } => MachTerminator::Cond(taken.as_label().unwrap(), not_taken.as_label().unwrap()),
            &Inst::OneWayCondBr { .. } => {
                // Explicitly invisible to CFG processing.
                MachTerminator::None
            }
            &Inst::IndirectBr { ref targets, .. } => MachTerminator::Indirect(&targets[..]),
            &Inst::JTSequence { ref info, .. } => {
                MachTerminator::Indirect(&info.targets_for_term[..])
            }
            _ => MachTerminator::None,
        }
    }

    fn stack_op_info(&self) -> Option<MachInstStackOpInfo> {
        match self {
            &Inst::VirtualSPOffsetAdj { offset } => Some(MachInstStackOpInfo::NomSPAdj(offset)),
            &Inst::Store64 {
                rd,
                mem: MemArg::NominalSPOffset { off },
            } => Some(MachInstStackOpInfo::StoreNomSPOff(rd, off)),
            &Inst::Load64 {
                rd,
                mem: MemArg::NominalSPOffset { off },
            } => Some(MachInstStackOpInfo::LoadNomSPOff(rd.to_reg(), off)),
            _ => None,
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

    fn maybe_direct_reload(&self, _reg: VirtualReg, _slot: SpillSlot) -> Option<Inst> {
        None
    }

    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> {
        match ty {
            types::I8 => Ok((&[RegClass::I64], &[types::I8])),
            types::I16 => Ok((&[RegClass::I64], &[types::I16])),
            types::I32 => Ok((&[RegClass::I64], &[types::I32])),
            types::I64 => Ok((&[RegClass::I64], &[types::I64])),
            types::B1 => Ok((&[RegClass::I64], &[types::B1])),
            types::B8 => Ok((&[RegClass::I64], &[types::B8])),
            types::B16 => Ok((&[RegClass::I64], &[types::B16])),
            types::B32 => Ok((&[RegClass::I64], &[types::B32])),
            types::B64 => Ok((&[RegClass::I64], &[types::B64])),
            types::R32 => panic!("32-bit reftype pointer should never be seen on s390x"),
            types::R64 => Ok((&[RegClass::I64], &[types::R64])),
            types::F32 => Ok((&[RegClass::F64], &[types::F32])),
            types::F64 => Ok((&[RegClass::F64], &[types::F64])),
            types::I128 => Ok((&[RegClass::I64, RegClass::I64], &[types::I64, types::I64])),
            types::B128 => Ok((&[RegClass::I64, RegClass::I64], &[types::B64, types::B64])),
            // FIXME: We don't really have IFLAGS, but need to allow it here
            // for now to support the SelectifSpectreGuard instruction.
            types::IFLAGS => Ok((&[RegClass::I64], &[types::I64])),
            _ => Err(CodegenError::Unsupported(format!(
                "Unexpected SSA-value type: {}",
                ty
            ))),
        }
    }

    fn gen_jump(target: MachLabel) -> Inst {
        Inst::Jump {
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
        44
    }

    fn ref_type_regclass(_: &settings::Flags) -> RegClass {
        RegClass::I64
    }

    fn gen_value_label_marker(label: ValueLabel, reg: Reg) -> Self {
        Inst::ValueLabelMarker { label, reg }
    }

    fn defines_value_label(&self) -> Option<(ValueLabel, Reg)> {
        match self {
            Inst::ValueLabelMarker { label, reg } => Some((*label, *reg)),
            _ => None,
        }
    }
}

//=============================================================================
// Pretty-printing of instructions.

fn mem_finalize_for_show(
    mem: &MemArg,
    mb_rru: Option<&RealRegUniverse>,
    state: &EmitState,
    have_d12: bool,
    have_d20: bool,
    have_pcrel: bool,
    have_index: bool,
) -> (String, MemArg) {
    let (mem_insts, mem) = mem_finalize(mem, state, have_d12, have_d20, have_pcrel, have_index);
    let mut mem_str = mem_insts
        .into_iter()
        .map(|inst| inst.show_rru(mb_rru))
        .collect::<Vec<_>>()
        .join(" ; ");
    if !mem_str.is_empty() {
        mem_str += " ; ";
    }

    (mem_str, mem)
}

impl PrettyPrint for Inst {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        self.pretty_print(mb_rru, &mut EmitState::default())
    }
}

impl Inst {
    fn print_with_state(&self, mb_rru: Option<&RealRegUniverse>, state: &mut EmitState) -> String {
        match self {
            &Inst::Nop0 => "nop-zero-len".to_string(),
            &Inst::Nop2 => "nop".to_string(),
            &Inst::AluRRR { alu_op, rd, rn, rm } => {
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
                    return inst.print_with_state(mb_rru, state);
                }
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("{} {}, {}, {}", op, rd, rn, rm)
            }
            &Inst::AluRRSImm16 {
                alu_op,
                rd,
                rn,
                imm,
            } => {
                if rd.to_reg() == rn {
                    let inst = Inst::AluRSImm16 { alu_op, rd, imm };
                    return inst.print_with_state(mb_rru, state);
                }
                let op = match alu_op {
                    ALUOp::Add32 => "ahik",
                    ALUOp::Add64 => "aghik",
                    _ => unreachable!(),
                };
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
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

                let (mem_str, mem) = mem_finalize_for_show(
                    mem,
                    mb_rru,
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

                let rd = rd.to_reg().show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                format!("{} {}, {}", op, rd, imm.bits)
            }
            &Inst::SMulWide { rn, rm } => {
                let op = "mgrk";
                let rd = gpr(0).show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("{} {}, {}, {}", op, rd, rn, rm)
            }
            &Inst::UMulWide { rn } => {
                let op = "mlgr";
                let rd = gpr(0).show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::SDivMod32 { rn, .. } => {
                let op = "dsgfr";
                let rd = gpr(0).show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::SDivMod64 { rn, .. } => {
                let op = "dsgr";
                let rd = gpr(0).show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::UDivMod32 { rn, .. } => {
                let op = "dlr";
                let rd = gpr(0).show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::UDivMod64 { rn, .. } => {
                let op = "dlgr";
                let rd = gpr(0).show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                format!("{} {}, {}", op, rd, rn)
            }
            &Inst::Flogr { rn } => {
                let op = "flogr";
                let rd = gpr(0).show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                let shift_reg = if shift_reg != zero_reg() {
                    format!("({})", shift_reg.show_rru(mb_rru))
                } else {
                    "".to_string()
                };
                format!("{} {}, {}, {}{}", op, rd, rn, shift_imm, shift_reg)
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
                };
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
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
                let rn = rn.show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
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

                let (mem_str, mem) = mem_finalize_for_show(
                    mem,
                    mb_rru,
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

                let rn = rn.show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
                format!("{}{} {}, {}", mem_str, op.unwrap(), rn, mem)
            }
            &Inst::CmpRSImm16 { op, rn, imm } => {
                let op = match op {
                    CmpOp::CmpS32 => "chi",
                    CmpOp::CmpS64 => "cghi",
                    _ => unreachable!(),
                };
                let rn = rn.show_rru(mb_rru);
                format!("{} {}, {}", op, rn, imm)
            }
            &Inst::CmpRSImm32 { op, rn, imm } => {
                let op = match op {
                    CmpOp::CmpS32 => "cfi",
                    CmpOp::CmpS64 => "cgfi",
                    _ => unreachable!(),
                };
                let rn = rn.show_rru(mb_rru);
                format!("{} {}, {}", op, rn, imm)
            }
            &Inst::CmpRUImm32 { op, rn, imm } => {
                let op = match op {
                    CmpOp::CmpL32 => "clfi",
                    CmpOp::CmpL64 => "clgfi",
                    _ => unreachable!(),
                };
                let rn = rn.show_rru(mb_rru);
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
                let rn = rn.show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                let cond = cond.show_rru(mb_rru);
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
                let rn = rn.show_rru(mb_rru);
                let cond = cond.show_rru(mb_rru);
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
                let rn = rn.show_rru(mb_rru);
                let cond = cond.show_rru(mb_rru);
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

                let (mem_str, mem) =
                    mem_finalize_for_show(mem, mb_rru, state, false, true, false, false);

                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
                format!("{}{} {}, {}, {}", mem_str, op, rd, rn, mem)
            }
            &Inst::AtomicCas32 { rd, rn, ref mem } | &Inst::AtomicCas64 { rd, rn, ref mem } => {
                let (opcode_rs, opcode_rsy) = match self {
                    &Inst::AtomicCas32 { .. } => (Some("cs"), Some("csy")),
                    &Inst::AtomicCas64 { .. } => (None, Some("csg")),
                    _ => unreachable!(),
                };

                let (mem_str, mem) = mem_finalize_for_show(
                    mem,
                    mb_rru,
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

                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
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

                let (mem_str, mem) = mem_finalize_for_show(
                    mem,
                    mb_rru,
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

                let rd = rd.to_reg().show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
                format!("{}{} {}, {}", mem_str, op.unwrap(), rd, mem)
            }
            &Inst::FpuLoadRev32 { rd, ref mem } | &Inst::FpuLoadRev64 { rd, ref mem } => {
                let (mem_str, mem) =
                    mem_finalize_for_show(mem, mb_rru, state, true, false, false, true);

                let op = match self {
                    &Inst::FpuLoadRev32 { .. } => "vlebrf",
                    &Inst::FpuLoadRev64 { .. } => "vlebrg",
                    _ => unreachable!(),
                };
                let rd = rd.to_reg().show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
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

                let (mem_str, mem) = mem_finalize_for_show(
                    mem,
                    mb_rru,
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

                let rd = rd.show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
                format!("{}{} {}, {}", mem_str, op.unwrap(), rd, mem)
            }
            &Inst::StoreImm8 { imm, ref mem } => {
                let (mem_str, mem) =
                    mem_finalize_for_show(mem, mb_rru, state, true, true, false, false);
                let op = match &mem {
                    &MemArg::BXD12 { .. } => "mvi",
                    &MemArg::BXD20 { .. } => "mviy",
                    _ => unreachable!(),
                };

                let mem = mem.show_rru(mb_rru);
                format!("{}{} {}, {}", mem_str, op, mem, imm)
            }
            &Inst::StoreImm16 { imm, ref mem }
            | &Inst::StoreImm32SExt16 { imm, ref mem }
            | &Inst::StoreImm64SExt16 { imm, ref mem } => {
                let (mem_str, mem) =
                    mem_finalize_for_show(mem, mb_rru, state, false, true, false, false);
                let op = match self {
                    &Inst::StoreImm16 { .. } => "mvhhi",
                    &Inst::StoreImm32SExt16 { .. } => "mvhi",
                    &Inst::StoreImm64SExt16 { .. } => "mvghi",
                    _ => unreachable!(),
                };

                let mem = mem.show_rru(mb_rru);
                format!("{}{} {}, {}", mem_str, op, mem, imm)
            }
            &Inst::FpuStoreRev32 { rd, ref mem } | &Inst::FpuStoreRev64 { rd, ref mem } => {
                let (mem_str, mem) =
                    mem_finalize_for_show(mem, mb_rru, state, true, false, false, true);

                let op = match self {
                    &Inst::FpuStoreRev32 { .. } => "vstebrf",
                    &Inst::FpuStoreRev64 { .. } => "vstebrg",
                    _ => unreachable!(),
                };
                let rd = rd.show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
                format!("{}{} {}, {}, 0", mem_str, op, rd, mem)
            }
            &Inst::LoadMultiple64 { rt, rt2, ref mem } => {
                let (mem_str, mem) =
                    mem_finalize_for_show(mem, mb_rru, state, false, true, false, false);
                let rt = rt.show_rru(mb_rru);
                let rt2 = rt2.show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
                format!("{}lmg {}, {}, {}", mem_str, rt, rt2, mem)
            }
            &Inst::StoreMultiple64 { rt, rt2, ref mem } => {
                let (mem_str, mem) =
                    mem_finalize_for_show(mem, mb_rru, state, false, true, false, false);
                let rt = rt.show_rru(mb_rru);
                let rt2 = rt2.show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
                format!("{}stmg {}, {}, {}", mem_str, rt, rt2, mem)
            }
            &Inst::Mov64 { rd, rm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("lgr {}, {}", rd, rm)
            }
            &Inst::Mov32 { rd, rm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("lr {}, {}", rd, rm)
            }
            &Inst::Mov32Imm { rd, ref imm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                format!("iilf {}, {}", rd, imm)
            }
            &Inst::Mov32SImm16 { rd, ref imm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                format!("lhi {}, {}", rd, imm)
            }
            &Inst::Mov64SImm16 { rd, ref imm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                format!("lghi {}, {}", rd, imm)
            }
            &Inst::Mov64SImm32 { rd, ref imm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                format!("lgfi {}, {}", rd, imm)
            }
            &Inst::Mov64UImm16Shifted { rd, ref imm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let op = match imm.shift {
                    0 => "llilf",
                    1 => "llihf",
                    _ => unreachable!(),
                };
                format!("{} {}, {}", op, rd, imm.bits)
            }
            &Inst::Insert64UImm16Shifted { rd, ref imm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let op = match imm.shift {
                    0 => "iilf",
                    1 => "iihf",
                    _ => unreachable!(),
                };
                format!("{} {}, {}", op, rd, imm.bits)
            }
            &Inst::CMov32 { rd, cond, rm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                let cond = cond.show_rru(mb_rru);
                format!("locr{} {}, {}", cond, rd, rm)
            }
            &Inst::CMov64 { rd, cond, rm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                let cond = cond.show_rru(mb_rru);
                format!("locgr{} {}, {}", cond, rd, rm)
            }
            &Inst::CMov32SImm16 { rd, cond, ref imm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let cond = cond.show_rru(mb_rru);
                format!("lochi{} {}, {}", cond, rd, imm)
            }
            &Inst::CMov64SImm16 { rd, cond, ref imm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let cond = cond.show_rru(mb_rru);
                format!("locghi{} {}, {}", cond, rd, imm)
            }
            &Inst::FpuMove32 { rd, rn } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                format!("ler {}, {}", rd, rn)
            }
            &Inst::FpuMove64 { rd, rn } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                format!("ldr {}, {}", rd, rn)
            }
            &Inst::FpuCMov32 { rd, cond, rm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                let cond = cond.invert().show_rru(mb_rru);
                format!("j{} 6 ; ler {}, {}", cond, rd, rm)
            }
            &Inst::FpuCMov64 { rd, cond, rm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                let cond = cond.invert().show_rru(mb_rru);
                format!("j{} 6 ; ldr {}, {}", cond, rd, rm)
            }
            &Inst::MovToFpr { rd, rn } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                format!("ldgr {}, {}", rd, rn)
            }
            &Inst::MovFromFpr { rd, rn } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("{} {}, {}", op, rd, rm)
            }
            &Inst::FpuRRRR { fpu_op, rd, rn, rm } => {
                let op = match fpu_op {
                    FPUOp3::MAdd32 => "maebr",
                    FPUOp3::MAdd64 => "madbr",
                    FPUOp3::MSub32 => "msebr",
                    FPUOp3::MSub64 => "msdbr",
                };
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("{} {}, {}, {}", op, rd, rn, rm)
            }
            &Inst::FpuCopysign { rd, rn, rm } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("cpsdr {}, {}, {}", rd, rm, rn)
            }
            &Inst::FpuCmp32 { rn, rm } => {
                let rn = rn.show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("cebr {}, {}", rn, rm)
            }
            &Inst::FpuCmp64 { rn, rm } => {
                let rn = rn.show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("cdbr {}, {}", rn, rm)
            }
            &Inst::LoadFpuConst32 { rd, const_data } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let tmp = writable_spilltmp_reg().to_reg().show_rru(mb_rru);
                format!(
                    "bras {}, 8 ; data.f32 {} ; le {}, 0({})",
                    tmp,
                    f32::from_bits(const_data),
                    rd,
                    tmp
                )
            }
            &Inst::LoadFpuConst64 { rd, const_data } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let tmp = writable_spilltmp_reg().to_reg().show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
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
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
                let rm = rm.show_rru(mb_rru);
                format!("{} {}, {}, {}, 1", op, rd, rn, rm)
            }
            &Inst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                to_bits,
            } => {
                let rd = rd.to_reg().show_rru(mb_rru);
                let rn = rn.show_rru(mb_rru);
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
                let link = link.show_rru(mb_rru);
                format!("brasl {}, {}", link, info.dest)
            }
            &Inst::CallInd { link, ref info, .. } => {
                let link = link.show_rru(mb_rru);
                let rn = info.rn.show_rru(mb_rru);
                format!("basr {}, {}", link, rn)
            }
            &Inst::Ret { link } => {
                let link = link.show_rru(mb_rru);
                format!("br {}", link)
            }
            &Inst::EpiloguePlaceholder => "epilogue placeholder".to_string(),
            &Inst::Jump { ref dest } => {
                let dest = dest.show_rru(mb_rru);
                format!("jg {}", dest)
            }
            &Inst::IndirectBr { rn, .. } => {
                let rn = rn.show_rru(mb_rru);
                format!("br {}", rn)
            }
            &Inst::CondBr {
                ref taken,
                ref not_taken,
                cond,
            } => {
                let taken = taken.show_rru(mb_rru);
                let not_taken = not_taken.show_rru(mb_rru);
                let cond = cond.show_rru(mb_rru);
                format!("jg{} {} ; jg {}", cond, taken, not_taken)
            }
            &Inst::OneWayCondBr { ref target, cond } => {
                let target = target.show_rru(mb_rru);
                let cond = cond.show_rru(mb_rru);
                format!("jg{} {}", cond, target)
            }
            &Inst::Debugtrap => "debugtrap".to_string(),
            &Inst::Trap { .. } => "trap".to_string(),
            &Inst::TrapIf { cond, .. } => {
                let cond = cond.invert().show_rru(mb_rru);
                format!("j{} 6 ; trap", cond)
            }
            &Inst::JTSequence {
                ref info,
                ridx,
                rtmp1,
                rtmp2,
                ..
            } => {
                let ridx = ridx.show_rru(mb_rru);
                let rtmp1 = rtmp1.show_rru(mb_rru);
                let rtmp2 = rtmp2.show_rru(mb_rru);
                let default_target = info.default_target.show_rru(mb_rru);
                format!(
                    concat!(
                        "clgfi {}, {} ; ",
                        "jghe {} ; ",
                        "sllg {}, {}, 2 ; ",
                        "larl {}, 18 ; ",
                        "lgf {}, 0({}, {}) ; ",
                        "agrk {}, {}, {} ; ",
                        "br {} ; ",
                        "jt_entries {:?}"
                    ),
                    ridx,
                    info.targets.len(),
                    default_target,
                    rtmp2,
                    ridx,
                    rtmp1,
                    rtmp2,
                    rtmp2,
                    rtmp1,
                    rtmp1,
                    rtmp1,
                    rtmp2,
                    rtmp1,
                    info.targets
                )
            }
            &Inst::LoadExtNameFar {
                rd,
                ref name,
                offset,
            } => {
                let rd = rd.show_rru(mb_rru);
                let tmp = writable_spilltmp_reg().to_reg().show_rru(mb_rru);
                format!(
                    "bras {}, 12 ; data {} + {} ; lg {}, 0({})",
                    tmp, name, offset, rd, tmp
                )
            }
            &Inst::LoadAddr { rd, ref mem } => {
                let (mem_str, mem) =
                    mem_finalize_for_show(mem, mb_rru, state, true, true, true, true);

                let op = match &mem {
                    &MemArg::BXD12 { .. } => "la",
                    &MemArg::BXD20 { .. } => "lay",
                    &MemArg::Label { .. } | &MemArg::Symbol { .. } => "larl",
                    _ => unreachable!(),
                };
                let rd = rd.show_rru(mb_rru);
                let mem = mem.show_rru(mb_rru);
                format!("{}{} {}, {}", mem_str, op, rd, mem)
            }
            &Inst::VirtualSPOffsetAdj { offset } => {
                state.virtual_sp_offset += offset;
                format!("virtual_sp_offset_adjust {}", offset)
            }
            &Inst::ValueLabelMarker { label, reg } => {
                format!("value_label {:?}, {}", label, reg.show_rru(mb_rru))
            }
            &Inst::Unwind { ref inst } => {
                format!("unwind {:?}", inst)
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
