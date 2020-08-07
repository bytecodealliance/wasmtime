use crate::cursor::{Cursor, FuncCursor};
use crate::dominator_tree::DominatorTree;
use crate::inst_predicates::is_safepoint;
use crate::ir::{Function, InstBuilder};
use crate::isa::TargetIsa;
use crate::regalloc::live_value_tracker::LiveValueTracker;
use crate::regalloc::liveness::Liveness;
use alloc::vec::Vec;

fn insert_and_encode_safepoint<'f>(
    pos: &mut FuncCursor<'f>,
    tracker: &LiveValueTracker,
    isa: &dyn TargetIsa,
) {
    // Iterate through all live values, collect only the references.
    let live_ref_values = tracker
        .live()
        .iter()
        .filter(|live_value| pos.func.dfg.value_type(live_value.value).is_ref())
        .map(|live_val| live_val.value)
        .collect::<Vec<_>>();

    if !live_ref_values.is_empty() {
        pos.ins().safepoint(&live_ref_values);
        // Move cursor to the new safepoint instruction to encode it.
        if let Some(inst) = pos.prev_inst() {
            let ok = pos.func.update_encoding(inst, isa).is_ok();
            debug_assert!(ok);
        }
        // Restore cursor position.
        pos.next_inst();
    }
}

// The emit_stack_maps() function analyzes each instruction to retrieve the liveness of
// the defs and operands by traversing a function's blocks in layout order.
pub fn emit_stack_maps(
    func: &mut Function,
    domtree: &DominatorTree,
    liveness: &Liveness,
    tracker: &mut LiveValueTracker,
    isa: &dyn TargetIsa,
) {
    let mut curr = func.layout.entry_block();

    while let Some(block) = curr {
        tracker.block_top(block, &func.dfg, liveness, &func.layout, domtree);
        tracker.drop_dead_params();
        let mut pos = FuncCursor::new(func);

        // From the top of the block, step through the instructions.
        pos.goto_top(block);

        while let Some(inst) = pos.next_inst() {
            if is_safepoint(&pos.func, inst) {
                insert_and_encode_safepoint(&mut pos, tracker, isa);
            }

            // Process the instruction and get rid of dead values.
            tracker.process_inst(inst, &pos.func.dfg, liveness);
            tracker.drop_dead(inst);
        }
        curr = func.layout.next_block(block);
    }
}
