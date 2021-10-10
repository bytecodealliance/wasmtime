//! Stack slots.
//!
//! The `StackSlotData` struct keeps track of a single stack slot in a function.
//!

use crate::entity::{Iter, IterMut, Keys, PrimaryMap};
use crate::ir::StackSlot;
use core::fmt;
use core::ops::{Index, IndexMut};
use core::slice;
use core::str::FromStr;

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

/// The size of an object on the stack, or the size of a stack frame.
///
/// We don't use `usize` to represent object sizes on the target platform because Cranelift supports
/// cross-compilation, and `usize` is a type that depends on the host platform, not the target
/// platform.
pub type StackSize = u32;

/// The kind of a stack slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum StackSlotKind {
    /// An explicit stack slot. This is a chunk of stack memory for use by the `stack_load`
    /// and `stack_store` instructions.
    ExplicitSlot,
}

impl FromStr for StackSlotKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        use self::StackSlotKind::*;
        match s {
            "explicit_slot" => Ok(ExplicitSlot),
            _ => Err(()),
        }
    }
}

impl fmt::Display for StackSlotKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::StackSlotKind::*;
        f.write_str(match *self {
            ExplicitSlot => "explicit_slot",
        })
    }
}

/// Contents of a stack slot.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct StackSlotData {
    /// The kind of stack slot.
    pub kind: StackSlotKind,

    /// Size of stack slot in bytes.
    pub size: StackSize,
}

impl StackSlotData {
    /// Create a stack slot with the specified byte size.
    pub fn new(kind: StackSlotKind, size: StackSize) -> Self {
        Self { kind, size }
    }

    /// Get the alignment in bytes of this stack slot given the stack pointer alignment.
    pub fn alignment(&self, max_align: StackSize) -> StackSize {
        debug_assert!(max_align.is_power_of_two());
        // We want to find the largest power of two that divides both `self.size` and `max_align`.
        // That is the same as isolating the rightmost bit in `x`.
        let x = self.size | max_align;
        // C.f. Hacker's delight.
        x & x.wrapping_neg()
    }
}

impl fmt::Display for StackSlotData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.kind, self.size)
    }
}

/// Stack frame manager.
///
/// Keep track of all the stack slots used by a function.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct StackSlots {
    /// All allocated stack slots.
    slots: PrimaryMap<StackSlot, StackSlotData>,
}

/// Stack slot manager functions that behave mostly like an entity map.
impl StackSlots {
    /// Create an empty stack slot manager.
    pub fn new() -> Self {
        StackSlots::default()
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
    pub fn iter(&self) -> Iter<StackSlot, StackSlotData> {
        self.slots.iter()
    }

    /// Get an iterator over all the stack slot keys, mutable edition.
    pub fn iter_mut(&mut self) -> IterMut<StackSlot, StackSlotData> {
        self.slots.iter_mut()
    }

    /// Get an iterator over all the stack slot records.
    pub fn values(&self) -> slice::Iter<StackSlotData> {
        self.slots.values()
    }

    /// Get an iterator over all the stack slot records, mutable edition.
    pub fn values_mut(&mut self) -> slice::IterMut<StackSlotData> {
        self.slots.values_mut()
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

impl Index<StackSlot> for StackSlots {
    type Output = StackSlotData;

    fn index(&self, ss: StackSlot) -> &StackSlotData {
        &self.slots[ss]
    }
}

impl IndexMut<StackSlot> for StackSlots {
    fn index_mut(&mut self, ss: StackSlot) -> &mut StackSlotData {
        &mut self.slots[ss]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Function;
    use alloc::string::ToString;

    #[test]
    fn stack_slot() {
        let mut func = Function::new();

        let ss0 = func.create_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 4));
        let ss1 = func.create_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8));
        assert_eq!(ss0.to_string(), "ss0");
        assert_eq!(ss1.to_string(), "ss1");

        assert_eq!(func.stack_slots[ss0].size, 4);
        assert_eq!(func.stack_slots[ss1].size, 8);

        assert_eq!(func.stack_slots[ss0].to_string(), "explicit_slot 4");
        assert_eq!(func.stack_slots[ss1].to_string(), "explicit_slot 8");
    }

    #[test]
    fn alignment() {
        let slot = StackSlotData::new(StackSlotKind::ExplicitSlot, 8);

        assert_eq!(slot.alignment(4), 4);
        assert_eq!(slot.alignment(8), 8);
        assert_eq!(slot.alignment(16), 8);

        let slot2 = StackSlotData::new(StackSlotKind::ExplicitSlot, 24);

        assert_eq!(slot2.alignment(4), 4);
        assert_eq!(slot2.alignment(8), 8);
        assert_eq!(slot2.alignment(16), 8);
        assert_eq!(slot2.alignment(32), 8);
    }
}
