//! Value locations.
//!
//! The register allocator assigns every SSA value to either a register or a stack slot. This
//! assignment is represented by a `ValueLoc` object.

use isa::RegUnit;
use ir::StackSlot;

/// Value location.
#[derive(Copy, Clone, Debug)]
pub enum ValueLoc {
    /// This value has not been assigned to a location yet.
    Unassigned,
    /// Value is assigned to a register.
    Reg(RegUnit),
    /// Value is assigned to a stack slot.
    Stack(StackSlot),
}

impl Default for ValueLoc {
    fn default() -> Self {
        ValueLoc::Unassigned
    }
}

/// Function argument location.
///
/// The ABI specifies how arguments are passed to a function, and where return values appear after
/// the call. Just like a `ValueLoc`, function arguments can be passed in registers or on the
/// stack.
///
/// Function arguments on the stack are accessed differently for the incoming arguments to the
/// current function and the outgoing arguments to a called external function. For this reason,
/// the location of stack arguments is described as an offset into the array of function arguments
/// on the stack.
///
/// An `ArgumentLoc` can be translated to a `ValueLoc` only when we know if we're talking about an
/// incoming argument or an outgoing argument.
///
/// - For stack arguments, different `StackSlot` entities are used to represent incoming and
///   outgoing arguments.
/// - For register arguments, there is usually no difference, but if we ever add support for a
///   register-window ISA like SPARC, register arguments would also need to be translated.
#[derive(Copy, Clone, Debug)]
pub enum ArgumentLoc {
    /// This argument has not been assigned to a location yet.
    Unassigned,
    /// Argument is passed in a register.
    Reg(RegUnit),
    /// Argument is passed on the stack, at the given byte offset into the argument array.
    Stack(u32),
}

impl Default for ArgumentLoc {
    fn default() -> Self {
        ArgumentLoc::Unassigned
    }
}
