//! External function calls.
//!
//! To a Cretonne function, all functions are "external". Directly called functions must be
//! declared in the preamble, and all function calls must have a signature.
//!
//! This module declares the data types used to represent external functions and call signatures.

use std::fmt::{self, Display, Formatter};
use ir::{Type, FunctionName, SigRef};

/// Function signature.
///
/// The function signature describes the types of arguments and return values along with other
/// details that are needed to call a function correctly.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Signature {
    /// Types of the arguments passed to the function.
    pub argument_types: Vec<ArgumentType>,
    /// Types returned from the function.
    pub return_types: Vec<ArgumentType>,
}

impl Signature {
    /// Create a new blank signature.
    pub fn new() -> Signature {
        Signature {
            argument_types: Vec::new(),
            return_types: Vec::new(),
        }
    }
}

fn write_list(f: &mut Formatter, args: &Vec<ArgumentType>) -> fmt::Result {
    match args.split_first() {
        None => {}
        Some((first, rest)) => {
            try!(write!(f, "{}", first));
            for arg in rest {
                try!(write!(f, ", {}", arg));
            }
        }
    }
    Ok(())
}

impl Display for Signature {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        try!(write!(f, "("));
        try!(write_list(f, &self.argument_types));
        try!(write!(f, ")"));
        if !self.return_types.is_empty() {
            try!(write!(f, " -> "));
            try!(write_list(f, &self.return_types));
        }
        Ok(())
    }
}

/// Function argument or return value type.
///
/// This describes the value type being passed to or from a function along with flags that affect
/// how the argument is passed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ArgumentType {
    /// Type of the argument value.
    pub value_type: Type,
    /// Method for extending argument to a full register.
    pub extension: ArgumentExtension,
    /// Place this argument in a register if possible.
    pub inreg: bool,
}

impl ArgumentType {
    /// Create an argument type with default flags.
    pub fn new(vt: Type) -> ArgumentType {
        ArgumentType {
            value_type: vt,
            extension: ArgumentExtension::None,
            inreg: false,
        }
    }
}

impl Display for ArgumentType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.value_type));
        match self.extension {
            ArgumentExtension::None => {}
            ArgumentExtension::Uext => try!(write!(f, " uext")),
            ArgumentExtension::Sext => try!(write!(f, " sext")),
        }
        if self.inreg {
            try!(write!(f, " inreg"));
        }
        Ok(())
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

impl Display for ExtFuncData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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
    }
}
