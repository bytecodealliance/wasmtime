//! Common helper code for ABI lowering.
//!
//! This module provides functions and data structures that are useful for implementing the
//! `TargetIsa::legalize_signature()` method.

use crate::ir::{AbiParam, ArgumentExtension, ArgumentLoc, Type};
use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::cmp::Ordering;

/// Legalization action to perform on a single argument or return value when converting a
/// signature.
///
/// An argument may go through a sequence of legalization steps before it reaches the final
/// `Assign` action.
#[derive(Clone, Copy, Debug)]
pub enum ArgAction {
    /// Assign the argument to the given location.
    Assign(ArgumentLoc),

    /// Convert the argument, then call again.
    ///
    /// This action can split an integer type into two smaller integer arguments, or it can split a
    /// SIMD vector into halves.
    Convert(ValueConversion),
}

impl From<ArgumentLoc> for ArgAction {
    fn from(x: ArgumentLoc) -> Self {
        Self::Assign(x)
    }
}

impl From<ValueConversion> for ArgAction {
    fn from(x: ValueConversion) -> Self {
        Self::Convert(x)
    }
}

/// Legalization action to be applied to a value that is being passed to or from a legalized ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueConversion {
    /// Split an integer types into low and high parts, using `isplit`.
    IntSplit,

    /// Split a vector type into halves with identical lane types, using `vsplit`.
    VectorSplit,

    /// Bit-cast to an integer type of the same size.
    IntBits,

    /// Sign-extend integer value to the required type.
    Sext(Type),

    /// Unsigned zero-extend value to the required type.
    Uext(Type),
}

impl ValueConversion {
    /// Apply this conversion to a type, return the converted type.
    pub fn apply(self, ty: Type) -> Type {
        match self {
            Self::IntSplit => ty.half_width().expect("Integer type too small to split"),
            Self::VectorSplit => ty.half_vector().expect("Not a vector"),
            Self::IntBits => Type::int(ty.bits()).expect("Bad integer size"),
            Self::Sext(nty) | Self::Uext(nty) => nty,
        }
    }

    /// Is this a split conversion that results in two arguments?
    pub fn is_split(self) -> bool {
        match self {
            Self::IntSplit | Self::VectorSplit => true,
            _ => false,
        }
    }
}

/// Common trait for assigning arguments to registers or stack locations.
///
/// This will be implemented by individual ISAs.
pub trait ArgAssigner {
    /// Pick an assignment action for function argument (or return value) `arg`.
    fn assign(&mut self, arg: &AbiParam) -> ArgAction;
}

/// Legalize the arguments in `args` using the given argument assigner.
///
/// This function can be used for both arguments and return values.
pub fn legalize_args<AA: ArgAssigner>(args: &[AbiParam], aa: &mut AA) -> Option<Vec<AbiParam>> {
    let mut args = Cow::Borrowed(args);

    // Iterate over the arguments.
    // We may need to mutate the vector in place, so don't use a normal iterator, and clone the
    // argument to avoid holding a reference.
    let mut argno = 0;
    while let Some(arg) = args.get(argno).cloned() {
        // Leave the pre-assigned arguments alone.
        // We'll assume that they don't interfere with our assignments.
        if arg.location.is_assigned() {
            argno += 1;
            continue;
        }

        match aa.assign(&arg) {
            // Assign argument to a location and move on to the next one.
            ArgAction::Assign(loc) => {
                args.to_mut()[argno].location = loc;
                argno += 1;
            }
            // Split this argument into two smaller ones. Then revisit both.
            ArgAction::Convert(conv) => {
                let value_type = conv.apply(arg.value_type);
                let new_arg = AbiParam { value_type, ..arg };
                args.to_mut()[argno].value_type = value_type;
                if conv.is_split() {
                    args.to_mut().insert(argno + 1, new_arg);
                }
            }
        }
    }

    match args {
        Cow::Borrowed(_) => None,
        Cow::Owned(a) => Some(a),
    }
}

/// Determine the right action to take when passing a `have` value type to a call signature where
/// the next argument is `arg` which has a different value type.
///
/// The signature legalization process in `legalize_args` above can replace a single argument value
/// with multiple arguments of smaller types. It can also change the type of an integer argument to
/// a larger integer type, requiring the smaller value to be sign- or zero-extended.
///
/// The legalizer needs to repair the values at all ABI boundaries:
///
/// - Incoming function arguments to the entry block.
/// - Function arguments passed to a call.
/// - Return values from a call.
/// - Return values passed to a return instruction.
///
/// The `legalize_abi_value` function helps the legalizer with the process. When the legalizer
/// needs to pass a pre-legalized `have` argument, but the ABI argument `arg` has a different value
/// type, `legalize_abi_value(have, arg)` tells the legalizer how to create the needed value type
/// for the argument.
///
/// It may be necessary to call `legalize_abi_value` more than once for a given argument before the
/// desired argument type appears. This will happen when a vector or integer type needs to be split
/// more than once, for example.
pub fn legalize_abi_value(have: Type, arg: &AbiParam) -> ValueConversion {
    let have_bits = have.bits();
    let arg_bits = arg.value_type.bits();

    match have_bits.cmp(&arg_bits) {
        // We have fewer bits than the ABI argument.
        Ordering::Less => {
            debug_assert!(
                have.is_int() && arg.value_type.is_int(),
                "Can only extend integer values"
            );
            match arg.extension {
                ArgumentExtension::Uext => ValueConversion::Uext(arg.value_type),
                ArgumentExtension::Sext => ValueConversion::Sext(arg.value_type),
                _ => panic!("No argument extension specified"),
            }
        }
        // We have the same number of bits as the argument.
        Ordering::Equal => {
            // This must be an integer vector that is split and then extended.
            debug_assert!(arg.value_type.is_int());
            debug_assert!(have.is_vector(), "expected vector type, got {}", have);
            ValueConversion::VectorSplit
        }
        // We have more bits than the argument.
        Ordering::Greater => {
            if have.is_vector() {
                ValueConversion::VectorSplit
            } else if have.is_float() {
                // Convert a float to int so it can be split the next time.
                // ARM would do this to pass an `f64` in two registers.
                ValueConversion::IntBits
            } else {
                ValueConversion::IntSplit
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::types;
    use crate::ir::AbiParam;

    #[test]
    fn legalize() {
        let mut arg = AbiParam::new(types::I32);

        assert_eq!(
            legalize_abi_value(types::I64X2, &arg),
            ValueConversion::VectorSplit
        );
        assert_eq!(
            legalize_abi_value(types::I64, &arg),
            ValueConversion::IntSplit
        );

        // Vector of integers is broken down, then sign-extended.
        arg.extension = ArgumentExtension::Sext;
        assert_eq!(
            legalize_abi_value(types::I16X4, &arg),
            ValueConversion::VectorSplit
        );
        assert_eq!(
            legalize_abi_value(types::I16.by(2).unwrap(), &arg),
            ValueConversion::VectorSplit
        );
        assert_eq!(
            legalize_abi_value(types::I16, &arg),
            ValueConversion::Sext(types::I32)
        );

        // 64-bit float is split as an integer.
        assert_eq!(
            legalize_abi_value(types::F64, &arg),
            ValueConversion::IntBits
        );
    }
}
