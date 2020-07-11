//! Instruction operand sub-components (aka "parts"): definitions and printing.

use std::fmt;
use std::string::{String, ToString};

use regalloc::{RealRegUniverse, Reg, RegClass, RegUsageCollector, RegUsageMapper};

use crate::ir::condcodes::{FloatCC, IntCC};
use crate::machinst::*;

use super::{
    regs::{self, show_ireg_sized},
    EmitState,
};

/// A possible addressing mode (amode) that can be used in instructions.
/// These denote a 64-bit value only.
#[derive(Clone)]
pub enum Amode {
    /// Immediate sign-extended and a Register.
    ImmReg { simm32: u32, base: Reg },

    /// sign-extend-32-to-64(Immediate) + Register1 + (Register2 << Shift)
    ImmRegRegShift {
        simm32: u32,
        base: Reg,
        index: Reg,
        shift: u8, /* 0 .. 3 only */
    },

    /// sign-extend-32-to-64(Immediate) + RIP (instruction pointer).
    /// To wit: not supported in 32-bits mode.
    RipRelative { target: BranchTarget },
}

impl Amode {
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

    pub(crate) fn rip_relative(target: BranchTarget) -> Self {
        Self::RipRelative { target }
    }

    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            Amode::ImmReg { base, .. } => {
                collector.add_use(*base);
            }
            Amode::ImmRegRegShift { base, index, .. } => {
                collector.add_use(*base);
                collector.add_use(*index);
            }
            Amode::RipRelative { .. } => {
                // RIP isn't involved in regalloc.
            }
        }
    }
}

impl ShowWithRRU for Amode {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            Amode::ImmReg { simm32, base } => {
                format!("{}({})", *simm32 as i32, base.show_rru(mb_rru))
            }
            Amode::ImmRegRegShift {
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
            Amode::RipRelative { ref target } => format!(
                "{}(%rip)",
                match target {
                    BranchTarget::Label(label) => format!("label{}", label.get()),
                    BranchTarget::ResolvedOffset(offset) => offset.to_string(),
                }
            ),
        }
    }
}

/// A Memory Address. These denote a 64-bit value only.
/// Used for usual addressing modes as well as addressing modes used during compilation, when the
/// moving SP offset is not known.
#[derive(Clone)]
pub enum SyntheticAmode {
    /// A real amode.
    Real(Amode),

    /// A (virtual) offset to the "nominal SP" value, which will be recomputed as we push and pop
    /// within the function.
    NominalSPOffset { simm32: u32 },
}

impl SyntheticAmode {
    pub(crate) fn nominal_sp_offset(simm32: u32) -> Self {
        SyntheticAmode::NominalSPOffset { simm32 }
    }

    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            SyntheticAmode::Real(addr) => addr.get_regs_as_uses(collector),
            SyntheticAmode::NominalSPOffset { .. } => {
                // Nothing to do; the base is SP and isn't involved in regalloc.
            }
        }
    }

    pub(crate) fn map_uses<RUM: RegUsageMapper>(&mut self, map: &RUM) {
        match self {
            SyntheticAmode::Real(addr) => addr.map_uses(map),
            SyntheticAmode::NominalSPOffset { .. } => {
                // Nothing to do.
            }
        }
    }

    pub(crate) fn finalize(&self, state: &mut EmitState) -> Amode {
        match self {
            SyntheticAmode::Real(addr) => addr.clone(),
            SyntheticAmode::NominalSPOffset { simm32 } => {
                let off = *simm32 as i64 + state.virtual_sp_offset;
                // TODO will require a sequence of add etc.
                assert!(
                    off <= u32::max_value() as i64,
                    "amode finalize: add sequence NYI"
                );
                Amode::imm_reg(off as u32, regs::rsp())
            }
        }
    }
}

impl Into<SyntheticAmode> for Amode {
    fn into(self) -> SyntheticAmode {
        SyntheticAmode::Real(self)
    }
}

impl ShowWithRRU for SyntheticAmode {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        match self {
            SyntheticAmode::Real(addr) => addr.show_rru(mb_rru),
            SyntheticAmode::NominalSPOffset { simm32 } => {
                format!("rsp({} + virtual offset)", *simm32 as i32)
            }
        }
    }
}

/// An operand which is either an integer Register, a value in Memory or an Immediate.  This can
/// denote an 8, 16, 32 or 64 bit value.  For the Immediate form, in the 8- and 16-bit case, only
/// the lower 8 or 16 bits of `simm32` is relevant.  In the 64-bit case, the value denoted by
/// `simm32` is its sign-extension out to 64 bits.
#[derive(Clone)]
pub enum RegMemImm {
    Reg { reg: Reg },
    Mem { addr: SyntheticAmode },
    Imm { simm32: u32 },
}

impl RegMemImm {
    pub(crate) fn reg(reg: Reg) -> Self {
        debug_assert!(reg.get_class() == RegClass::I64);
        Self::Reg { reg }
    }
    pub(crate) fn mem(addr: impl Into<SyntheticAmode>) -> Self {
        Self::Mem { addr: addr.into() }
    }
    pub(crate) fn imm(simm32: u32) -> Self {
        Self::Imm { simm32 }
    }

    /// Asserts that in register mode, the reg class is the one that's expected.
    pub(crate) fn assert_regclass_is(&self, expected_reg_class: RegClass) {
        if let Self::Reg { reg } = self {
            debug_assert_eq!(reg.get_class(), expected_reg_class);
        }
    }

    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            Self::Reg { reg } => collector.add_use(*reg),
            Self::Mem { addr } => addr.get_regs_as_uses(collector),
            Self::Imm { .. } => {}
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
pub enum RegMem {
    Reg { reg: Reg },
    Mem { addr: SyntheticAmode },
}

impl RegMem {
    pub(crate) fn reg(reg: Reg) -> Self {
        debug_assert!(reg.get_class() == RegClass::I64 || reg.get_class() == RegClass::V128);
        Self::Reg { reg }
    }
    pub(crate) fn mem(addr: impl Into<SyntheticAmode>) -> Self {
        Self::Mem { addr: addr.into() }
    }
    /// Asserts that in register mode, the reg class is the one that's expected.
    pub(crate) fn assert_regclass_is(&self, expected_reg_class: RegClass) {
        if let Self::Reg { reg } = self {
            debug_assert_eq!(reg.get_class(), expected_reg_class);
        }
    }
    /// Add the regs mentioned by `self` to `collector`.
    pub(crate) fn get_regs_as_uses(&self, collector: &mut RegUsageCollector) {
        match self {
            RegMem::Reg { reg } => collector.add_use(*reg),
            RegMem::Mem { addr, .. } => addr.get_regs_as_uses(collector),
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
            RegMem::Mem { addr, .. } => addr.show_rru(mb_rru),
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

impl fmt::Display for AluRmiROpcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone, PartialEq)]
pub enum UnaryRmROpcode {
    /// Bit-scan reverse.
    Bsr,
    /// Bit-scan forward.
    Bsf,
}

impl fmt::Debug for UnaryRmROpcode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnaryRmROpcode::Bsr => write!(fmt, "bsr"),
            UnaryRmROpcode::Bsf => write!(fmt, "bsf"),
        }
    }
}

impl fmt::Display for UnaryRmROpcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
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
    Andps,
    Andnps,
    Comiss,
    Comisd,
    Cmpss,
    Cmpsd,
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
    Insertps,
    Maxss,
    Maxsd,
    Minss,
    Minsd,
    Movaps,
    Movd,
    Movq,
    Movss,
    Movsd,
    Mulss,
    Mulsd,
    Orps,
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
            | SseOpcode::Andps
            | SseOpcode::Andnps
            | SseOpcode::Cvtsi2ss
            | SseOpcode::Cvtss2si
            | SseOpcode::Cvttss2si
            | SseOpcode::Divss
            | SseOpcode::Maxss
            | SseOpcode::Movaps
            | SseOpcode::Minss
            | SseOpcode::Movss
            | SseOpcode::Mulss
            | SseOpcode::Orps
            | SseOpcode::Rcpss
            | SseOpcode::Rsqrtss
            | SseOpcode::Subss
            | SseOpcode::Ucomiss
            | SseOpcode::Sqrtss
            | SseOpcode::Comiss
            | SseOpcode::Cmpss => SSE,

            SseOpcode::Addsd
            | SseOpcode::Cvtsd2ss
            | SseOpcode::Cvtsd2si
            | SseOpcode::Cvtsi2sd
            | SseOpcode::Cvtss2sd
            | SseOpcode::Cvttsd2si
            | SseOpcode::Divsd
            | SseOpcode::Maxsd
            | SseOpcode::Minsd
            | SseOpcode::Movd
            | SseOpcode::Movq
            | SseOpcode::Movsd
            | SseOpcode::Mulsd
            | SseOpcode::Sqrtsd
            | SseOpcode::Subsd
            | SseOpcode::Ucomisd
            | SseOpcode::Comisd
            | SseOpcode::Cmpsd => SSE2,

            SseOpcode::Insertps | SseOpcode::Roundss | SseOpcode::Roundsd => SSE41,
        }
    }

    /// Returns the src operand size for an instruction.
    pub(crate) fn src_size(&self) -> u8 {
        match self {
            SseOpcode::Movd => 4,
            _ => 8,
        }
    }
}

impl fmt::Debug for SseOpcode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            SseOpcode::Addss => "addss",
            SseOpcode::Addsd => "addsd",
            SseOpcode::Andps => "andps",
            SseOpcode::Andnps => "andnps",
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
            SseOpcode::Movaps => "movaps",
            SseOpcode::Movd => "movd",
            SseOpcode::Movq => "movq",
            SseOpcode::Movss => "movss",
            SseOpcode::Movsd => "movsd",
            SseOpcode::Mulss => "mulss",
            SseOpcode::Mulsd => "mulsd",
            SseOpcode::Orps => "orps",
            SseOpcode::Rcpss => "rcpss",
            SseOpcode::Roundss => "roundss",
            SseOpcode::Roundsd => "roundsd",
            SseOpcode::Rsqrtss => "rsqrtss",
            SseOpcode::Sqrtss => "sqrtss",
            SseOpcode::Sqrtsd => "sqrtsd",
            SseOpcode::Subss => "subss",
            SseOpcode::Subsd => "subsd",
            SseOpcode::Ucomiss => "ucomiss",
            SseOpcode::Ucomisd => "ucomisd",
            SseOpcode::Cmpss => "cmpss",
            SseOpcode::Cmpsd => "cmpsd",
            SseOpcode::Insertps => "insertps",
        };
        write!(fmt, "{}", name)
    }
}

impl fmt::Display for SseOpcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
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
    pub(crate) fn src_size(&self) -> u8 {
        match self {
            ExtMode::BL | ExtMode::BQ => 1,
            ExtMode::WL | ExtMode::WQ => 2,
            ExtMode::LQ => 4,
        }
    }
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

impl fmt::Display for ExtMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// These indicate the form of a scalar shift/rotate: left, signed right, unsigned right.
#[derive(Clone)]
pub enum ShiftKind {
    ShiftLeft,
    /// Inserts zeros in the most significant bits.
    ShiftRightLogical,
    /// Replicates the sign bit in the most significant bits.
    ShiftRightArithmetic,
    RotateLeft,
    RotateRight,
}

impl fmt::Debug for ShiftKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            ShiftKind::ShiftLeft => "shl",
            ShiftKind::ShiftRightLogical => "shr",
            ShiftKind::ShiftRightArithmetic => "sar",
            ShiftKind::RotateLeft => "rol",
            ShiftKind::RotateRight => "ror",
        };
        write!(fmt, "{}", name)
    }
}

impl fmt::Display for ShiftKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// What kind of division or remainer instruction this is?
#[derive(Clone)]
pub enum DivOrRemKind {
    SignedDiv,
    UnsignedDiv,
    SignedRem,
    UnsignedRem,
}

impl DivOrRemKind {
    pub(crate) fn is_signed(&self) -> bool {
        match self {
            DivOrRemKind::SignedDiv | DivOrRemKind::SignedRem => true,
            _ => false,
        }
    }

    pub(crate) fn is_div(&self) -> bool {
        match self {
            DivOrRemKind::SignedDiv | DivOrRemKind::UnsignedDiv => true,
            _ => false,
        }
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
    /// > unsigned
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

    /// parity
    P = 10,

    /// not parity
    NP = 11,
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

            CC::P => CC::NP,
            CC::NP => CC::P,
        }
    }

    pub(crate) fn from_floatcc(floatcc: FloatCC) -> Self {
        match floatcc {
            FloatCC::Ordered => CC::NP,
            FloatCC::Unordered => CC::P,
            // Alias for NE
            FloatCC::NotEqual | FloatCC::OrderedNotEqual => CC::NZ,
            // Alias for E
            FloatCC::UnorderedOrEqual => CC::Z,
            // Alias for A
            FloatCC::GreaterThan => CC::NBE,
            // Alias for AE
            FloatCC::GreaterThanOrEqual => CC::NB,
            FloatCC::UnorderedOrLessThan => CC::B,
            FloatCC::UnorderedOrLessThanOrEqual => CC::BE,
            FloatCC::Equal
            | FloatCC::LessThan
            | FloatCC::LessThanOrEqual
            | FloatCC::UnorderedOrGreaterThan
            | FloatCC::UnorderedOrGreaterThanOrEqual => unimplemented!(
                "No single condition code to guarantee ordered. Treat as special case."
            ),
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
            CC::P => "p",
            CC::NP => "np",
        };
        write!(fmt, "{}", name)
    }
}

impl fmt::Display for CC {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
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
