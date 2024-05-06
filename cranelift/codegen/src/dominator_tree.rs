//! A Dominator Tree represented as mappings of Blocks to their immediate dominator.

use crate::entity::SecondaryMap;
use crate::flowgraph::{BlockPredecessor, ControlFlowGraph};
use crate::ir::{Block, Function, Inst, Layout, ProgramPoint};
use crate::packed_option::PackedOption;
use crate::timing;
use alloc::vec::Vec;
use core::cmp;
use core::cmp::Ordering;
use core::mem;

/// RPO numbers are not first assigned in a contiguous way but as multiples of STRIDE, to leave
/// room for modifications of the dominator tree.
const STRIDE: u32 = 4;

/// Special RPO numbers used during `compute_postorder`.
const SEEN: u32 = 1;

/// Dominator tree node. We keep one of these per block.
#[derive(Clone, Default)]
struct DomNode {
    /// Number of this node in a reverse post-order traversal of the CFG, starting from 1.
    /// This number is monotonic in the reverse postorder but not contiguous, since we leave
    /// holes for later localized modifications of the dominator tree.
    /// Unreachable nodes get number 0, all others are positive.
    rpo_number: u32,

    /// The immediate dominator of this block, represented as the branch or jump instruction at the
    /// end of the dominating basic block.
    ///
    /// This is `None` for unreachable blocks and the entry block which doesn't have an immediate
    /// dominator.
    idom: PackedOption<Inst>,
}

/// DFT stack state marker for computing the cfg postorder.
enum Visit {
    First,
    Last,
}

/// The dominator tree for a single function.
pub struct DominatorTree {
    nodes: SecondaryMap<Block, DomNode>,

    /// CFG post-order of all reachable blocks.
    postorder: Vec<Block>,

    /// Scratch memory used by `compute_postorder()`.
    stack: Vec<(Visit, Block)>,

    valid: bool,
}

/// Methods for querying the dominator tree.
impl DominatorTree {
    /// Is `block` reachable from the entry block?
    pub fn is_reachable(&self, block: Block) -> bool {
        self.nodes[block].rpo_number != 0
    }

    /// Get the CFG post-order of blocks that was used to compute the dominator tree.
    ///
    /// Note that this post-order is not updated automatically when the CFG is modified. It is
    /// computed from scratch and cached by `compute()`.
    pub fn cfg_postorder(&self) -> &[Block] {
        debug_assert!(self.is_valid());
        &self.postorder
    }

    /// Returns the immediate dominator of `block`.
    ///
    /// The immediate dominator of a basic block is a basic block which we represent by
    /// the branch or jump instruction at the end of the basic block. This does not have to be the
    /// terminator of its block.
    ///
    /// A branch or jump is said to *dominate* `block` if all control flow paths from the function
    /// entry to `block` must go through the branch.
    ///
    /// The *immediate dominator* is the dominator that is closest to `block`. All other dominators
    /// also dominate the immediate dominator.
    ///
    /// This returns `None` if `block` is not reachable from the entry block, or if it is the entry block
    /// which has no dominators.
    pub fn idom(&self, block: Block) -> Option<Inst> {
        self.nodes[block].idom.into()
    }

    /// Compare two blocks relative to the reverse post-order.
    pub fn rpo_cmp_block(&self, a: Block, b: Block) -> Ordering {
        self.nodes[a].rpo_number.cmp(&self.nodes[b].rpo_number)
    }

    /// Compare two program points relative to a reverse post-order traversal of the control-flow
    /// graph.
    ///
    /// Return `Ordering::Less` if `a` comes before `b` in the RPO.
    ///
    /// If `a` and `b` belong to the same block, compare their relative position in the block.
    pub fn rpo_cmp<A, B>(&self, a: A, b: B, layout: &Layout) -> Ordering
    where
        A: Into<ProgramPoint>,
        B: Into<ProgramPoint>,
    {
        let a = a.into();
        let b = b.into();
        self.rpo_cmp_block(layout.pp_block(a), layout.pp_block(b))
            .then_with(|| layout.pp_cmp(a, b))
    }

    /// Returns `true` if `a` dominates `b`.
    ///
    /// This means that every control-flow path from the function entry to `b` must go through `a`.
    ///
    /// Dominance is ill defined for unreachable blocks. This function can always determine
    /// dominance for instructions in the same block, but otherwise returns `false` if either block
    /// is unreachable.
    ///
    /// An instruction is considered to dominate itself.
    pub fn dominates<A, B>(&self, a: A, b: B, layout: &Layout) -> bool
    where
        A: Into<ProgramPoint>,
        B: Into<ProgramPoint>,
    {
        let a = a.into();
        let b = b.into();
        match a {
            ProgramPoint::Block(block_a) => {
                a == b || self.last_dominator(block_a, b, layout).is_some()
            }
            ProgramPoint::Inst(inst_a) => {
                let block_a = layout
                    .inst_block(inst_a)
                    .expect("Instruction not in layout.");
                match self.last_dominator(block_a, b, layout) {
                    Some(last) => layout.pp_cmp(inst_a, last) != Ordering::Greater,
                    None => false,
                }
            }
        }
    }

    /// Find the last instruction in `a` that dominates `b`.
    /// If no instructions in `a` dominate `b`, return `None`.
    pub fn last_dominator<B>(&self, a: Block, b: B, layout: &Layout) -> Option<Inst>
    where
        B: Into<ProgramPoint>,
    {
        let (mut block_b, mut inst_b) = match b.into() {
            ProgramPoint::Block(block) => (block, None),
            ProgramPoint::Inst(inst) => (
                layout.inst_block(inst).expect("Instruction not in layout."),
                Some(inst),
            ),
        };
        let rpo_a = self.nodes[a].rpo_number;

        // Run a finger up the dominator tree from b until we see a.
        // Do nothing if b is unreachable.
        while rpo_a < self.nodes[block_b].rpo_number {
            let idom = match self.idom(block_b) {
                Some(idom) => idom,
                None => return None, // a is unreachable, so we climbed past the entry
            };
            block_b = layout.inst_block(idom).expect("Dominator got removed.");
            inst_b = Some(idom);
        }
        if a == block_b {
            inst_b
        } else {
            None
        }
    }

    /// Compute the common dominator of two basic blocks.
    ///
    /// Both basic blocks are assumed to be reachable.
    pub fn common_dominator(
        &self,
        mut a: BlockPredecessor,
        mut b: BlockPredecessor,
        layout: &Layout,
    ) -> BlockPredecessor {
        loop {
            match self.rpo_cmp_block(a.block, b.block) {
                Ordering::Less => {
                    // `a` comes before `b` in the RPO. Move `b` up.
                    let idom = self.nodes[b.block].idom.expect("Unreachable basic block?");
                    b = BlockPredecessor::new(
                        layout.inst_block(idom).expect("Dangling idom instruction"),
                        idom,
                    );
                }
                Ordering::Greater => {
                    // `b` comes before `a` in the RPO. Move `a` up.
                    let idom = self.nodes[a.block].idom.expect("Unreachable basic block?");
                    a = BlockPredecessor::new(
                        layout.inst_block(idom).expect("Dangling idom instruction"),
                        idom,
                    );
                }
                Ordering::Equal => break,
            }
        }

        debug_assert_eq!(
            a.block, b.block,
            "Unreachable block passed to common_dominator?"
        );

        // We're in the same block. The common dominator is the earlier instruction.
        if layout.pp_cmp(a.inst, b.inst) == Ordering::Less {
            a
        } else {
            b
        }
    }
}

impl DominatorTree {
    /// Allocate a new blank dominator tree. Use `compute` to compute the dominator tree for a
    /// function.
    pub fn new() -> Self {
        Self {
            nodes: SecondaryMap::new(),
            postorder: Vec::new(),
            stack: Vec::new(),
            valid: false,
        }
    }

    /// Allocate and compute a dominator tree.
    pub fn with_function(func: &Function, cfg: &ControlFlowGraph) -> Self {
        let block_capacity = func.layout.block_capacity();
        let mut domtree = Self {
            nodes: SecondaryMap::with_capacity(block_capacity),
            postorder: Vec::with_capacity(block_capacity),
            stack: Vec::new(),
            valid: false,
        };
        domtree.compute(func, cfg);
        domtree
    }

    /// Reset and compute a CFG post-order and dominator tree.
    pub fn compute(&mut self, func: &Function, cfg: &ControlFlowGraph) {
        let _tt = timing::domtree();
        debug_assert!(cfg.is_valid());
        self.compute_postorder(func);
        self.compute_domtree(func, cfg);
        self.valid = true;
    }

    /// Clear the data structures used to represent the dominator tree. This will leave the tree in
    /// a state where `is_valid()` returns false.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.postorder.clear();
        debug_assert!(self.stack.is_empty());
        self.valid = false;
    }

    /// Check if the dominator tree is in a valid state.
    ///
    /// Note that this doesn't perform any kind of validity checks. It simply checks if the
    /// `compute()` method has been called since the last `clear()`. It does not check that the
    /// dominator tree is consistent with the CFG.
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Reset all internal data structures and compute a post-order of the control flow graph.
    ///
    /// This leaves `rpo_number == 1` for all reachable blocks, 0 for unreachable ones.
    fn compute_postorder(&mut self, func: &Function) {
        self.clear();
        self.nodes.resize(func.dfg.num_blocks());

        // This algorithm is a depth first traversal (DFT) of the control flow graph, computing a
        // post-order of the blocks that are reachable form the entry block. A DFT post-order is not
        // unique. The specific order we get is controlled by the order each node's children are
        // visited.
        //
        // We view the CFG as a graph where each `BlockCall` value of a terminating branch
        // instruction is an edge. A consequence of this is that we visit successor nodes in the
        // reverse order specified by the branch instruction that terminates the basic block.
        // (Reversed because we are using a stack to control traversal, and push the successors in
        // the order the branch instruction specifies -- there's no good reason for this particular
        // order.)
        //
        // During this algorithm only, use `rpo_number` to hold the following state:
        //
        //   0:    block has not yet had its first visit
        //   SEEN: block has been visited at least once, implying that all of its successors are on
        //         the stack

        match func.layout.entry_block() {
            Some(block) => {
                self.stack.push((Visit::First, block));
            }
            None => return,
        }

        while let Some((visit, block)) = self.stack.pop() {
            match visit {
                Visit::First => {
                    if self.nodes[block].rpo_number == 0 {
                        // This is the first time we pop the block, so we need to scan its
                        // successors and then revisit it.
                        self.nodes[block].rpo_number = SEEN;
                        self.stack.push((Visit::Last, block));
                        if let Some(inst) = func.stencil.layout.last_inst(block) {
                            // Heuristic: chase the children in reverse. This puts the first
                            // successor block first in the postorder, all other things being
                            // equal, which tends to prioritize loop backedges over out-edges,
                            // putting the edge-block closer to the loop body and minimizing
                            // live-ranges in linear instruction space. This heuristic doesn't have
                            // any effect on the computation of dominators, and is purely for other
                            // consumers of the postorder we cache here.
                            for block in func.stencil.dfg.insts[inst]
                                .branch_destination(&func.stencil.dfg.jump_tables)
                                .iter()
                                .rev()
                            {
                                let succ = block.block(&func.stencil.dfg.value_lists);

                                // This is purely an optimization to avoid additional iterations of
                                // the loop, and is not required; it's merely inlining the check
                                // from the outer conditional of this case to avoid the extra loop
                                // iteration.
                                if self.nodes[succ].rpo_number == 0 {
                                    self.stack.push((Visit::First, succ))
                                }
                            }
                        }
                    }
                }

                Visit::Last => {
                    // We've finished all this node's successors.
                    self.postorder.push(block);
                }
            }
        }
    }

    /// Build a dominator tree from a control flow graph using Keith D. Cooper's
    /// "Simple, Fast Dominator Algorithm."
    fn compute_domtree(&mut self, func: &Function, cfg: &ControlFlowGraph) {
        // During this algorithm, `rpo_number` has the following values:
        //
        // 0: block is not reachable.
        // 1: block is reachable, but has not yet been visited during the first pass. This is set by
        // `compute_postorder`.
        // 2+: block is reachable and has an assigned RPO number.

        // We'll be iterating over a reverse post-order of the CFG, skipping the entry block.
        let (entry_block, postorder) = match self.postorder.as_slice().split_last() {
            Some((&eb, rest)) => (eb, rest),
            None => return,
        };
        debug_assert_eq!(Some(entry_block), func.layout.entry_block());

        // Do a first pass where we assign RPO numbers to all reachable nodes.
        self.nodes[entry_block].rpo_number = 2 * STRIDE;
        for (rpo_idx, &block) in postorder.iter().rev().enumerate() {
            // Update the current node and give it an RPO number.
            // The entry block got 2, the rest start at 3 by multiples of STRIDE to leave
            // room for future dominator tree modifications.
            //
            // Since `compute_idom` will only look at nodes with an assigned RPO number, the
            // function will never see an uninitialized predecessor.
            //
            // Due to the nature of the post-order traversal, every node we visit will have at
            // least one predecessor that has previously been visited during this RPO.
            self.nodes[block] = DomNode {
                idom: self.compute_idom(block, cfg, &func.layout).into(),
                rpo_number: (rpo_idx as u32 + 3) * STRIDE,
            }
        }

        // Now that we have RPO numbers for everything and initial immediate dominator estimates,
        // iterate until convergence.
        //
        // If the function is free of irreducible control flow, this will exit after one iteration.
        let mut changed = true;
        while changed {
            changed = false;
            for &block in postorder.iter().rev() {
                let idom = self.compute_idom(block, cfg, &func.layout).into();
                if self.nodes[block].idom != idom {
                    self.nodes[block].idom = idom;
                    changed = true;
                }
            }
        }
    }

    // Compute the immediate dominator for `block` using the current `idom` states for the reachable
    // nodes.
    fn compute_idom(&self, block: Block, cfg: &ControlFlowGraph, layout: &Layout) -> Inst {
        // Get an iterator with just the reachable, already visited predecessors to `block`.
        // Note that during the first pass, `rpo_number` is 1 for reachable blocks that haven't
        // been visited yet, 0 for unreachable blocks.
        let mut reachable_preds = cfg
            .pred_iter(block)
            .filter(|&BlockPredecessor { block: pred, .. }| self.nodes[pred].rpo_number > 1);

        // The RPO must visit at least one predecessor before this node.
        let mut idom = reachable_preds
            .next()
            .expect("block node must have one reachable predecessor");

        for pred in reachable_preds {
            idom = self.common_dominator(idom, pred, layout);
        }

        idom.inst
    }
}

/// Optional pre-order information that can be computed for a dominator tree.
///
/// This data structure is computed from a `DominatorTree` and provides:
///
/// - A forward traversable dominator tree through the `children()` iterator.
/// - An ordering of blocks according to a dominator tree pre-order.
/// - Constant time dominance checks at the block granularity.
///
/// The information in this auxiliary data structure is not easy to update when the control flow
/// graph changes, which is why it is kept separate.
pub struct DominatorTreePreorder {
    nodes: SecondaryMap<Block, ExtraNode>,

    // Scratch memory used by `compute_postorder()`.
    stack: Vec<Block>,
}

#[derive(Default, Clone)]
struct ExtraNode {
    /// First child node in the domtree.
    child: PackedOption<Block>,

    /// Next sibling node in the domtree. This linked list is ordered according to the CFG RPO.
    sibling: PackedOption<Block>,

    /// Sequence number for this node in a pre-order traversal of the dominator tree.
    /// Unreachable blocks have number 0, the entry block is 1.
    pre_number: u32,

    /// Maximum `pre_number` for the sub-tree of the dominator tree that is rooted at this node.
    /// This is always >= `pre_number`.
    pre_max: u32,
}

/// Creating and computing the dominator tree pre-order.
impl DominatorTreePreorder {
    /// Create a new blank `DominatorTreePreorder`.
    pub fn new() -> Self {
        Self {
            nodes: SecondaryMap::new(),
            stack: Vec::new(),
        }
    }

    /// Checks if one instruction dominates another.
    pub fn dominates_inst(&self, a: Inst, b: Inst, layout: &Layout) -> bool {
        match (layout.inst_block(a), layout.inst_block(b)) {
            (Some(block_a), Some(block_b)) => {
                if block_a == block_b {
                    layout.pp_cmp(a, b) != Ordering::Greater
                } else {
                    self.dominates(block_a, block_b)
                }
            }
            _ => false,
        }
    }

    /// Recompute this data structure to match `domtree`.
    pub fn compute(&mut self, domtree: &DominatorTree, layout: &Layout) {
        self.nodes.clear();
        debug_assert_eq!(self.stack.len(), 0);

        // Step 1: Populate the child and sibling links.
        //
        // By following the CFG post-order and pushing to the front of the lists, we make sure that
        // sibling lists are ordered according to the CFG reverse post-order.
        for &block in domtree.cfg_postorder() {
            if let Some(idom_inst) = domtree.idom(block) {
                let idom = layout
                    .inst_block(idom_inst)
                    .expect("Instruction not in layout.");
                let sib = mem::replace(&mut self.nodes[idom].child, block.into());
                self.nodes[block].sibling = sib;
            } else {
                // The only block without an immediate dominator is the entry.
                self.stack.push(block);
            }
        }

        // Step 2. Assign pre-order numbers from a DFS of the dominator tree.
        debug_assert!(self.stack.len() <= 1);
        let mut n = 0;
        while let Some(block) = self.stack.pop() {
            n += 1;
            let node = &mut self.nodes[block];
            node.pre_number = n;
            node.pre_max = n;
            if let Some(n) = node.sibling.expand() {
                self.stack.push(n);
            }
            if let Some(n) = node.child.expand() {
                self.stack.push(n);
            }
        }

        // Step 3. Propagate the `pre_max` numbers up the tree.
        // The CFG post-order is topologically ordered w.r.t. dominance so a node comes after all
        // its dominator tree children.
        for &block in domtree.cfg_postorder() {
            if let Some(idom_inst) = domtree.idom(block) {
                let idom = layout
                    .inst_block(idom_inst)
                    .expect("Instruction not in layout.");
                let pre_max = cmp::max(self.nodes[block].pre_max, self.nodes[idom].pre_max);
                self.nodes[idom].pre_max = pre_max;
            }
        }
    }
}

/// An iterator that enumerates the direct children of a block in the dominator tree.
pub struct ChildIter<'a> {
    dtpo: &'a DominatorTreePreorder,
    next: PackedOption<Block>,
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Block> {
        let n = self.next.expand();
        if let Some(block) = n {
            self.next = self.dtpo.nodes[block].sibling;
        }
        n
    }
}

/// Query interface for the dominator tree pre-order.
impl DominatorTreePreorder {
    /// Get an iterator over the direct children of `block` in the dominator tree.
    ///
    /// These are the block's whose immediate dominator is an instruction in `block`, ordered according
    /// to the CFG reverse post-order.
    pub fn children(&self, block: Block) -> ChildIter {
        ChildIter {
            dtpo: self,
            next: self.nodes[block].child,
        }
    }

    /// Fast, constant time dominance check with block granularity.
    ///
    /// This computes the same result as `domtree.dominates(a, b)`, but in guaranteed fast constant
    /// time. This is less general than the `DominatorTree` method because it only works with block
    /// program points.
    ///
    /// A block is considered to dominate itself.
    pub fn dominates(&self, a: Block, b: Block) -> bool {
        let na = &self.nodes[a];
        let nb = &self.nodes[b];
        na.pre_number <= nb.pre_number && na.pre_max >= nb.pre_max
    }

    /// Compare two blocks according to the dominator pre-order.
    pub fn pre_cmp_block(&self, a: Block, b: Block) -> Ordering {
        self.nodes[a].pre_number.cmp(&self.nodes[b].pre_number)
    }

    /// Compare two program points according to the dominator tree pre-order.
    ///
    /// This ordering of program points have the property that given a program point, pp, all the
    /// program points dominated by pp follow immediately and contiguously after pp in the order.
    pub fn pre_cmp<A, B>(&self, a: A, b: B, layout: &Layout) -> Ordering
    where
        A: Into<ProgramPoint>,
        B: Into<ProgramPoint>,
    {
        let a = a.into();
        let b = b.into();
        self.pre_cmp_block(layout.pp_block(a), layout.pp_block(b))
            .then_with(|| layout.pp_cmp(a, b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::types::*;
    use crate::ir::{InstBuilder, TrapCode};

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::with_function(&func);
        debug_assert!(cfg.is_valid());
        let dtree = DominatorTree::with_function(&func, &cfg);
        assert_eq!(0, dtree.nodes.keys().count());
        assert_eq!(dtree.cfg_postorder(), &[]);

        let mut dtpo = DominatorTreePreorder::new();
        dtpo.compute(&dtree, &func.layout);
    }

    #[test]
    fn unreachable_node() {
        let mut func = Function::new();
        let block0 = func.dfg.make_block();
        let v0 = func.dfg.append_block_param(block0, I32);
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();
        let trap_block = func.dfg.make_block();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_block(block0);
        cur.ins().brif(v0, block2, &[], trap_block, &[]);

        cur.insert_block(trap_block);
        cur.ins().trap(TrapCode::User(0));

        cur.insert_block(block1);
        let v1 = cur.ins().iconst(I32, 1);
        let v2 = cur.ins().iadd(v0, v1);
        cur.ins().jump(block0, &[v2]);

        cur.insert_block(block2);
        cur.ins().return_(&[v0]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let dt = DominatorTree::with_function(cur.func, &cfg);

        // Fall-through-first, prune-at-source DFT:
        //
        // block0 {
        //   brif block2 {
        //     trap
        //     block2 {
        //       return
        //     } block2
        // } block0
        assert_eq!(dt.cfg_postorder(), &[block2, trap_block, block0]);

        let v2_def = cur.func.dfg.value_def(v2).unwrap_inst();
        assert!(!dt.dominates(v2_def, block0, &cur.func.layout));
        assert!(!dt.dominates(block0, v2_def, &cur.func.layout));

        let mut dtpo = DominatorTreePreorder::new();
        dtpo.compute(&dt, &cur.func.layout);
        assert!(dtpo.dominates(block0, block0));
        assert!(!dtpo.dominates(block0, block1));
        assert!(dtpo.dominates(block0, block2));
        assert!(!dtpo.dominates(block1, block0));
        assert!(dtpo.dominates(block1, block1));
        assert!(!dtpo.dominates(block1, block2));
        assert!(!dtpo.dominates(block2, block0));
        assert!(!dtpo.dominates(block2, block1));
        assert!(dtpo.dominates(block2, block2));
    }

    #[test]
    fn non_zero_entry_block() {
        let mut func = Function::new();
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();
        let block3 = func.dfg.make_block();
        let cond = func.dfg.append_block_param(block3, I32);

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_block(block3);
        let jmp_block3_block1 = cur.ins().jump(block1, &[]);

        cur.insert_block(block1);
        let br_block1_block0_block2 = cur.ins().brif(cond, block0, &[], block2, &[]);

        cur.insert_block(block2);
        cur.ins().jump(block0, &[]);

        cur.insert_block(block0);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let dt = DominatorTree::with_function(cur.func, &cfg);

        // Fall-through-first, prune-at-source DFT:
        //
        // block3 {
        //   block3:jump block1 {
        //     block1 {
        //       block1:brif block0 {
        //         block1:jump block2 {
        //           block2 {
        //             block2:jump block0 (seen)
        //           } block2
        //         } block1:jump block2
        //         block0 {
        //         } block0
        //       } block1:brif block0
        //     } block1
        //   } block3:jump block1
        // } block3

        assert_eq!(dt.cfg_postorder(), &[block0, block2, block1, block3]);

        assert_eq!(cur.func.layout.entry_block().unwrap(), block3);
        assert_eq!(dt.idom(block3), None);
        assert_eq!(dt.idom(block1).unwrap(), jmp_block3_block1);
        assert_eq!(dt.idom(block2).unwrap(), br_block1_block0_block2);
        assert_eq!(dt.idom(block0).unwrap(), br_block1_block0_block2);

        assert!(dt.dominates(
            br_block1_block0_block2,
            br_block1_block0_block2,
            &cur.func.layout
        ));
        assert!(!dt.dominates(br_block1_block0_block2, jmp_block3_block1, &cur.func.layout));
        assert!(dt.dominates(jmp_block3_block1, br_block1_block0_block2, &cur.func.layout));

        assert_eq!(
            dt.rpo_cmp(block3, block3, &cur.func.layout),
            Ordering::Equal
        );
        assert_eq!(dt.rpo_cmp(block3, block1, &cur.func.layout), Ordering::Less);
        assert_eq!(
            dt.rpo_cmp(block3, jmp_block3_block1, &cur.func.layout),
            Ordering::Less
        );
        assert_eq!(
            dt.rpo_cmp(jmp_block3_block1, br_block1_block0_block2, &cur.func.layout),
            Ordering::Less
        );
    }

    #[test]
    fn backwards_layout() {
        let mut func = Function::new();
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_block(block0);
        let jmp02 = cur.ins().jump(block2, &[]);

        cur.insert_block(block1);
        let trap = cur.ins().trap(TrapCode::User(5));

        cur.insert_block(block2);
        let jmp21 = cur.ins().jump(block1, &[]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let dt = DominatorTree::with_function(cur.func, &cfg);

        assert_eq!(cur.func.layout.entry_block(), Some(block0));
        assert_eq!(dt.idom(block0), None);
        assert_eq!(dt.idom(block1), Some(jmp21));
        assert_eq!(dt.idom(block2), Some(jmp02));

        assert!(dt.dominates(block0, block0, &cur.func.layout));
        assert!(dt.dominates(block0, jmp02, &cur.func.layout));
        assert!(dt.dominates(block0, block1, &cur.func.layout));
        assert!(dt.dominates(block0, trap, &cur.func.layout));
        assert!(dt.dominates(block0, block2, &cur.func.layout));
        assert!(dt.dominates(block0, jmp21, &cur.func.layout));

        assert!(!dt.dominates(jmp02, block0, &cur.func.layout));
        assert!(dt.dominates(jmp02, jmp02, &cur.func.layout));
        assert!(dt.dominates(jmp02, block1, &cur.func.layout));
        assert!(dt.dominates(jmp02, trap, &cur.func.layout));
        assert!(dt.dominates(jmp02, block2, &cur.func.layout));
        assert!(dt.dominates(jmp02, jmp21, &cur.func.layout));

        assert!(!dt.dominates(block1, block0, &cur.func.layout));
        assert!(!dt.dominates(block1, jmp02, &cur.func.layout));
        assert!(dt.dominates(block1, block1, &cur.func.layout));
        assert!(dt.dominates(block1, trap, &cur.func.layout));
        assert!(!dt.dominates(block1, block2, &cur.func.layout));
        assert!(!dt.dominates(block1, jmp21, &cur.func.layout));

        assert!(!dt.dominates(trap, block0, &cur.func.layout));
        assert!(!dt.dominates(trap, jmp02, &cur.func.layout));
        assert!(!dt.dominates(trap, block1, &cur.func.layout));
        assert!(dt.dominates(trap, trap, &cur.func.layout));
        assert!(!dt.dominates(trap, block2, &cur.func.layout));
        assert!(!dt.dominates(trap, jmp21, &cur.func.layout));

        assert!(!dt.dominates(block2, block0, &cur.func.layout));
        assert!(!dt.dominates(block2, jmp02, &cur.func.layout));
        assert!(dt.dominates(block2, block1, &cur.func.layout));
        assert!(dt.dominates(block2, trap, &cur.func.layout));
        assert!(dt.dominates(block2, block2, &cur.func.layout));
        assert!(dt.dominates(block2, jmp21, &cur.func.layout));

        assert!(!dt.dominates(jmp21, block0, &cur.func.layout));
        assert!(!dt.dominates(jmp21, jmp02, &cur.func.layout));
        assert!(dt.dominates(jmp21, block1, &cur.func.layout));
        assert!(dt.dominates(jmp21, trap, &cur.func.layout));
        assert!(!dt.dominates(jmp21, block2, &cur.func.layout));
        assert!(dt.dominates(jmp21, jmp21, &cur.func.layout));
    }
}
