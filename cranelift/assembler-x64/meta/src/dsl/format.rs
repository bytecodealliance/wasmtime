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

    /// Returns the `IsleConstructorRaw` variant that this format will be using.
    ///
    /// This is used to generate the Rust function which is called from ISLE to
    /// construct an `MInst` and construct an assembler instruction.
    pub fn isle_constructor_raw(&self) -> IsleConstructorRaw {
        let write_operands = self
            .operands
            .iter()
            .filter(|o| o.mutability.is_write())
            .collect::<Vec<_>>();
        match &write_operands[..] {
            // No outputs? Just return the instruction.
            [] => IsleConstructorRaw::MInst,
            [one] => match one.mutability {
                Mutability::Read => unreachable!(),
                Mutability::ReadWrite => match one.location.kind() {
                    OperandKind::Imm(_) => unreachable!(),
                    // One read/write register output? Output the instruction
                    // and that register.
                    OperandKind::FixedReg(_) | OperandKind::Reg(_) => IsleConstructorRaw::MInstAndGpr,
                    // One read/write regmem output? We need to output
                    // everything and it'll internally disambiguate which was
                    // emitted (e.g. the mem variant or the register variant).
                    OperandKind::RegMem(_) => IsleConstructorRaw::MInstAndGprMem,
                },
            },
            other => panic!("unsupported number of write operands {other:?}"),
        }
    }

    /// Returns the ISLE constructors that are going to be used when generating
    /// this instruction.
    ///
    /// Note that one instruction might need multiple constructors, such as one
    /// for operating on memory and one for operating on registers.
    pub fn isle_constructors(&self) -> Vec<IsleConstructor> {
        // TODO: in the future this should also check and generate constructors
        // that return `ProducesFlags` or `ConsumesFlags` or similar.
        match self.isle_constructor_raw() {
            IsleConstructorRaw::MInst => vec![IsleConstructor::RetSideEffectNoResult],
            IsleConstructorRaw::MInstAndGpr => vec![IsleConstructor::RetGpr],
            // If the writable output is `GprMem` then one constructor will
            // operate on memory, hence returning a `SideEffectNoResult`. The
            // other constructor will operate on gprs, hence returning a `Gpr`.
            IsleConstructorRaw::MInstAndGprMem => {
                vec![IsleConstructor::RetSideEffectNoResult, IsleConstructor::RetGpr]
            }
        }
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

impl Operand {
    /// Returns the type of this operand in ISLE as part of the
    /// `IsleConstructorRaw` variants.
    pub fn isle_param_raw(&self) -> String {
        match self.location.kind() {
            OperandKind::Imm(loc) => {
                let bits = loc.bits();
                if self.extension.is_sign_extended() {
                    format!("AssemblerSimm{bits}")
                } else {
                    format!("AssemblerImm{bits}")
                }
            }
            OperandKind::Reg(_) => "Gpr".to_string(),
            OperandKind::FixedReg(_) => "Gpr".to_string(),
            OperandKind::RegMem(_) => "GprMem".to_string(),
        }
    }

    /// Returns the parameter type used for the `IsleConstructor` variant
    /// provided.
    pub fn isle_param_for_ctor(&self, ctor: IsleConstructor) -> String {
        match self.location.kind() {
            // Writable `RegMem` operands are special here: in one constructor
            // it's operating on memory so the argument is `Amode` and in the
            // other constructor it's operating on registers so the argument is
            // a `Gpr`.
            OperandKind::RegMem(_) if self.mutability.is_write() => match ctor {
                IsleConstructor::RetSideEffectNoResult => "Amode".to_string(),
                IsleConstructor::RetGpr => "Gpr".to_string(),
            },

            // everything else is the same as the "raw" variant
            _ => self.isle_param_raw(),
        }
    }

    /// Returns the Rust type used for the `IsleConstructorRaw` variants.
    pub fn rust_param_raw(&self) -> String {
        match self.location.kind() {
            OperandKind::Imm(loc) => {
                let bits = loc.bits();
                if self.extension.is_sign_extended() {
                    format!("&cranelift_assembler_x64::Simm{bits}")
                } else {
                    format!("&cranelift_assembler_x64::Imm{bits}")
                }
            }
            OperandKind::RegMem(_) => "&GprMem".to_string(),
            OperandKind::Reg(_) | OperandKind::FixedReg(_) => "Gpr".to_string(),
        }
    }

    /// Returns the conversion function, if any, when converting the ISLE type
    /// for this parameter to the assembler type for this parameter.
    /// Effectively converts `self.rust_param_raw()` to the assembler type.
    pub fn rust_convert_isle_to_assembler(&self) -> Option<&'static str> {
        match self.location.kind() {
            OperandKind::Reg(_) => Some(match self.mutability {
                Mutability::Read => "cranelift_assembler_x64::Gpr::new",
                Mutability::ReadWrite => "self.convert_gpr_to_assembler_read_write_gpr",
            }),
            OperandKind::RegMem(_) => Some(match self.mutability {
                Mutability::Read => "self.convert_gpr_mem_to_assembler_read_gpr_mem",
                Mutability::ReadWrite => "self.convert_gpr_mem_to_assembler_read_write_gpr_mem",
            }),
            OperandKind::FixedReg(_) | OperandKind::Imm(_) => None,
        }
    }
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

    cl,

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
            al | cl | imm8 | r8 | rm8 => 8,
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
            al | cl | ax | eax | rax | imm8 | imm16 | imm32 | r8 | r16 | r32 | r64 => false,
            rm8 | rm16 | rm32 | rm64 => true,
        }
    }

    /// Return `true` if the location accepts a variable register (i.e., not a
    /// fixed register, immediate); return `false` otherwise.
    #[must_use]
    pub fn uses_variable_register(&self) -> bool {
        use Location::*;
        match self {
            al | ax | eax | rax | cl | imm8 | imm16 | imm32 => false,
            r8 | r16 | r32 | r64 | rm8 | rm16 | rm32 | rm64 => true,
        }
    }

    /// Convert the location to an [`OperandKind`].
    #[must_use]
    pub fn kind(&self) -> OperandKind {
        use Location::*;
        match self {
            al | ax | eax | rax | cl => OperandKind::FixedReg(*self),
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

            cl => write!(f, "cl"),

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

impl Mutability {
    /// Returns whether this represents a read of the operand in question.
    ///
    /// Note that for read/write operands this returns `true`.
    pub fn is_read(&self) -> bool {
        match self {
            Mutability::Read | Mutability::ReadWrite => true,
        }
    }

    /// Returns whether this represents a write of the operand in question.
    ///
    /// Note that for read/write operands this returns `true`.
    pub fn is_write(&self) -> bool {
        match self {
            Mutability::Read => false,
            Mutability::ReadWrite => true,
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

/// Different kinds of "raw" constructors used to power the `IsleConstructor`
/// variants below.
///
/// Variants of this `enum` are generated as an "FFI layer" where each one
/// generates an `extern` constructor that's implemented in Rust. These raw
/// constructors generally return the `MInst` being generated in addition to
/// any other operands that are produced (e.g. a Gpr).
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum IsleConstructorRaw {
    /// This constructor only generates an `MInst` for non-result-producing
    /// instructions.
    MInst,

    /// This constructor generates `MInstAndGpr`, returning both an instruction
    /// as well as the `Gpr` that holds the result.
    MInstAndGpr,

    /// This constructor generats `MInstAndGprMem`. The exact value of the
    /// output will depend on the input parameters and this'll get
    /// re-pattern-matched in ISLE on the other end.
    ///
    /// This variant is used to power two ISLE constructors frequently, one
    /// returning `SideEffectNoResult` and one returning `Gpr` for instructions
    /// that modify either memory or GPRs
    MInstAndGprMem,
}

impl IsleConstructorRaw {
    /// Returns all known raw ISLE constructors.
    pub fn all() -> &'static [IsleConstructorRaw] {
        &[
            IsleConstructorRaw::MInst,
            IsleConstructorRaw::MInstAndGpr,
            IsleConstructorRaw::MInstAndGprMem,
        ]
    }

    /// Returns the result type, in ISLE, that this constructor produces.
    pub fn isle_type(&self) -> &str {
        match self {
            IsleConstructorRaw::MInst => "MInst",
            IsleConstructorRaw::MInstAndGpr => "MInstAndGpr",
            IsleConstructorRaw::MInstAndGprMem => "MInstAndGprMem",
        }
    }

    /// Returns an ISLE snippet, and helpers, used to define the type associated
    /// with this instruction's return value. May return nothing if no such
    /// snippet is necessary.
    pub fn isle_type_definition(&self) -> Option<&str> {
        match self {
            IsleConstructorRaw::MInst => None,
            IsleConstructorRaw::MInstAndGpr => Some(
                "
                    (type MInstAndGpr (enum (Both (inst MInst) (gpr Gpr))))

                    (decl emit_minst_and_gpr (MInstAndGpr) Gpr)
                    (rule (emit_minst_and_gpr (MInstAndGpr.Both inst gpr))
                        (let ((_ Unit (emit inst))) gpr))
                ",
            ),
            IsleConstructorRaw::MInstAndGprMem => Some(
                "
                    (type MInstAndGprMem
                        (enum (Gpr (inst MInst) (gpr Gpr))
                              (Mem (inst MInst))))

                    (decl emit_minst_and_gpr_mem (MInstAndGprMem) Gpr)
                    (rule (emit_minst_and_gpr_mem (MInstAndGprMem.Gpr inst gpr))
                        (let ((_ Unit (emit inst))) gpr))

                    (decl side_effect_minst_and_gpr_mem (MInstAndGprMem) SideEffectNoResult)
                    (rule (side_effect_minst_and_gpr_mem (MInstAndGprMem.Mem inst))
                        (SideEffectNoResult.Inst inst))
                ",
            ),
        }
    }
}

/// Different kinds of ISLE constructors generated for a particular instruction.
///
/// One instruction may generate a single constructor or multiple constructors.
/// For example an instruction that writes its result to a register will
/// generate only a single constructor. An instruction where the destination
/// read/write operand is `GprMem` will generate two constructors though, one
/// for memory and one for in registers.
#[derive(Copy, Clone, Debug)]
pub enum IsleConstructor {
    /// This constructor only produces a side effect, meaning that the
    /// instruction does not produce results in registers. This may produce
    /// a result in memory, however.
    RetSideEffectNoResult,

    /// This constructor produces a `Gpr` value, meaning that it will write the
    /// result to a `Gpr`.
    RetGpr,
}

impl IsleConstructor {
    /// Returns the result type, in ISLE, that this constructor generates.
    pub fn result_ty(&self) -> &'static str {
        match self {
            IsleConstructor::RetSideEffectNoResult => "SideEffectNoResult",
            IsleConstructor::RetGpr => "Gpr",
        }
    }
}
