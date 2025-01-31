//! Defines a domain-specific language (DSL) for describing x64 instructions.
//!
//! This language is intended to be:
//! - compact--i.e., define an x64 instruction on a single line, and
//! - a close-to-direct mapping of what we read in the x64 reference manual.

mod encoding;
mod features;
pub mod format;

pub use encoding::{rex, vex, Encoding, LegacyPrefix, Rex};
pub use features::{Feature, Features};
pub use format::{fmt, r, rw, sxl, sxq, sxw};
pub use format::{Extension, Format, Location, Mutability, Operand, OperandKind};

/// Abbreviated constructor for an x64 instruction.
pub fn inst(
    mnemonic: impl Into<String>,
    format: Format,
    encoding: impl Into<Encoding>,
    features: impl Into<Features>,
) -> Inst {
    let encoding = encoding.into();
    encoding.validate(&format.operands);
    Inst {
        mnemonic: mnemonic.into(),
        format,
        encoding,
        features: features.into(),
    }
}

/// An x64 instruction.
///
/// Use [`inst`] to construct this within the
/// [`instructions`](super::instructions) module. This structure is designed to
/// represent all of the information for one instruction (a table row) in the
/// x64 _Instruction Set Reference_ or at least enough to generate code to emit
/// the instruction.
pub struct Inst {
    /// The instruction name as represented in the x64 reference manual. This is
    /// the pretty-printed name used for disassembly. Multiple instructions may
    /// have the same mnemonic, though; the combination of this field and the
    /// format name must be unique (see [`Inst::name`]).
    pub mnemonic: String,
    /// The instruction operands, typically represented in the "Instruction"
    /// column of the x64 reference manual.
    pub format: Format,
    /// The instruction encoding, typically represented in the "Opcode" column
    /// of the x64 reference manual.
    pub encoding: Encoding,
    /// The CPU features required to use this instruction; this combines the
    /// "64-bit/32-bit Mode Support" and "CPUID Feature Flag" columns of the x64
    /// reference manual.
    pub features: Features,
}

impl Inst {
    /// The unique name for this instruction.
    ///
    /// To avoid ambiguity, this name combines the instruction mnemonic and the
    /// format name in snake case. This is used in generated code to name the
    /// instruction `struct` and builder functions.
    ///
    /// In rare cases, this `<mnemonic>_<format>` scheme does not uniquely
    /// identify an instruction in x64 ISA (e.g., some extended versions,
    /// VEX/EVEX). In these cases, we append a minimal identifier to
    /// the format name (e.g., `sx*`) to keep this unique.
    #[must_use]
    pub fn name(&self) -> String {
        format!("{}_{}", self.mnemonic.to_lowercase(), self.format.name.to_lowercase())
    }
}

impl core::fmt::Display for Inst {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let Inst { mnemonic: name, format, encoding, features } = self;
        write!(f, "{name}: {format} => {encoding}")?;
        if !features.is_empty() {
            write!(f, " [{features}]")?;
        }
        Ok(())
    }
}
