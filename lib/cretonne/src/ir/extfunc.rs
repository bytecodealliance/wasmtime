//! External function calls.
//!
//! To a Cretonne function, all functions are "external". Directly called functions must be
//! declared in the preamble, and all function calls must have a signature.
//!
//! This module declares the data types used to represent external functions and call signatures.

use ir::{Type, FunctionName, SigRef, ArgumentLoc};
use isa::{RegInfo, RegUnit};
use std::cmp;
use std::fmt;
use std::str::FromStr;

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

    /// Calling convention.
    pub call_conv: CallConv,

    /// When the signature has been legalized to a specific ISA, this holds the size of the
    /// argument array on the stack. Before legalization, this is `None`.
    ///
    /// This can be computed from the legalized `argument_types` array as the maximum (offset plus
    /// byte size) of the `ArgumentLoc::Stack(offset)` argument.
    pub argument_bytes: Option<u32>,
}

impl Signature {
    /// Create a new blank signature.
    pub fn new(call_conv: CallConv) -> Signature {
        Signature {
            argument_types: Vec::new(),
            return_types: Vec::new(),
            call_conv,
            argument_bytes: None,
        }
    }

    /// Clear the signature so it is identical to a fresh one returned by `new()`.
    pub fn clear(&mut self, call_conv: CallConv) {
        self.argument_types.clear();
        self.return_types.clear();
        self.call_conv = call_conv;
        self.argument_bytes = None;
    }

    /// Compute the size of the stack arguments and mark signature as legalized.
    ///
    /// Even if there are no stack arguments, this will set `argument_types` to `Some(0)` instead
    /// of `None`. This indicates that the signature has been legalized.
    pub fn compute_argument_bytes(&mut self) {
        let bytes = self.argument_types
            .iter()
            .filter_map(|arg| match arg.location {
                ArgumentLoc::Stack(offset) if offset >= 0 => {
                    Some(offset as u32 + arg.value_type.bytes())
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

fn write_list(
    f: &mut fmt::Formatter,
    args: &[ArgumentType],
    regs: Option<&RegInfo>,
) -> fmt::Result {
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
        write!(f, " {}", self.0.call_conv)
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
    /// Special purpose of argument, or `Normal`.
    pub purpose: ArgumentPurpose,
    /// Method for extending argument to a full register.
    pub extension: ArgumentExtension,

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
            purpose: ArgumentPurpose::Normal,
            location: Default::default(),
        }
    }

    /// Create an argument type for a special-purpose register.
    pub fn special_reg(vt: Type, purpose: ArgumentPurpose, regunit: RegUnit) -> ArgumentType {
        ArgumentType {
            value_type: vt,
            extension: ArgumentExtension::None,
            purpose,
            location: ArgumentLoc::Reg(regunit),
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
        if self.0.purpose != ArgumentPurpose::Normal {
            write!(f, " {}", self.0.purpose)?;
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

/// The special purpose of a function argument.
///
/// Function arguments and return values are used to pass user program values between functions,
/// but they are also used to represent special registers with significance to the ABI such as
/// frame pointers and callee-saved registers.
///
/// The argument purpose is used to indicate any special meaning of an argument or return value.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ArgumentPurpose {
    /// A normal user program value passed to or from a function.
    Normal,

    /// Struct return pointer.
    ///
    /// When a function needs to return more data than will fit in registers, the caller passes a
    /// pointer to a memory location where the return value can be written. In some ABIs, this
    /// struct return pointer is passed in a specific register.
    ///
    /// This argument kind can also appear as a return value for ABIs that require a function with
    /// a `StructReturn` pointer argument to also return that pointer in a register.
    StructReturn,

    /// The link register.
    ///
    /// Most RISC architectures implement calls by saving the return address in a designated
    /// register rather than pushing it on the stack. This is represented with a `Link` argument.
    ///
    /// Similarly, some return instructions expect the return address in a register represented as
    /// a `Link` return value.
    Link,

    /// The frame pointer.
    ///
    /// This indicates the frame pointer register which has a special meaning in some ABIs.
    ///
    /// The frame pointer appears as an argument and as a return value since it is a callee-saved
    /// register.
    FramePointer,

    /// A callee-saved register.
    ///
    /// Some calling conventions have registers that must be saved by the callee. These registers
    /// are represented as `CalleeSaved` arguments and return values.
    CalleeSaved,

    /// A VM context pointer.
    ///
    /// This is a pointer to a context struct containing details about the current sandbox. It is
    /// used as a base pointer for `vmctx` global variables.
    VMContext,
}

/// Text format names of the `ArgumentPurpose` variants.
static PURPOSE_NAMES: [&str; 6] = ["normal", "sret", "link", "fp", "csr", "vmctx"];

impl fmt::Display for ArgumentPurpose {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(PURPOSE_NAMES[*self as usize])
    }
}

impl FromStr for ArgumentPurpose {
    type Err = ();
    fn from_str(s: &str) -> Result<ArgumentPurpose, ()> {
        match s {
            "normal" => Ok(ArgumentPurpose::Normal),
            "sret" => Ok(ArgumentPurpose::StructReturn),
            "link" => Ok(ArgumentPurpose::Link),
            "fp" => Ok(ArgumentPurpose::FramePointer),
            "csr" => Ok(ArgumentPurpose::CalleeSaved),
            "vmctx" => Ok(ArgumentPurpose::VMContext),
            _ => Err(()),
        }
    }
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

/// A Calling convention.
///
/// A function's calling convention determines exactly how arguments and return values are passed,
/// and how stack frames are managed. Since all of these details depend on both the instruction set
/// architecture and possibly the operating system, a function's calling convention is only fully
/// determined by a `(TargetIsa, CallConv)` tuple.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallConv {
    /// The C calling convention.
    ///
    /// This is the native calling convention that a C compiler would use on the platform.
    Native,

    /// A JIT-compiled WebAssembly function in the SpiderMonkey VM.
    SpiderWASM,
}

impl fmt::Display for CallConv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::CallConv::*;
        f.write_str(match *self {
            Native => "native",
            SpiderWASM => "spiderwasm",
        })
    }
}

impl FromStr for CallConv {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::CallConv::*;
        match s {
            "native" => Ok(Native),
            "spiderwasm" => Ok(SpiderWASM),
            _ => Err(()),
        }
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
        t.purpose = ArgumentPurpose::StructReturn;
        assert_eq!(t.to_string(), "i32 uext sret");
    }

    #[test]
    fn argument_purpose() {
        let all_purpose = [
            ArgumentPurpose::Normal,
            ArgumentPurpose::StructReturn,
            ArgumentPurpose::Link,
            ArgumentPurpose::FramePointer,
            ArgumentPurpose::CalleeSaved,
            ArgumentPurpose::VMContext,
        ];
        for (&e, &n) in all_purpose.iter().zip(PURPOSE_NAMES.iter()) {
            assert_eq!(e.to_string(), n);
            assert_eq!(Ok(e), n.parse());
        }
    }

    #[test]
    fn call_conv() {
        for &cc in &[CallConv::Native, CallConv::SpiderWASM] {
            assert_eq!(Ok(cc), cc.to_string().parse())
        }
    }

    #[test]
    fn signatures() {
        let mut sig = Signature::new(CallConv::SpiderWASM);
        assert_eq!(sig.to_string(), "() spiderwasm");
        sig.argument_types.push(ArgumentType::new(I32));
        assert_eq!(sig.to_string(), "(i32) spiderwasm");
        sig.return_types.push(ArgumentType::new(F32));
        assert_eq!(sig.to_string(), "(i32) -> f32 spiderwasm");
        sig.argument_types.push(
            ArgumentType::new(I32.by(4).unwrap()),
        );
        assert_eq!(sig.to_string(), "(i32, i32x4) -> f32 spiderwasm");
        sig.return_types.push(ArgumentType::new(B8));
        assert_eq!(sig.to_string(), "(i32, i32x4) -> f32, b8 spiderwasm");

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
        assert_eq!(
            sig.to_string(),
            "(i32 [24], i32x4 [8]) -> f32, b8 spiderwasm"
        );
    }
}
