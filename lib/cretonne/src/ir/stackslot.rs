//! Stack slots.
//!
//! The `StackSlotData` struct keeps track of a single stack slot in a function.
//!

use entity_map::{EntityMap, PrimaryEntityData, Keys};
use ir::{Type, StackSlot};
use std::cmp::{min, max};
use std::fmt;
use std::ops::Index;
use std::str::FromStr;

/// The size of an object on the stack, or the size of a stack frame.
///
/// We don't use `usize` to represent object sizes on the target platform because Cretonne supports
/// cross-compilation, and `usize` is a type that depends on the host platform, not the target
/// platform.
type StackSize = u32;

/// A stack offset.
///
/// The location of a stack offset relative to a stack pointer or frame pointer.
type StackOffset = i32;

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
    pub size: StackSize,

    /// Offset of stack slot relative to the stack pointer in the caller.
    ///
    /// On Intel ISAs, the base address is the stack pointer *before* the return address was
    /// pushed. On RISC ISAs, the base address is the value of the stack pointer on entry to the
    /// function.
    ///
    /// For `OutgoingArg` stack slots, the offset is relative to the current function's stack
    /// pointer immediately before the call.
    pub offset: StackOffset,
}

impl StackSlotData {
    /// Create a stack slot with the specified byte size.
    pub fn new(kind: StackSlotKind, size: StackSize) -> StackSlotData {
        StackSlotData {
            kind,
            size,
            offset: 0,
        }
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

    /// The total size of the stack frame.
    ///
    /// This is the distance from the stack pointer in the current function to the stack pointer in
    /// the calling function, so it includes a pushed return address as well as space for outgoing
    /// call arguments.
    ///
    /// This is computed by the `layout()` method.
    pub frame_size: Option<StackSize>,
}

/// Stack slot manager functions that behave mostly like an entity map.
impl StackSlots {
    /// Create an empty stack slot manager.
    pub fn new() -> StackSlots {
        StackSlots {
            slots: EntityMap::new(),
            outgoing: Vec::new(),
            frame_size: None,
        }
    }

    /// Clear out everything.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.outgoing.clear();
        self.frame_size = None;
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

impl Index<StackSlot> for StackSlots {
    type Output = StackSlotData;

    fn index(&self, ss: StackSlot) -> &StackSlotData {
        &self.slots[ss]
    }
}

/// Higher-level stack frame manipulation functions.
impl StackSlots {
    /// Create a new spill slot for spilling values of type `ty`.
    pub fn make_spill_slot(&mut self, ty: Type) -> StackSlot {
        self.push(StackSlotData::new(StackSlotKind::SpillSlot, ty.bytes()))
    }

    /// Create a stack slot representing an incoming function argument.
    pub fn make_incoming_arg(&mut self, ty: Type, offset: StackOffset) -> StackSlot {
        let mut data = StackSlotData::new(StackSlotKind::IncomingArg, ty.bytes());
        assert!(offset <= StackOffset::max_value() - data.size as StackOffset);
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
    pub fn get_outgoing_arg(&mut self, ty: Type, offset: StackOffset) -> StackSlot {
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
        assert!(offset <= StackOffset::max_value() - size as StackOffset);
        data.offset = offset;
        let ss = self.slots.push(data);
        self.outgoing.insert(inspos, ss);
        ss
    }

    /// Compute the stack frame layout.
    ///
    /// Determine the total size of this function's stack frame and assign offsets to all `Spill`
    /// and `Local` stack slots.
    ///
    /// The total frame size will be a multiple of `alignment` which must be a power of two.
    ///
    /// Returns the total stack frame size which is also saved in `self.frame_size`.
    pub fn layout(&mut self, alignment: StackSize) -> StackSize {
        assert!(alignment.is_power_of_two() && alignment <= StackOffset::max_value() as StackSize,
                "Invalid stack alignment {}",
                alignment);

        // We assume a stack that grows toward lower addresses as implemented by modern ISAs. The
        // stack layout from high to low addresses will be:
        //
        // 1. incoming arguments.
        // 2. spills + locals.
        // 3. outgoing arguments.
        //
        // The incoming arguments can have both positive and negative offsets. A negative offset
        // incoming arguments is usually the x86 return address pushed by the call instruction, but
        // it can also be fixed stack slots pushed by an externally generated prologue.
        //
        // Both incoming and outgoing argument slots have fixed offsets that are treated as
        // reserved zones by the layout algorithm.

        let mut incoming_min = 0;
        let mut outgoing_max = 0;
        let mut min_align = alignment;

        for ss in self.keys() {
            let slot = &self[ss];
            assert!(slot.size <= StackOffset::max_value() as StackSize);
            match slot.kind {
                StackSlotKind::IncomingArg => {
                    incoming_min = min(incoming_min, slot.offset);
                }
                StackSlotKind::OutgoingArg => {
                    let offset = slot.offset
                        .checked_add(slot.size as StackOffset)
                        .expect("Outgoing call argument overflows stack");
                    outgoing_max = max(outgoing_max, offset);
                }
                StackSlotKind::SpillSlot | StackSlotKind::Local => {
                    // Determine the smallest alignment of any local or spill slot.
                    min_align = slot.alignment(min_align);
                }
            }
        }

        // Lay out spill slots and locals below the incoming arguments.
        // The offset is negative, growing downwards.
        // Start with the smallest alignments for better packing.
        let mut offset = incoming_min;
        assert!(min_align.is_power_of_two());
        while min_align <= alignment {
            for ss in self.keys() {
                let slot = &mut self.slots[ss];

                // Pick out locals and spill slots with exact alignment `min_align`.
                match slot.kind {
                    StackSlotKind::SpillSlot | StackSlotKind::Local => {
                        if slot.alignment(alignment) != min_align {
                            continue;
                        }
                    }
                    _ => continue,
                }

                // These limits should never be exceeded by spill slots, but locals can be
                // arbitrarily large.
                assert!(slot.size <= StackOffset::max_value() as StackSize);
                offset = offset
                    .checked_sub(slot.size as StackOffset)
                    .expect("Stack frame larger than 2 GB");

                // Aligning the negative offset can never cause overflow. We're only clearing bits.
                offset &= -(min_align as StackOffset);
                slot.offset = offset;
            }

            // Move on to the next higher alignment.
            min_align *= 2;
        }

        // Finally, make room for the outgoing arguments.
        offset = offset
            .checked_sub(outgoing_max)
            .expect("Stack frame larger than 2 GB");
        offset &= -(alignment as StackOffset);

        let frame_size = (offset as StackSize).wrapping_neg();
        self.frame_size = Some(frame_size);
        frame_size
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

    #[test]
    fn alignment() {
        let slot = StackSlotData::new(StackSlotKind::SpillSlot, 8);

        assert_eq!(slot.alignment(4), 4);
        assert_eq!(slot.alignment(8), 8);
        assert_eq!(slot.alignment(16), 8);

        let slot2 = StackSlotData::new(StackSlotKind::Local, 24);

        assert_eq!(slot2.alignment(4), 4);
        assert_eq!(slot2.alignment(8), 8);
        assert_eq!(slot2.alignment(16), 8);
        assert_eq!(slot2.alignment(32), 8);
    }

    #[test]
    fn layout() {
        let mut sss = StackSlots::new();

        // An empty layout should have 0-sized stack frame.
        assert_eq!(sss.layout(1), 0);
        assert_eq!(sss.layout(16), 0);

        // Same for incoming arguments with non-negative offsets.
        let in0 = sss.make_incoming_arg(types::I64, 0);
        let in1 = sss.make_incoming_arg(types::I64, 8);

        assert_eq!(sss.layout(1), 0);
        assert_eq!(sss.layout(16), 0);
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);

        // Add some spill slots.
        let ss0 = sss.make_spill_slot(types::I64);
        let ss1 = sss.make_spill_slot(types::I32);

        assert_eq!(sss.layout(1), 12);
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[ss0].offset, -8);
        assert_eq!(sss[ss1].offset, -12);

        assert_eq!(sss.layout(16), 16);
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[ss0].offset, -16);
        assert_eq!(sss[ss1].offset, -4);

        // An incoming argument with negative offset counts towards the total frame size, but it
        // should still pack nicely with the spill slots.
        let in2 = sss.make_incoming_arg(types::I32, -4);

        assert_eq!(sss.layout(1), 16);
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[in2].offset, -4);
        assert_eq!(sss[ss0].offset, -12);
        assert_eq!(sss[ss1].offset, -16);

        assert_eq!(sss.layout(16), 16);
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[in2].offset, -4);
        assert_eq!(sss[ss0].offset, -16);
        assert_eq!(sss[ss1].offset, -8);

        // Finally, make sure there is room for the outgoing args.
        let out0 = sss.get_outgoing_arg(types::I32, 0);

        assert_eq!(sss.layout(1), 20);
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[in2].offset, -4);
        assert_eq!(sss[ss0].offset, -12);
        assert_eq!(sss[ss1].offset, -16);
        assert_eq!(sss[out0].offset, 0);

        assert_eq!(sss.layout(16), 32);
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[in2].offset, -4);
        assert_eq!(sss[ss0].offset, -16);
        assert_eq!(sss[ss1].offset, -8);
        assert_eq!(sss[out0].offset, 0);
    }
}
