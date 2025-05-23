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
pub fn fmt(
    name: impl Into<String>,
    operands: impl IntoIterator<Item = impl Into<Operand>>,
) -> Format {
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
pub fn rw(op: impl Into<Operand>) -> Operand {
    let op = op.into();
    assert!(!matches!(op.location.kind(), OperandKind::Imm(_)));
    Operand {
        mutability: Mutability::ReadWrite,
        ..op
    }
}

/// An abbreviated constructor for a "read" operand.
#[must_use]
pub fn r(op: impl Into<Operand>) -> Operand {
    let op = op.into();
    assert!(op.mutability.is_read());
    op
}

/// An abbreviated constructor for a "write" operand.
#[must_use]
pub fn w(op: impl Into<Operand>) -> Operand {
    let op = op.into();
    Operand {
        mutability: Mutability::Write,
        ..op
    }
}

/// An abbreviated constructor for a memory operand that requires alignment.
pub fn align(location: Location) -> Operand {
    assert!(location.uses_memory());
    Operand {
        align: true,
        ..Operand::from(location)
    }
}

/// An abbreviated constructor for an operand that is used by the instruction
/// but not visible in its disassembly.
pub fn implicit(location: Location) -> Operand {
    assert!(matches!(location.kind(), OperandKind::FixedReg(_)));
    Operand {
        implicit: true,
        ..Operand::from(location)
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
        extension: Extension::SignExtendQuad,
        ..Operand::from(location)
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
        extension: Extension::SignExtendLong,
        ..Operand::from(location)
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
        extension: Extension::SignExtendWord,
        ..Operand::from(location)
    }
}

/// A format describes the operands for an instruction.
pub struct Format {
    /// This name, when combined with the instruction mnemonic, uniquely
    /// identifies an instruction. The reference manual uses this name in the
    /// "Instruction Operand Encoding" table.
    pub name: String,
    /// These operands should match the "Instruction" column in the reference
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
        debug_assert!(
            self.locations()
                .copied()
                .filter(Location::uses_memory)
                .count()
                <= 1
        );
        self.locations().copied().find(Location::uses_memory)
    }

    /// Return `true` if any of the operands accepts a register (i.e., not an
    /// immediate); return `false` otherwise.
    #[must_use]
    pub fn uses_register(&self) -> bool {
        self.locations().any(Location::uses_register)
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
/// # use cranelift_assembler_x64_meta::dsl::{align, r, rw, sxq, Location::*};
/// assert_eq!(r(r8).to_string(), "r8");
/// assert_eq!(rw(rm16).to_string(), "rm16[rw]");
/// assert_eq!(sxq(imm32).to_string(), "imm32[sxq]");
/// assert_eq!(align(xmm_m128).to_string(), "xmm_m128[align]");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Operand {
    /// The location of the data: memory, register, immediate.
    pub location: Location,
    /// An operand can be read-only or read-write.
    pub mutability: Mutability,
    /// Some operands are sign- or zero-extended.
    pub extension: Extension,
    /// Some memory operands require alignment; `true` indicates that the memory
    /// address used in the operand must align to the size of the operand (e.g.,
    /// `m128` must be 16-byte aligned).
    pub align: bool,
    /// Some register operands are implicit: that is, they do not appear in the
    /// disassembled output even though they are used in the instruction.
    pub implicit: bool,
}

impl core::fmt::Display for Operand {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let Self {
            location,
            mutability,
            extension,
            align,
            implicit,
        } = self;
        write!(f, "{location}")?;
        let mut flags = vec![];
        if !matches!(mutability, Mutability::Read) {
            flags.push(format!("{mutability}"));
        }
        if !matches!(extension, Extension::None) {
            flags.push(format!("{extension}"));
        }
        if *align != false {
            flags.push("align".to_owned());
        }
        if *implicit {
            flags.push("implicit".to_owned());
        }
        if !flags.is_empty() {
            write!(f, "[{}]", flags.join(","))?;
        }
        Ok(())
    }
}

impl From<Location> for Operand {
    fn from(location: Location) -> Self {
        let mutability = Mutability::default();
        let extension = Extension::default();
        let align = false;
        let implicit = false;
        Self {
            location,
            mutability,
            extension,
            align,
            implicit,
        }
    }
}

/// The kind of register used in a [`Location`].
pub enum RegClass {
    Gpr,
    Xmm,
}

impl core::fmt::Display for RegClass {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            RegClass::Gpr => write!(f, "Gpr"),
            RegClass::Xmm => write!(f, "Xmm"),
        }
    }
}

/// An operand location, as expressed in Intel's _Instruction Set Reference_.
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types, reason = "makes DSL definitions easier to read")]
pub enum Location {
    // Fixed registers.
    al,
    ax,
    eax,
    rax,
    dx,
    edx,
    rdx,
    cl,

    // Immediate values.
    imm8,
    imm16,
    imm32,

    // General-purpose registers, and their memory forms.
    r8,
    r16,
    r32,
    r64,
    rm8,
    rm16,
    rm32,
    rm64,

    // XMM registers, and their memory forms.
    xmm_m32,
    xmm_m64,
    xmm_m128,
    ymm_m256,
    zmm_m512,

    // Memory-only locations.
    m8,
    m16,
    m32,
    m64,
    xmm1,
    xmm2,
    xmm3,
    ymm1,
    ymm2,
    ymm3,
    zmm1,
    zmm2,
    zmm3,
}

impl Location {
    /// Return the number of bits accessed.
    #[must_use]
    pub fn bits(&self) -> u16 {
        use Location::*;
        match self {
            al | cl | imm8 | r8 | rm8 | m8 => 8,
            ax | dx | imm16 | r16 | rm16 | m16 => 16,
            eax | edx | imm32 | r32 | rm32 | m32 | xmm_m32 => 32,
            rax | rdx | r64 | rm64 | m64 | xmm_m64 => 64,
            xmm1 | xmm2 | xmm3 | xmm_m128 => 128,
            ymm1 | ymm2 | ymm3 | ymm_m256 => 256,
            zmm1 | zmm2 | zmm3 | zmm_m512 => 512,
        }
    }

    /// Return the number of bytes accessed, for convenience.
    #[must_use]
    pub fn bytes(&self) -> u16 {
        self.bits() / 8
    }

    /// Return `true` if the location accesses memory; `false` otherwise.
    #[must_use]
    pub fn uses_memory(&self) -> bool {
        use Location::*;
        match self {
            al | ax | eax | rax | cl | dx | edx | rdx | imm8 | imm16 | imm32 | r8 | r16 | r32
            | r64 | xmm1 | xmm2 | xmm3 | ymm1 | ymm2 | ymm3 | zmm1 | zmm2 | zmm3 => false,
            rm8 | rm16 | rm32 | rm64 | xmm_m32 | xmm_m64 | xmm_m128 | m8 | m16 | m32 | m64
            | ymm_m256 | zmm_m512 => true,
        }
    }

    /// Return `true` if any of the operands accepts a register (i.e., not an
    /// immediate); return `false` otherwise.
    #[must_use]
    pub fn uses_register(&self) -> bool {
        use Location::*;
        match self {
            imm8 | imm16 | imm32 => false,
            al | ax | eax | rax | cl | dx | edx | rdx | r8 | r16 | r32 | r64 | rm8 | rm16
            | rm32 | rm64 | m8 | m16 | m32 | m64 | xmm1 | xmm2 | xmm3 | xmm_m32 | xmm_m64
            | ymm1 | ymm2 | ymm3 | zmm1 | zmm2 | zmm3 | xmm_m128 | ymm_m256 | zmm_m512 => true,
        }
    }

    /// Convert the location to an [`OperandKind`].
    #[must_use]
    pub fn kind(&self) -> OperandKind {
        use Location::*;
        match self {
            al | ax | eax | rax | cl | dx | edx | rdx => OperandKind::FixedReg(*self),
            imm8 | imm16 | imm32 => OperandKind::Imm(*self),
            r8 | r16 | r32 | r64 | xmm1 | xmm2 | xmm3 | ymm1 | ymm2 | ymm3 | zmm1 | zmm2 | zmm3 => {
                OperandKind::Reg(*self)
            }
            rm8 | rm16 | rm32 | rm64 | xmm_m32 | xmm_m64 | xmm_m128 | ymm_m256 | zmm_m512 => {
                OperandKind::RegMem(*self)
            }
            m8 | m16 | m32 | m64 => OperandKind::Mem(*self),
        }
    }

    /// If a location directly uses data from a register, return the register
    /// class; otherwise, return `None`. Memory-only locations, though their
    /// address is stored in a register, use data from memory and thus also
    /// return `None`.
    #[must_use]
    pub fn reg_class(&self) -> Option<RegClass> {
        use Location::*;
        match self {
            imm8 | imm16 | imm32 | m8 | m16 | m32 | m64 => None,
            al | ax | eax | rax | cl | dx | edx | rdx | r8 | r16 | r32 | r64 | rm8 | rm16
            | rm32 | rm64 => Some(RegClass::Gpr),
            xmm1 | xmm2 | xmm3 | ymm1 | ymm2 | ymm3 | zmm1 | zmm2 | zmm3 | xmm_m32 | xmm_m64
            | xmm_m128 | ymm_m256 | zmm_m512 => Some(RegClass::Xmm),
        }
    }
}

impl core::fmt::Display for Location {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        use Location::*;
        match self {
            imm8 => write!(f, "imm8"),
            imm16 => write!(f, "imm16"),
            imm32 => write!(f, "imm32"),

            al => write!(f, "al"),
            ax => write!(f, "ax"),
            eax => write!(f, "eax"),
            rax => write!(f, "rax"),
            cl => write!(f, "cl"),
            dx => write!(f, "dx"),
            edx => write!(f, "edx"),
            rdx => write!(f, "rdx"),

            r8 => write!(f, "r8"),
            r16 => write!(f, "r16"),
            r32 => write!(f, "r32"),
            r64 => write!(f, "r64"),
            rm8 => write!(f, "rm8"),
            rm16 => write!(f, "rm16"),
            rm32 => write!(f, "rm32"),
            rm64 => write!(f, "rm64"),

            xmm_m32 => write!(f, "xmm_m32"),
            xmm_m64 => write!(f, "xmm_m64"),
            xmm_m128 => write!(f, "xmm_m128"),
            ymm_m256 => write!(f, "ymm_m256"),
            zmm_m512 => write!(f, "zmm_m512"),

            m8 => write!(f, "m8"),
            m16 => write!(f, "m16"),
            m32 => write!(f, "m32"),
            m64 => write!(f, "m64"),

            xmm1 => write!(f, "xmm1"),
            xmm2 => write!(f, "xmm2"),
            xmm3 => write!(f, "xmm3"),

            ymm1 => write!(f, "ymm1"),
            ymm2 => write!(f, "ymm2"),
            ymm3 => write!(f, "ymm3"),

            zmm1 => write!(f, "zmm1"),
            zmm2 => write!(f, "zmm2"),
            zmm3 => write!(f, "zmm3"),
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
    Mem(Location),
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
    Write,
}

impl Mutability {
    /// Returns whether this represents a read of the operand in question.
    ///
    /// Note that for read/write operands this returns `true`.
    pub fn is_read(&self) -> bool {
        match self {
            Mutability::Read | Mutability::ReadWrite => true,
            Mutability::Write => false,
        }
    }

    /// Returns whether this represents a write of the operand in question.
    ///
    /// Note that for read/write operands this returns `true`.
    pub fn is_write(&self) -> bool {
        match self {
            Mutability::Read => false,
            Mutability::ReadWrite | Mutability::Write => true,
        }
    }
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
            Self::Write => write!(f, "w"),
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
        matches!(
            self,
            Self::SignExtendQuad | Self::SignExtendLong | Self::SignExtendWord
        )
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
