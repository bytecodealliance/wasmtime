//! A post-dominator tree for a single function.
//!
//! The *post-dominator tree* is the dual of the [`DominatorTree`]: it answers
//! whether every path from a block to a function exit (a `return`, `trap`,
//! etc.) must pass through some other block.
//!
//! It is computed by reusing the ordinary dominator-tree machinery on a
//! modified version of the control-flow graph:
//!
//! * Add a virtual *sink* node.
//!
//! * Every block whose terminator does not branch anywhere (`return`,
//!   `return_call`, `trap`, etc.) is given an edge to the virtual sink.
//!
//! * Reverse every edge in the graph, so `a -> b` becomes `b -> a`.
//!
//! * Compute the dominator tree of this reversed graph, rooted at the virtual
//!   sink.
//!
//! Note that we don't actually reify this modified version of the control-flow
//! graph, we instead use the `ReverseGraph` implementation of the
//! `DomTreeGraph` trait.

use crate::dominator_tree::{ChildIter, DomTreeGraph, DominatorTree};
use crate::flowgraph::{BlockPredecessor, ControlFlowGraph};
use crate::ir::{Block, Function, Layout, ProgramPoint};
use core::cmp::Ordering;

/// The reversed control-flow graph, augmented with a virtual sink above the
/// function's exit blocks. Computing a `DominatorTree` over this graph yields
/// the post-dominator tree.
struct ReverseGraph<'a> {
    func: &'a Function,
    cfg: &'a ControlFlowGraph,
}

impl DomTreeGraph for ReverseGraph<'_> {
    fn num_blocks(&self) -> usize {
        self.func.dfg.num_blocks()
    }

    fn roots(&self) -> impl Iterator<Item = Block> {
        // The roots of the post-dominator forest are the function's exit
        // blocks: those whose terminator branches nowhere (e.g. `return`,
        // `return_call`, `trap`, etc...). These are exactly the blocks with no
        // CFG successors, and they are precisely the blocks with an edge to the
        // virtual sink.
        self.func
            .layout
            .blocks()
            .filter(|&block| self.func.block_successors(block).next().is_none())
    }

    fn successors(&self, block: Block) -> impl Iterator<Item = Block> {
        // Edges are reversed: a successor in the reversed graph is a
        // predecessor in the CFG.
        self.cfg
            .pred_iter(block)
            .map(|pred: BlockPredecessor| pred.block)
    }

    fn predecessors(&self, block: Block) -> impl Iterator<Item = Block> {
        // Edges are reversed: a predecessor in the reversed graph is a
        // successor in the CFG.
        self.func.block_successors(block)
    }
}

/// The post-dominator tree for a single function.
pub struct PostDominatorTree {
    /// The dominator tree of the reversed CFG. Its "dominates" relation is
    /// post-domination in the original function.
    dom_tree: DominatorTree,
}

impl Default for PostDominatorTree {
    fn default() -> Self {
        Self::new()
    }
}

impl PostDominatorTree {
    /// Allocate a new blank post-dominator tree.
    ///
    /// Use `compute` to compute the post-dominator tree for a function.
    pub fn new() -> Self {
        Self {
            dom_tree: DominatorTree::new(),
        }
    }

    /// Allocate and compute a post-dominator tree.
    pub fn with_function(func: &Function, cfg: &ControlFlowGraph) -> Self {
        let mut post_domtree = Self::new();
        post_domtree.compute(func, cfg);
        post_domtree
    }

    /// Reset and compute the post-dominator tree for `func`, using the
    /// control-flow graph `cfg`.
    pub fn compute(&mut self, func: &Function, cfg: &ControlFlowGraph) {
        debug_assert!(cfg.is_valid());
        self.dom_tree
            .compute_from_graph(&ReverseGraph { func, cfg });
    }

    /// Clear the data structures used to represent the post-dominator
    /// tree.
    ///
    /// This will leave the tree in a state where `is_valid()` returns `false`.
    pub fn clear(&mut self) {
        self.dom_tree.clear();
    }

    /// Check if the post-dominator tree is in a valid state.
    ///
    /// Note that this doesn't perform any kind of validity checks. It simply
    /// checks if the `compute()` method has been called since the last
    /// `clear()`. It does not check that the post-dominator tree is consistent
    /// with the CFG.
    pub fn is_valid(&self) -> bool {
        self.dom_tree.is_valid()
    }

    /// Returns the immediate post-dominator of `block`.
    ///
    /// `block_a` is said to *post-dominate* `block_b` if all control-flow paths
    /// from `block_b` out of this function (via return or trap) must go through
    /// `block_a`.
    ///
    /// The *immediate post-dominator* is the post-dominator that is closest to
    /// `block`. All other post-dominators also post-dominate the immediate
    /// post-dominator.
    ///
    /// This returns `None` if `block` diverges and cannot exit the function, or
    /// if `block` directly exits the function (returns or traps).
    pub fn immediate_post_dominator(&self, block: Block) -> Option<Block> {
        self.dom_tree.idom(block)
    }

    /// Returns `true` if every path from `b` out of this function (via return
    /// or trap) must go through `a`.
    pub fn post_dominates<A, B>(&self, a: A, b: B, layout: &Layout) -> bool
    where
        A: Into<ProgramPoint>,
        B: Into<ProgramPoint>,
    {
        let a = a.into();
        let b = b.into();
        match a {
            ProgramPoint::Block(block_a) => match b {
                ProgramPoint::Block(block_b) => self.block_post_dominates(block_a, block_b),
                ProgramPoint::Inst(inst_b) => {
                    let block_b = layout
                        .inst_block(inst_b)
                        .expect("instruction not in layout");
                    // A block header does not post-dominate a later instruction
                    // in its own block, but a header does post-dominate
                    // instructions in blocks that it strictly post-dominates.
                    block_a != block_b && self.block_post_dominates(block_a, block_b)
                }
            },
            ProgramPoint::Inst(inst_a) => {
                let block_a: Block = layout
                    .inst_block(inst_a)
                    .expect("Instruction not in layout.");
                match b {
                    ProgramPoint::Block(block_b) => {
                        // An instruction post-dominates the header of its own
                        // block: control reaches the instruction after the
                        // header.
                        self.block_post_dominates(block_a, block_b)
                    }
                    ProgramPoint::Inst(inst_b) => {
                        let block_b = layout
                            .inst_block(inst_b)
                            .expect("instruction not in layout");
                        if block_a == block_b {
                            // Within a block, `a` post-dominates `b` iff `a` is
                            // at or after `b`.
                            layout.pp_cmp(a, b) != Ordering::Less
                        } else {
                            self.block_post_dominates(block_a, block_b)
                        }
                    }
                }
            }
        }
    }

    /// Returns `true` if every path from `b` to a function exit (return or
    /// trap) must go through `a`.
    pub fn block_post_dominates(&self, block_a: Block, block_b: Block) -> bool {
        self.dom_tree.block_dominates(block_a, block_b)
    }

    /// Get an iterator over the direct children of `block` in the
    /// post-dominator tree.
    ///
    /// These are the blocks whose immediate post-dominator is `block`.
    pub fn children(&self, block: Block) -> ChildIter<'_> {
        self.dom_tree.children(block)
    }

    /// Is function exit (via return or trap) unreachable from the given block?
    pub fn diverges(&self, block: Block) -> bool {
        // A block is reachable in the reversed graph iff it can reach a
        // function exit; if it cannot, then function exit diverges away from
        // it.
        !self.dom_tree.is_reachable(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::types::*;
    use crate::ir::{InstBuilder, TrapCode};
    use alloc::string::String;
    use alloc::vec::Vec;
    use mutatis::{Mutate, check::Check, mutators as m};

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::with_function(&func);
        let pdt = PostDominatorTree::with_function(&func, &cfg);
        assert!(pdt.is_valid());
    }

    #[test]
    fn lifecycle() {
        let mut func = Function::new();
        let block0 = func.dfg.make_block();
        let mut cur = FuncCursor::new(&mut func);
        cur.insert_block(block0);
        cur.ins().return_(&[]);
        let cfg = ControlFlowGraph::with_function(cur.func);

        let mut pdt = PostDominatorTree::new();
        assert!(!pdt.is_valid());
        pdt.compute(cur.func, &cfg);
        assert!(pdt.is_valid());
        pdt.clear();
        assert!(!pdt.is_valid());
        // Recompute after clear.
        pdt.compute(cur.func, &cfg);
        assert!(pdt.is_valid());
    }

    #[test]
    fn straight_line() {
        let mut func = Function::new();
        let block0 = func.dfg.make_block();
        let mut cur = FuncCursor::new(&mut func);
        cur.insert_block(block0);
        let v0 = cur.ins().iconst(I32, 1);
        let v1 = cur.ins().iadd(v0, v0);
        cur.ins().return_(&[]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let pdt = PostDominatorTree::with_function(cur.func, &cfg);

        // The single block is an exit, so it has no post-dominator and does not
        // diverge.
        assert_eq!(pdt.immediate_post_dominator(block0), None);
        assert!(pdt.block_post_dominates(block0, block0));
        assert!(!pdt.diverges(block0));

        // Instruction-level: a later instruction post-dominates an earlier one.
        let v0_def = cur.func.dfg.value_def(v0).unwrap_inst();
        let v1_def = cur.func.dfg.value_def(v1).unwrap_inst();
        assert!(pdt.post_dominates(v1_def, v0_def, &cur.func.layout));
        assert!(!pdt.post_dominates(v0_def, v1_def, &cur.func.layout));
        assert!(pdt.post_dominates(v0_def, v0_def, &cur.func.layout));
    }

    #[test]
    fn if_else_diamond() {
        let mut func = Function::new();
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();
        let join = func.dfg.make_block();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_block(block0);
        let v0 = cur.ins().iconst(I32, 0);
        cur.ins().brif(v0, block1, &[], block2, &[]);

        cur.insert_block(block1);
        cur.ins().jump(join, &[]);

        cur.insert_block(block2);
        cur.ins().jump(join, &[]);

        cur.insert_block(join);
        cur.ins().return_(&[]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let pdt = PostDominatorTree::with_function(cur.func, &cfg);

        // Every path out of the function passes through `join`.
        assert_eq!(pdt.immediate_post_dominator(block0), Some(join));
        assert_eq!(pdt.immediate_post_dominator(block1), Some(join));
        assert_eq!(pdt.immediate_post_dominator(block2), Some(join));
        assert_eq!(pdt.immediate_post_dominator(join), None);

        assert!(pdt.block_post_dominates(join, block0));
        assert!(pdt.block_post_dominates(join, block1));
        // An arm does not post-dominate the entry (the other arm avoids it).
        assert!(!pdt.block_post_dominates(block1, block0));
        assert!(!pdt.block_post_dominates(block2, block0));
        // The entry does not post-dominate the join.
        assert!(!pdt.block_post_dominates(block0, join));

        for block in [block0, block1, block2, join] {
            assert!(!pdt.diverges(block));
        }

        // Cross-block `post_dominates` with instruction/block endpoints.
        let layout = &cur.func.layout;
        let entry_term = layout.last_inst(block0).unwrap();
        let join_term = layout.last_inst(join).unwrap();
        // The join's terminator post-dominates the entry's terminator...
        assert!(pdt.post_dominates(join_term, entry_term, layout));
        // ...but not vice versa.
        assert!(!pdt.post_dominates(entry_term, join_term, layout));
        // Block/instruction mixes across blocks defer to block post-domination.
        assert!(pdt.post_dominates(join, entry_term, layout));
        assert!(!pdt.post_dominates(entry_term, join, layout));

        // `join` post-dominates the three other blocks.
        let mut kids = pdt.children(join).collect::<alloc::vec::Vec<_>>();
        kids.sort();
        assert_eq!(kids, [block0, block1, block2]);
    }

    #[test]
    fn terminating_loop() {
        let mut func = Function::new();
        let entry = func.dfg.make_block();
        let header = func.dfg.make_block();
        let body = func.dfg.make_block();
        let exit = func.dfg.make_block();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_block(entry);
        cur.ins().jump(header, &[]);
        cur.insert_block(header);
        let v0 = cur.ins().iconst(I32, 0);
        cur.ins().brif(v0, body, &[], exit, &[]);

        cur.insert_block(body);
        cur.ins().jump(header, &[]);
        cur.insert_block(exit);
        cur.ins().return_(&[]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let pdt = PostDominatorTree::with_function(cur.func, &cfg);

        assert_eq!(pdt.immediate_post_dominator(entry), Some(header));
        assert_eq!(pdt.immediate_post_dominator(header), Some(exit));
        assert_eq!(pdt.immediate_post_dominator(body), Some(header));
        assert_eq!(pdt.immediate_post_dominator(exit), None);

        assert!(pdt.block_post_dominates(exit, entry));
        assert!(pdt.block_post_dominates(exit, body));
        assert!(pdt.block_post_dominates(header, body));
        assert!(!pdt.block_post_dominates(body, header));

        // The loop can always exit, so nothing diverges.
        for block in [entry, header, body, exit] {
            assert!(!pdt.diverges(block));
        }
    }

    #[test]
    fn infinite_loop() {
        let mut func = Function::new();
        let block0 = func.dfg.make_block();
        let mut cur = FuncCursor::new(&mut func);
        cur.insert_block(block0);
        cur.ins().jump(block0, &[]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let pdt = PostDominatorTree::with_function(cur.func, &cfg);

        // There is no exit block, so the function never returns: every block
        // diverges and has no post-dominator.
        assert!(pdt.is_valid());
        assert!(pdt.diverges(block0));
        assert_eq!(pdt.immediate_post_dominator(block0), None);
    }

    #[test]
    fn infinite_loop_with_side_exit() {
        let mut func = Function::new();
        let entry = func.dfg.make_block();
        let header = func.dfg.make_block();
        let body = func.dfg.make_block();
        let exit = func.dfg.make_block();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_block(entry);
        cur.ins().jump(header, &[]);

        cur.insert_block(header);
        let v0 = cur.ins().iconst(I32, 0);
        cur.ins().brif(v0, exit, &[], body, &[]);

        // `body` loops forever and never reaches an exit.
        cur.insert_block(body);
        cur.ins().jump(body, &[]);

        cur.insert_block(exit);
        cur.ins().return_(&[]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let pdt = PostDominatorTree::with_function(cur.func, &cfg);

        // Only `body` diverges.
        assert!(pdt.diverges(body));
        assert_eq!(pdt.immediate_post_dominator(body), None);

        assert!(!pdt.diverges(entry));
        assert!(!pdt.diverges(header));
        assert!(!pdt.diverges(exit));
        assert_eq!(pdt.immediate_post_dominator(header), Some(exit));
        assert_eq!(pdt.immediate_post_dominator(entry), Some(header));
        assert_eq!(pdt.immediate_post_dominator(exit), None);
    }

    #[test]
    fn multiple_returns() {
        let mut func = Function::new();
        let entry = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_block(entry);
        let v0 = cur.ins().iconst(I32, 0);
        cur.ins().brif(v0, block1, &[], block2, &[]);

        cur.insert_block(block1);
        cur.ins().return_(&[]);

        cur.insert_block(block2);
        cur.ins().return_(&[]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let pdt = PostDominatorTree::with_function(cur.func, &cfg);

        // Two distinct exit blocks: neither post-dominates the entry, and the
        // entry's only post-dominator is the (virtual) sink.
        assert_eq!(pdt.immediate_post_dominator(block1), None);
        assert_eq!(pdt.immediate_post_dominator(block2), None);
        assert_eq!(pdt.immediate_post_dominator(entry), None);

        assert!(!pdt.block_post_dominates(block1, entry));
        assert!(!pdt.block_post_dominates(block2, entry));

        // Blocks in distinct exit subtrees do not post-dominate each other.
        assert!(!pdt.block_post_dominates(block1, block2));
        assert!(!pdt.block_post_dominates(block2, block1));

        for block in [entry, block1, block2] {
            assert!(!pdt.diverges(block));
        }
    }

    #[test]
    fn trap_as_exit() {
        let mut func = Function::new();
        let entry = func.dfg.make_block();
        let ret_block = func.dfg.make_block();
        let trap_block = func.dfg.make_block();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_block(entry);
        let v0 = cur.ins().iconst(I32, 0);
        cur.ins().brif(v0, ret_block, &[], trap_block, &[]);

        cur.insert_block(ret_block);
        cur.ins().return_(&[]);

        cur.insert_block(trap_block);
        cur.ins().trap(TrapCode::unwrap_user(1));

        let cfg = ControlFlowGraph::with_function(cur.func);
        let pdt = PostDominatorTree::with_function(cur.func, &cfg);

        // A `trap` is a function exit, so `trap_block` is a root of the forest
        // and does not diverge.
        assert_eq!(pdt.immediate_post_dominator(trap_block), None);
        assert_eq!(pdt.immediate_post_dominator(ret_block), None);
        assert_eq!(pdt.immediate_post_dominator(entry), None);

        assert!(!pdt.diverges(trap_block));
        assert!(!pdt.diverges(ret_block));
        assert!(!pdt.diverges(entry));
    }

    #[test]
    fn insts_post_dominate_same_block() {
        let mut func = Function::new();
        let block0 = func.dfg.make_block();

        let mut cur = FuncCursor::new(&mut func);
        cur.insert_block(block0);
        let v1 = cur.ins().iconst(I32, 1);
        let v2 = cur.ins().iadd(v1, v1);
        let v3 = cur.ins().iadd(v2, v2);
        cur.ins().return_(&[]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let pdt = PostDominatorTree::with_function(cur.func, &cfg);

        let v1_def = cur.func.dfg.value_def(v1).unwrap_inst();
        let v2_def = cur.func.dfg.value_def(v2).unwrap_inst();
        let v3_def = cur.func.dfg.value_def(v3).unwrap_inst();
        let layout = &cur.func.layout;

        // Later instructions post-dominate earlier ones.
        assert!(pdt.post_dominates(v2_def, v1_def, layout));
        assert!(pdt.post_dominates(v3_def, v1_def, layout));
        assert!(pdt.post_dominates(v3_def, v2_def, layout));

        // Earlier instructions do not post-dominate later ones.
        assert!(!pdt.post_dominates(v1_def, v2_def, layout));
        assert!(!pdt.post_dominates(v1_def, v3_def, layout));

        // An instruction post-dominates itself.
        assert!(pdt.post_dominates(v2_def, v2_def, layout));

        // An instruction post-dominates the header of its own block...
        assert!(pdt.post_dominates(v1_def, block0, layout));
        // ...but a block header does not post-dominate a later instruction in
        // its own block.
        assert!(!pdt.post_dominates(block0, v1_def, layout));

        // A block post-dominates itself.
        assert!(pdt.post_dominates(block0, block0, layout));
    }

    /// Property-based test against a brute-force oracle.
    ///
    /// We mutate a small abstract control-flow graph with `mutatis`, build a
    /// corresponding Cranelift function, and compare the `PostDominatorTree`
    /// against an independent post-dominance dataflow computed on the abstract
    /// graph.
    #[test]
    fn post_dominators_match_oracle() -> mutatis::check::CheckResult<GraphSpec> {
        use Terminator::*;

        let corpus = [
            // Straight-line: a single returning block.
            GraphSpec {
                blocks: alloc::vec![Return],
            },
            // Straight-line: chained jumps and a return.
            GraphSpec {
                blocks: alloc::vec![Jump(1), Jump(2), Return],
            },
            // If-else diamond.
            GraphSpec {
                blocks: alloc::vec![Brif(1, 2), Jump(3), Jump(3), Return],
            },
            // Terminating loop.
            GraphSpec {
                blocks: alloc::vec![Jump(1), Brif(1, 2), Return],
            },
            // Infinite loop.
            GraphSpec {
                blocks: alloc::vec![Jump(1), Jump(0)],
            },
        ];

        Check::new()
            .iters(10_000)
            .run_with(m::default::<GraphSpec>(), corpus, check_post_dominance)
    }

    /// Cap on the number of blocks we build, so node indices (plus the virtual
    /// sink) fit in a `u64` bitmask.
    const MAX_BLOCKS: usize = 12;

    /// Description of a whole control-flow graph.
    #[derive(Clone, Debug, Default, Mutate)]
    struct GraphSpec {
        blocks: Vec<Terminator>,
    }

    impl GraphSpec {
        fn fixup(&self) -> Option<Self> {
            let n = self.blocks.len().min(MAX_BLOCKS);
            if n == 0 {
                return None;
            }
            let mut graph = GraphSpec {
                blocks: self.blocks[..n].to_vec(),
            };
            for terminator in &mut graph.blocks {
                terminator.fixup(n);
            }
            Some(graph)
        }

        /// Build a Cranelift function realizing `terminators`. Block 0 is the entry.
        fn build(&self) -> (Function, Vec<Block>) {
            let mut func = Function::new();

            let blocks: Vec<Block> = (0..self.blocks.len())
                .map(|_| func.dfg.make_block())
                .collect();

            let mut cur = FuncCursor::new(&mut func);
            for (i, terminator) in self.blocks.iter().enumerate() {
                cur.insert_block(blocks[i]);
                match *terminator {
                    Terminator::Return => {
                        cur.ins().return_(&[]);
                    }
                    Terminator::Jump(t) => {
                        cur.ins().jump(blocks[t], &[]);
                    }
                    Terminator::Brif(t1, t2) => {
                        let c = cur.ins().iconst(I32, 0);
                        cur.ins().brif(c, blocks[t1], &[], blocks[t2], &[]);
                    }
                }
            }
            (func, blocks)
        }
    }

    /// A block's terminator.
    #[derive(Clone, Copy, Debug, Default, Mutate)]
    enum Terminator {
        #[default]
        Return,
        Jump(usize),
        Brif(usize, usize),
    }

    impl Terminator {
        fn fixup(&mut self, n: usize) {
            match self {
                Self::Return => {}
                Self::Jump(a) => {
                    *a %= n;
                }
                Self::Brif(a, b) => {
                    *a %= n;
                    *b %= n;
                }
            }
        }
    }

    /// Check that the `PostDominatorTree` agrees with a brute-force
    /// post-dominance dataflow on the abstract graph.
    fn check_post_dominance(graph: &GraphSpec) -> Result<(), String> {
        let Some(graph) = graph.fixup() else {
            return Ok(());
        };

        let (func, blocks) = graph.build();
        let cfg = ControlFlowGraph::with_function(&func);
        let pdt = PostDominatorTree::with_function(&func, &cfg);

        // The virtual sink is node `n`. Exit blocks have an edge to it.
        let sink = graph.blocks.len();
        let succ: Vec<Vec<usize>> = graph
            .blocks
            .iter()
            .map(|t| match *t {
                Terminator::Return => alloc::vec![sink],
                Terminator::Jump(x) => alloc::vec![x],
                Terminator::Brif(x, y) => alloc::vec![x, y],
            })
            .collect();

        // Which nodes can reach the sink? Those that cannot are the diverging
        // blocks. Post-domination is only well-defined for the rest.
        let mut reaches = alloc::vec![false; graph.blocks.len() + 1];
        reaches[sink] = true;
        loop {
            let mut changed = false;
            for i in 0..graph.blocks.len() {
                if !reaches[i] && succ[i].iter().any(|&s| reaches[s]) {
                    reaches[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // Post-dominance sets as bitmasks over node indices `0..=sink`, via the
        // greatest fixpoint of `pdom(i) = {i} ∪ ⋂_{s ∈ succ(i)} pdom(s)`.
        let bit = |x: usize| 1u64 << x;
        let universe = bit(graph.blocks.len() + 1) - 1;
        let mut pdom = alloc::vec![universe; graph.blocks.len() + 1];
        pdom[sink] = bit(sink);
        loop {
            let mut changed = false;
            for i in 0..graph.blocks.len() {
                let mut inter = u64::MAX;
                for &s in &succ[i] {
                    inter &= pdom[s];
                }
                let next = bit(i) | inter;
                if next != pdom[i] {
                    pdom[i] = next;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        for i in 0..graph.blocks.len() {
            let expect_diverges = !reaches[i];
            if pdt.diverges(blocks[i]) != expect_diverges {
                return Err(format!(
                    "diverges({i}) = {}, expected {expect_diverges}",
                    pdt.diverges(blocks[i]),
                ));
            }

            if expect_diverges {
                if pdt.immediate_post_dominator(blocks[i]).is_some() {
                    return Err(format!(
                        "immediate_post_dominator({i}) should be None for a diverging block;",
                    ));
                }
                continue;
            }

            // `a` post-dominates `i` iff `a ∈ pdom(i)`.
            for a in 0..graph.blocks.len() {
                let expect = pdom[i] & bit(a) != 0;
                if pdt.block_post_dominates(blocks[a], blocks[i]) != expect {
                    return Err(format!(
                        "block_post_dominates({a}, {i}) = {}, expected {expect}",
                        pdt.block_post_dominates(blocks[a], blocks[i]),
                    ));
                }
            }

            // The immediate post-dominator is the strict post-dominator with
            // the largest post-dominator set (i.e. closest to `i`). The
            // post-dominators form a chain to the sink with strictly decreasing
            // set sizes, so this is unique. The virtual sink maps to `None`.
            let strict = pdom[i] & !bit(i);
            let mut best: Option<(u32, usize)> = None;
            for x in 0..=graph.blocks.len() {
                if strict & bit(x) != 0 {
                    let size = pdom[x].count_ones();
                    if best.map_or(true, |(best_size, _)| size > best_size) {
                        best = Some((size, x));
                    }
                }
            }
            let expect_ipdom = match best {
                Some((_, x)) if x != sink => Some(blocks[x]),
                _ => None,
            };
            if pdt.immediate_post_dominator(blocks[i]) != expect_ipdom {
                return Err(format!(
                    "immediate_post_dominator({i}) = {:?}, expected {expect_ipdom:?}",
                    pdt.immediate_post_dominator(blocks[i]),
                ));
            }
        }

        Ok(())
    }
}
