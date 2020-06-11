//! Instruction operand sub-components (aka "parts"): definitions and printing.

use std::fmt;
use std::string::{String, ToString};

use regalloc::{RealRegUniverse, Reg, RegClass, RegUsageCollector};

use crate::ir::condcodes::IntCC;
use crate::machinst::*;

use super::regs::show_ireg_sized;

/// A Memory Address. These denote a 64-bit value only.
#[derive(Clone)]
pub(crate) enum Addr {
    /// Immediate sign-extended and a Register.
    ImmReg { simm32: u32, base: Reg },

    /// sign-extend-32-to-64(Immediate) + Register1 + (Register2 << Shift)
    ImmRegRegShift {
        simm32: u32,
        base: Reg,
        index: Reg,
        shift: u8, /* 0 .. 3 only */
    },
}

impl Addr {
    pub(crate) fn imm_reg(simm32: u32, base: Reg) -> Self {
        debug_assert!(base.get_class() == RegClass::I64);
        Self::ImmReg { simm32, base }
    }

    pub(crate) fn imm_reg_reg_shift(simm32: u32, base: Reg, index: Reg, shift: u8) -> Self {
        debug_assert!(base.get_class() == RegClass::I64);
        debug_assert!(index.get_class() == RegClass::I64);
        debug_assert!(shift <= 3);
        Self::ImmRegRegShift {
            simm32,
            base,
            index,
            shift,
        }
    }

    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            Addr::ImmReg { simm32: _, base } => {
                collector.add_use(*base);
            }
            Addr::ImmRegRegShift {
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
            Addr::ImmReg { simm32, base } => {
                format!("{}({})", *simm32 as i32, base.show_rru(mb_rru))
            }
            Addr::ImmRegRegShift {
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
pub(crate) enum RegMemImm {
    Reg { reg: Reg },
    Mem { addr: Addr },
    Imm { simm32: u32 },
}

impl RegMemImm {
    pub(crate) fn reg(reg: Reg) -> Self {
        debug_assert!(reg.get_class() == RegClass::I64);
        Self::Reg { reg }
    }
    pub(crate) fn mem(addr: Addr) -> Self {
        Self::Mem { addr }
    }
    pub(crate) fn imm(simm32: u32) -> Self {
        Self::Imm { simm32 }
    }

    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            Self::Reg { reg } => collector.add_use(*reg),
            Self::Mem { addr } => addr.get_regs_as_uses(collector),
            Self::Imm { simm32: _ } => {}
        }
    }
}

impl ShowWithRRU for RegMemImm {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        self.show_rru_sized(mb_rru, 8)
    }

    fn show_rru_sized(&self, mb_rru: Option<&RealRegUniverse>, size: u8) -> String {
        match self {
            Self::Reg { reg } => show_ireg_sized(*reg, mb_rru, size),
            Self::Mem { addr } => addr.show_rru(mb_rru),
            Self::Imm { simm32 } => format!("${}", *simm32 as i32),
        }
    }
}

/// An operand which is either an integer Register or a value in Memory.  This can denote an 8, 16,
/// 32 or 64 bit value.
#[derive(Clone)]
pub(crate) enum RegMem {
    Reg { reg: Reg },
    Mem { addr: Addr },
}

impl RegMem {
    pub(crate) fn reg(reg: Reg) -> Self {
        debug_assert!(reg.get_class() == RegClass::I64 || reg.get_class() == RegClass::V128);
        Self::Reg { reg }
    }
    pub(crate) fn mem(addr: Addr) -> Self {
        Self::Mem { addr }
    }

    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            RegMem::Reg { reg } => collector.add_use(*reg),
            RegMem::Mem { addr } => addr.get_regs_as_uses(collector),
        }
    }
}

impl ShowWithRRU for RegMem {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        self.show_rru_sized(mb_rru, 8)
    }

    fn show_rru_sized(&self, mb_rru: Option<&RealRegUniverse>, size: u8) -> String {
        match self {
            RegMem::Reg { reg } => show_ireg_sized(*reg, mb_rru, size),
            RegMem::Mem { addr } => addr.show_rru(mb_rru),
        }
    }
}

/// Some basic ALU operations.  TODO: maybe add Adc, Sbb.
#[derive(Clone, PartialEq)]
pub enum AluRmiROpcode {
    Add,
    Sub,
    And,
    Or,
    Xor,
    /// The signless, non-extending (N x N -> N, for N in {32,64}) variant.
    Mul,
}

impl fmt::Debug for AluRmiROpcode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            AluRmiROpcode::Add => "add",
            AluRmiROpcode::Sub => "sub",
            AluRmiROpcode::And => "and",
            AluRmiROpcode::Or => "or",
            AluRmiROpcode::Xor => "xor",
            AluRmiROpcode::Mul => "imul",
        };
        write!(fmt, "{}", name)
    }
}

impl ToString for AluRmiROpcode {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

pub(crate) enum InstructionSet {
    SSE,
    SSE2,
    SSE41,
}

/// Some scalar SSE operations requiring 2 operands r/m and r.
/// TODO: Below only includes scalar operations. To be seen if packed will be added here.
#[derive(Clone, PartialEq)]
pub enum SseOpcode {
    Addss,
    Addsd,
    Comiss,
    Comisd,
    Cvtsd2ss,
    Cvtsd2si,
    Cvtsi2ss,
    Cvtsi2sd,
    Cvtss2si,
    Cvtss2sd,
    Cvttss2si,
    Cvttsd2si,
    Divss,
    Divsd,
    Maxss,
    Maxsd,
    Minss,
    Minsd,
    Movss,
    Movsd,
    Mulss,
    Mulsd,
    Rcpss,
    Roundss,
    Roundsd,
    Rsqrtss,
    Sqrtss,
    Sqrtsd,
    Subss,
    Subsd,
    Ucomiss,
    Ucomisd,
}

impl SseOpcode {
    /// Which `InstructionSet` is the first supporting this opcode?
    pub(crate) fn available_from(&self) -> InstructionSet {
        use InstructionSet::*;
        match self {
            SseOpcode::Addss
            | SseOpcode::Cvtsi2ss
            | SseOpcode::Cvtss2si
            | SseOpcode::Cvttss2si
            | SseOpcode::Divss
            | SseOpcode::Maxss
            | SseOpcode::Minss
            | SseOpcode::Movss
            | SseOpcode::Mulss
            | SseOpcode::Rcpss
            | SseOpcode::Rsqrtss
            | SseOpcode::Subss
            | SseOpcode::Ucomiss
            | SseOpcode::Sqrtss
            | SseOpcode::Comiss => SSE,

            SseOpcode::Addsd
            | SseOpcode::Cvtsd2ss
            | SseOpcode::Cvtsd2si
            | SseOpcode::Cvtsi2sd
            | SseOpcode::Cvtss2sd
            | SseOpcode::Cvttsd2si
            | SseOpcode::Divsd
            | SseOpcode::Maxsd
            | SseOpcode::Minsd
            | SseOpcode::Movsd
            | SseOpcode::Mulsd
            | SseOpcode::Sqrtsd
            | SseOpcode::Subsd
            | SseOpcode::Ucomisd
            | SseOpcode::Comisd => SSE2,

            SseOpcode::Roundss | SseOpcode::Roundsd => SSE41,
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self {
            SseOpcode::Addss => "addss".to_string(),
            SseOpcode::Subss => "subss".to_string(),
            SseOpcode::Movss => "movss".to_string(),
            SseOpcode::Movsd => "movsd".to_string(),
            _ => "unimplemented sse_op".to_string(),
        }
    }
}

impl fmt::Debug for SseOpcode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            SseOpcode::Addss => "addss",
            SseOpcode::Addsd => "addsd",
            SseOpcode::Comiss => "comiss",
            SseOpcode::Comisd => "comisd",
            SseOpcode::Cvtsd2ss => "cvtsd2ss",
            SseOpcode::Cvtsd2si => "cvtsd2si",
            SseOpcode::Cvtsi2ss => "cvtsi2ss",
            SseOpcode::Cvtsi2sd => "cvtsi2sd",
            SseOpcode::Cvtss2si => "cvtss2si",
            SseOpcode::Cvtss2sd => "cvtss2sd",
            SseOpcode::Cvttss2si => "cvttss2si",
            SseOpcode::Cvttsd2si => "cvttsd2si",
            SseOpcode::Divss => "divss",
            SseOpcode::Divsd => "divsd",
            SseOpcode::Maxss => "maxss",
            SseOpcode::Maxsd => "maxsd",
            SseOpcode::Minss => "minss",
            SseOpcode::Minsd => "minsd",
            SseOpcode::Movss => "movss",
            SseOpcode::Movsd => "movsd",
            SseOpcode::Mulss => "mulss",
            SseOpcode::Mulsd => "mulsd",
            SseOpcode::Rcpss => "rcpss",
            SseOpcode::Roundss => "roundss",
            SseOpcode::Roundsd => "roundsd",
            SseOpcode::Rsqrtss => "rsqrtss",
            SseOpcode::Sqrtss => "srtqss",
            SseOpcode::Sqrtsd => "sqrtsd",
            SseOpcode::Subss => "subss",
            SseOpcode::Subsd => "subsd",
            SseOpcode::Ucomiss => "ucomiss",
            SseOpcode::Ucomisd => "ucomisd",
        };
        write!(fmt, "{}", name)
    }
}

impl ToString for SseOpcode {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

/// Some SSE operations requiring 3 operands i, r/m, and r.
#[derive(Clone, PartialEq)]
pub enum SseRmiOpcode {
    Cmpss,
    Cmpsd,
    Insertps,
}

impl SseRmiOpcode {
    /// Which `InstructionSet` is the first supporting this opcode?
    pub(crate) fn available_from(&self) -> InstructionSet {
        use InstructionSet::*;
        match self {
            SseRmiOpcode::Cmpss => SSE,
            SseRmiOpcode::Cmpsd => SSE2,
            SseRmiOpcode::Insertps => SSE41,
        }
    }
}

/// These indicate ways of extending (widening) a value, using the Intel
/// naming: B(yte) = u8, W(ord) = u16, L(ong)word = u32, Q(uad)word = u64
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
    pub(crate) fn dst_size(&self) -> u8 {
        match self {
            ExtMode::BL | ExtMode::WL => 4,
            ExtMode::BQ | ExtMode::WQ | ExtMode::LQ => 8,
        }
    }
}

impl fmt::Debug for ExtMode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            ExtMode::BL => "bl",
            ExtMode::BQ => "bq",
            ExtMode::WL => "wl",
            ExtMode::WQ => "wq",
            ExtMode::LQ => "lq",
        };
        write!(fmt, "{}", name)
    }
}

impl ToString for ExtMode {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

/// These indicate the form of a scalar shift: left, signed right, unsigned right.
#[derive(Clone)]
pub enum ShiftKind {
    Left,
    RightZ,
    RightS,
}

impl fmt::Debug for ShiftKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            ShiftKind::Left => "shl",
            ShiftKind::RightZ => "shr",
            ShiftKind::RightS => "sar",
        };
        write!(fmt, "{}", name)
    }
}

impl ToString for ShiftKind {
    fn to_string(&self) -> String {
        format!("{:?}", self)
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
    pub(crate) fn from_intcc(intcc: IntCC) -> Self {
        match intcc {
            IntCC::Equal => CC::Z,
            IntCC::NotEqual => CC::NZ,
            IntCC::SignedGreaterThanOrEqual => CC::NL,
            IntCC::SignedGreaterThan => CC::NLE,
            IntCC::SignedLessThanOrEqual => CC::LE,
            IntCC::SignedLessThan => CC::L,
            IntCC::UnsignedGreaterThanOrEqual => CC::NB,
            IntCC::UnsignedGreaterThan => CC::NBE,
            IntCC::UnsignedLessThanOrEqual => CC::BE,
            IntCC::UnsignedLessThan => CC::B,
            IntCC::Overflow => CC::O,
            IntCC::NotOverflow => CC::NO,
        }
    }

    pub(crate) fn invert(&self) -> Self {
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
        let name = match self {
            CC::O => "o",
            CC::NO => "no",
            CC::B => "b",
            CC::NB => "nb",
            CC::Z => "z",
            CC::NZ => "nz",
            CC::BE => "be",
            CC::NBE => "nbe",
            CC::S => "s",
            CC::NS => "ns",
            CC::L => "l",
            CC::NL => "nl",
            CC::LE => "le",
            CC::NLE => "nle",
        };
        write!(fmt, "{}", name)
    }
}

impl ToString for CC {
    fn to_string(&self) -> String {
        format!("{:?}", self)
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
