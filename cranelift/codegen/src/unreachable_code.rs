//! Unreachable code elimination.

use crate::cursor::{Cursor, FuncCursor};
use crate::dominator_tree::DominatorTree;
use crate::flowgraph::ControlFlowGraph;
use crate::ir;
use crate::timing;
use log::debug;

/// Eliminate unreachable code.
///
/// This pass deletes whole blocks that can't be reached from the entry block. It does not delete
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
    while let Some(block) = pos.next_block() {
        if domtree.is_reachable(block) {
            continue;
        }

        debug!("Eliminating unreachable {}", block);
        // Move the cursor out of the way and make sure the next lop iteration goes to the right
        // block.
        pos.prev_block();

        // Remove all instructions from `block`.
        while let Some(inst) = pos.func.layout.first_inst(block) {
            debug!(" - {}", pos.func.dfg.display_inst(inst, None));
            pos.func.layout.remove_inst(inst);
        }

        // Once the block is completely empty, we can update the CFG which removes it from any
        // predecessor lists.
        cfg.recompute_block(pos.func, block);

        // Finally, remove the block from the layout.
        pos.func.layout.remove_block(block);
    }
}
