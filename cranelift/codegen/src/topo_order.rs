//! Topological order of blocks, according to the dominator tree.

use crate::dominator_tree::DominatorTree;
use crate::entity::EntitySet;
use crate::ir::{Block, Layout};
use alloc::vec::Vec;

/// Present blocks in a topological order such that all dominating blocks are guaranteed to be visited
/// before the current block.
///
/// There are many topological orders of the blocks in a function, so it is possible to provide a
/// preferred order, and the `TopoOrder` will present blocks in an order that is as close as possible
/// to the preferred order.
pub struct TopoOrder {
    /// Preferred order of blocks to visit.
    preferred: Vec<Block>,

    /// Next entry to get from `preferred`.
    next: usize,

    /// Set of visited blocks.
    visited: EntitySet<Block>,

    /// Stack of blocks to be visited next, already in `visited`.
    stack: Vec<Block>,
}

impl TopoOrder {
    /// Create a new empty topological order.
    pub fn new() -> Self {
        Self {
            preferred: Vec::new(),
            next: 0,
            visited: EntitySet::new(),
            stack: Vec::new(),
        }
    }

    /// Clear all data structures in this topological order.
    pub fn clear(&mut self) {
        self.preferred.clear();
        self.next = 0;
        self.visited.clear();
        self.stack.clear();
    }

    /// Reset and initialize with a preferred sequence of blocks. The resulting topological order is
    /// guaranteed to contain all of the blocks in `preferred` as well as any dominators.
    pub fn reset<Blocks>(&mut self, preferred: Blocks)
    where
        Blocks: IntoIterator<Item = Block>,
    {
        self.preferred.clear();
        self.preferred.extend(preferred);
        self.next = 0;
        self.visited.clear();
        self.stack.clear();
    }

    /// Get the next block in the topological order.
    ///
    /// Two things are guaranteed about the blocks returned by this function:
    ///
    /// - All blocks in the `preferred` iterator given to `reset` will be returned.
    /// - All dominators are visited before the block returned.
    pub fn next(&mut self, layout: &Layout, domtree: &DominatorTree) -> Option<Block> {
        self.visited.resize(layout.block_capacity());
        // Any entries in `stack` should be returned immediately. They have already been added to
        // `visited`.
        while self.stack.is_empty() {
            match self.preferred.get(self.next).cloned() {
                None => return None,
                Some(mut block) => {
                    // We have the next block in the preferred order.
                    self.next += 1;
                    // Push it along with any non-visited dominators.
                    while self.visited.insert(block) {
                        self.stack.push(block);
                        match domtree.idom(block) {
                            Some(idom) => {
                                block = layout.inst_block(idom).expect("idom not in layout")
                            }
                            None => break,
                        }
                    }
                }
            }
        }
        self.stack.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::dominator_tree::DominatorTree;
    use crate::flowgraph::ControlFlowGraph;
    use crate::ir::{Function, InstBuilder};
    use core::iter;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let mut topo = TopoOrder::new();

        assert_eq!(topo.next(&func.layout, &domtree), None);
        topo.reset(func.layout.blocks());
        assert_eq!(topo.next(&func.layout, &domtree), None);
    }

    #[test]
    fn simple() {
        let mut func = Function::new();
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();

        {
            let mut cur = FuncCursor::new(&mut func);

            cur.insert_block(block0);
            cur.ins().jump(block1, &[]);
            cur.insert_block(block1);
            cur.ins().jump(block1, &[]);
        }

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let mut topo = TopoOrder::new();

        topo.reset(iter::once(block1));
        assert_eq!(topo.next(&func.layout, &domtree), Some(block0));
        assert_eq!(topo.next(&func.layout, &domtree), Some(block1));
        assert_eq!(topo.next(&func.layout, &domtree), None);
    }
}
