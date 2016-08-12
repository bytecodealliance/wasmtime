//! Stack slots.
//!
//! The `StackSlotData` struct keeps track of a single stack slot in a function.
//!

use std::fmt::{self, Display, Formatter};

/// Contents of a stack slot.
#[derive(Debug)]
pub struct StackSlotData {
    /// Size of stack slot in bytes.
    pub size: u32,
}

impl StackSlotData {
    /// Create a stack slot with the specified byte size.
    pub fn new(size: u32) -> StackSlotData {
        StackSlotData { size: size }
    }
}

impl Display for StackSlotData {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "stack_slot {}", self.size)
    }
}

#[cfg(test)]
mod tests {
    use ir::Function;
    use super::StackSlotData;

    #[test]
    fn stack_slot() {
        let mut func = Function::new();

        let ss0 = func.stack_slots.push(StackSlotData::new(4));
        let ss1 = func.stack_slots.push(StackSlotData::new(8));
        assert_eq!(ss0.to_string(), "ss0");
        assert_eq!(ss1.to_string(), "ss1");

        assert_eq!(func.stack_slots[ss0].size, 4);
        assert_eq!(func.stack_slots[ss1].size, 8);
    }
}
