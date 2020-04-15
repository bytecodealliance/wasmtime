//! Computation of basic block order in emitted code.

use crate::machinst::*;
use regalloc::{BlockIx, Function};

/// Simple reverse postorder-based block order emission.
///
/// TODO: use a proper algorithm, such as the bottom-up straight-line-section
/// construction algorithm.
struct BlockRPO {
    visited: Vec<bool>,
    postorder: Vec<BlockIndex>,
    deferred_last: Option<BlockIndex>,
}

impl BlockRPO {
    fn new<I: VCodeInst>(vcode: &VCode<I>) -> BlockRPO {
        BlockRPO {
            visited: vec![false; vcode.num_blocks()],
            postorder: vec![],
            deferred_last: None,
        }
    }

    fn visit<I: VCodeInst>(&mut self, vcode: &VCode<I>, block: BlockIndex) {
        self.visited[block as usize] = true;
        for succ in vcode.succs(block) {
            if !self.visited[*succ as usize] {
                self.visit(vcode, *succ);
            }
        }

        for i in vcode.block_insns(BlockIx::new(block)) {
            if vcode.get_insn(i).is_epilogue_placeholder() {
                debug_assert!(self.deferred_last.is_none());
                self.deferred_last = Some(block);
                return;
            }
        }

        self.postorder.push(block);
    }

    fn rpo(self) -> Vec<BlockIndex> {
        let mut rpo = self.postorder;
        rpo.reverse();
        if let Some(block) = self.deferred_last {
            rpo.push(block);
        }
        rpo
    }
}

/// Compute the final block order.
pub fn compute_final_block_order<I: VCodeInst>(vcode: &VCode<I>) -> Vec<BlockIndex> {
    let mut rpo = BlockRPO::new(vcode);
    rpo.visit(vcode, vcode.entry());
    rpo.rpo()
}
