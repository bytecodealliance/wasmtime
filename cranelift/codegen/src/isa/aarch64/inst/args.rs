//! AArch64 ISA definitions: instruction arguments.

// Some variants are never constructed, but we still want them as options in the future.
#![allow(dead_code)]

use crate::ir::Type;
use crate::isa::aarch64::inst::*;
use crate::isa::aarch64::lower::ty_bits;
use crate::machinst::MachLabel;

use regalloc::{RealRegUniverse, Reg, Writable};

use core::convert::Into;
use std::string::String;

/// A shift operator for a register or immediate.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum ShiftOp {
    LSL = 0b00,
    LSR = 0b01,
    ASR = 0b10,
    ROR = 0b11,
}

impl ShiftOp {
    /// Get the encoding of this shift op.
    pub fn bits(self) -> u8 {
        self as u8
    }
}

/// A shift operator amount.
#[derive(Clone, Copy, Debug)]
pub struct ShiftOpShiftImm(u8);

impl ShiftOpShiftImm {
    /// Maximum shift for shifted-register operands.
    pub const MAX_SHIFT: u64 = 63;

    /// Create a new shiftop shift amount, if possible.
    pub fn maybe_from_shift(shift: u64) -> Option<ShiftOpShiftImm> {
        if shift <= Self::MAX_SHIFT {
            Some(ShiftOpShiftImm(shift as u8))
        } else {
            None
        }
    }

    /// Return the shift amount.
    pub fn value(self) -> u8 {
        self.0
    }
}

/// A shift operator with an amount, guaranteed to be within range.
#[derive(Clone, Debug)]
pub struct ShiftOpAndAmt {
    op: ShiftOp,
    shift: ShiftOpShiftImm,
}

impl ShiftOpAndAmt {
    pub fn new(op: ShiftOp, shift: ShiftOpShiftImm) -> ShiftOpAndAmt {
        ShiftOpAndAmt { op, shift }
    }

    /// Get the shift op.
    pub fn op(&self) -> ShiftOp {
        self.op
    }

    /// Get the shift amount.
    pub fn amt(&self) -> ShiftOpShiftImm {
        self.shift
    }
}

/// An extend operator for a register.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum ExtendOp {
    UXTB = 0b000,
    UXTH = 0b001,
    UXTW = 0b010,
    UXTX = 0b011,
    SXTB = 0b100,
    SXTH = 0b101,
    SXTW = 0b110,
    SXTX = 0b111,
}

impl ExtendOp {
    /// Encoding of this op.
    pub fn bits(self) -> u8 {
        self as u8
    }
}

//=============================================================================
// Instruction sub-components (memory addresses): definitions

/// A reference to some memory address.
#[derive(Clone, Debug)]
pub enum MemLabel {
    /// An address in the code, a constant pool or jumptable, with relative
    /// offset from this instruction. This form must be used at emission time;
    /// see `memlabel_finalize()` for how other forms are lowered to this one.
    PCRel(i32),
}

/// A memory argument to load/store, encapsulating the possible addressing modes.
#[derive(Clone, Debug)]
pub enum MemArg {
    //
    // Real ARM64 addressing modes:
    //
    /// "post-indexed" mode as per AArch64 docs: postincrement reg after address computation.
    PostIndexed(Writable<Reg>, SImm9),
    /// "pre-indexed" mode as per AArch64 docs: preincrement reg before address computation.
    PreIndexed(Writable<Reg>, SImm9),

    // N.B.: RegReg, RegScaled, and RegScaledExtended all correspond to
    // what the ISA calls the "register offset" addressing mode. We split out
    // several options here for more ergonomic codegen.
    /// Register plus register offset.
    RegReg(Reg, Reg),

    /// Register plus register offset, scaled by type's size.
    RegScaled(Reg, Reg, Type),

    /// Register plus register offset, scaled by type's size, with index sign- or zero-extended
    /// first.
    RegScaledExtended(Reg, Reg, Type, ExtendOp),

    /// Register plus register offset, with index sign- or zero-extended first.
    RegExtended(Reg, Reg, ExtendOp),

    /// Unscaled signed 9-bit immediate offset from reg.
    Unscaled(Reg, SImm9),

    /// Scaled (by size of a type) unsigned 12-bit immediate offset from reg.
    UnsignedOffset(Reg, UImm12Scaled),

    //
    // virtual addressing modes that are lowered at emission time:
    //
    /// Reference to a "label": e.g., a symbol.
    Label(MemLabel),

    /// Arbitrary offset from a register. Converted to generation of large
    /// offsets with multiple instructions as necessary during code emission.
    RegOffset(Reg, i64, Type),

    /// Offset from the stack pointer.
    SPOffset(i64, Type),

    /// Offset from the frame pointer.
    FPOffset(i64, Type),

    /// Offset from the "nominal stack pointer", which is where the real SP is
    /// just after stack and spill slots are allocated in the function prologue.
    /// At emission time, this is converted to `SPOffset` with a fixup added to
    /// the offset constant. The fixup is a running value that is tracked as
    /// emission iterates through instructions in linear order, and can be
    /// adjusted up and down with [Inst::VirtualSPOffsetAdj].
    ///
    /// The standard ABI is in charge of handling this (by emitting the
    /// adjustment meta-instructions). It maintains the invariant that "nominal
    /// SP" is where the actual SP is after the function prologue and before
    /// clobber pushes. See the diagram in the documentation for
    /// [crate::isa::aarch64::abi](the ABI module) for more details.
    NominalSPOffset(i64, Type),
}

impl MemArg {
    /// Memory reference using an address in a register.
    pub fn reg(reg: Reg) -> MemArg {
        // Use UnsignedOffset rather than Unscaled to use ldr rather than ldur.
        // This also does not use PostIndexed / PreIndexed as they update the register.
        MemArg::UnsignedOffset(reg, UImm12Scaled::zero(I64))
    }

    /// Memory reference using the sum of two registers as an address.
    pub fn reg_plus_reg(reg1: Reg, reg2: Reg) -> MemArg {
        MemArg::RegReg(reg1, reg2)
    }

    /// Memory reference using `reg1 + sizeof(ty) * reg2` as an address.
    pub fn reg_plus_reg_scaled(reg1: Reg, reg2: Reg, ty: Type) -> MemArg {
        MemArg::RegScaled(reg1, reg2, ty)
    }

    /// Memory reference using `reg1 + sizeof(ty) * reg2` as an address, with `reg2` sign- or
    /// zero-extended as per `op`.
    pub fn reg_plus_reg_scaled_extended(reg1: Reg, reg2: Reg, ty: Type, op: ExtendOp) -> MemArg {
        MemArg::RegScaledExtended(reg1, reg2, ty, op)
    }

    /// Memory reference to a label: a global function or value, or data in the constant pool.
    pub fn label(label: MemLabel) -> MemArg {
        MemArg::Label(label)
    }
}

/// A memory argument to a load/store-pair.
#[derive(Clone, Debug)]
pub enum PairMemArg {
    SignedOffset(Reg, SImm7Scaled),
    PreIndexed(Writable<Reg>, SImm7Scaled),
    PostIndexed(Writable<Reg>, SImm7Scaled),
}

//=============================================================================
// Instruction sub-components (conditions, branches and branch targets):
// definitions

/// Condition for conditional branches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Cond {
    Eq = 0,
    Ne = 1,
    Hs = 2,
    Lo = 3,
    Mi = 4,
    Pl = 5,
    Vs = 6,
    Vc = 7,
    Hi = 8,
    Ls = 9,
    Ge = 10,
    Lt = 11,
    Gt = 12,
    Le = 13,
    Al = 14,
    Nv = 15,
}

impl Cond {
    /// Return the inverted condition.
    pub fn invert(self) -> Cond {
        match self {
            Cond::Eq => Cond::Ne,
            Cond::Ne => Cond::Eq,

            Cond::Hs => Cond::Lo,
            Cond::Lo => Cond::Hs,

            Cond::Mi => Cond::Pl,
            Cond::Pl => Cond::Mi,

            Cond::Vs => Cond::Vc,
            Cond::Vc => Cond::Vs,

            Cond::Hi => Cond::Ls,
            Cond::Ls => Cond::Hi,

            Cond::Ge => Cond::Lt,
            Cond::Lt => Cond::Ge,

            Cond::Gt => Cond::Le,
            Cond::Le => Cond::Gt,

            Cond::Al => Cond::Nv,
            Cond::Nv => Cond::Al,
        }
    }

    /// Return the machine encoding of this condition.
    pub fn bits(self) -> u32 {
        self as u32
    }
}

/// The kind of conditional branch: the common-case-optimized "reg-is-zero" /
/// "reg-is-nonzero" variants, or the generic one that tests the machine
/// condition codes.
#[derive(Clone, Copy, Debug)]
pub enum CondBrKind {
    /// Condition: given register is zero.
    Zero(Reg),
    /// Condition: given register is nonzero.
    NotZero(Reg),
    /// Condition: the given condition-code test is true.
    Cond(Cond),
}

impl CondBrKind {
    /// Return the inverted branch condition.
    pub fn invert(self) -> CondBrKind {
        match self {
            CondBrKind::Zero(reg) => CondBrKind::NotZero(reg),
            CondBrKind::NotZero(reg) => CondBrKind::Zero(reg),
            CondBrKind::Cond(c) => CondBrKind::Cond(c.invert()),
        }
    }
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
    pub fn as_label(self) -> Option<MachLabel> {
        match self {
            BranchTarget::Label(l) => Some(l),
            _ => None,
        }
    }

    /// Return the target's offset, if specified, or zero if label-based.
    pub fn as_offset19_or_zero(self) -> u32 {
        let off = match self {
            BranchTarget::ResolvedOffset(off) => off >> 2,
            _ => 0,
        };
        assert!(off <= 0x3ffff);
        assert!(off >= -0x40000);
        (off as u32) & 0x7ffff
    }

    /// Return the target's offset, if specified, or zero if label-based.
    pub fn as_offset26_or_zero(self) -> u32 {
        let off = match self {
            BranchTarget::ResolvedOffset(off) => off >> 2,
            _ => 0,
        };
        assert!(off <= 0x1ffffff);
        assert!(off >= -0x2000000);
        (off as u32) & 0x3ffffff
    }
}

impl ShowWithRRU for ShiftOpAndAmt {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        format!("{:?} {}", self.op(), self.amt().value())
    }
}

impl ShowWithRRU for ExtendOp {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        format!("{:?}", self)
    }
}

impl ShowWithRRU for MemLabel {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            &MemLabel::PCRel(off) => format!("pc+{}", off),
        }
    }
}

fn shift_for_type(ty: Type) -> usize {
    match ty.bytes() {
        1 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        16 => 4,
        _ => panic!("unknown type: {}", ty),
    }
}

impl ShowWithRRU for MemArg {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            &MemArg::Unscaled(reg, simm9) => {
                if simm9.value != 0 {
                    format!("[{}, {}]", reg.show_rru(mb_rru), simm9.show_rru(mb_rru))
                } else {
                    format!("[{}]", reg.show_rru(mb_rru))
                }
            }
            &MemArg::UnsignedOffset(reg, uimm12) => {
                if uimm12.value != 0 {
                    format!("[{}, {}]", reg.show_rru(mb_rru), uimm12.show_rru(mb_rru))
                } else {
                    format!("[{}]", reg.show_rru(mb_rru))
                }
            }
            &MemArg::RegReg(r1, r2) => {
                format!("[{}, {}]", r1.show_rru(mb_rru), r2.show_rru(mb_rru),)
            }
            &MemArg::RegScaled(r1, r2, ty) => {
                let shift = shift_for_type(ty);
                format!(
                    "[{}, {}, LSL #{}]",
                    r1.show_rru(mb_rru),
                    r2.show_rru(mb_rru),
                    shift,
                )
            }
            &MemArg::RegScaledExtended(r1, r2, ty, op) => {
                let shift = shift_for_type(ty);
                let size = match op {
                    ExtendOp::SXTW | ExtendOp::UXTW => OperandSize::Size32,
                    _ => OperandSize::Size64,
                };
                let op = op.show_rru(mb_rru);
                format!(
                    "[{}, {}, {} #{}]",
                    r1.show_rru(mb_rru),
                    show_ireg_sized(r2, mb_rru, size),
                    op,
                    shift
                )
            }
            &MemArg::RegExtended(r1, r2, op) => {
                let size = match op {
                    ExtendOp::SXTW | ExtendOp::UXTW => OperandSize::Size32,
                    _ => OperandSize::Size64,
                };
                let op = op.show_rru(mb_rru);
                format!(
                    "[{}, {}, {}]",
                    r1.show_rru(mb_rru),
                    show_ireg_sized(r2, mb_rru, size),
                    op,
                )
            }
            &MemArg::Label(ref label) => label.show_rru(mb_rru),
            &MemArg::PreIndexed(r, simm9) => format!(
                "[{}, {}]!",
                r.to_reg().show_rru(mb_rru),
                simm9.show_rru(mb_rru)
            ),
            &MemArg::PostIndexed(r, simm9) => format!(
                "[{}], {}",
                r.to_reg().show_rru(mb_rru),
                simm9.show_rru(mb_rru)
            ),
            // Eliminated by `mem_finalize()`.
            &MemArg::SPOffset(..)
            | &MemArg::FPOffset(..)
            | &MemArg::NominalSPOffset(..)
            | &MemArg::RegOffset(..) => {
                panic!("Unexpected pseudo mem-arg mode (stack-offset or generic reg-offset)!")
            }
        }
    }
}

impl ShowWithRRU for PairMemArg {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            &PairMemArg::SignedOffset(reg, simm7) => {
                if simm7.value != 0 {
                    format!("[{}, {}]", reg.show_rru(mb_rru), simm7.show_rru(mb_rru))
                } else {
                    format!("[{}]", reg.show_rru(mb_rru))
                }
            }
            &PairMemArg::PreIndexed(reg, simm7) => format!(
                "[{}, {}]!",
                reg.to_reg().show_rru(mb_rru),
                simm7.show_rru(mb_rru)
            ),
            &PairMemArg::PostIndexed(reg, simm7) => format!(
                "[{}], {}",
                reg.to_reg().show_rru(mb_rru),
                simm7.show_rru(mb_rru)
            ),
        }
    }
}

impl ShowWithRRU for Cond {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        let mut s = format!("{:?}", self);
        s.make_ascii_lowercase();
        s
    }
}

impl ShowWithRRU for BranchTarget {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            &BranchTarget::Label(label) => format!("label{:?}", label.get()),
            &BranchTarget::ResolvedOffset(off) => format!("{}", off),
        }
    }
}

/// Type used to communicate the operand size of a machine instruction, as AArch64 has 32- and
/// 64-bit variants of many instructions (and integer registers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandSize {
    Size32,
    Size64,
}

impl OperandSize {
    /// 32-bit case?
    pub fn is32(self) -> bool {
        self == OperandSize::Size32
    }
    /// 64-bit case?
    pub fn is64(self) -> bool {
        self == OperandSize::Size64
    }
    /// Convert from an `is32` boolean flag to an `OperandSize`.
    pub fn from_is32(is32: bool) -> OperandSize {
        if is32 {
            OperandSize::Size32
        } else {
            OperandSize::Size64
        }
    }
    /// Convert from a needed width to the smallest size that fits.
    pub fn from_bits<I: Into<usize>>(bits: I) -> OperandSize {
        let bits: usize = bits.into();
        assert!(bits <= 64);
        if bits <= 32 {
            OperandSize::Size32
        } else {
            OperandSize::Size64
        }
    }

    /// Convert from an integer type into the smallest size that fits.
    pub fn from_ty(ty: Type) -> OperandSize {
        Self::from_bits(ty_bits(ty))
    }

    /// Convert to I32, I64, or I128.
    pub fn to_ty(self) -> Type {
        match self {
            OperandSize::Size32 => I32,
            OperandSize::Size64 => I64,
        }
    }

    pub fn sf_bit(&self) -> u32 {
        match self {
            OperandSize::Size32 => 0,
            OperandSize::Size64 => 1,
        }
    }
}

/// Type used to communicate the size of a scalar SIMD & FP operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarSize {
    Size8,
    Size16,
    Size32,
    Size64,
    Size128,
}

impl ScalarSize {
    /// Convert from a needed width to the smallest size that fits.
    pub fn from_bits<I: Into<usize>>(bits: I) -> ScalarSize {
        match bits.into().next_power_of_two() {
            8 => ScalarSize::Size8,
            16 => ScalarSize::Size16,
            32 => ScalarSize::Size32,
            64 => ScalarSize::Size64,
            128 => ScalarSize::Size128,
            _ => panic!("Unexpected type width"),
        }
    }

    /// Convert from a type into the smallest size that fits.
    pub fn from_ty(ty: Type) -> ScalarSize {
        Self::from_bits(ty_bits(ty))
    }

    /// Return the encoding bits that are used by some scalar FP instructions
    /// for a particular operand size.
    pub fn ftype(&self) -> u32 {
        match self {
            ScalarSize::Size16 => 0b11,
            ScalarSize::Size32 => 0b00,
            ScalarSize::Size64 => 0b01,
            _ => panic!("Unexpected scalar FP operand size"),
        }
    }
}
