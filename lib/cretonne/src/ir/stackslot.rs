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

    /// Offset of stack slot relative to the stack pointer in the caller.
    ///
    /// On Intel ISAs, the base address is the stack pointer *before* the return address was
    /// pushed. On RISC ISAs, the base address is the value of the stack pointer on entry to the
    /// function.
    ///
    /// For `OutgoingArg` stack slots, the offset is relative to the current function's stack
    /// pointer immediately before the call.
    pub offset: i32,
}

impl StackSlotData {
    /// Create a stack slot with the specified byte size.
    pub fn new(kind: StackSlotKind, size: u32) -> StackSlotData {
        StackSlotData {
            kind,
            size,
            offset: 0,
        }
    }
}

impl fmt::Display for StackSlotData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.kind, self.size)?;
        if self.offset != 0 {
            write!(f, ", offset {}", self.offset)?;
        }
        Ok(())
    }
}

impl PrimaryEntityData for StackSlotData {}

/// Stack frame manager.
///
/// Keep track of all the stack slots used by a function.
#[derive(Clone, Debug)]
pub struct StackSlots {
    /// All allocated stack slots.
    slots: EntityMap<StackSlot, StackSlotData>,

    /// All the outgoing stack slots, ordered by offset.
    outgoing: Vec<StackSlot>,
}

/// Stack slot manager functions that behave mostly like an entity map.
impl StackSlots {
    /// Create an empty stack slot manager.
    pub fn new() -> StackSlots {
        StackSlots {
            slots: EntityMap::new(),
            outgoing: Vec::new(),
        }
    }

    /// Clear out everything.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.outgoing.clear();
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
        self.push(StackSlotData::new(StackSlotKind::SpillSlot, ty.bytes()))
    }

    /// Create a stack slot representing an incoming function argument.
    pub fn make_incoming_arg(&mut self, ty: Type, offset: i32) -> StackSlot {
        let mut data = StackSlotData::new(StackSlotKind::IncomingArg, ty.bytes());
        assert!(offset <= i32::max_value() - data.size as i32);
        data.offset = offset;
        self.push(data)
    }

    /// Get a stack slot representing an outgoing argument.
    ///
    /// This may create a new stack slot, or reuse an existing outgoing stack slot with the
    /// requested offset and size.
    ///
    /// The requested offset is relative to this function's stack pointer immediately before making
    /// the call.
    pub fn get_outgoing_arg(&mut self, ty: Type, offset: i32) -> StackSlot {
        let size = ty.bytes();

        // Look for an existing outgoing stack slot with the same offset and size.
        let inspos = match self.outgoing
                  .binary_search_by_key(&(offset, size),
                                        |&ss| (self[ss].offset, self[ss].size)) {
            Ok(idx) => return self.outgoing[idx],
            Err(idx) => idx,
        };

        // No existing slot found. Make one and insert it into `outgoing`.
        let mut data = StackSlotData::new(StackSlotKind::OutgoingArg, size);
        assert!(offset <= i32::max_value() - size as i32);
        data.offset = offset;
        let ss = self.slots.push(data);
        self.outgoing.insert(inspos, ss);
        ss
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
    use ir::types;
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

    #[test]
    fn outgoing() {
        let mut sss = StackSlots::new();

        let ss0 = sss.get_outgoing_arg(types::I32, 8);
        let ss1 = sss.get_outgoing_arg(types::I32, 4);
        let ss2 = sss.get_outgoing_arg(types::I64, 8);

        assert_eq!(sss[ss0].offset, 8);
        assert_eq!(sss[ss0].size, 4);

        assert_eq!(sss[ss1].offset, 4);
        assert_eq!(sss[ss1].size, 4);

        assert_eq!(sss[ss2].offset, 8);
        assert_eq!(sss[ss2].size, 8);

        assert_eq!(sss.get_outgoing_arg(types::I32, 8), ss0);
        assert_eq!(sss.get_outgoing_arg(types::I32, 4), ss1);
        assert_eq!(sss.get_outgoing_arg(types::I64, 8), ss2);
    }
}
