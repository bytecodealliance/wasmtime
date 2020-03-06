//! Computing stack layout.

use crate::ir::stackslot::{StackOffset, StackSize, StackSlotKind};
use crate::ir::{StackLayoutInfo, StackSlots};
use crate::result::{CodegenError, CodegenResult};
use core::cmp::{max, min};

/// Compute the stack frame layout.
///
/// Determine the total size of this stack frame and assign offsets to all `Spill` and `Explicit`
/// stack slots.
///
/// The total frame size will be a multiple of `alignment` which must be a power of two, unless the
/// function doesn't perform any call.
///
/// Returns the total stack frame size which is also saved in `frame.frame_size`.
///
/// If the stack frame is too big, returns an `ImplLimitExceeded` error.
pub fn layout_stack(
    frame: &mut StackSlots,
    is_leaf: bool,
    alignment: StackSize,
) -> CodegenResult<StackSize> {
    // Each object and the whole stack frame must fit in 2 GB such that any relative offset within
    // the frame fits in a `StackOffset`.
    let max_size = StackOffset::max_value() as StackSize;
    debug_assert!(alignment.is_power_of_two() && alignment <= max_size);

    // We assume a stack that grows toward lower addresses as implemented by modern ISAs. The
    // stack layout from high to low addresses will be:
    //
    // 1. incoming arguments.
    // 2. spills + explicits + struct returns.
    // 3. outgoing arguments.
    //
    // The incoming arguments can have both positive and negative offsets. A negative offset
    // incoming arguments is usually the x86 return address pushed by the call instruction, but
    // it can also be fixed stack slots pushed by an externally generated prologue.
    //
    // Both incoming and outgoing argument slots have fixed offsets that are treated as
    // reserved zones by the layout algorithm.
    //
    // If a function only has incoming arguments and does not perform any calls, then it doesn't
    // require the stack to be aligned.

    let mut incoming_min = 0;
    let mut incoming_max = 0;
    let mut outgoing_max = 0;
    let mut min_align = alignment;
    let mut must_align = !is_leaf;

    for slot in frame.values() {
        if slot.size > max_size {
            return Err(CodegenError::ImplLimitExceeded);
        }

        match slot.kind {
            StackSlotKind::IncomingArg => {
                incoming_min = min(incoming_min, slot.offset.unwrap());
                incoming_max = max(incoming_max, slot.offset.unwrap() + slot.size as i32);
            }
            StackSlotKind::OutgoingArg => {
                let offset = slot
                    .offset
                    .unwrap()
                    .checked_add(slot.size as StackOffset)
                    .ok_or(CodegenError::ImplLimitExceeded)?;
                outgoing_max = max(outgoing_max, offset);
                must_align = true;
            }
            StackSlotKind::StructReturnSlot
            | StackSlotKind::SpillSlot
            | StackSlotKind::ExplicitSlot
            | StackSlotKind::EmergencySlot => {
                // Determine the smallest alignment of any explicit or spill slot.
                min_align = slot.alignment(min_align);
                must_align = true;
            }
        }
    }

    // Lay out spill slots, struct return slots, and explicit slots below the
    // incoming arguments. The offset is negative, growing downwards. Start with
    // the smallest alignments for better packing.
    let mut offset = incoming_min;
    debug_assert!(min_align.is_power_of_two());
    while min_align <= alignment {
        for slot in frame.values_mut() {
            // Pick out explicit and spill slots with exact alignment `min_align`.
            match slot.kind {
                StackSlotKind::SpillSlot
                | StackSlotKind::StructReturnSlot
                | StackSlotKind::ExplicitSlot
                | StackSlotKind::EmergencySlot => {
                    if slot.alignment(alignment) != min_align {
                        continue;
                    }
                }
                StackSlotKind::IncomingArg | StackSlotKind::OutgoingArg => continue,
            }

            offset = offset
                .checked_sub(slot.size as StackOffset)
                .ok_or(CodegenError::ImplLimitExceeded)?;

            // Aligning the negative offset can never cause overflow. We're only clearing bits.
            offset &= -(min_align as StackOffset);
            slot.offset = Some(offset);
        }

        // Move on to the next higher alignment.
        min_align *= 2;
    }

    // Finally, make room for the outgoing arguments.
    offset = offset
        .checked_sub(outgoing_max)
        .ok_or(CodegenError::ImplLimitExceeded)?;

    if must_align {
        offset &= -(alignment as StackOffset);
    }

    // Set the computed layout information for the frame
    let frame_size = (offset as StackSize).wrapping_neg();
    let inbound_args_size = incoming_max as u32;
    frame.layout_info = Some(StackLayoutInfo {
        frame_size,
        inbound_args_size,
    });

    Ok(frame_size)
}

#[cfg(test)]
mod tests {
    use super::layout_stack;
    use crate::ir::stackslot::StackOffset;
    use crate::ir::types;
    use crate::ir::{StackSlotData, StackSlotKind, StackSlots};
    use crate::result::CodegenError;

    #[test]
    fn layout() {
        let sss = &mut StackSlots::new();

        // For all these test cases, assume it will call.
        let is_leaf = false;

        // An empty layout should have 0-sized stack frame.
        assert_eq!(layout_stack(sss, is_leaf, 1), Ok(0));
        assert_eq!(layout_stack(sss, is_leaf, 16), Ok(0));

        // Same for incoming arguments with non-negative offsets.
        let in0 = sss.make_incoming_arg(types::I64, 0);
        let in1 = sss.make_incoming_arg(types::I64, 8);

        assert_eq!(layout_stack(sss, is_leaf, 1), Ok(0));
        assert_eq!(layout_stack(sss, is_leaf, 16), Ok(0));
        assert_eq!(sss[in0].offset, Some(0));
        assert_eq!(sss[in1].offset, Some(8));

        // Add some spill slots.
        let ss0 = sss.make_spill_slot(types::I64);
        let ss1 = sss.make_spill_slot(types::I32);

        assert_eq!(layout_stack(sss, is_leaf, 1), Ok(12));
        assert_eq!(sss[in0].offset, Some(0));
        assert_eq!(sss[in1].offset, Some(8));
        assert_eq!(sss[ss0].offset, Some(-8));
        assert_eq!(sss[ss1].offset, Some(-12));

        assert_eq!(layout_stack(sss, is_leaf, 16), Ok(16));
        assert_eq!(sss[in0].offset, Some(0));
        assert_eq!(sss[in1].offset, Some(8));
        assert_eq!(sss[ss0].offset, Some(-16));
        assert_eq!(sss[ss1].offset, Some(-4));

        // An incoming argument with negative offset counts towards the total frame size, but it
        // should still pack nicely with the spill slots.
        let in2 = sss.make_incoming_arg(types::I32, -4);

        assert_eq!(layout_stack(sss, is_leaf, 1), Ok(16));
        assert_eq!(sss[in0].offset, Some(0));
        assert_eq!(sss[in1].offset, Some(8));
        assert_eq!(sss[in2].offset, Some(-4));
        assert_eq!(sss[ss0].offset, Some(-12));
        assert_eq!(sss[ss1].offset, Some(-16));

        assert_eq!(layout_stack(sss, is_leaf, 16), Ok(16));
        assert_eq!(sss[in0].offset, Some(0));
        assert_eq!(sss[in1].offset, Some(8));
        assert_eq!(sss[in2].offset, Some(-4));
        assert_eq!(sss[ss0].offset, Some(-16));
        assert_eq!(sss[ss1].offset, Some(-8));

        // Finally, make sure there is room for the outgoing args.
        let out0 = sss.get_outgoing_arg(types::I32, 0);

        assert_eq!(layout_stack(sss, is_leaf, 1), Ok(20));
        assert_eq!(sss[in0].offset, Some(0));
        assert_eq!(sss[in1].offset, Some(8));
        assert_eq!(sss[in2].offset, Some(-4));
        assert_eq!(sss[ss0].offset, Some(-12));
        assert_eq!(sss[ss1].offset, Some(-16));
        assert_eq!(sss[out0].offset, Some(0));

        assert_eq!(layout_stack(sss, is_leaf, 16), Ok(32));
        assert_eq!(sss[in0].offset, Some(0));
        assert_eq!(sss[in1].offset, Some(8));
        assert_eq!(sss[in2].offset, Some(-4));
        assert_eq!(sss[ss0].offset, Some(-16));
        assert_eq!(sss[ss1].offset, Some(-8));
        assert_eq!(sss[out0].offset, Some(0));

        // Also test that an unsupported offset is rejected.
        sss.get_outgoing_arg(types::I8, StackOffset::max_value() - 1);
        assert_eq!(
            layout_stack(sss, is_leaf, 1),
            Err(CodegenError::ImplLimitExceeded)
        );
    }

    #[test]
    fn slot_kinds() {
        let sss = &mut StackSlots::new();

        // Add some slots of various kinds.
        let ss0 = sss.make_spill_slot(types::I32);
        let ss1 = sss.push(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            types::I32.bytes(),
        ));
        let ss2 = sss.get_emergency_slot(types::I32, &[]);

        assert_eq!(layout_stack(sss, true, 1), Ok(12));
        assert_eq!(sss[ss0].offset, Some(-4));
        assert_eq!(sss[ss1].offset, Some(-8));
        assert_eq!(sss[ss2].offset, Some(-12));
    }
}
