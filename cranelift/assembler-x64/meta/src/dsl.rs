//! Defines a domain-specific language (DSL) for describing x64 instructions.
//!
//! This language is intended to be:
//! - compact--i.e., define an x64 instruction on a single line, and
//! - a close-to-direct mapping of what we read in the x64 reference manual.

mod encoding;
mod features;
pub mod format;

pub use encoding::{rex, vex, Encoding, LegacyPrefixes, Rex};
pub use features::{Feature, Features};
pub use format::{fmt, r, rw, sxl, sxq, sxw};
pub use format::{Extension, Format, Location, Mutability, Operand, OperandKind};

/// Abbreviated constructor for an instruction.
pub fn inst(
    name: impl Into<String>,
    format: Format,
    encoding: impl Into<Encoding>,
    features: impl Into<Features>,
) -> Inst {
    let encoding = encoding.into();
    encoding.validate(&format.operands);
    Inst {
        name: name.into(),
        format,
        encoding,
        features: features.into(),
    }
}

pub struct Inst {
    pub name: String,
    pub format: Format,
    pub encoding: Encoding,
    pub features: Features,
}

impl core::fmt::Display for Inst {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let Inst { name, format, encoding, features } = self;
        write!(f, "{name}: {format} => {encoding}")?;
        if !features.is_empty() {
            write!(f, " [{features}]")?;
        }
        Ok(())
    }
}
