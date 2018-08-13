//! Unreachable code elimination.

use cursor::{Cursor, FuncCursor};
use dominator_tree::DominatorTree;
use flowgraph::ControlFlowGraph;
use ir;
use timing;

/// Eliminate unreachable code.
///
/// This pass deletes whole EBBs that can't be reached from the entry block. It does not delete
/// individual instructions whose results are unused.
///
/// The reachability analysis is performed by the dominator tree analysis.
pub fn eliminate_unreachable_code(
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    domtree: &DominatorTree,
) {
    let _tt = timing::unreachable_code();
    let mut pos = FuncCursor::new(func);
    while let Some(ebb) = pos.next_ebb() {
        if domtree.is_reachable(ebb) {
            continue;
        }

        debug!("Eliminating unreachable {}", ebb);
        // Move the cursor out of the way and make sure the next lop iteration goes to the right
        // EBB.
        pos.prev_ebb();

        // Remove all instructions from `ebb`.
        while let Some(inst) = pos.func.layout.first_inst(ebb) {
            debug!(" - {}", pos.func.dfg.display_inst(inst, None));
            pos.func.layout.remove_inst(inst);
        }

        // Once the EBB is completely empty, we can update the CFG which removes it from any
        // predecessor lists.
        cfg.recompute_ebb(pos.func, ebb);

        // Finally, remove the EBB from the layout.
        pos.func.layout.remove_ebb(ebb);
    }
}
