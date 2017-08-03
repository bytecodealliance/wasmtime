//! Computing stack layout.

use ir::StackSlots;
use ir::stackslot::{StackSize, StackOffset, StackSlotKind};
use result::CtonError;
use std::cmp::{min, max};

/// Compute the stack frame layout.
///
/// Determine the total size of this stack frame and assign offsets to all `Spill` and `Local`
/// stack slots.
///
/// The total frame size will be a multiple of `alignment` which must be a power of two.
///
/// Returns the total stack frame size which is also saved in `frame.frame_size`.
///
/// If the stack frame is too big, returns an `ImplLimitExceeded` error.
pub fn layout_stack(frame: &mut StackSlots, alignment: StackSize) -> Result<StackSize, CtonError> {
    // Each object and the whole stack frame must fit in 2 GB such that any relative offset within
    // the frame fits in a `StackOffset`.
    let max_size = StackOffset::max_value() as StackSize;
    assert!(alignment.is_power_of_two() && alignment <= max_size);

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

    for ss in frame.keys() {
        let slot = &frame[ss];

        if slot.size > max_size {
            return Err(CtonError::ImplLimitExceeded);
        }

        match slot.kind {
            StackSlotKind::IncomingArg => {
                incoming_min = min(incoming_min, slot.offset);
            }
            StackSlotKind::OutgoingArg => {
                let offset = slot.offset
                    .checked_add(slot.size as StackOffset)
                    .ok_or(CtonError::ImplLimitExceeded)?;
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
        for ss in frame.keys() {
            let slot = frame[ss].clone();

            // Pick out locals and spill slots with exact alignment `min_align`.
            match slot.kind {
                StackSlotKind::SpillSlot | StackSlotKind::Local => {
                    if slot.alignment(alignment) != min_align {
                        continue;
                    }
                }
                _ => continue,
            }

            offset = offset
                .checked_sub(slot.size as StackOffset)
                .ok_or(CtonError::ImplLimitExceeded)?;

            // Aligning the negative offset can never cause overflow. We're only clearing bits.
            offset &= -(min_align as StackOffset);
            frame.set_offset(ss, offset);
        }

        // Move on to the next higher alignment.
        min_align *= 2;
    }

    // Finally, make room for the outgoing arguments.
    offset = offset
        .checked_sub(outgoing_max)
        .ok_or(CtonError::ImplLimitExceeded)?;
    offset &= -(alignment as StackOffset);

    let frame_size = (offset as StackSize).wrapping_neg();
    frame.frame_size = Some(frame_size);
    Ok(frame_size)
}

#[cfg(test)]
mod tests {
    use ir::StackSlots;
    use ir::types;
    use super::layout_stack;

    #[test]
    fn layout() {
        let sss = &mut StackSlots::new();

        // An empty layout should have 0-sized stack frame.
        assert_eq!(layout_stack(sss, 1), Ok(0));
        assert_eq!(layout_stack(sss, 16), Ok(0));

        // Same for incoming arguments with non-negative offsets.
        let in0 = sss.make_incoming_arg(types::I64, 0);
        let in1 = sss.make_incoming_arg(types::I64, 8);

        assert_eq!(layout_stack(sss, 1), Ok(0));
        assert_eq!(layout_stack(sss, 16), Ok(0));
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);

        // Add some spill slots.
        let ss0 = sss.make_spill_slot(types::I64);
        let ss1 = sss.make_spill_slot(types::I32);

        assert_eq!(layout_stack(sss, 1), Ok(12));
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[ss0].offset, -8);
        assert_eq!(sss[ss1].offset, -12);

        assert_eq!(layout_stack(sss, 16), Ok(16));
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[ss0].offset, -16);
        assert_eq!(sss[ss1].offset, -4);

        // An incoming argument with negative offset counts towards the total frame size, but it
        // should still pack nicely with the spill slots.
        let in2 = sss.make_incoming_arg(types::I32, -4);

        assert_eq!(layout_stack(sss, 1), Ok(16));
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[in2].offset, -4);
        assert_eq!(sss[ss0].offset, -12);
        assert_eq!(sss[ss1].offset, -16);

        assert_eq!(layout_stack(sss, 16), Ok(16));
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[in2].offset, -4);
        assert_eq!(sss[ss0].offset, -16);
        assert_eq!(sss[ss1].offset, -8);

        // Finally, make sure there is room for the outgoing args.
        let out0 = sss.get_outgoing_arg(types::I32, 0);

        assert_eq!(layout_stack(sss, 1), Ok(20));
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[in2].offset, -4);
        assert_eq!(sss[ss0].offset, -12);
        assert_eq!(sss[ss1].offset, -16);
        assert_eq!(sss[out0].offset, 0);

        assert_eq!(layout_stack(sss, 16), Ok(32));
        assert_eq!(sss[in0].offset, 0);
        assert_eq!(sss[in1].offset, 8);
        assert_eq!(sss[in2].offset, -4);
        assert_eq!(sss[ss0].offset, -16);
        assert_eq!(sss[ss1].offset, -8);
        assert_eq!(sss[out0].offset, 0);
    }
}
