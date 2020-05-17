//! Instruction operand sub-components (aka "parts"): definitions and printing.

use std::fmt;
use std::string::{String, ToString};

use regalloc::{RealRegUniverse, Reg, RegClass, RegUsageCollector};

use crate::machinst::*;

use super::regs::show_ireg_sized;

/// A Memory Address. These denote a 64-bit value only.
#[derive(Clone)]
pub(crate) enum Addr {
    /// Immediate sign-extended and a Register.
    IR { simm32: u32, base: Reg },

    /// sign-extend-32-to-64(Immediate) + Register1 + (Register2 << Shift)
    IRRS {
        simm32: u32,
        base: Reg,
        index: Reg,
        shift: u8, /* 0 .. 3 only */
    },
}

impl Addr {
    // Constructors.

    pub(crate) fn imm_reg(simm32: u32, base: Reg) -> Self {
        debug_assert!(base.get_class() == RegClass::I64);
        Self::IR { simm32, base }
    }

    pub(crate) fn imm_reg_reg_shift(simm32: u32, base: Reg, index: Reg, shift: u8) -> Self {
        debug_assert!(base.get_class() == RegClass::I64);
        debug_assert!(index.get_class() == RegClass::I64);
        debug_assert!(shift <= 3);
        Addr::IRRS {
            simm32,
            base,
            index,
            shift,
        }
    }

    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            Addr::IR { simm32: _, base } => {
                collector.add_use(*base);
            }
            Addr::IRRS {
                simm32: _,
                base,
                index,
                shift: _,
            } => {
                collector.add_use(*base);
                collector.add_use(*index);
            }
        }
    }
}

impl ShowWithRRU for Addr {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            Addr::IR { simm32, base } => format!("{}({})", *simm32 as i32, base.show_rru(mb_rru)),
            Addr::IRRS {
                simm32,
                base,
                index,
                shift,
            } => format!(
                "{}({},{},{})",
                *simm32 as i32,
                base.show_rru(mb_rru),
                index.show_rru(mb_rru),
                1 << shift
            ),
        }
    }
}

/// An operand which is either an integer Register, a value in Memory or an Immediate.  This can
/// denote an 8, 16, 32 or 64 bit value.  For the Immediate form, in the 8- and 16-bit case, only
/// the lower 8 or 16 bits of `simm32` is relevant.  In the 64-bit case, the value denoted by
/// `simm32` is its sign-extension out to 64 bits.
#[derive(Clone)]
pub(crate) enum RMI {
    R { reg: Reg },
    M { addr: Addr },
    I { simm32: u32 },
}

impl RMI {
    // Constructors

    pub(crate) fn reg(reg: Reg) -> RMI {
        debug_assert!(reg.get_class() == RegClass::I64);
        RMI::R { reg }
    }
    pub(crate) fn mem(addr: Addr) -> RMI {
        RMI::M { addr }
    }
    pub(crate) fn imm(simm32: u32) -> RMI {
        RMI::I { simm32 }
    }

    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            RMI::R { reg } => collector.add_use(*reg),
            RMI::M { addr } => addr.get_regs_as_uses(collector),
            RMI::I { simm32: _ } => {}
        }
    }
}

impl ShowWithRRU for RMI {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        self.show_rru_sized(mb_rru, 8)
    }

    fn show_rru_sized(&self, mb_rru: Option<&RealRegUniverse>, size: u8) -> String {
        match self {
            RMI::R { reg } => show_ireg_sized(*reg, mb_rru, size),
            RMI::M { addr } => addr.show_rru(mb_rru),
            RMI::I { simm32 } => format!("${}", *simm32 as i32),
        }
    }
}

/// An operand which is either an integer Register or a value in Memory.  This can denote an 8, 16,
/// 32 or 64 bit value.
#[derive(Clone)]
pub(crate) enum RM {
    R { reg: Reg },
    M { addr: Addr },
}

impl RM {
    // Constructors.

    pub(crate) fn reg(reg: Reg) -> Self {
        debug_assert!(reg.get_class() == RegClass::I64);
        RM::R { reg }
    }

    pub(crate) fn mem(addr: Addr) -> Self {
        RM::M { addr }
    }

    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            RM::R { reg } => collector.add_use(*reg),
            RM::M { addr } => addr.get_regs_as_uses(collector),
        }
    }
}

impl ShowWithRRU for RM {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        self.show_rru_sized(mb_rru, 8)
    }

    fn show_rru_sized(&self, mb_rru: Option<&RealRegUniverse>, size: u8) -> String {
        match self {
            RM::R { reg } => show_ireg_sized(*reg, mb_rru, size),
            RM::M { addr } => addr.show_rru(mb_rru),
        }
    }
}

/// Some basic ALU operations.  TODO: maybe add Adc, Sbb.
#[derive(Clone, PartialEq)]
pub enum RMI_R_Op {
    Add,
    Sub,
    And,
    Or,
    Xor,
    /// The signless, non-extending (N x N -> N, for N in {32,64}) variant.
    Mul,
}

impl RMI_R_Op {
    pub(crate) fn to_string(&self) -> String {
        match self {
            RMI_R_Op::Add => "add".to_string(),
            RMI_R_Op::Sub => "sub".to_string(),
            RMI_R_Op::And => "and".to_string(),
            RMI_R_Op::Or => "or".to_string(),
            RMI_R_Op::Xor => "xor".to_string(),
            RMI_R_Op::Mul => "imul".to_string(),
        }
    }
}

impl fmt::Debug for RMI_R_Op {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.to_string())
    }
}

/// These indicate ways of extending (widening) a value, using the Intel naming:
/// B(yte) = u8, W(ord) = u16, L(ong)word = u32, Q(uad)word = u64
#[derive(Clone, PartialEq)]
pub enum ExtMode {
    /// Byte -> Longword.
    BL,
    /// Byte -> Quadword.
    BQ,
    /// Word -> Longword.
    WL,
    /// Word -> Quadword.
    WQ,
    /// Longword -> Quadword.
    LQ,
}

impl ExtMode {
    pub(crate) fn to_string(&self) -> String {
        match self {
            ExtMode::BL => "bl".to_string(),
            ExtMode::BQ => "bq".to_string(),
            ExtMode::WL => "wl".to_string(),
            ExtMode::WQ => "wq".to_string(),
            ExtMode::LQ => "lq".to_string(),
        }
    }

    pub(crate) fn dst_size(&self) -> u8 {
        match self {
            ExtMode::BL => 4,
            ExtMode::BQ => 8,
            ExtMode::WL => 4,
            ExtMode::WQ => 8,
            ExtMode::LQ => 8,
        }
    }
}

impl fmt::Debug for ExtMode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.to_string())
    }
}

/// These indicate the form of a scalar shift: left, signed right, unsigned right.
#[derive(Clone)]
pub enum ShiftKind {
    Left,
    RightZ,
    RightS,
}

impl ShiftKind {
    pub(crate) fn to_string(&self) -> String {
        match self {
            ShiftKind::Left => "shl".to_string(),
            ShiftKind::RightZ => "shr".to_string(),
            ShiftKind::RightS => "sar".to_string(),
        }
    }
}

impl fmt::Debug for ShiftKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.to_string())
    }
}

/// These indicate condition code tests.  Not all are represented since not all are useful in
/// compiler-generated code.
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum CC {
    ///  overflow
    O = 0,
    /// no overflow
    NO = 1,

    /// < unsigned
    B = 2,
    /// >= unsigned
    NB = 3,

    /// zero
    Z = 4,
    /// not-zero
    NZ = 5,

    /// <= unsigned
    BE = 6,
    /// > unsigend
    NBE = 7,

    /// negative
    S = 8,
    /// not-negative
    NS = 9,

    /// < signed
    L = 12,
    /// >= signed
    NL = 13,

    /// <= signed
    LE = 14,
    /// > signed
    NLE = 15,
}

impl CC {
    pub(crate) fn to_string(&self) -> String {
        match self {
            CC::O => "o".to_string(),
            CC::NO => "no".to_string(),
            CC::B => "b".to_string(),
            CC::NB => "nb".to_string(),
            CC::Z => "z".to_string(),
            CC::NZ => "nz".to_string(),
            CC::BE => "be".to_string(),
            CC::NBE => "nbe".to_string(),
            CC::S => "s".to_string(),
            CC::NS => "ns".to_string(),
            CC::L => "l".to_string(),
            CC::NL => "nl".to_string(),
            CC::LE => "le".to_string(),
            CC::NLE => "nle".to_string(),
        }
    }

    pub(crate) fn invert(&self) -> CC {
        match self {
            CC::O => CC::NO,
            CC::NO => CC::O,

            CC::B => CC::NB,
            CC::NB => CC::B,

            CC::Z => CC::NZ,
            CC::NZ => CC::Z,

            CC::BE => CC::NBE,
            CC::NBE => CC::BE,

            CC::S => CC::NS,
            CC::NS => CC::S,

            CC::L => CC::NL,
            CC::NL => CC::L,

            CC::LE => CC::NLE,
            CC::NLE => CC::LE,
        }
    }

    pub(crate) fn get_enc(self) -> u8 {
        self as u8
    }
}

impl fmt::Debug for CC {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.to_string())
    }
}

/// A branch target. Either unresolved (basic-block index) or resolved (offset
/// from end of current instruction).
#[derive(Clone, Copy, Debug)]
pub enum BranchTarget {
    /// An unresolved reference to a MachLabel.
    Label(MachLabel),

    /// A resolved reference to another instruction, in bytes.
    ResolvedOffset(isize),
}

impl ShowWithRRU for BranchTarget {
    fn show_rru(&self, _mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            BranchTarget::Label(l) => format!("{:?}", l),
            BranchTarget::ResolvedOffset(offs) => format!("(offset {})", offs),
        }
    }
}

impl BranchTarget {
    /// Get the label.
    pub fn as_label(&self) -> Option<MachLabel> {
        match self {
            &BranchTarget::Label(l) => Some(l),
            _ => None,
        }
    }

    /// Get the offset as a signed 32 bit byte offset.  This returns the
    /// offset in bytes between the first byte of the source and the first
    /// byte of the target.  It does not take into account the Intel-specific
    /// rule that a branch offset is encoded as relative to the start of the
    /// following instruction.  That is a problem for the emitter to deal
    /// with. If a label, returns zero.
    pub fn as_offset32_or_zero(&self) -> i32 {
        match self {
            &BranchTarget::ResolvedOffset(off) => {
                // Leave a bit of slack so that the emitter is guaranteed to
                // be able to add the length of the jump instruction encoding
                // to this value and still have a value in signed-32 range.
                assert!(off >= -0x7FFF_FF00 && off <= 0x7FFF_FF00);
                off as i32
            }
            _ => 0,
        }
    }
}
