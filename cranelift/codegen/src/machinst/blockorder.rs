//! Computation of basic block order in emitted code.

use crate::machinst::*;

/// Simple reverse postorder-based block order emission.
///
/// TODO: use a proper algorithm, such as the bottom-up straight-line-section
/// construction algorithm.
struct BlockRPO {
    visited: Vec<bool>,
    postorder: Vec<BlockIndex>,
}

impl BlockRPO {
    fn new<I: VCodeInst>(vcode: &VCode<I>) -> BlockRPO {
        BlockRPO {
            visited: vec![false; vcode.num_blocks()],
            postorder: Vec::with_capacity(vcode.num_blocks()),
        }
    }

    fn visit<I: VCodeInst>(&mut self, vcode: &VCode<I>, block: BlockIndex) {
        self.visited[block as usize] = true;
        for succ in vcode.succs(block) {
            if !self.visited[*succ as usize] {
                self.visit(vcode, *succ);
            }
        }
        if Some(block) != vcode.fallthrough_return_block {
            self.postorder.push(block);
        }
    }

    fn rpo<I: VCodeInst>(self, vcode: &VCode<I>) -> Vec<BlockIndex> {
        let mut rpo = self.postorder;
        rpo.reverse();
        if let Some(block) = vcode.fallthrough_return_block {
            rpo.push(block);
        }
        rpo
    }
}

/// Compute the final block order.
pub fn compute_final_block_order<I: VCodeInst>(vcode: &VCode<I>) -> Vec<BlockIndex> {
    let mut rpo = BlockRPO::new(vcode);
    rpo.visit(vcode, vcode.entry());
    rpo.rpo(vcode)
}
