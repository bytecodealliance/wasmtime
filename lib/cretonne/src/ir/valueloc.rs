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
