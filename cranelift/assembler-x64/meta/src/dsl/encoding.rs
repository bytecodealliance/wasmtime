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
    }
}

/// An abbreviated constructor for VEX-encoded instructions.
#[must_use]
pub fn vex() -> Vex {
    Vex {}
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
            Encoding::Vex(vex) => vex.validate(),
        }
    }
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Encoding::Rex(rex) => write!(f, "{rex}"),
            Encoding::Vex(_vex) => todo!(),
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

    /// Check a subset of the rules for valid encodings outlined in chapter 2,
    /// _Instruction Format_, of the Intel® 64 and IA-32 Architectures Software
    /// Developer’s Manual, Volume 2A.
    fn validate(&self, operands: &[Operand]) {
        assert!(!(self.r && self.digit.is_some()));
        assert!(!(self.r && self.imm != Imm::None));
        assert!(
            !(self.w && (self.opcodes.prefixes.has_operand_size_override())),
            "though valid, if REX.W is set then the 66 prefix is ignored--avoid encoding this"
        );

        if self.opcodes.prefixes.has_operand_size_override() {
            assert!(
                operands.iter().all(|&op| matches!(
                    op.location.kind(),
                    OperandKind::Imm(_) | OperandKind::FixedReg(_)
                ) || op.location.bits() == 16
                    || op.location.bits() == 128),
                "when we encode the 66 prefix, we expect all operands to be 16-bit wide"
            );
        }

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
        write!(f, "{:#04x}", self.opcodes.primary)?;
        if let Some(secondary) = self.opcodes.secondary {
            write!(f, " {secondary:#04x}")?;
        }
        if self.r {
            write!(f, " /r")?;
        }
        if let Some(digit) = self.digit {
            write!(f, " /{digit}")?;
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

    /// Check if the `0x66` prefix is present.
    fn has_operand_size_override(&self) -> bool {
        matches!(self.group3, Some(Group3Prefix::OperandSizeOverride))
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
    fn bits(&self) -> u8 {
        match self {
            Imm::None => 0,
            Imm::ib => 8,
            Imm::iw => 16,
            Imm::id => 32,
            Imm::io => 64,
        }
    }
}

impl fmt::Display for Imm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Imm::None => write!(f, ""),
            Imm::ib => write!(f, "ib"),
            Imm::iw => write!(f, "iw"),
            Imm::id => write!(f, "id"),
            Imm::io => write!(f, "io"),
        }
    }
}

pub struct Vex {}

impl Vex {
    fn validate(&self) {
        todo!()
    }
}
