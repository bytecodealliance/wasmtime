//! A Dominator Tree represented as mappings of Ebbs to their immediate dominator.

use entity::SecondaryMap;
use flowgraph::{BasicBlock, ControlFlowGraph};
use ir::instructions::BranchInfo;
use ir::{Ebb, ExpandedProgramPoint, Function, Inst, Layout, ProgramOrder, Value};
use packed_option::PackedOption;
use std::cmp;
use std::cmp::Ordering;
use std::mem;
use std::vec::Vec;
use timing;

/// RPO numbers are not first assigned in a contiguous way but as multiples of STRIDE, to leave
/// room for modifications of the dominator tree.
const STRIDE: u32 = 4;

/// Special RPO numbers used during `compute_postorder`.
const DONE: u32 = 1;
const SEEN: u32 = 2;

/// Dominator tree node. We keep one of these per EBB.
#[derive(Clone, Default)]
struct DomNode {
    /// Number of this node in a reverse post-order traversal of the CFG, starting from 1.
    /// This number is monotonic in the reverse postorder but not contiguous, since we leave
    /// holes for later localized modifications of the dominator tree.
    /// Unreachable nodes get number 0, all others are positive.
    rpo_number: u32,

    /// The immediate dominator of this EBB, represented as the branch or jump instruction at the
    /// end of the dominating basic block.
    ///
    /// This is `None` for unreachable blocks and the entry block which doesn't have an immediate
    /// dominator.
    idom: PackedOption<Inst>,
}

/// The dominator tree for a single function.
pub struct DominatorTree {
    nodes: SecondaryMap<Ebb, DomNode>,

    /// CFG post-order of all reachable EBBs.
    postorder: Vec<Ebb>,

    /// Scratch memory used by `compute_postorder()`.
    stack: Vec<Ebb>,

    valid: bool,
}

/// Methods for querying the dominator tree.
impl DominatorTree {
    /// Is `ebb` reachable from the entry block?
    pub fn is_reachable(&self, ebb: Ebb) -> bool {
        self.nodes[ebb].rpo_number != 0
    }

    /// Get the CFG post-order of EBBs that was used to compute the dominator tree.
    ///
    /// Note that this post-order is not updated automatically when the CFG is modified. It is
    /// computed from scratch and cached by `compute()`.
    pub fn cfg_postorder(&self) -> &[Ebb] {
        debug_assert!(self.is_valid());
        &self.postorder
    }

    /// Returns the immediate dominator of `ebb`.
    ///
    /// The immediate dominator of an extended basic block is a basic block which we represent by
    /// the branch or jump instruction at the end of the basic block. This does not have to be the
    /// terminator of its EBB.
    ///
    /// A branch or jump is said to *dominate* `ebb` if all control flow paths from the function
    /// entry to `ebb` must go through the branch.
    ///
    /// The *immediate dominator* is the dominator that is closest to `ebb`. All other dominators
    /// also dominate the immediate dominator.
    ///
    /// This returns `None` if `ebb` is not reachable from the entry EBB, or if it is the entry EBB
    /// which has no dominators.
    pub fn idom(&self, ebb: Ebb) -> Option<Inst> {
        self.nodes[ebb].idom.into()
    }

    /// Compare two EBBs relative to the reverse post-order.
    fn rpo_cmp_ebb(&self, a: Ebb, b: Ebb) -> Ordering {
        self.nodes[a].rpo_number.cmp(&self.nodes[b].rpo_number)
    }

    /// Compare two program points relative to a reverse post-order traversal of the control-flow
    /// graph.
    ///
    /// Return `Ordering::Less` if `a` comes before `b` in the RPO.
    ///
    /// If `a` and `b` belong to the same EBB, compare their relative position in the EBB.
    pub fn rpo_cmp<A, B>(&self, a: A, b: B, layout: &Layout) -> Ordering
    where
        A: Into<ExpandedProgramPoint>,
        B: Into<ExpandedProgramPoint>,
    {
        let a = a.into();
        let b = b.into();
        self.rpo_cmp_ebb(layout.pp_ebb(a), layout.pp_ebb(b))
            .then(layout.cmp(a, b))
    }

    /// Returns `true` if `a` dominates `b`.
    ///
    /// This means that every control-flow path from the function entry to `b` must go through `a`.
    ///
    /// Dominance is ill defined for unreachable blocks. This function can always determine
    /// dominance for instructions in the same EBB, but otherwise returns `false` if either block
    /// is unreachable.
    ///
    /// An instruction is considered to dominate itself.
    pub fn dominates<A, B>(&self, a: A, b: B, layout: &Layout) -> bool
    where
        A: Into<ExpandedProgramPoint>,
        B: Into<ExpandedProgramPoint>,
    {
        let a = a.into();
        let b = b.into();
        match a {
            ExpandedProgramPoint::Ebb(ebb_a) => {
                a == b || self.last_dominator(ebb_a, b, layout).is_some()
            }
            ExpandedProgramPoint::Inst(inst_a) => {
                let ebb_a = layout.inst_ebb(inst_a).expect("Instruction not in layout.");
                match self.last_dominator(ebb_a, b, layout) {
                    Some(last) => layout.cmp(inst_a, last) != Ordering::Greater,
                    None => false,
                }
            }
        }
    }

    /// Find the last instruction in `a` that dominates `b`.
    /// If no instructions in `a` dominate `b`, return `None`.
    pub fn last_dominator<B>(&self, a: Ebb, b: B, layout: &Layout) -> Option<Inst>
    where
        B: Into<ExpandedProgramPoint>,
    {
        let (mut ebb_b, mut inst_b) = match b.into() {
            ExpandedProgramPoint::Ebb(ebb) => (ebb, None),
            ExpandedProgramPoint::Inst(inst) => (
                layout.inst_ebb(inst).expect("Instruction not in layout."),
                Some(inst),
            ),
        };
        let rpo_a = self.nodes[a].rpo_number;

        // Run a finger up the dominator tree from b until we see a.
        // Do nothing if b is unreachable.
        while rpo_a < self.nodes[ebb_b].rpo_number {
            let idom = match self.idom(ebb_b) {
                Some(idom) => idom,
                None => return None, // a is unreachable, so we climbed past the entry
            };
            ebb_b = layout.inst_ebb(idom).expect("Dominator got removed.");
            inst_b = Some(idom);
        }
        if a == ebb_b {
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
        mut a: BasicBlock,
        mut b: BasicBlock,
        layout: &Layout,
    ) -> BasicBlock {
        loop {
            match self.rpo_cmp_ebb(a.ebb, b.ebb) {
                Ordering::Less => {
                    // `a` comes before `b` in the RPO. Move `b` up.
                    let idom = self.nodes[b.ebb].idom.expect("Unreachable basic block?");
                    b = BasicBlock::new(
                        layout.inst_ebb(idom).expect("Dangling idom instruction"),
                        idom,
                    );
                }
                Ordering::Greater => {
                    // `b` comes before `a` in the RPO. Move `a` up.
                    let idom = self.nodes[a.ebb].idom.expect("Unreachable basic block?");
                    a = BasicBlock::new(
                        layout.inst_ebb(idom).expect("Dangling idom instruction"),
                        idom,
                    );
                }
                Ordering::Equal => break,
            }
        }

        debug_assert_eq!(
            a.ebb, b.ebb,
            "Unreachable block passed to common_dominator?"
        );

        // We're in the same EBB. The common dominator is the earlier instruction.
        if layout.cmp(a.inst, b.inst) == Ordering::Less {
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
        let mut domtree = Self::new();
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
    /// This leaves `rpo_number == 1` for all reachable EBBs, 0 for unreachable ones.
    fn compute_postorder(&mut self, func: &Function) {
        self.clear();
        self.nodes.resize(func.dfg.num_ebbs());

        // This algorithm is a depth first traversal (DFT) of the control flow graph, computing a
        // post-order of the EBBs that are reachable form the entry block. A DFT post-order is not
        // unique. The specific order we get is controlled by two factors:
        //
        // 1. The order each node's children are visited, and
        // 2. The method used for pruning graph edges to get a tree.
        //
        // There are two ways of viewing the CFG as a graph:
        //
        // 1. Each EBB is a node, with outgoing edges for all the branches in the EBB.
        // 2. Each basic block is a node, with outgoing edges for the single branch at the end of
        //    the BB. (An EBB is a linear sequence of basic blocks).
        //
        // The first graph is a contraction of the second one. We want to compute an EBB post-order
        // that is compatible both graph interpretations. That is, if you compute a BB post-order
        // and then remove those BBs that do not correspond to EBB headers, you get a post-order of
        // the EBB graph.
        //
        // Node child order:
        //
        //     In the BB graph, we always go down the fall-through path first and follow the branch
        //     destination second.
        //
        //     In the EBB graph, this is equivalent to visiting EBB successors in a bottom-up
        //     order, starting from the destination of the EBB's terminating jump, ending at the
        //     destination of the first branch in the EBB.
        //
        // Edge pruning:
        //
        //     In the BB graph, we keep an edge to an EBB the first time we visit the *source* side
        //     of the edge. Any subsequent edges to the same EBB are pruned.
        //
        //     The equivalent tree is reached in the EBB graph by keeping the first edge to an EBB
        //     in a top-down traversal of the successors. (And then visiting edges in a bottom-up
        //     order).
        //
        // This pruning method makes it possible to compute the DFT without storing lots of
        // information about the progress through an EBB.

        // During this algorithm only, use `rpo_number` to hold the following state:
        //
        //   0:    EBB has not yet been reached in the pre-order.
        //   SEEN: EBB has been pushed on the stack but successors not yet pushed.
        //   DONE: Successors pushed.

        match func.layout.entry_block() {
            Some(ebb) => {
                self.stack.push(ebb);
                self.nodes[ebb].rpo_number = SEEN;
            }
            None => return,
        }

        while let Some(ebb) = self.stack.pop() {
            match self.nodes[ebb].rpo_number {
                SEEN => {
                    // This is the first time we pop the EBB, so we need to scan its successors and
                    // then revisit it.
                    self.nodes[ebb].rpo_number = DONE;
                    self.stack.push(ebb);
                    self.push_successors(func, ebb);
                }
                DONE => {
                    // This is the second time we pop the EBB, so all successors have been
                    // processed.
                    self.postorder.push(ebb);
                }
                _ => unreachable!(),
            }
        }
    }

    /// Push `ebb` successors onto `self.stack`, filtering out those that have already been seen.
    ///
    /// The successors are pushed in program order which is important to get a split-invariant
    /// post-order. Split-invariant means that if an EBB is split in two, we get the same
    /// post-order except for the insertion of the new EBB header at the split point.
    fn push_successors(&mut self, func: &Function, ebb: Ebb) {
        for inst in func.layout.ebb_insts(ebb) {
            match func.dfg.analyze_branch(inst) {
                BranchInfo::SingleDest(succ, _) => self.push_if_unseen(succ),
                BranchInfo::Table(jt, dest) => {
                    for succ in func.jump_tables[jt].iter() {
                        self.push_if_unseen(*succ);
                    }
                    if let Some(dest) = dest {
                        self.push_if_unseen(dest);
                    }
                }
                BranchInfo::NotABranch => {}
            }
        }
    }

    /// Push `ebb` onto `self.stack` if it has not already been seen.
    fn push_if_unseen(&mut self, ebb: Ebb) {
        if self.nodes[ebb].rpo_number == 0 {
            self.nodes[ebb].rpo_number = SEEN;
            self.stack.push(ebb);
        }
    }

    /// Build a dominator tree from a control flow graph using Keith D. Cooper's
    /// "Simple, Fast Dominator Algorithm."
    fn compute_domtree(&mut self, func: &Function, cfg: &ControlFlowGraph) {
        // During this algorithm, `rpo_number` has the following values:
        //
        // 0: EBB is not reachable.
        // 1: EBB is reachable, but has not yet been visited during the first pass. This is set by
        // `compute_postorder`.
        // 2+: EBB is reachable and has an assigned RPO number.

        // We'll be iterating over a reverse post-order of the CFG, skipping the entry block.
        let (entry_block, postorder) = match self.postorder.as_slice().split_last() {
            Some((&eb, rest)) => (eb, rest),
            None => return,
        };
        debug_assert_eq!(Some(entry_block), func.layout.entry_block());

        // Do a first pass where we assign RPO numbers to all reachable nodes.
        self.nodes[entry_block].rpo_number = 2 * STRIDE;
        for (rpo_idx, &ebb) in postorder.iter().rev().enumerate() {
            // Update the current node and give it an RPO number.
            // The entry block got 2, the rest start at 3 by multiples of STRIDE to leave
            // room for future dominator tree modifications.
            //
            // Since `compute_idom` will only look at nodes with an assigned RPO number, the
            // function will never see an uninitialized predecessor.
            //
            // Due to the nature of the post-order traversal, every node we visit will have at
            // least one predecessor that has previously been visited during this RPO.
            self.nodes[ebb] = DomNode {
                idom: self.compute_idom(ebb, cfg, &func.layout).into(),
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
            for &ebb in postorder.iter().rev() {
                let idom = self.compute_idom(ebb, cfg, &func.layout).into();
                if self.nodes[ebb].idom != idom {
                    self.nodes[ebb].idom = idom;
                    changed = true;
                }
            }
        }
    }

    // Compute the immediate dominator for `ebb` using the current `idom` states for the reachable
    // nodes.
    fn compute_idom(&self, ebb: Ebb, cfg: &ControlFlowGraph, layout: &Layout) -> Inst {
        // Get an iterator with just the reachable, already visited predecessors to `ebb`.
        // Note that during the first pass, `rpo_number` is 1 for reachable blocks that haven't
        // been visited yet, 0 for unreachable blocks.
        let mut reachable_preds = cfg
            .pred_iter(ebb)
            .filter(|&BasicBlock { ebb: pred, .. }| self.nodes[pred].rpo_number > 1);

        // The RPO must visit at least one predecessor before this node.
        let mut idom = reachable_preds
            .next()
            .expect("EBB node must have one reachable predecessor");

        for pred in reachable_preds {
            idom = self.common_dominator(idom, pred, layout);
        }

        idom.inst
    }
}

impl DominatorTree {
    /// When splitting an `Ebb` using `Layout::split_ebb`, you can use this method to update
    /// the dominator tree locally rather than recomputing it.
    ///
    /// `old_ebb` is the `Ebb` before splitting, and `new_ebb` is the `Ebb` which now contains
    /// the second half of `old_ebb`. `split_jump_inst` is the terminator jump instruction of
    /// `old_ebb` that points to `new_ebb`.
    pub fn recompute_split_ebb(&mut self, old_ebb: Ebb, new_ebb: Ebb, split_jump_inst: Inst) {
        if !self.is_reachable(old_ebb) {
            // old_ebb is unreachable, it stays so and new_ebb is unreachable too
            self.nodes[new_ebb] = Default::default();
            return;
        }
        // We use the RPO comparison on the postorder list so we invert the operands of the
        // comparison
        let old_ebb_postorder_index = self
            .postorder
            .as_slice()
            .binary_search_by(|probe| self.rpo_cmp_ebb(old_ebb, *probe))
            .expect("the old ebb is not declared to the dominator tree");
        let new_ebb_rpo = self.insert_after_rpo(old_ebb, old_ebb_postorder_index, new_ebb);
        self.nodes[new_ebb] = DomNode {
            rpo_number: new_ebb_rpo,
            idom: Some(split_jump_inst).into(),
        };
    }

    // Insert new_ebb just after ebb in the RPO. This function checks
    // if there is a gap in rpo numbers; if yes it returns the number in the gap and if
    // not it renumbers.
    fn insert_after_rpo(&mut self, ebb: Ebb, ebb_postorder_index: usize, new_ebb: Ebb) -> u32 {
        let ebb_rpo_number = self.nodes[ebb].rpo_number;
        let inserted_rpo_number = ebb_rpo_number + 1;
        // If there is no gaps in RPo numbers to insert this new number, we iterate
        // forward in RPO numbers and backwards in the postorder list of EBBs, renumbering the Ebbs
        // until we find a gap
        for (&current_ebb, current_rpo) in self.postorder[0..ebb_postorder_index]
            .iter()
            .rev()
            .zip(inserted_rpo_number + 1..)
        {
            if self.nodes[current_ebb].rpo_number < current_rpo {
                // There is no gap, we renumber
                self.nodes[current_ebb].rpo_number = current_rpo;
            } else {
                // There is a gap, we stop the renumbering and exit
                break;
            }
        }
        // TODO: insert in constant time?
        self.postorder.insert(ebb_postorder_index, new_ebb);
        inserted_rpo_number
    }
}

/// Optional pre-order information that can be computed for a dominator tree.
///
/// This data structure is computed from a `DominatorTree` and provides:
///
/// - A forward traversable dominator tree through the `children()` iterator.
/// - An ordering of EBBs according to a dominator tree pre-order.
/// - Constant time dominance checks at the EBB granularity.
///
/// The information in this auxillary data structure is not easy to update when the control flow
/// graph changes, which is why it is kept separate.
pub struct DominatorTreePreorder {
    nodes: SecondaryMap<Ebb, ExtraNode>,

    // Scratch memory used by `compute_postorder()`.
    stack: Vec<Ebb>,
}

#[derive(Default, Clone)]
struct ExtraNode {
    /// First child node in the domtree.
    child: PackedOption<Ebb>,

    /// Next sibling node in the domtree. This linked list is ordered according to the CFG RPO.
    sibling: PackedOption<Ebb>,

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

    /// Recompute this data structure to match `domtree`.
    pub fn compute(&mut self, domtree: &DominatorTree, layout: &Layout) {
        self.nodes.clear();
        debug_assert_eq!(self.stack.len(), 0);

        // Step 1: Populate the child and sibling links.
        //
        // By following the CFG post-order and pushing to the front of the lists, we make sure that
        // sibling lists are ordered according to the CFG reverse post-order.
        for &ebb in domtree.cfg_postorder() {
            if let Some(idom_inst) = domtree.idom(ebb) {
                let idom = layout.pp_ebb(idom_inst);
                let sib = mem::replace(&mut self.nodes[idom].child, ebb.into());
                self.nodes[ebb].sibling = sib;
            } else {
                // The only EBB without an immediate dominator is the entry.
                self.stack.push(ebb);
            }
        }

        // Step 2. Assign pre-order numbers from a DFS of the dominator tree.
        debug_assert!(self.stack.len() <= 1);
        let mut n = 0;
        while let Some(ebb) = self.stack.pop() {
            n += 1;
            let node = &mut self.nodes[ebb];
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
        for &ebb in domtree.cfg_postorder() {
            if let Some(idom_inst) = domtree.idom(ebb) {
                let idom = layout.pp_ebb(idom_inst);
                let pre_max = cmp::max(self.nodes[ebb].pre_max, self.nodes[idom].pre_max);
                self.nodes[idom].pre_max = pre_max;
            }
        }
    }
}

/// An iterator that enumerates the direct children of an EBB in the dominator tree.
pub struct ChildIter<'a> {
    dtpo: &'a DominatorTreePreorder,
    next: PackedOption<Ebb>,
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = Ebb;

    fn next(&mut self) -> Option<Ebb> {
        let n = self.next.expand();
        if let Some(ebb) = n {
            self.next = self.dtpo.nodes[ebb].sibling;
        }
        n
    }
}

/// Query interface for the dominator tree pre-order.
impl DominatorTreePreorder {
    /// Get an iterator over the direct children of `ebb` in the dominator tree.
    ///
    /// These are the EBB's whose immediate dominator is an instruction in `ebb`, ordered according
    /// to the CFG reverse post-order.
    pub fn children(&self, ebb: Ebb) -> ChildIter {
        ChildIter {
            dtpo: self,
            next: self.nodes[ebb].child,
        }
    }

    /// Fast, constant time dominance check with EBB granularity.
    ///
    /// This computes the same result as `domtree.dominates(a, b)`, but in guaranteed fast constant
    /// time. This is less general than the `DominatorTree` method because it only works with EBB
    /// program points.
    ///
    /// An EBB is considered to dominate itself.
    pub fn dominates(&self, a: Ebb, b: Ebb) -> bool {
        let na = &self.nodes[a];
        let nb = &self.nodes[b];
        na.pre_number <= nb.pre_number && na.pre_max >= nb.pre_max
    }

    /// Compare two EBBs according to the dominator pre-order.
    pub fn pre_cmp_ebb(&self, a: Ebb, b: Ebb) -> Ordering {
        self.nodes[a].pre_number.cmp(&self.nodes[b].pre_number)
    }

    /// Compare two program points according to the dominator tree pre-order.
    ///
    /// This ordering of program points have the property that given a program point, pp, all the
    /// program points dominated by pp follow immediately and contiguously after pp in the order.
    pub fn pre_cmp<A, B>(&self, a: A, b: B, layout: &Layout) -> Ordering
    where
        A: Into<ExpandedProgramPoint>,
        B: Into<ExpandedProgramPoint>,
    {
        let a = a.into();
        let b = b.into();
        self.pre_cmp_ebb(layout.pp_ebb(a), layout.pp_ebb(b))
            .then(layout.cmp(a, b))
    }

    /// Compare two value defs according to the dominator tree pre-order.
    ///
    /// Two values defined at the same program point are compared according to their parameter or
    /// result order.
    ///
    /// This is a total ordering of the values in the function.
    pub fn pre_cmp_def(&self, a: Value, b: Value, func: &Function) -> Ordering {
        let da = func.dfg.value_def(a);
        let db = func.dfg.value_def(b);
        self.pre_cmp(da, db, &func.layout)
            .then_with(|| da.num().cmp(&db.num()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cursor::{Cursor, FuncCursor};
    use flowgraph::ControlFlowGraph;
    use ir::types::*;
    use ir::{Function, InstBuilder, TrapCode};
    use settings;
    use verifier::{verify_context, VerifierErrors};

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
        let ebb0 = func.dfg.make_ebb();
        let v0 = func.dfg.append_ebb_param(ebb0, I32);
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_ebb(ebb0);
        cur.ins().brnz(v0, ebb2, &[]);
        cur.ins().trap(TrapCode::User(0));

        cur.insert_ebb(ebb1);
        let v1 = cur.ins().iconst(I32, 1);
        let v2 = cur.ins().iadd(v0, v1);
        cur.ins().jump(ebb0, &[v2]);

        cur.insert_ebb(ebb2);
        cur.ins().return_(&[v0]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let dt = DominatorTree::with_function(cur.func, &cfg);

        // Fall-through-first, prune-at-source DFT:
        //
        // ebb0 {
        //   brnz ebb2 {
        //     trap
        //     ebb2 {
        //       return
        //     } ebb2
        // } ebb0
        assert_eq!(dt.cfg_postorder(), &[ebb2, ebb0]);

        let v2_def = cur.func.dfg.value_def(v2).unwrap_inst();
        assert!(!dt.dominates(v2_def, ebb0, &cur.func.layout));
        assert!(!dt.dominates(ebb0, v2_def, &cur.func.layout));

        let mut dtpo = DominatorTreePreorder::new();
        dtpo.compute(&dt, &cur.func.layout);
        assert!(dtpo.dominates(ebb0, ebb0));
        assert!(!dtpo.dominates(ebb0, ebb1));
        assert!(dtpo.dominates(ebb0, ebb2));
        assert!(!dtpo.dominates(ebb1, ebb0));
        assert!(dtpo.dominates(ebb1, ebb1));
        assert!(!dtpo.dominates(ebb1, ebb2));
        assert!(!dtpo.dominates(ebb2, ebb0));
        assert!(!dtpo.dominates(ebb2, ebb1));
        assert!(dtpo.dominates(ebb2, ebb2));
    }

    #[test]
    fn non_zero_entry_block() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        let ebb3 = func.dfg.make_ebb();
        let cond = func.dfg.append_ebb_param(ebb3, I32);

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_ebb(ebb3);
        let jmp_ebb3_ebb1 = cur.ins().jump(ebb1, &[]);

        cur.insert_ebb(ebb1);
        let br_ebb1_ebb0 = cur.ins().brnz(cond, ebb0, &[]);
        let jmp_ebb1_ebb2 = cur.ins().jump(ebb2, &[]);

        cur.insert_ebb(ebb2);
        cur.ins().jump(ebb0, &[]);

        cur.insert_ebb(ebb0);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let dt = DominatorTree::with_function(cur.func, &cfg);

        // Fall-through-first, prune-at-source DFT:
        //
        // ebb3 {
        //   ebb3:jump ebb1 {
        //     ebb1 {
        //       ebb1:brnz ebb0 {
        //         ebb1:jump ebb2 {
        //           ebb2 {
        //             ebb2:jump ebb0 (seen)
        //           } ebb2
        //         } ebb1:jump ebb2
        //         ebb0 {
        //         } ebb0
        //       } ebb1:brnz ebb0
        //     } ebb1
        //   } ebb3:jump ebb1
        // } ebb3

        assert_eq!(dt.cfg_postorder(), &[ebb2, ebb0, ebb1, ebb3]);

        assert_eq!(cur.func.layout.entry_block().unwrap(), ebb3);
        assert_eq!(dt.idom(ebb3), None);
        assert_eq!(dt.idom(ebb1).unwrap(), jmp_ebb3_ebb1);
        assert_eq!(dt.idom(ebb2).unwrap(), jmp_ebb1_ebb2);
        assert_eq!(dt.idom(ebb0).unwrap(), br_ebb1_ebb0);

        assert!(dt.dominates(br_ebb1_ebb0, br_ebb1_ebb0, &cur.func.layout));
        assert!(!dt.dominates(br_ebb1_ebb0, jmp_ebb3_ebb1, &cur.func.layout));
        assert!(dt.dominates(jmp_ebb3_ebb1, br_ebb1_ebb0, &cur.func.layout));

        assert_eq!(dt.rpo_cmp(ebb3, ebb3, &cur.func.layout), Ordering::Equal);
        assert_eq!(dt.rpo_cmp(ebb3, ebb1, &cur.func.layout), Ordering::Less);
        assert_eq!(
            dt.rpo_cmp(ebb3, jmp_ebb3_ebb1, &cur.func.layout),
            Ordering::Less
        );
        assert_eq!(
            dt.rpo_cmp(jmp_ebb3_ebb1, jmp_ebb1_ebb2, &cur.func.layout),
            Ordering::Less
        );
    }

    #[test]
    fn backwards_layout() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_ebb(ebb0);
        let jmp02 = cur.ins().jump(ebb2, &[]);

        cur.insert_ebb(ebb1);
        let trap = cur.ins().trap(TrapCode::User(5));

        cur.insert_ebb(ebb2);
        let jmp21 = cur.ins().jump(ebb1, &[]);

        let cfg = ControlFlowGraph::with_function(cur.func);
        let dt = DominatorTree::with_function(cur.func, &cfg);

        assert_eq!(cur.func.layout.entry_block(), Some(ebb0));
        assert_eq!(dt.idom(ebb0), None);
        assert_eq!(dt.idom(ebb1), Some(jmp21));
        assert_eq!(dt.idom(ebb2), Some(jmp02));

        assert!(dt.dominates(ebb0, ebb0, &cur.func.layout));
        assert!(dt.dominates(ebb0, jmp02, &cur.func.layout));
        assert!(dt.dominates(ebb0, ebb1, &cur.func.layout));
        assert!(dt.dominates(ebb0, trap, &cur.func.layout));
        assert!(dt.dominates(ebb0, ebb2, &cur.func.layout));
        assert!(dt.dominates(ebb0, jmp21, &cur.func.layout));

        assert!(!dt.dominates(jmp02, ebb0, &cur.func.layout));
        assert!(dt.dominates(jmp02, jmp02, &cur.func.layout));
        assert!(dt.dominates(jmp02, ebb1, &cur.func.layout));
        assert!(dt.dominates(jmp02, trap, &cur.func.layout));
        assert!(dt.dominates(jmp02, ebb2, &cur.func.layout));
        assert!(dt.dominates(jmp02, jmp21, &cur.func.layout));

        assert!(!dt.dominates(ebb1, ebb0, &cur.func.layout));
        assert!(!dt.dominates(ebb1, jmp02, &cur.func.layout));
        assert!(dt.dominates(ebb1, ebb1, &cur.func.layout));
        assert!(dt.dominates(ebb1, trap, &cur.func.layout));
        assert!(!dt.dominates(ebb1, ebb2, &cur.func.layout));
        assert!(!dt.dominates(ebb1, jmp21, &cur.func.layout));

        assert!(!dt.dominates(trap, ebb0, &cur.func.layout));
        assert!(!dt.dominates(trap, jmp02, &cur.func.layout));
        assert!(!dt.dominates(trap, ebb1, &cur.func.layout));
        assert!(dt.dominates(trap, trap, &cur.func.layout));
        assert!(!dt.dominates(trap, ebb2, &cur.func.layout));
        assert!(!dt.dominates(trap, jmp21, &cur.func.layout));

        assert!(!dt.dominates(ebb2, ebb0, &cur.func.layout));
        assert!(!dt.dominates(ebb2, jmp02, &cur.func.layout));
        assert!(dt.dominates(ebb2, ebb1, &cur.func.layout));
        assert!(dt.dominates(ebb2, trap, &cur.func.layout));
        assert!(dt.dominates(ebb2, ebb2, &cur.func.layout));
        assert!(dt.dominates(ebb2, jmp21, &cur.func.layout));

        assert!(!dt.dominates(jmp21, ebb0, &cur.func.layout));
        assert!(!dt.dominates(jmp21, jmp02, &cur.func.layout));
        assert!(dt.dominates(jmp21, ebb1, &cur.func.layout));
        assert!(dt.dominates(jmp21, trap, &cur.func.layout));
        assert!(!dt.dominates(jmp21, ebb2, &cur.func.layout));
        assert!(dt.dominates(jmp21, jmp21, &cur.func.layout));
    }

    #[test]
    fn renumbering() {
        let mut func = Function::new();
        let entry = func.dfg.make_ebb();
        let ebb0 = func.dfg.make_ebb();
        let ebb100 = func.dfg.make_ebb();

        let mut cur = FuncCursor::new(&mut func);

        cur.insert_ebb(entry);
        cur.ins().jump(ebb0, &[]);

        cur.insert_ebb(ebb0);
        let cond = cur.ins().iconst(I32, 0);
        let inst2 = cur.ins().brz(cond, ebb0, &[]);
        let inst3 = cur.ins().brz(cond, ebb0, &[]);
        let inst4 = cur.ins().brz(cond, ebb0, &[]);
        let inst5 = cur.ins().brz(cond, ebb0, &[]);
        cur.ins().jump(ebb100, &[]);
        cur.insert_ebb(ebb100);
        cur.ins().return_(&[]);

        let mut cfg = ControlFlowGraph::with_function(cur.func);
        let mut dt = DominatorTree::with_function(cur.func, &cfg);

        let ebb1 = cur.func.dfg.make_ebb();
        cur.func.layout.split_ebb(ebb1, inst2);
        cur.goto_bottom(ebb0);
        let middle_jump_inst = cur.ins().jump(ebb1, &[]);

        dt.recompute_split_ebb(ebb0, ebb1, middle_jump_inst);

        let ebb2 = cur.func.dfg.make_ebb();
        cur.func.layout.split_ebb(ebb2, inst3);
        cur.goto_bottom(ebb1);
        let middle_jump_inst = cur.ins().jump(ebb2, &[]);
        dt.recompute_split_ebb(ebb1, ebb2, middle_jump_inst);

        let ebb3 = cur.func.dfg.make_ebb();
        cur.func.layout.split_ebb(ebb3, inst4);
        cur.goto_bottom(ebb2);
        let middle_jump_inst = cur.ins().jump(ebb3, &[]);
        dt.recompute_split_ebb(ebb2, ebb3, middle_jump_inst);

        let ebb4 = cur.func.dfg.make_ebb();
        cur.func.layout.split_ebb(ebb4, inst5);
        cur.goto_bottom(ebb3);
        let middle_jump_inst = cur.ins().jump(ebb4, &[]);
        dt.recompute_split_ebb(ebb3, ebb4, middle_jump_inst);

        cfg.compute(cur.func);

        let flags = settings::Flags::new(settings::builder());
        let mut errors = VerifierErrors::default();

        verify_context(cur.func, &cfg, &dt, &flags, &mut errors).unwrap();

        assert!(errors.0.is_empty());
    }
}
