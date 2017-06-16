//! Stack slots.
//!
//! The `StackSlotData` struct keeps track of a single stack slot in a function.
//!

use entity_map::{EntityMap, PrimaryEntityData, Keys};
use ir::{Type, StackSlot};
use std::fmt;
use std::ops::Index;
use std::str::FromStr;

/// The kind of a stack slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackSlotKind {
    /// A spill slot. This is a stack slot created by the register allocator.
    SpillSlot,

    /// A local variable. This is a chunk of local stack memory for use by the `stack_load` and
    /// `stack_store` instructions.
    Local,

    /// An incoming function argument.
    ///
    /// If the current function has more arguments than fits in registers, the remaining arguments
    /// are passed on the stack by the caller. These incoming arguments are represented as SSA
    /// values assigned to incoming stack slots.
    IncomingArg,

    /// An outgoing function argument.
    ///
    /// When preparing to call a function whose arguments don't fit in registers, outgoing argument
    /// stack slots are used to represent individual arguments in the outgoing call frame. These
    /// stack slots are only valid while setting up a call.
    OutgoingArg,
}

impl FromStr for StackSlotKind {
    type Err = ();

    fn from_str(s: &str) -> Result<StackSlotKind, ()> {
        use self::StackSlotKind::*;
        match s {
            "local" => Ok(Local),
            "spill_slot" => Ok(SpillSlot),
            "incoming_arg" => Ok(IncomingArg),
            "outgoing_arg" => Ok(OutgoingArg),
            _ => Err(()),
        }
    }
}

impl fmt::Display for StackSlotKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::StackSlotKind::*;
        f.write_str(match *self {
                        Local => "local",
                        SpillSlot => "spill_slot",
                        IncomingArg => "incoming_arg",
                        OutgoingArg => "outgoing_arg",
                    })
    }
}

/// Contents of a stack slot.
#[derive(Clone, Debug)]
pub struct StackSlotData {
    /// The kind of stack slot.
    pub kind: StackSlotKind,

    /// Size of stack slot in bytes.
    pub size: u32,
}

impl StackSlotData {
    /// Create a stack slot with the specified byte size.
    pub fn new(kind: StackSlotKind, size: u32) -> StackSlotData {
        StackSlotData { kind, size }
    }
}

impl fmt::Display for StackSlotData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.kind, self.size)
    }
}

impl PrimaryEntityData for StackSlotData {}

/// Stack frame manager.
///
/// Keep track of all the stack slots used by a function.
#[derive(Clone, Debug)]
pub struct StackSlots {
    slots: EntityMap<StackSlot, StackSlotData>,
}

/// Stack slot manager functions that behave mostly like an entity map.
impl StackSlots {
    /// Create an empty stack slot manager.
    pub fn new() -> StackSlots {
        StackSlots { slots: EntityMap::new() }
    }

    /// Clear out everything.
    pub fn clear(&mut self) {
        self.slots.clear();
    }

    /// Allocate a new stack slot.
    ///
    /// This function should be primarily used by the text format parser. There are more convenient
    /// functions for creating specific kinds of stack slots below.
    pub fn push(&mut self, data: StackSlotData) -> StackSlot {
        self.slots.push(data)
    }

    /// Check if `ss` is a valid stack slot reference.
    pub fn is_valid(&self, ss: StackSlot) -> bool {
        self.slots.is_valid(ss)
    }

    /// Get an iterator over all the stack slot keys.
    pub fn keys(&self) -> Keys<StackSlot> {
        self.slots.keys()
    }

    /// Get a reference to the next stack slot that would be created by `push()`.
    ///
    /// This should just be used by the parser.
    pub fn next_key(&self) -> StackSlot {
        self.slots.next_key()
    }
}

/// Higher-level stack frame manipulation functions.
impl StackSlots {
    /// Create a new spill slot for spilling values of type `ty`.
    pub fn make_spill_slot(&mut self, ty: Type) -> StackSlot {
        let bytes = (ty.bits() as u32 + 7) / 8;
        self.push(StackSlotData::new(StackSlotKind::SpillSlot, bytes))
    }
}

impl Index<StackSlot> for StackSlots {
    type Output = StackSlotData;

    fn index(&self, ss: StackSlot) -> &StackSlotData {
        &self.slots[ss]
    }
}

#[cfg(test)]
mod tests {
    use ir::Function;
    use super::*;

    #[test]
    fn stack_slot() {
        let mut func = Function::new();

        let ss0 = func.stack_slots
            .push(StackSlotData::new(StackSlotKind::IncomingArg, 4));
        let ss1 = func.stack_slots
            .push(StackSlotData::new(StackSlotKind::SpillSlot, 8));
        assert_eq!(ss0.to_string(), "ss0");
        assert_eq!(ss1.to_string(), "ss1");

        assert_eq!(func.stack_slots[ss0].size, 4);
        assert_eq!(func.stack_slots[ss1].size, 8);

        assert_eq!(func.stack_slots[ss0].to_string(), "incoming_arg 4");
        assert_eq!(func.stack_slots[ss1].to_string(), "spill_slot 8");
    }
}
