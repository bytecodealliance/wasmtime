//! 32-bit ARM ISA definitions: instruction arguments.

use crate::isa::arm32::inst::*;

use regalloc::{PrettyPrint, RealRegUniverse, Reg};

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
    pub const MAX_SHIFT: u32 = 31;

    /// Create a new shiftop shift amount, if possible.
    pub fn maybe_from_shift(shift: u32) -> Option<ShiftOpShiftImm> {
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

// An unsigned 8-bit immediate.
#[derive(Clone, Copy, Debug)]
pub struct UImm8 {
    /// The value.
    value: u8,
}

impl UImm8 {
    pub fn maybe_from_i64(value: i64) -> Option<UImm8> {
        if 0 <= value && value < (1 << 8) {
            Some(UImm8 { value: value as u8 })
        } else {
            None
        }
    }

    /// Bits for encoding.
    pub fn bits(&self) -> u32 {
        u32::from(self.value)
    }
}

/// An unsigned 12-bit immediate.
#[derive(Clone, Copy, Debug)]
pub struct UImm12 {
    /// The value.
    value: u16,
}

impl UImm12 {
    pub fn maybe_from_i64(value: i64) -> Option<UImm12> {
        if 0 <= value && value < (1 << 12) {
            Some(UImm12 {
                value: value as u16,
            })
        } else {
            None
        }
    }

    /// Bits for encoding.
    pub fn bits(&self) -> u32 {
        u32::from(self.value)
    }
}

/// An addressing mode specified for a load/store operation.
#[derive(Clone, Debug)]
pub enum AMode {
    // Real addressing modes
    /// Register plus register offset, which can be shifted left by imm2.
    RegReg(Reg, Reg, u8),

    /// Unsigned 12-bit immediate offset from reg.
    RegOffset12(Reg, UImm12),

    /// Immediate offset from program counter aligned to 4.
    /// Cannot be used by store instructions.
    PCRel(i32),

    // Virtual addressing modes that are lowered at emission time:
    /// Immediate offset from reg.
    RegOffset(Reg, i64),

    /// Signed immediate offset from stack pointer.
    SPOffset(i64, Type),

    /// Offset from the frame pointer.
    FPOffset(i64, Type),

    /// Signed immediate offset from "nominal stack pointer".
    NominalSPOffset(i64, Type),
}

impl AMode {
    /// Memory reference using the sum of two registers as an address.
    pub fn reg_plus_reg(reg1: Reg, reg2: Reg, shift_amt: u8) -> AMode {
        assert!(shift_amt <= 3);
        AMode::RegReg(reg1, reg2, shift_amt)
    }

    /// Memory reference using the sum of a register and an immediate offset
    /// as an address.
    pub fn reg_plus_imm(reg: Reg, offset: i64) -> AMode {
        AMode::RegOffset(reg, offset)
    }
}

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

            Cond::Al => panic!("Cannot inverse {:?} condition", self),
        }
    }

    /// Return the machine encoding of this condition.
    pub fn bits(self) -> u16 {
        self as u16
    }
}

/// A branch target. Either unresolved (basic-block index) or resolved (offset
/// from end of current instruction).
#[derive(Clone, Copy, Debug)]
pub enum BranchTarget {
    /// An unresolved reference to a Label.
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

    // Ready for embedding in instruction.
    fn as_offset(self, inst_16_bit: bool) -> i32 {
        match self {
            BranchTarget::ResolvedOffset(off) => {
                if inst_16_bit {
                    // pc is equal to end of the current inst + 2.
                    (off - 2) >> 1
                } else {
                    // pc points to end of the current inst.
                    off >> 1
                }
            }
            _ => 0,
        }
    }

    // For 32-bit unconditional jump.
    pub fn as_off24(self) -> u32 {
        let off = self.as_offset(false);
        assert!(off < (1 << 24));
        assert!(off >= -(1 << 24));
        (off as u32) & ((1 << 24) - 1)
    }

    // For 32-bit conditional jump.
    pub fn as_off20(self) -> u32 {
        let off = self.as_offset(false);
        assert!(off < (1 << 20));
        assert!(off >= -(1 << 20));
        (off as u32) & ((1 << 20) - 1)
    }
}

impl PrettyPrint for ShiftOpAndAmt {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        let op = match self.op() {
            ShiftOp::LSL => "lsl",
            ShiftOp::LSR => "lsr",
            ShiftOp::ASR => "asr",
            ShiftOp::ROR => "ror",
        };
        format!("{} #{}", op, self.amt().value())
    }
}

impl PrettyPrint for UImm8 {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        format!("#{}", self.value)
    }
}

impl PrettyPrint for UImm12 {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        format!("#{}", self.value)
    }
}

impl PrettyPrint for AMode {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            &AMode::RegReg(rn, rm, imm2) => {
                let shift = if imm2 != 0 {
                    format!(", lsl #{}", imm2)
                } else {
                    "".to_string()
                };
                format!(
                    "[{}, {}{}]",
                    rn.show_rru(mb_rru),
                    rm.show_rru(mb_rru),
                    shift
                )
            }
            &AMode::RegOffset12(rn, off) => {
                format!("[{}, {}]", rn.show_rru(mb_rru), off.show_rru(mb_rru))
            }
            &AMode::PCRel(off) => format!("[pc, #{}]", off),
            &AMode::RegOffset(..)
            | &AMode::SPOffset(..)
            | &AMode::FPOffset(..)
            | &AMode::NominalSPOffset(..) => panic!("unexpected mem mode"),
        }
    }
}

impl PrettyPrint for Cond {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        let mut s = format!("{:?}", self);
        s.make_ascii_lowercase();
        s
    }
}

impl PrettyPrint for BranchTarget {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            &BranchTarget::Label(label) => format!("label{:?}", label.get()),
            &BranchTarget::ResolvedOffset(off) => format!("{}", off),
        }
    }
}
