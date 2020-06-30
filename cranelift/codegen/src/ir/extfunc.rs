//! External function calls.
//!
//! To a Cranelift function, all functions are "external". Directly called functions must be
//! declared in the preamble, and all function calls must have a signature.
//!
//! This module declares the data types used to represent external functions and call signatures.

use crate::ir::{ArgumentLoc, ExternalName, SigRef, Type};
use crate::isa::{CallConv, RegInfo, RegUnit};
use crate::machinst::RelocDistance;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

/// Function signature.
///
/// The function signature describes the types of formal parameters and return values along with
/// other details that are needed to call a function correctly.
///
/// A signature can optionally include ISA-specific ABI information which specifies exactly how
/// arguments and return values are passed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature {
    /// The arguments passed to the function.
    pub params: Vec<AbiParam>,
    /// Values returned from the function.
    pub returns: Vec<AbiParam>,

    /// Calling convention.
    pub call_conv: CallConv,
}

impl Signature {
    /// Create a new blank signature.
    pub fn new(call_conv: CallConv) -> Self {
        Self {
            params: Vec::new(),
            returns: Vec::new(),
            call_conv,
        }
    }

    /// Clear the signature so it is identical to a fresh one returned by `new()`.
    pub fn clear(&mut self, call_conv: CallConv) {
        self.params.clear();
        self.returns.clear();
        self.call_conv = call_conv;
    }

    /// Return an object that can display `self` with correct register names.
    pub fn display<'a, R: Into<Option<&'a RegInfo>>>(&'a self, regs: R) -> DisplaySignature<'a> {
        DisplaySignature(self, regs.into())
    }

    /// Find the index of a presumed unique special-purpose parameter.
    pub fn special_param_index(&self, purpose: ArgumentPurpose) -> Option<usize> {
        self.params.iter().rposition(|arg| arg.purpose == purpose)
    }

    /// Find the index of a presumed unique special-purpose parameter.
    pub fn special_return_index(&self, purpose: ArgumentPurpose) -> Option<usize> {
        self.returns.iter().rposition(|arg| arg.purpose == purpose)
    }

    /// Does this signature have a parameter whose `ArgumentPurpose` is
    /// `purpose`?
    pub fn uses_special_param(&self, purpose: ArgumentPurpose) -> bool {
        self.special_param_index(purpose).is_some()
    }

    /// Does this signature have a return whose `ArgumentPurpose` is `purpose`?
    pub fn uses_special_return(&self, purpose: ArgumentPurpose) -> bool {
        self.special_return_index(purpose).is_some()
    }

    /// How many special parameters does this function have?
    pub fn num_special_params(&self) -> usize {
        self.params
            .iter()
            .filter(|p| p.purpose != ArgumentPurpose::Normal)
            .count()
    }

    /// How many special returns does this function have?
    pub fn num_special_returns(&self) -> usize {
        self.returns
            .iter()
            .filter(|r| r.purpose != ArgumentPurpose::Normal)
            .count()
    }

    /// Does this signature take an struct return pointer parameter?
    pub fn uses_struct_return_param(&self) -> bool {
        self.uses_special_param(ArgumentPurpose::StructReturn)
    }

    /// Does this return more than one normal value? (Pre-struct return
    /// legalization)
    pub fn is_multi_return(&self) -> bool {
        self.returns
            .iter()
            .filter(|r| r.purpose == ArgumentPurpose::Normal)
            .count()
            > 1
    }
}

/// Wrapper type capable of displaying a `Signature` with correct register names.
pub struct DisplaySignature<'a>(&'a Signature, Option<&'a RegInfo>);

fn write_list(f: &mut fmt::Formatter, args: &[AbiParam], regs: Option<&RegInfo>) -> fmt::Result {
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
        write_list(f, &self.0.params, self.1)?;
        write!(f, ")")?;
        if !self.0.returns.is_empty() {
            write!(f, " -> ")?;
            write_list(f, &self.0.returns, self.1)?;
        }
        write!(f, " {}", self.0.call_conv)
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display(None).fmt(f)
    }
}

/// Function parameter or return value descriptor.
///
/// This describes the value type being passed to or from a function along with flags that affect
/// how the argument is passed.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AbiParam {
    /// Type of the argument value.
    pub value_type: Type,
    /// Special purpose of argument, or `Normal`.
    pub purpose: ArgumentPurpose,
    /// Method for extending argument to a full register.
    pub extension: ArgumentExtension,

    /// ABI-specific location of this argument, or `Unassigned` for arguments that have not yet
    /// been legalized.
    pub location: ArgumentLoc,
    /// Was the argument converted to pointer during legalization?
    pub legalized_to_pointer: bool,
}

impl AbiParam {
    /// Create a parameter with default flags.
    pub fn new(vt: Type) -> Self {
        Self {
            value_type: vt,
            extension: ArgumentExtension::None,
            purpose: ArgumentPurpose::Normal,
            location: Default::default(),
            legalized_to_pointer: false,
        }
    }

    /// Create a special-purpose parameter that is not (yet) bound to a specific register.
    pub fn special(vt: Type, purpose: ArgumentPurpose) -> Self {
        Self {
            value_type: vt,
            extension: ArgumentExtension::None,
            purpose,
            location: Default::default(),
            legalized_to_pointer: false,
        }
    }

    /// Create a parameter for a special-purpose register.
    pub fn special_reg(vt: Type, purpose: ArgumentPurpose, regunit: RegUnit) -> Self {
        Self {
            value_type: vt,
            extension: ArgumentExtension::None,
            purpose,
            location: ArgumentLoc::Reg(regunit),
            legalized_to_pointer: false,
        }
    }

    /// Convert `self` to a parameter with the `uext` flag set.
    pub fn uext(self) -> Self {
        debug_assert!(self.value_type.is_int(), "uext on {} arg", self.value_type);
        Self {
            extension: ArgumentExtension::Uext,
            ..self
        }
    }

    /// Convert `self` to a parameter type with the `sext` flag set.
    pub fn sext(self) -> Self {
        debug_assert!(self.value_type.is_int(), "sext on {} arg", self.value_type);
        Self {
            extension: ArgumentExtension::Sext,
            ..self
        }
    }

    /// Return an object that can display `self` with correct register names.
    pub fn display<'a, R: Into<Option<&'a RegInfo>>>(&'a self, regs: R) -> DisplayAbiParam<'a> {
        DisplayAbiParam(self, regs.into())
    }
}

/// Wrapper type capable of displaying a `AbiParam` with correct register names.
pub struct DisplayAbiParam<'a>(&'a AbiParam, Option<&'a RegInfo>);

impl<'a> fmt::Display for DisplayAbiParam<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.value_type)?;
        if self.0.legalized_to_pointer {
            write!(f, " ptr")?;
        }
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

impl fmt::Display for AbiParam {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display(None).fmt(f)
    }
}

/// Function argument extension options.
///
/// On some architectures, small integer function arguments are extended to the width of a
/// general-purpose register.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
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
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
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
    /// used as a base pointer for `vmctx` global values.
    VMContext,

    /// A signature identifier.
    ///
    /// This is a special-purpose argument used to identify the calling convention expected by the
    /// caller in an indirect call. The callee can verify that the expected signature ID matches.
    SignatureId,

    /// A stack limit pointer.
    ///
    /// This is a pointer to a stack limit. It is used to check the current stack pointer
    /// against. Can only appear once in a signature.
    StackLimit,
}

/// Text format names of the `ArgumentPurpose` variants.
static PURPOSE_NAMES: [&str; 8] = [
    "normal",
    "sret",
    "link",
    "fp",
    "csr",
    "vmctx",
    "sigid",
    "stack_limit",
];

impl fmt::Display for ArgumentPurpose {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(PURPOSE_NAMES[*self as usize])
    }
}

impl FromStr for ArgumentPurpose {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "normal" => Ok(Self::Normal),
            "sret" => Ok(Self::StructReturn),
            "link" => Ok(Self::Link),
            "fp" => Ok(Self::FramePointer),
            "csr" => Ok(Self::CalleeSaved),
            "vmctx" => Ok(Self::VMContext),
            "sigid" => Ok(Self::SignatureId),
            "stack_limit" => Ok(Self::StackLimit),
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
    pub name: ExternalName,
    /// Call signature of function.
    pub signature: SigRef,
    /// Will this function be defined nearby, such that it will always be a certain distance away,
    /// after linking? If so, references to it can avoid going through a GOT or PLT. Note that
    /// symbols meant to be preemptible cannot be considered colocated.
    ///
    /// If `true`, some backends may use relocation forms that have limited range. The exact
    /// distance depends on the code model in use. Currently on AArch64, for example, Cranelift
    /// uses a custom code model supporting up to +/- 128MB displacements. If it is unknown how
    /// far away the target will be, it is best not to set the `colocated` flag; in general, this
    /// flag is best used when the target is known to be in the same unit of code generation, such
    /// as a Wasm module.
    ///
    /// See the documentation for [`RelocDistance`](crate::machinst::RelocDistance) for more details. A
    /// `colocated` flag value of `true` implies `RelocDistance::Near`.
    pub colocated: bool,
}

impl fmt::Display for ExtFuncData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.colocated {
            write!(f, "colocated ")?;
        }
        write!(f, "{} {}", self.name, self.signature)
    }
}

impl ExtFuncData {
    /// Return an estimate of the distance to the referred-to function symbol.
    pub fn reloc_distance(&self) -> RelocDistance {
        if self.colocated {
            RelocDistance::Near
        } else {
            RelocDistance::Far
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::types::{B8, F32, I32};
    use alloc::string::ToString;

    #[test]
    fn argument_type() {
        let t = AbiParam::new(I32);
        assert_eq!(t.to_string(), "i32");
        let mut t = t.uext();
        assert_eq!(t.to_string(), "i32 uext");
        assert_eq!(t.sext().to_string(), "i32 sext");
        t.purpose = ArgumentPurpose::StructReturn;
        assert_eq!(t.to_string(), "i32 uext sret");
        t.legalized_to_pointer = true;
        assert_eq!(t.to_string(), "i32 ptr uext sret");
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
            ArgumentPurpose::SignatureId,
            ArgumentPurpose::StackLimit,
        ];
        for (&e, &n) in all_purpose.iter().zip(PURPOSE_NAMES.iter()) {
            assert_eq!(e.to_string(), n);
            assert_eq!(Ok(e), n.parse());
        }
    }

    #[test]
    fn call_conv() {
        for &cc in &[
            CallConv::Fast,
            CallConv::Cold,
            CallConv::SystemV,
            CallConv::WindowsFastcall,
            CallConv::BaldrdashSystemV,
            CallConv::BaldrdashWindows,
        ] {
            assert_eq!(Ok(cc), cc.to_string().parse())
        }
    }

    #[test]
    fn signatures() {
        let mut sig = Signature::new(CallConv::BaldrdashSystemV);
        assert_eq!(sig.to_string(), "() baldrdash_system_v");
        sig.params.push(AbiParam::new(I32));
        assert_eq!(sig.to_string(), "(i32) baldrdash_system_v");
        sig.returns.push(AbiParam::new(F32));
        assert_eq!(sig.to_string(), "(i32) -> f32 baldrdash_system_v");
        sig.params.push(AbiParam::new(I32.by(4).unwrap()));
        assert_eq!(sig.to_string(), "(i32, i32x4) -> f32 baldrdash_system_v");
        sig.returns.push(AbiParam::new(B8));
        assert_eq!(
            sig.to_string(),
            "(i32, i32x4) -> f32, b8 baldrdash_system_v"
        );

        // Order does not matter.
        sig.params[0].location = ArgumentLoc::Stack(24);
        sig.params[1].location = ArgumentLoc::Stack(8);

        // Writing ABI-annotated signatures.
        assert_eq!(
            sig.to_string(),
            "(i32 [24], i32x4 [8]) -> f32, b8 baldrdash_system_v"
        );
    }
}
