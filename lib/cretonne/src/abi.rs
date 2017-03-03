//! Common helper code for ABI lowering.
//!
//! This module provides functions and data structures that are useful for implementing the
//! `TargetIsa::legalize_signature()` method.

use ir::{ArgumentLoc, ArgumentType, Type};

/// Legalization action to perform on a single argument or return value.
///
/// An argument may go through a sequence of legalization steps before it reaches the final
/// `Assign` action.
pub enum ArgAction {
    /// Assign the argument to the given location.
    Assign(ArgumentLoc),

    /// Split the argument into smaller parts, then call again.
    ///
    /// This action can split an integer type into two smaller integer arguments, or it can split a
    /// SIMD vector into halves.
    ///
    /// Floating point scalar types can't be split.
    Split,
}

/// Common trait for assigning arguments to registers or stack locations.
///
/// This will be implemented by individual ISAs.
pub trait ArgAssigner {
    /// Pick an assignment action for function argument (or return value) `arg`.
    fn assign(&mut self, arg: &ArgumentType) -> ArgAction;
}

/// Legalize the arguments in `args` using the given argument assigner.
///
/// This function can be used for both arguments and return values.
pub fn legalize_args<AA: ArgAssigner>(args: &mut Vec<ArgumentType>, aa: &mut AA) {
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
                args[argno].location = loc;
                argno += 1;
            }
            // Split this argument into two smaller ones. Then revisit both.
            ArgAction::Split => {
                let new_arg = ArgumentType { value_type: split_type(arg.value_type), ..arg };
                args[argno].value_type = new_arg.value_type;
                args.insert(argno + 1, new_arg);
            }
        }
    }
}

/// Given a value type that isn't legal, compute a replacement type half the size.
fn split_type(ty: Type) -> Type {
    if ty.is_int() {
        ty.half_width().expect("Integer type too small to split")
    } else {
        ty.half_vector().expect("Can only split integers and vectors")
    }
}
