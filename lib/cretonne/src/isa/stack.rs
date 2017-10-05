//! Low-level details of stack accesses.
//!
//! The `ir::StackSlots` type deals with stack slots and stack frame layout. The `StackRef` type
//! defined in this module expresses the low-level details of accessing a stack slot from an
//! encoded instruction.

use ir::stackslot::{StackSlots, StackOffset};
use ir::StackSlot;

/// A method for referencing a stack slot in the current stack frame.
///
/// Stack slots are addressed with a constant offset from a base register. The base can be the
/// stack pointer, the frame pointer, or (in the future) a zone register pointing to an inner zone
/// of a large stack frame.
#[derive(Clone, Copy, Debug)]
pub struct StackRef {
    /// The base register to use for addressing.
    pub base: StackBase,

    /// Immediate offset from the base register to the first byte of the stack slot.
    pub offset: StackOffset,
}

impl StackRef {
    /// Get a reference to the stack slot `ss` using one of the base pointers in `mask`.
    pub fn masked(ss: StackSlot, mask: StackBaseMask, frame: &StackSlots) -> Option<StackRef> {
        // Try an SP-relative reference.
        if mask.contains(StackBase::SP) {
            return Some(StackRef::sp(ss, frame));
        }

        // No reference possible with this mask.
        None
    }

    /// Get a reference to `ss` using the stack pointer as a base.
    pub fn sp(ss: StackSlot, frame: &StackSlots) -> StackRef {
        let size = frame.frame_size.expect(
            "Stack layout must be computed before referencing stack slots",
        );

        // Offset where SP is pointing. (All ISAs have stacks growing downwards.)
        let sp_offset = -(size as StackOffset);
        return StackRef {
            base: StackBase::SP,
            offset: frame[ss].offset - sp_offset,
        };
    }
}

/// Generic base register for referencing stack slots.
///
/// Most ISAs have a stack pointer and an optional frame pointer, so provide generic names for
/// those two base pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackBase {
    /// Use the stack pointer.
    SP = 0,

    /// Use the frame pointer (if one is present).
    FP = 1,

    /// Use an explicit zone pointer in a general-purpose register.
    Zone = 2,
}

/// Bit mask of supported stack bases.
///
/// Many instruction encodings can use different base registers while others only work with the
/// stack pointer, say. A `StackBaseMask` is a bit mask of supported stack bases for a given
/// instruction encoding.
///
/// This behaves like a set of `StackBase` variants.
///
/// The internal representation as a `u8` is public because stack base masks are used in constant
/// tables generated from the Python encoding definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackBaseMask(pub u8);

impl StackBaseMask {
    /// Check if this mask contains the `base` variant.
    pub fn contains(self, base: StackBase) -> bool {
        self.0 & (1 << base as usize) != 0
    }
}
