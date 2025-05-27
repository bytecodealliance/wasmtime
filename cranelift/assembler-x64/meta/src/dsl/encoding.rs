//! A DSL for describing x64 encodings.
//!
//! Intended use:
//! - construct an encoding using an abbreviated helper, e.g., [`rex`]
//! - then, configure the encoding using builder methods, e.g., [`Rex::w`]
//!
//! ```
//! # use cranelift_assembler_x64_meta::dsl::rex;
//! let enc = rex(0x25).w().id();
//! assert_eq!(enc.to_string(), "REX.W + 0x25 id")
//! ```
//!
//! This module references the Intel® 64 and IA-32 Architectures Software
//! Development Manual, Volume 2: [link].
//!
//! [link]: https://software.intel.com/content/www/us/en/develop/articles/intel-sdm.html

use super::{Operand, OperandKind};
use core::fmt;

/// An abbreviated constructor for REX-encoded instructions.
#[must_use]
pub fn rex(opcode: impl Into<Opcodes>) -> Rex {
    Rex {
        opcodes: opcode.into(),
        w: false,
        r: false,
        digit: None,
        imm: Imm::None,
        opcode_mod: None,
    }
}

/// An abbreviated constructor for VEX-encoded instructions.
#[must_use]
pub fn vex(opcode: impl Into<Opcodes>) -> Vex {
    Vex {
        opcodes: opcode.into(),
        w: false,
        length: VexLength::_128,
        mmmmm: VexMMMMM::None,
        pp: VexPP::None,
        imm: None,
    }
}

/// Enumerate the ways x64 encodes instructions.
pub enum Encoding {
    Rex(Rex),
    Vex(Vex),
}

impl Encoding {
    /// Check that the encoding is valid for the given operands; this can find
    /// issues earlier, before generating any Rust code.
    pub fn validate(&self, operands: &[Operand]) {
        match self {
            Encoding::Rex(rex) => rex.validate(operands),
            Encoding::Vex(vex) => vex.validate(operands),
        }
    }
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Encoding::Rex(rex) => write!(f, "{rex}"),
            Encoding::Vex(vex) => write!(f, "{vex}"),
        }
    }
}

/// The traditional x64 encoding.
///
/// We use the "REX" name here in a slightly unorthodox way: "REX" is the name
/// for the optional _byte_ extending the number of available registers, e.g.,
/// but we use it here to distinguish this from other encoding formats (e.g.,
/// VEX, EVEX). The "REX" _byte_ is still optional in this encoding and only
/// emitted when necessary.
pub struct Rex {
    /// The opcodes for this instruction.
    ///
    /// Multi-byte opcodes are handled by passing an array of opcodes (including
    /// prefixes like `0x66` and escape bytes like `0x0f`) to the constructor.
    /// E.g., `66 0F 54` (`ANDPD`) is expressed as follows:
    ///
    /// ```
    /// # use cranelift_assembler_x64_meta::dsl::rex;
    /// let enc = rex([0x66, 0x0f, 0x54]);
    /// ```
    pub opcodes: Opcodes,
    /// Indicates setting the REX.W bit.
    ///
    /// From the reference manual: "Indicates the use of a REX prefix that
    /// affects operand size or instruction semantics. The ordering of the REX
    /// prefix and other optional/mandatory instruction prefixes are discussed
    /// in chapter 2. Note that REX prefixes that promote legacy instructions to
    /// 64-bit behavior are not listed explicitly in the opcode column."
    pub w: bool,
    /// From the reference manual: "indicates that the ModR/M byte of the
    /// instruction contains a register operand and an r/m operand."
    pub r: bool,
    /// From the reference manual: "a digit between 0 and 7 indicates that the
    /// ModR/M byte of the instruction uses only the r/m (register or memory)
    /// operand. The reg field contains the digit that provides an extension to
    /// the instruction's opcode."
    pub digit: Option<u8>,
    /// The number of bits used as an immediate operand to the instruction.
    pub imm: Imm,
    /// Used for `+rb`, `+rw`, `+rd`, and `+ro` instructions, which encode `reg`
    /// bits in the opcode byte; if `Some`, this contains the expected bit width
    /// of `reg`.
    ///
    /// From the reference manual: "[...] the lower 3 bits of the opcode byte is
    /// used to encode the register operand without a modR/M byte. The
    /// instruction lists the corresponding hexadecimal value of the opcode byte
    /// with low 3 bits as 000b. In non-64-bit mode, a register code, from 0
    /// through 7, is added to the hexadecimal value of the opcode byte. In
    /// 64-bit mode, indicates the four bit field of REX.b and opcode[2:0] field
    /// encodes the register operand of the instruction. “+ro” is applicable
    /// only in 64-bit mode."
    pub opcode_mod: Option<OpcodeMod>,
}

impl Rex {
    /// Set the `REX.W` bit.
    #[must_use]
    pub fn w(self) -> Self {
        Self { w: true, ..self }
    }

    /// Set the ModR/M byte to contain a register operand and an r/m operand;
    /// equivalent to `/r` in the reference manual.
    #[must_use]
    pub fn r(self) -> Self {
        Self { r: true, ..self }
    }

    /// Set the digit extending the opcode; equivalent to `/<digit>` in the
    /// reference manual.
    ///
    /// # Panics
    ///
    /// Panics if `digit` is too large.
    #[must_use]
    pub fn digit(self, digit: u8) -> Self {
        assert!(digit <= 0b111, "must fit in 3 bits");
        Self {
            digit: Some(digit),
            ..self
        }
    }

    /// Append a byte-sized immediate operand (8-bit); equivalent to `ib` in the
    /// reference manual.
    ///
    /// # Panics
    ///
    /// Panics if an immediate operand is already set.
    #[must_use]
    pub fn ib(self) -> Self {
        assert_eq!(self.imm, Imm::None);
        Self {
            imm: Imm::ib,
            ..self
        }
    }

    /// Append a word-sized immediate operand (16-bit); equivalent to `iw` in
    /// the reference manual.
    ///
    /// # Panics
    ///
    /// Panics if an immediate operand is already set.
    #[must_use]
    pub fn iw(self) -> Self {
        assert_eq!(self.imm, Imm::None);
        Self {
            imm: Imm::iw,
            ..self
        }
    }

    /// Append a doubleword-sized immediate operand (32-bit); equivalent to `id`
    /// in the reference manual.
    ///
    /// # Panics
    ///
    /// Panics if an immediate operand is already set.
    #[must_use]
    pub fn id(self) -> Self {
        assert_eq!(self.imm, Imm::None);
        Self {
            imm: Imm::id,
            ..self
        }
    }

    /// Append a quadword-sized immediate operand (64-bit); equivalent to `io`
    /// in the reference manual.
    ///
    /// # Panics
    ///
    /// Panics if an immediate operand is already set.
    #[must_use]
    pub fn io(self) -> Self {
        assert_eq!(self.imm, Imm::None);
        Self {
            imm: Imm::io,
            ..self
        }
    }

    /// Modify the opcode byte with bits from an 8-bit `reg`; equivalent to
    /// `+rb` in the reference manual.
    #[must_use]
    pub fn rb(self) -> Self {
        Self {
            opcode_mod: Some(OpcodeMod::rb),
            ..self
        }
    }

    /// Modify the opcode byte with bits from a 16-bit `reg`; equivalent to
    /// `+rw` in the reference manual.
    #[must_use]
    pub fn rw(self) -> Self {
        Self {
            opcode_mod: Some(OpcodeMod::rw),
            ..self
        }
    }

    /// Modify the opcode byte with bits from a 32-bit `reg`; equivalent to
    /// `+rd` in the reference manual.
    #[must_use]
    pub fn rd(self) -> Self {
        Self {
            opcode_mod: Some(OpcodeMod::rd),
            ..self
        }
    }

    /// Modify the opcode byte with bits from a 64-bit `reg`; equivalent to
    /// `+ro` in the reference manual.
    #[must_use]
    pub fn ro(self) -> Self {
        Self {
            opcode_mod: Some(OpcodeMod::ro),
            ..self
        }
    }

    /// Check a subset of the rules for valid encodings outlined in chapter 2,
    /// _Instruction Format_, of the Intel® 64 and IA-32 Architectures Software
    /// Developer’s Manual, Volume 2A.
    fn validate(&self, operands: &[Operand]) {
        assert!(!(self.r && self.digit.is_some()));

        if let Some(OperandKind::Imm(op)) = operands
            .iter()
            .map(|o| o.location.kind())
            .find(|k| matches!(k, OperandKind::Imm(_)))
        {
            assert_eq!(
                op.bits(),
                self.imm.bits(),
                "for an immediate, the encoding width must match the declared operand width"
            );
        }

        if let Some(opcode_mod) = &self.opcode_mod {
            assert!(
                self.opcodes.primary & 0b111 == 0,
                "the lower three bits of the opcode byte should be 0"
            );
            assert!(
                operands
                    .iter()
                    .all(|o| o.location.bits() == opcode_mod.bits().into()),
                "the opcode modifier width must match the operand widths"
            );
            assert!(!self.r, "the opcode modifier cannot be used with /r");
        }
    }
}

impl From<Rex> for Encoding {
    fn from(rex: Rex) -> Encoding {
        Encoding::Rex(rex)
    }
}

impl fmt::Display for Rex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(group1) = &self.opcodes.prefixes.group1 {
            write!(f, "{group1} + ")?;
        }
        if let Some(group2) = &self.opcodes.prefixes.group2 {
            write!(f, "{group2} + ")?;
        }
        if let Some(group3) = &self.opcodes.prefixes.group3 {
            write!(f, "{group3} + ")?;
        }
        if let Some(group4) = &self.opcodes.prefixes.group4 {
            write!(f, "{group4} + ")?;
        }
        if self.w {
            write!(f, "REX.W + ")?;
        }
        if self.opcodes.escape {
            write!(f, "0x0F + ")?;
        }
        write!(f, "{:#04X}", self.opcodes.primary)?;
        if let Some(secondary) = self.opcodes.secondary {
            write!(f, " {secondary:#04X}")?;
        }
        if self.r {
            write!(f, " /r")?;
        }
        if let Some(digit) = self.digit {
            write!(f, " /{digit}")?;
        }
        if let Some(opcode_mod) = &self.opcode_mod {
            write!(f, " {opcode_mod}")?;
        }
        if self.imm != Imm::None {
            write!(f, " {}", self.imm)?;
        }
        Ok(())
    }
}

/// Describe an instruction's opcodes. From section 2.1.2 "Opcodes" in the
/// reference manual:
///
/// > A primary opcode can be 1, 2, or 3 bytes in length. An additional 3-bit
/// > opcode field is sometimes encoded in the ModR/M byte. Smaller fields can
/// > be defined within the primary opcode. Such fields define the direction of
/// > operation, size of displacements, register encoding, condition codes, or
/// > sign extension. Encoding fields used by an opcode vary depending on the
/// > class of operation.
/// >
/// > Two-byte opcode formats for general-purpose and SIMD instructions consist
/// > of one of the following:
/// > - An escape opcode byte `0FH` as the primary opcode and a second opcode
/// >   byte.
/// > - A mandatory prefix (`66H`, `F2H`, or `F3H`), an escape opcode byte, and
/// >   a second opcode byte (same as previous bullet).
/// >
/// > For example, `CVTDQ2PD` consists of the following sequence: `F3 0F E6`.
/// > The first byte is a mandatory prefix (it is not considered as a repeat
/// > prefix).
/// >
/// > Three-byte opcode formats for general-purpose and SIMD instructions
/// > consist of one of the following:
/// > - An escape opcode byte `0FH` as the primary opcode, plus two additional
/// >   opcode bytes.
/// > - A mandatory prefix (`66H`, `F2H`, or `F3H`), an escape opcode byte, plus
/// >   two additional opcode bytes (same as previous bullet).
/// >
/// > For example, `PHADDW` for XMM registers consists of the following
/// > sequence: `66 0F 38 01`. The first byte is the mandatory prefix.
pub struct Opcodes {
    /// The prefix bytes for this instruction.
    pub prefixes: Prefixes,
    /// Indicates the use of an escape opcode byte, `0x0f`.
    pub escape: bool,
    /// The primary opcode.
    pub primary: u8,
    /// Some instructions (e.g., SIMD) may have a secondary opcode.
    pub secondary: Option<u8>,
}

impl From<u8> for Opcodes {
    fn from(primary: u8) -> Opcodes {
        Opcodes {
            prefixes: Prefixes::default(),
            escape: false,
            primary,
            secondary: None,
        }
    }
}

impl<const N: usize> From<[u8; N]> for Opcodes {
    fn from(bytes: [u8; N]) -> Self {
        let (prefixes, remaining) = Prefixes::parse(&bytes);
        let (escape, primary, secondary) = match remaining {
            [primary] => (false, *primary, None),
            [0x0f, primary] => (true, *primary, None),
            [0x0f, primary, secondary] => (true, *primary, Some(*secondary)),
            _ => panic!(
                "invalid opcodes after prefix; expected [opcode], [0x0f, opcode], or [0x0f, opcode, opcode], found {remaining:?}"
            ),
        };
        Self {
            prefixes,
            escape,
            primary,
            secondary,
        }
    }
}

/// The allowed prefixes for an instruction. From the reference manual (section
/// 2.1.1):
///
/// > Instruction prefixes are divided into four groups, each with a set of
/// > allowable prefix codes. For each instruction, it is only useful to include
/// > up to one prefix code from each of the four groups (Groups 1, 2, 3, 4).
/// > Groups 1 through 4 may be placed in any order relative to each other.
#[derive(Default)]
pub struct Prefixes {
    pub group1: Option<Group1Prefix>,
    pub group2: Option<Group2Prefix>,
    pub group3: Option<Group3Prefix>,
    pub group4: Option<Group4Prefix>,
}

impl Prefixes {
    /// Parse a slice of `bytes` into a set of prefixes, returning both the
    /// configured [`Prefixes`] as well as any remaining bytes.
    fn parse(mut bytes: &[u8]) -> (Self, &[u8]) {
        let mut prefixes = Self::default();
        while !bytes.is_empty() && prefixes.try_assign(bytes[0]).is_ok() {
            bytes = &bytes[1..];
        }
        (prefixes, bytes)
    }

    /// Attempt to parse a `byte` as a prefix and, if successful, assigns it to
    /// the correct prefix group.
    ///
    /// # Panics
    ///
    /// This function panics if the prefix for a group is already set; this
    /// disallows specifying multiple prefixes per group.
    fn try_assign(&mut self, byte: u8) -> Result<(), ()> {
        if let Ok(p) = Group1Prefix::try_from(byte) {
            assert!(self.group1.is_none());
            self.group1 = Some(p);
            Ok(())
        } else if let Ok(p) = Group2Prefix::try_from(byte) {
            assert!(self.group2.is_none());
            self.group2 = Some(p);
            Ok(())
        } else if let Ok(p) = Group3Prefix::try_from(byte) {
            assert!(self.group3.is_none());
            self.group3 = Some(p);
            Ok(())
        } else if let Ok(p) = Group4Prefix::try_from(byte) {
            assert!(self.group4.is_none());
            self.group4 = Some(p);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Check if any prefix is present.
    pub fn is_empty(&self) -> bool {
        self.group1.is_none()
            && self.group2.is_none()
            && self.group3.is_none()
            && self.group4.is_none()
    }
}

pub enum Group1Prefix {
    /// The LOCK prefix (`0xf0`). From the reference manual:
    ///
    /// > The LOCK prefix (F0H) forces an operation that ensures exclusive use
    /// > of shared memory in a multiprocessor environment. See "LOCK—Assert
    /// > LOCK# Signal Prefix" in Chapter 3, Instruction Set Reference, A-L, for
    /// > a description of this prefix.
    Lock,
    /// A REPNE/REPNZ prefix (`0xf2`) or a BND prefix under certain conditions.
    /// `REP*` prefixes apply only to string and input/output instructions but
    /// can be used as mandatory prefixes in other kinds of instructions (e.g.,
    /// SIMD) From the reference manual:
    ///
    /// > Repeat prefixes (F2H, F3H) cause an instruction to be repeated for
    /// > each element of a string. Use these prefixes only with string and I/O
    /// > instructions (MOVS, CMPS, SCAS, LODS, STOS, INS, and OUTS). Use of
    /// > repeat prefixes and/or undefined opcodes with other Intel 64 or IA-32
    /// > instructions is reserved; such use may cause unpredictable behavior.
    /// >
    /// > Some instructions may use F2H, F3H as a mandatory prefix to express
    /// > distinct functionality.
    REPNorBND,
    /// A REPE/REPZ prefix (`0xf3`); `REP*` prefixes apply only to string and
    /// input/output instructions but can be used as mandatory prefixes in other
    /// kinds of instructions (e.g., SIMD). See `REPNorBND` for more details.
    REP_,
}

impl TryFrom<u8> for Group1Prefix {
    type Error = u8;
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Ok(match byte {
            0xF0 => Group1Prefix::Lock,
            0xF2 => Group1Prefix::REPNorBND,
            0xF3 => Group1Prefix::REP_,
            byte => return Err(byte),
        })
    }
}

impl fmt::Display for Group1Prefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Group1Prefix::Lock => write!(f, "0xF0"),
            Group1Prefix::REPNorBND => write!(f, "0xF2"),
            Group1Prefix::REP_ => write!(f, "0xF3"),
        }
    }
}

/// Contains the segment override prefixes or a (deprecated) branch hint when
/// used on a `Jcc` instruction. Note that using the segment override prefixes
/// on a branch instruction is reserved. See section 2.1.1, "Instruction
/// Prefixes," in the reference manual.
pub enum Group2Prefix {
    /// The CS segment override prefix (`0x2e`); also the "branch not taken"
    /// hint.
    CSorBNT,
    /// The SS segment override prefix (`0x36`).
    SS,
    /// The DS segment override prefix (`0x3e`); also the "branch taken" hint.
    DSorBT,
    /// The ES segment override prefix (`0x26`).
    ES,
    /// The FS segment override prefix (`0x64`).
    FS,
    /// The GS segment override prefix (`0x65`).
    GS,
}

impl TryFrom<u8> for Group2Prefix {
    type Error = u8;
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Ok(match byte {
            0x2E => Group2Prefix::CSorBNT,
            0x36 => Group2Prefix::SS,
            0x3E => Group2Prefix::DSorBT,
            0x26 => Group2Prefix::ES,
            0x64 => Group2Prefix::FS,
            0x65 => Group2Prefix::GS,
            byte => return Err(byte),
        })
    }
}

impl fmt::Display for Group2Prefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Group2Prefix::CSorBNT => write!(f, "0x2E"),
            Group2Prefix::SS => write!(f, "0x36"),
            Group2Prefix::DSorBT => write!(f, "0x3E"),
            Group2Prefix::ES => write!(f, "0x26"),
            Group2Prefix::FS => write!(f, "0x64"),
            Group2Prefix::GS => write!(f, "0x65"),
        }
    }
}

/// Contains the operand-size override prefix (`0x66`); also used as a SIMD
/// prefix. From the reference manual:
///
/// > The operand-size override prefix allows a program to switch between 16-
/// > and 32-bit operand sizes. Either size can be the default; use of the
/// > prefix selects the non-default size. Some SSE2/SSE3/SSSE3/SSE4
/// > instructions and instructions using a three-byte sequence of primary
/// > opcode bytes may use 66H as a mandatory prefix to express distinct
/// > functionality.
pub enum Group3Prefix {
    OperandSizeOverride,
}

impl TryFrom<u8> for Group3Prefix {
    type Error = u8;
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Ok(match byte {
            0x66 => Group3Prefix::OperandSizeOverride,
            byte => return Err(byte),
        })
    }
}

impl fmt::Display for Group3Prefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Group3Prefix::OperandSizeOverride => write!(f, "0x66"),
        }
    }
}

/// Contains the address-size override prefix (`0x67`). From the reference
/// manual:
///
/// > The address-size override prefix (67H) allows programs to switch between
/// > 16- and 32-bit addressing. Either size can be the default; the prefix
/// > selects the non-default size.
pub enum Group4Prefix {
    AddressSizeOverride,
}

impl TryFrom<u8> for Group4Prefix {
    type Error = u8;
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Ok(match byte {
            0x67 => Group4Prefix::AddressSizeOverride,
            byte => return Err(byte),
        })
    }
}

impl fmt::Display for Group4Prefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Group4Prefix::AddressSizeOverride => write!(f, "0x67"),
        }
    }
}

/// Indicate the size of an immediate operand. From the reference manual:
///
/// > A 1-byte (ib), 2-byte (iw), 4-byte (id) or 8-byte (io) immediate operand
/// > to the instruction that follows the opcode, ModR/M bytes or scale-indexing
/// > bytes. The opcode determines if the operand is a signed value. All words,
/// > doublewords, and quadwords are given with the low-order byte first.
#[derive(Debug, PartialEq)]
#[allow(non_camel_case_types, reason = "makes DSL definitions easier to read")]
pub enum Imm {
    None,
    ib,
    iw,
    id,
    io,
}

impl Imm {
    fn bits(&self) -> u16 {
        match self {
            Self::None => 0,
            Self::ib => 8,
            Self::iw => 16,
            Self::id => 32,
            Self::io => 64,
        }
    }
}

impl fmt::Display for Imm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::None => write!(f, ""),
            Self::ib => write!(f, "ib"),
            Self::iw => write!(f, "iw"),
            Self::id => write!(f, "id"),
            Self::io => write!(f, "io"),
        }
    }
}

/// Indicate the size of the `reg` used when modifying the lower three bits of
/// the opcode byte; this corresponds to the `+rb`, `+rw`, `+rd`, and `+ro`
/// modifiers in the reference manual.
///
/// ```
/// # use cranelift_assembler_x64_meta::dsl::{rex};
/// // The `bswap` instruction extends the opcode byte:
/// let enc = rex([0x0F, 0xC8]).rd();
/// assert_eq!(enc.to_string(), "0x0F + 0xC8 +rd");
/// ```
#[derive(Debug, PartialEq)]
#[allow(non_camel_case_types, reason = "makes DSL definitions easier to read")]
pub enum OpcodeMod {
    rb,
    rw,
    rd,
    ro,
}

impl OpcodeMod {
    fn bits(&self) -> u8 {
        match self {
            Self::rb => 8,
            Self::rw => 16,
            Self::rd => 32,
            Self::ro => 64,
        }
    }
}

impl fmt::Display for OpcodeMod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::rb => write!(f, "+rb"),
            Self::rw => write!(f, "+rw"),
            Self::rd => write!(f, "+rd"),
            Self::ro => write!(f, "+ro"),
        }
    }
}

pub struct Vex {
    pub opcodes: Opcodes,
    pub w: bool,
    pub length: VexLength,
    pub mmmmm: VexMMMMM,
    pub pp: VexPP,
    pub imm: Option<u8>,
}

#[derive(PartialEq)]
pub enum VexPP {
    None,
    /// Operand size override -- here, denoting "16-bit operation".
    _66,
    /// REPNE, but no specific meaning here -- is just an opcode extension.
    _F2,
    /// REP/REPE, but no specific meaning here -- is just an opcode extension.
    _F3,
}

impl fmt::Display for VexPP {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VexPP::None => write!(f, "None"),
            VexPP::_66 => write!(f, "_66"),
            VexPP::_F3 => write!(f, "_F3"),
            VexPP::_F2 => write!(f, "_F2"),
        }
    }
}

pub enum VexLength {
    _128,
}

impl fmt::Display for VexLength {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VexLength::_128 => write!(f, "_128"),
        }
    }
}

#[derive(PartialEq)]
pub enum VexMMMMM {
    None,
    _OF,
    /// Operand size override -- here, denoting "16-bit operation".
    _OF3A,
    /// The lock prefix.
    _OF38,
}

impl fmt::Display for VexMMMMM {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VexMMMMM::None => write!(f, "None"),
            VexMMMMM::_OF => write!(f, "_0F"),
            VexMMMMM::_OF3A => write!(f, "_OF3A"),
            VexMMMMM::_OF38 => write!(f, "_OF38"),
        }
    }
}

/// Describe the register index to use. This wrapper is a type-safe way to pass
/// around the registers defined in `inst/regs.rs`.
#[derive(Debug, Copy, Clone, Default)]
pub struct Register(u8);
impl From<u8> for Register {
    fn from(reg: u8) -> Self {
        debug_assert!(reg < 16);
        Self(reg)
    }
}
impl From<Register> for u8 {
    fn from(reg: Register) -> u8 {
        reg.0
    }
}

impl Vex {
    pub fn length(self, length: VexLength) -> Self {
        Self { length, ..self }
    }
    pub fn pp(self, pp: VexPP) -> Self {
        Self { pp, ..self }
    }
    pub fn mmmmm(self, mmmmm: VexMMMMM) -> Self {
        Self { mmmmm, ..self }
    }

    fn validate(&self, _operands: &[Operand]) {}
}

impl From<Vex> for Encoding {
    fn from(vex: Vex) -> Encoding {
        Encoding::Vex(vex)
    }
}

impl fmt::Display for Vex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VEX")?;
        match self.length {
            VexLength::_128 => write!(f, ".128")?,
        }
        write!(f, " {:#04x}", self.opcodes.primary)?;
        Ok(())
    }
}
