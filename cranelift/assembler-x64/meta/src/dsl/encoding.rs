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
        digit: 0,
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
    pub digit: u8,
    /// The number of bits used as an immediate operand to the instruction.
    ///
    /// From the reference manual: "a 1-byte (ib), 2-byte (iw), 4-byte (id) or
    /// 8-byte (io) immediate operand to the instruction that follows the
    /// opcode, ModR/M bytes or scale-indexing bytes. The opcode determines if
    /// the operand is a signed value. All words, doublewords, and quadwords are
    /// given with the low-order byte first."
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
        assert!(digit < 8);
        Self { digit, ..self }
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
        Self { imm: Imm::ib, ..self }
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
        Self { imm: Imm::iw, ..self }
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
        Self { imm: Imm::id, ..self }
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
        Self { imm: Imm::io, ..self }
    }

    /// Check a subset of the rules for valid encodings outlined in chapter 2,
    /// _Instruction Format_, of the Intel® 64 and IA-32 Architectures Software
    /// Developer’s Manual, Volume 2A.
    fn validate(&self, operands: &[Operand]) {
        assert!(self.digit < 8);
        assert!(!(self.r && self.digit > 0));
        assert!(!(self.r && self.imm != Imm::None));
        assert!(
            !(self.w && (self.opcodes.prefix.contains_66())),
            "though valid, if REX.W is set then the 66 prefix is ignored--avoid encoding this"
        );

        if self.opcodes.prefix.contains_66() {
            assert!(
                operands.iter().all(|&op| op.location.bits() == 16),
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
        match self.opcodes.prefix {
            LegacyPrefix::NoPrefix => {}
            LegacyPrefix::_66 => write!(f, "0x66 + ")?,
            LegacyPrefix::_F0 => write!(f, "0xF0 + ")?,
            LegacyPrefix::_66F0 => write!(f, "0x66 0xF0 + ")?,
            LegacyPrefix::_F2 => write!(f, "0xF2 + ")?,
            LegacyPrefix::_F3 => write!(f, "0xF3 + ")?,
            LegacyPrefix::_66F3 => write!(f, "0x66 0xF3 + ")?,
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
        if self.digit > 0 {
            write!(f, " /{}", self.digit)?;
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
    pub prefix: LegacyPrefix,
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
            prefix: LegacyPrefix::NoPrefix,
            escape: false,
            primary,
            secondary: None,
        }
    }
}

impl From<[u8; 1]> for Opcodes {
    fn from(bytes: [u8; 1]) -> Opcodes {
        Opcodes::from(bytes[0])
    }
}

impl From<[u8; 2]> for Opcodes {
    fn from(bytes: [u8; 2]) -> Opcodes {
        let [a, b] = bytes;
        match (LegacyPrefix::try_from(a), b) {
            (Ok(prefix), primary) => Opcodes { prefix, escape: false, primary, secondary: None },
            (Err(0x0f), primary) => Opcodes {
                prefix: LegacyPrefix::NoPrefix,
                escape: true,
                primary,
                secondary: None,
            },
            _ => panic!("invalid opcodes; expected [prefix, opcode] or [0x0f, opcode]"),
        }
    }
}

impl From<[u8; 3]> for Opcodes {
    fn from(bytes: [u8; 3]) -> Opcodes {
        let [a, b, c] = bytes;
        match (LegacyPrefix::try_from(a), b, c) {
            (Ok(prefix), 0x0f, primary) => Opcodes { prefix, escape: false, primary, secondary: None },
            (Err(0x0f), primary, secondary) => Opcodes {
                prefix: LegacyPrefix::NoPrefix,
                escape: true,
                primary,
                secondary: Some(secondary),
            },
            _ => panic!("invalid opcodes; expected [prefix, 0x0f, opcode] or [0x0f, opcode, opcode]"),
        }
    }
}

impl From<[u8; 4]> for Opcodes {
    fn from(bytes: [u8; 4]) -> Opcodes {
        let [a, b, c, d] = bytes;
        match (LegacyPrefix::try_from(a), b, c, d) {
            (Ok(prefix), 0x0f, primary, secondary) => Opcodes {
                prefix,
                escape: false,
                primary,
                secondary: Some(secondary),
            },
            _ => panic!("invalid opcodes; expected [prefix, 0x0f, opcode, opcode]"),
        }
    }
}

/// A prefix byte for an instruction.
#[derive(PartialEq)]
pub enum LegacyPrefix {
    /// No prefix bytes.
    NoPrefix,
    /// An operand size override typically denoting "16-bit operation". But the
    /// reference manual is more nuanced:
    ///
    /// > The operand-size override prefix allows a program to switch between
    /// > 16- and 32-bit operand sizes. Either size can be the default; use of
    /// > the prefix selects the non-default.
    _66,
    /// The lock prefix.
    _F0,
    /// Operand size override and lock.
    _66F0,
    /// REPNE, but no specific meaning here -- is just an opcode extension.
    _F2,
    /// REP/REPE, but no specific meaning here -- is just an opcode extension.
    _F3,
    /// Operand size override and same effect as F3.
    _66F3,
}

impl LegacyPrefix {
    #[must_use]
    pub fn contains_66(&self) -> bool {
        match self {
            LegacyPrefix::_66 | LegacyPrefix::_66F0 | LegacyPrefix::_66F3 => true,
            LegacyPrefix::NoPrefix | LegacyPrefix::_F0 | LegacyPrefix::_F2 | LegacyPrefix::_F3 => false,
        }
    }
}

impl TryFrom<u8> for LegacyPrefix {
    type Error = u8;
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Ok(match byte {
            0x66 => LegacyPrefix::_66,
            0xF0 => LegacyPrefix::_F0,
            0xF2 => LegacyPrefix::_F2,
            0xF3 => LegacyPrefix::_F3,
            byte => return Err(byte),
        })
    }
}

#[derive(Debug, PartialEq)]
#[allow(non_camel_case_types)]
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
