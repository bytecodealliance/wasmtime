//! Stack slots.
//!
//! The `StackSlotData` struct keeps track of a single stack slot in a function.
//!

use crate::entity::{Iter, IterMut, Keys, PrimaryMap};
use crate::ir::{StackSlot, Type};
use crate::packed_option::PackedOption;
use alloc::vec::Vec;
use core::cmp;
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

/// A stack offset.
///
/// The location of a stack offset relative to a stack pointer or frame pointer.
pub type StackOffset = i32;

/// The minimum size of a spill slot in bytes.
///
/// ISA implementations are allowed to assume that small types like `b1` and `i8` get a full 4-byte
/// spill slot.
const MIN_SPILL_SLOT_SIZE: StackSize = 4;

/// Get the spill slot size to use for `ty`.
fn spill_size(ty: Type) -> StackSize {
    cmp::max(MIN_SPILL_SLOT_SIZE, ty.bytes())
}

/// The kind of a stack slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum StackSlotKind {
    /// A spill slot. This is a stack slot created by the register allocator.
    SpillSlot,

    /// An explicit stack slot. This is a chunk of stack memory for use by the `stack_load`
    /// and `stack_store` instructions.
    ExplicitSlot,

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

    /// Space allocated in the caller's frame for the callee's return values
    /// that are passed out via return pointer.
    ///
    /// If there are more return values than registers available for the callee's calling
    /// convention, or the return value is larger than the available registers' space, then we
    /// allocate stack space in this frame and pass a pointer to the callee, which then writes its
    /// return values into this space.
    StructReturnSlot,

    /// An emergency spill slot.
    ///
    /// Emergency slots are allocated late when the register's constraint solver needs extra space
    /// to shuffle registers around. They are only used briefly, and can be reused.
    EmergencySlot,
}

impl FromStr for StackSlotKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        use self::StackSlotKind::*;
        match s {
            "explicit_slot" => Ok(ExplicitSlot),
            "spill_slot" => Ok(SpillSlot),
            "incoming_arg" => Ok(IncomingArg),
            "outgoing_arg" => Ok(OutgoingArg),
            "sret_slot" => Ok(StructReturnSlot),
            "emergency_slot" => Ok(EmergencySlot),
            _ => Err(()),
        }
    }
}

impl fmt::Display for StackSlotKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::StackSlotKind::*;
        f.write_str(match *self {
            ExplicitSlot => "explicit_slot",
            SpillSlot => "spill_slot",
            IncomingArg => "incoming_arg",
            OutgoingArg => "outgoing_arg",
            StructReturnSlot => "sret_slot",
            EmergencySlot => "emergency_slot",
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

    /// Offset of stack slot relative to the stack pointer in the caller.
    ///
    /// On x86, the base address is the stack pointer *before* the return address was pushed. On
    /// RISC ISAs, the base address is the value of the stack pointer on entry to the function.
    ///
    /// For `OutgoingArg` stack slots, the offset is relative to the current function's stack
    /// pointer immediately before the call.
    pub offset: Option<StackOffset>,
}

impl StackSlotData {
    /// Create a stack slot with the specified byte size.
    pub fn new(kind: StackSlotKind, size: StackSize) -> Self {
        Self {
            kind,
            size,
            offset: None,
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
        if let Some(offset) = self.offset {
            write!(f, ", offset {}", offset)?;
        }
        Ok(())
    }
}

/// Stack frame layout information.
///
/// This is computed by the `layout_stack()` method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct StackLayoutInfo {
    /// The total size of the stack frame.
    ///
    /// This is the distance from the stack pointer in the current function to the stack pointer in
    /// the calling function, so it includes a pushed return address as well as space for outgoing
    /// call arguments.
    pub frame_size: StackSize,

    /// The total size of the stack frame for inbound arguments pushed by the caller.
    pub inbound_args_size: StackSize,
}

/// Stack frame manager.
///
/// Keep track of all the stack slots used by a function.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct StackSlots {
    /// All allocated stack slots.
    slots: PrimaryMap<StackSlot, StackSlotData>,

    /// All the outgoing stack slots, ordered by offset.
    outgoing: Vec<StackSlot>,

    /// All the emergency slots.
    emergency: Vec<StackSlot>,

    /// Layout information computed from `layout_stack`.
    pub layout_info: Option<StackLayoutInfo>,
}

/// Stack slot manager functions that behave mostly like an entity map.
impl StackSlots {
    /// Create an empty stack slot manager.
    pub fn new() -> Self {
        Self {
            slots: PrimaryMap::new(),
            outgoing: Vec::new(),
            emergency: Vec::new(),
            layout_info: None,
        }
    }

    /// Clear out everything.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.outgoing.clear();
        self.emergency.clear();
        self.layout_info = None;
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

/// Higher-level stack frame manipulation functions.
impl StackSlots {
    /// Create a new spill slot for spilling values of type `ty`.
    pub fn make_spill_slot(&mut self, ty: Type) -> StackSlot {
        self.push(StackSlotData::new(StackSlotKind::SpillSlot, spill_size(ty)))
    }

    /// Create a stack slot representing an incoming function argument.
    pub fn make_incoming_arg(&mut self, ty: Type, offset: StackOffset) -> StackSlot {
        let mut data = StackSlotData::new(StackSlotKind::IncomingArg, ty.bytes());
        debug_assert!(offset <= StackOffset::max_value() - data.size as StackOffset);
        data.offset = Some(offset);
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
        let inspos = match self.outgoing.binary_search_by_key(&(offset, size), |&ss| {
            (self[ss].offset.unwrap(), self[ss].size)
        }) {
            Ok(idx) => return self.outgoing[idx],
            Err(idx) => idx,
        };

        // No existing slot found. Make one and insert it into `outgoing`.
        let mut data = StackSlotData::new(StackSlotKind::OutgoingArg, size);
        debug_assert!(offset <= StackOffset::max_value() - size as StackOffset);
        data.offset = Some(offset);
        let ss = self.slots.push(data);
        self.outgoing.insert(inspos, ss);
        ss
    }

    /// Get an emergency spill slot that can be used to store a `ty` value.
    ///
    /// This may allocate a new slot, or it may reuse an existing emergency spill slot, excluding
    /// any slots in the `in_use` list.
    pub fn get_emergency_slot(
        &mut self,
        ty: Type,
        in_use: &[PackedOption<StackSlot>],
    ) -> StackSlot {
        let size = spill_size(ty);

        // Find the smallest existing slot that can fit the type.
        if let Some(&ss) = self
            .emergency
            .iter()
            .filter(|&&ss| self[ss].size >= size && !in_use.contains(&ss.into()))
            .min_by_key(|&&ss| self[ss].size)
        {
            return ss;
        }

        // Alternatively, use the largest available slot and make it larger.
        if let Some(&ss) = self
            .emergency
            .iter()
            .filter(|&&ss| !in_use.contains(&ss.into()))
            .max_by_key(|&&ss| self[ss].size)
        {
            self.slots[ss].size = size;
            return ss;
        }

        // No existing slot found. Make one and insert it into `emergency`.
        let data = StackSlotData::new(StackSlotKind::EmergencySlot, size);
        let ss = self.slots.push(data);
        self.emergency.push(ss);
        ss
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::types;
    use crate::ir::Function;
    use alloc::string::ToString;

    #[test]
    fn stack_slot() {
        let mut func = Function::new();

        let ss0 = func.create_stack_slot(StackSlotData::new(StackSlotKind::IncomingArg, 4));
        let ss1 = func.create_stack_slot(StackSlotData::new(StackSlotKind::SpillSlot, 8));
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

        assert_eq!(sss[ss0].offset, Some(8));
        assert_eq!(sss[ss0].size, 4);

        assert_eq!(sss[ss1].offset, Some(4));
        assert_eq!(sss[ss1].size, 4);

        assert_eq!(sss[ss2].offset, Some(8));
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

        let slot2 = StackSlotData::new(StackSlotKind::ExplicitSlot, 24);

        assert_eq!(slot2.alignment(4), 4);
        assert_eq!(slot2.alignment(8), 8);
        assert_eq!(slot2.alignment(16), 8);
        assert_eq!(slot2.alignment(32), 8);
    }

    #[test]
    fn emergency() {
        let mut sss = StackSlots::new();

        let ss0 = sss.get_emergency_slot(types::I32, &[]);
        assert_eq!(sss[ss0].size, 4);

        // When a smaller size is requested, we should simply get the same slot back.
        assert_eq!(sss.get_emergency_slot(types::I8, &[]), ss0);
        assert_eq!(sss[ss0].size, 4);
        assert_eq!(sss.get_emergency_slot(types::F32, &[]), ss0);
        assert_eq!(sss[ss0].size, 4);

        // Ask for a larger size and the slot should grow.
        assert_eq!(sss.get_emergency_slot(types::F64, &[]), ss0);
        assert_eq!(sss[ss0].size, 8);

        // When one slot is in use, we should get a new one.
        let ss1 = sss.get_emergency_slot(types::I32, &[None.into(), ss0.into()]);
        assert_eq!(sss[ss0].size, 8);
        assert_eq!(sss[ss1].size, 4);

        // Now we should get the smallest fit of the two available slots.
        assert_eq!(sss.get_emergency_slot(types::F32, &[]), ss1);
        assert_eq!(sss.get_emergency_slot(types::F64, &[]), ss0);
    }
}
