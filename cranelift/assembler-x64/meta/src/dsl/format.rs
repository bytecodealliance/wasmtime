//! A DSL for describing x64 instruction formats--the shape of the operands.
//!
//! Every instruction has a format that corresponds to its encoding's expected
//! operands. The format is what allows us to generate code that accepts
//! operands of the right type and check that the operands are used in the right
//! way.
//!
//! The entry point for this module is [`fmt`].
//!
//! ```
//! # use cranelift_assembler_x64_meta::dsl::{fmt, rw, r, Location::*};
//! let f = fmt("rm", [rw(r32), r(rm32)]);
//! assert_eq!(f.to_string(), "rm(r32[rw], rm32)")
//! ```

/// An abbreviated constructor for an instruction "format."
///
/// These model what the reference manual calls "instruction operand encodings,"
/// usually defined in a table after an instruction's opcodes.
pub fn fmt(name: impl Into<String>, operands: impl IntoIterator<Item = impl Into<Operand>>) -> Format {
    Format {
        name: name.into(),
        operands: operands.into_iter().map(Into::into).collect(),
    }
}

/// An abbreviated constructor for a "read-write" operand.
///
/// # Panics
///
/// This function panics if the location is an immediate (i.e., an immediate
/// cannot be written to).
#[must_use]
pub fn rw(location: Location) -> Operand {
    assert!(!matches!(location.kind(), OperandKind::Imm(_)));
    Operand {
        location,
        mutability: Mutability::ReadWrite,
        extension: Extension::default(),
    }
}

/// An abbreviated constructor for a "read" operand.
#[must_use]
pub fn r(location: Location) -> Operand {
    Operand {
        location,
        mutability: Mutability::Read,
        extension: Extension::None,
    }
}

/// An abbreviated constructor for a "read" operand that is sign-extended to 64
/// bits (quadword).
///
/// # Panics
///
/// This function panics if the location size is too large to extend.
#[must_use]
pub fn sxq(location: Location) -> Operand {
    assert!(location.bits() <= 64);
    Operand {
        location,
        mutability: Mutability::Read,
        extension: Extension::SignExtendQuad,
    }
}

/// An abbreviated constructor for a "read" operand that is sign-extended to 32
/// bits (longword).
///
/// # Panics
///
/// This function panics if the location size is too large to extend.
#[must_use]
pub fn sxl(location: Location) -> Operand {
    assert!(location.bits() <= 32);
    Operand {
        location,
        mutability: Mutability::Read,
        extension: Extension::SignExtendLong,
    }
}

/// An abbreviated constructor for a "read" operand that is sign-extended to 16
/// bits (word).
///
/// # Panics
///
/// This function panics if the location size is too large to extend.
#[must_use]
pub fn sxw(location: Location) -> Operand {
    assert!(location.bits() <= 16);
    Operand {
        location,
        mutability: Mutability::Read,
        extension: Extension::SignExtendWord,
    }
}

/// A format describes the operands for an instruction.
pub struct Format {
    /// This name, when combined with the instruction mnemonic, uniquely
    /// identifies an instruction. The reference manual uses this name in the
    /// "Instruction Operand Encoding" table.
    pub name: String,
    /// These operands should match the "Instruction" column ing the reference
    /// manual.
    pub operands: Vec<Operand>,
}

impl Format {
    /// Iterate over the operand locations.
    pub fn locations(&self) -> impl Iterator<Item = &Location> + '_ {
        self.operands.iter().map(|o| &o.location)
    }

    /// Return the location of the operand that uses memory, if any; return
    /// `None` otherwise.
    pub fn uses_memory(&self) -> Option<Location> {
        debug_assert!(self.locations().copied().filter(Location::uses_memory).count() <= 1);
        self.locations().copied().find(Location::uses_memory)
    }

    /// Return `true` if any of the operands accepts a variable register (i.e.,
    /// not a fixed register, immediate); return `false` otherwise.
    #[must_use]
    pub fn uses_variable_register(&self) -> bool {
        self.locations().any(Location::uses_variable_register)
    }

    /// Collect into operand kinds.
    pub fn operands_by_kind(&self) -> Vec<OperandKind> {
        self.locations().map(Location::kind).collect()
    }
}

impl core::fmt::Display for Format {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let Format { name, operands } = self;
        let operands = operands
            .iter()
            .map(|operand| format!("{operand}"))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{name}({operands})")
    }
}

/// An x64 operand.
///
/// This is designed to look and feel like the operands as expressed in Intel's
/// _Instruction Set Reference_.
///
/// ```
/// # use cranelift_assembler_x64_meta::dsl::{Operand, r, rw, sxq, Location::*};
/// assert_eq!(r(r8).to_string(), "r8");
/// assert_eq!(rw(rm16).to_string(), "rm16[rw]");
/// assert_eq!(sxq(imm32).to_string(), "imm32[sxq]");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Operand {
    /// The location of the data: memory, register, immediate.
    pub location: Location,
    /// An operand can be read-only or read-write.
    pub mutability: Mutability,
    /// Some operands are sign- or zero-extended.
    pub extension: Extension,
}

impl core::fmt::Display for Operand {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let Self { location, mutability, extension } = self;
        write!(f, "{location}")?;
        let has_default_mutability = matches!(mutability, Mutability::Read);
        let has_default_extension = matches!(extension, Extension::None);
        match (has_default_mutability, has_default_extension) {
            (true, true) => {}
            (true, false) => write!(f, "[{extension}]")?,
            (false, true) => write!(f, "[{mutability}]")?,
            (false, false) => write!(f, "[{mutability},{extension}]")?,
        }
        Ok(())
    }
}

impl From<Location> for Operand {
    fn from(location: Location) -> Self {
        let mutability = Mutability::default();
        let extension = Extension::default();
        Self { location, mutability, extension }
    }
}

/// An operand location, as expressed in Intel's _Instruction Set Reference_.
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types, reason = "makes DSL definitions easier to read")]
pub enum Location {
    al,
    ax,
    eax,
    rax,

    imm8,
    imm16,
    imm32,

    r8,
    r16,
    r32,
    r64,

    rm8,
    rm16,
    rm32,
    rm64,
}

impl Location {
    /// Return the number of bits accessed.
    #[must_use]
    pub fn bits(&self) -> u8 {
        use Location::*;
        match self {
            al | imm8 | r8 | rm8 => 8,
            ax | imm16 | r16 | rm16 => 16,
            eax | imm32 | r32 | rm32 => 32,
            rax | r64 | rm64 => 64,
        }
    }

    /// Return the number of bytes accessed, for convenience.
    #[must_use]
    pub fn bytes(&self) -> u8 {
        self.bits() / 8
    }

    /// Return `true` if the location accesses memory; `false` otherwise.
    #[must_use]
    pub fn uses_memory(&self) -> bool {
        use Location::*;
        match self {
            al | ax | eax | rax | imm8 | imm16 | imm32 | r8 | r16 | r32 | r64 => false,
            rm8 | rm16 | rm32 | rm64 => true,
        }
    }

    /// Return `true` if the location accepts a variable register (i.e., not a
    /// fixed register, immediate); return `false` otherwise.
    #[must_use]
    pub fn uses_variable_register(&self) -> bool {
        use Location::*;
        match self {
            al | ax | eax | rax | imm8 | imm16 | imm32 => false,
            r8 | r16 | r32 | r64 | rm8 | rm16 | rm32 | rm64 => true,
        }
    }

    /// Convert the location to an [`OperandKind`].
    #[must_use]
    pub fn kind(&self) -> OperandKind {
        use Location::*;
        match self {
            al | ax | eax | rax => OperandKind::FixedReg(*self),
            imm8 | imm16 | imm32 => OperandKind::Imm(*self),
            r8 | r16 | r32 | r64 => OperandKind::Reg(*self),
            rm8 | rm16 | rm32 | rm64 => OperandKind::RegMem(*self),
        }
    }
}

impl core::fmt::Display for Location {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        use Location::*;
        match self {
            al => write!(f, "al"),
            ax => write!(f, "ax"),
            eax => write!(f, "eax"),
            rax => write!(f, "rax"),

            imm8 => write!(f, "imm8"),
            imm16 => write!(f, "imm16"),
            imm32 => write!(f, "imm32"),

            r8 => write!(f, "r8"),
            r16 => write!(f, "r16"),
            r32 => write!(f, "r32"),
            r64 => write!(f, "r64"),

            rm8 => write!(f, "rm8"),
            rm16 => write!(f, "rm16"),
            rm32 => write!(f, "rm32"),
            rm64 => write!(f, "rm64"),
        }
    }
}

/// Organize the operand locations by kind.
///
/// ```
/// # use cranelift_assembler_x64_meta::dsl::{OperandKind, Location};
/// let k: OperandKind = Location::imm32.kind();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum OperandKind {
    FixedReg(Location),
    Imm(Location),
    Reg(Location),
    RegMem(Location),
}

/// x64 operands can be mutable or not.
///
/// ```
/// # use cranelift_assembler_x64_meta::dsl::{r, rw, Location::r8, Mutability};
/// assert_eq!(r(r8).mutability, Mutability::Read);
/// assert_eq!(rw(r8).mutability, Mutability::ReadWrite);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mutability {
    Read,
    ReadWrite,
}

impl Default for Mutability {
    fn default() -> Self {
        Self::Read
    }
}

impl core::fmt::Display for Mutability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read => write!(f, "r"),
            Self::ReadWrite => write!(f, "rw"),
        }
    }
}

/// x64 operands may be sign- or zero-extended.
///
/// ```
/// # use cranelift_assembler_x64_meta::dsl::{Location::r8, sxw, Extension};
/// assert_eq!(sxw(r8).extension, Extension::SignExtendWord);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extension {
    None,
    SignExtendQuad,
    SignExtendLong,
    SignExtendWord,
}

impl Extension {
    /// Check if the extension is sign-extended.
    #[must_use]
    pub fn is_sign_extended(&self) -> bool {
        matches!(self, Self::SignExtendQuad | Self::SignExtendLong | Self::SignExtendWord)
    }
}

impl Default for Extension {
    fn default() -> Self {
        Self::None
    }
}

impl core::fmt::Display for Extension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Extension::None => write!(f, ""),
            Extension::SignExtendQuad => write!(f, "sxq"),
            Extension::SignExtendLong => write!(f, "sxl"),
            Extension::SignExtendWord => write!(f, "sxw"),
        }
    }
}
