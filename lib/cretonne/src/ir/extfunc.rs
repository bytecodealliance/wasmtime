//! External function calls.
//!
//! To a Cretonne function, all functions are "external". Directly called functions must be
//! declared in the preamble, and all function calls must have a signature.
//!
//! This module declares the data types used to represent external functions and call signatures.

use ir::{Type, FunctionName, SigRef, ArgumentLoc};
use isa::RegInfo;
use std::cmp;
use std::fmt;

/// Function signature.
///
/// The function signature describes the types of arguments and return values along with other
/// details that are needed to call a function correctly.
///
/// A signature can optionally include ISA-specific ABI information which specifies exactly how
/// arguments and return values are passed.
#[derive(Clone, Debug)]
pub struct Signature {
    /// Types of the arguments passed to the function.
    pub argument_types: Vec<ArgumentType>,
    /// Types returned from the function.
    pub return_types: Vec<ArgumentType>,

    /// When the signature has been legalized to a specific ISA, this holds the size of the
    /// argument array on the stack. Before legalization, this is `None`.
    ///
    /// This can be computed from the legalized `argument_types` array as the maximum (offset plus
    /// byte size) of the `ArgumentLoc::Stack(offset)` argument.
    pub argument_bytes: Option<u32>,
}

impl Signature {
    /// Create a new blank signature.
    pub fn new() -> Signature {
        Signature {
            argument_types: Vec::new(),
            return_types: Vec::new(),
            argument_bytes: None,
        }
    }

    /// Compute the size of the stack arguments and mark signature as legalized.
    ///
    /// Even if there are no stack arguments, this will set `argument_types` to `Some(0)` instead
    /// of `None`. This indicates that the signature has been legalized.
    pub fn compute_argument_bytes(&mut self) {
        let bytes = self.argument_types
            .iter()
            .filter_map(|arg| match arg.location {
                            ArgumentLoc::Stack(offset) => {
                                Some(offset + arg.value_type.bits() as u32 / 8)
                            }
                            _ => None,
                        })
            .fold(0, cmp::max);
        self.argument_bytes = Some(bytes);
    }

    /// Return an object that can display `self` with correct register names.
    pub fn display<'a, R: Into<Option<&'a RegInfo>>>(&'a self, regs: R) -> DisplaySignature<'a> {
        DisplaySignature(self, regs.into())
    }
}

/// Wrapper type capable of displaying a `Signature` with correct register names.
pub struct DisplaySignature<'a>(&'a Signature, Option<&'a RegInfo>);

fn write_list(f: &mut fmt::Formatter,
              args: &Vec<ArgumentType>,
              regs: Option<&RegInfo>)
              -> fmt::Result {
    match args.split_first() {
        None => {}
        Some((first, rest)) => {
            write!(f, "{}", first.display(regs))?;
            for arg in rest {
                write!(f, ", {}", arg.display(regs))?;
            }
        }
    }
    Ok(())
}

impl<'a> fmt::Display for DisplaySignature<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(")?;
        write_list(f, &self.0.argument_types, self.1)?;
        write!(f, ")")?;
        if !self.0.return_types.is_empty() {
            write!(f, " -> ")?;
            write_list(f, &self.0.return_types, self.1)?;
        }
        Ok(())
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display(None).fmt(f)
    }
}

/// Function argument or return value type.
///
/// This describes the value type being passed to or from a function along with flags that affect
/// how the argument is passed.
#[derive(Copy, Clone, Debug)]
pub struct ArgumentType {
    /// Type of the argument value.
    pub value_type: Type,
    /// Method for extending argument to a full register.
    pub extension: ArgumentExtension,
    /// Place this argument in a register if possible.
    pub inreg: bool,

    /// ABI-specific location of this argument, or `Unassigned` for arguments that have not yet
    /// been legalized.
    pub location: ArgumentLoc,
}

impl ArgumentType {
    /// Create an argument type with default flags.
    pub fn new(vt: Type) -> ArgumentType {
        ArgumentType {
            value_type: vt,
            extension: ArgumentExtension::None,
            inreg: false,
            location: Default::default(),
        }
    }

    /// Return an object that can display `self` with correct register names.
    pub fn display<'a, R: Into<Option<&'a RegInfo>>>(&'a self, regs: R) -> DisplayArgumentType<'a> {
        DisplayArgumentType(self, regs.into())
    }
}

/// Wrapper type capable of displaying an `ArgumentType` with correct register names.
pub struct DisplayArgumentType<'a>(&'a ArgumentType, Option<&'a RegInfo>);

impl<'a> fmt::Display for DisplayArgumentType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.value_type)?;
        match self.0.extension {
            ArgumentExtension::None => {}
            ArgumentExtension::Uext => write!(f, " uext")?,
            ArgumentExtension::Sext => write!(f, " sext")?,
        }
        if self.0.inreg {
            write!(f, " inreg")?;
        }

        if self.0.location.is_assigned() {
            write!(f, " [{}]", self.0.location.display(self.1))?;
        }

        Ok(())
    }
}

impl fmt::Display for ArgumentType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display(None).fmt(f)
    }
}

/// Function argument extension options.
///
/// On some architectures, small integer function arguments are extended to the width of a
/// general-purpose register.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ArgumentExtension {
    /// No extension, high bits are indeterminate.
    None,
    /// Unsigned extension: high bits in register are 0.
    Uext,
    /// Signed extension: high bits in register replicate sign bit.
    Sext,
}

/// An external function.
///
/// Information about a function that can be called directly with a direct `call` instruction.
#[derive(Clone, Debug)]
pub struct ExtFuncData {
    /// Name of the external function.
    pub name: FunctionName,
    /// Call signature of function.
    pub signature: SigRef,
}

impl fmt::Display for ExtFuncData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.signature, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::types::{I32, F32, B8};

    #[test]
    fn argument_type() {
        let mut t = ArgumentType::new(I32);
        assert_eq!(t.to_string(), "i32");
        t.extension = ArgumentExtension::Uext;
        assert_eq!(t.to_string(), "i32 uext");
        t.inreg = true;
        assert_eq!(t.to_string(), "i32 uext inreg");
    }

    #[test]
    fn signatures() {
        let mut sig = Signature::new();
        assert_eq!(sig.to_string(), "()");
        sig.argument_types.push(ArgumentType::new(I32));
        assert_eq!(sig.to_string(), "(i32)");
        sig.return_types.push(ArgumentType::new(F32));
        assert_eq!(sig.to_string(), "(i32) -> f32");
        sig.argument_types.push(ArgumentType::new(I32.by(4).unwrap()));
        assert_eq!(sig.to_string(), "(i32, i32x4) -> f32");
        sig.return_types.push(ArgumentType::new(B8));
        assert_eq!(sig.to_string(), "(i32, i32x4) -> f32, b8");

        // Test the offset computation algorithm.
        assert_eq!(sig.argument_bytes, None);
        sig.argument_types[1].location = ArgumentLoc::Stack(8);
        sig.compute_argument_bytes();
        // An `i32x4` at offset 8 requires a 24-byte argument array.
        assert_eq!(sig.argument_bytes, Some(24));
        // Order does not matter.
        sig.argument_types[0].location = ArgumentLoc::Stack(24);
        sig.compute_argument_bytes();
        assert_eq!(sig.argument_bytes, Some(28));

        // Writing ABI-annotated signatures.
        assert_eq!(sig.to_string(), "(i32 [24], i32x4 [8]) -> f32, b8");
    }
}
