//! A Dominator Tree represented as mappings of Ebbs to their immediate dominator.

use entity::EntityMap;
use flowgraph::{ControlFlowGraph, BasicBlock};
use ir::{Ebb, Inst, Function, Layout, ProgramOrder, ExpandedProgramPoint};
use packed_option::PackedOption;

use std::cmp::Ordering;

// RPO numbers are not first assigned in a contiguous way but as multiples of STRIDE, to leave
// room for modifications of the dominator tree.
const STRIDE: u32 = 4;

// Dominator tree node. We keep one of these per EBB.
#[derive(Clone, Default)]
struct DomNode {
    // Number of this node in a reverse post-order traversal of the CFG, starting from 1.
    // This number is monotonic in the reverse postorder but not contiguous, since we leave
    // holes for later localized modifications of the dominator tree.
    // Unreachable nodes get number 0, all others are positive.
    rpo_number: u32,

    // The immediate dominator of this EBB, represented as the branch or jump instruction at the
    // end of the dominating basic block.
    //
    // This is `None` for unreachable blocks and the entry block which doesn't have an immediate
    // dominator.
    idom: PackedOption<Inst>,
}

/// The dominator tree for a single function.
pub struct DominatorTree {
    nodes: EntityMap<Ebb, DomNode>,

    // CFG post-order of all reachable EBBs.
    postorder: Vec<Ebb>,

    // Scratch memory used by `compute_postorder()`.
    stack: Vec<Ebb>,
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
        self.rpo_cmp_ebb(layout.pp_ebb(a), layout.pp_ebb(b)).then(
            layout.cmp(a, b),
        )
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
    fn last_dominator<B>(&self, a: Ebb, b: B, layout: &Layout) -> Option<Inst>
    where
        B: Into<ExpandedProgramPoint>,
    {
        let (mut ebb_b, mut inst_b) = match b.into() {
            ExpandedProgramPoint::Ebb(ebb) => (ebb, None),
            ExpandedProgramPoint::Inst(inst) => {
                (
                    layout.inst_ebb(inst).expect("Instruction not in layout."),
                    Some(inst),
                )
            }
        };
        let rpo_a = self.nodes[a].rpo_number;

        // Run a finger up the dominator tree from b until we see a.
        // Do nothing if b is unreachable.
        while rpo_a < self.nodes[ebb_b].rpo_number {
            let idom = self.idom(ebb_b).expect("Shouldn't meet unreachable here.");
            ebb_b = layout.inst_ebb(idom).expect("Dominator got removed.");
            inst_b = Some(idom);
        }
        if a == ebb_b { inst_b } else { None }
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
            match self.rpo_cmp_ebb(a.0, b.0) {
                Ordering::Less => {
                    // `a` comes before `b` in the RPO. Move `b` up.
                    let idom = self.nodes[b.0].idom.expect("Unreachable basic block?");
                    b = (
                        layout.inst_ebb(idom).expect("Dangling idom instruction"),
                        idom,
                    );
                }
                Ordering::Greater => {
                    // `b` comes before `a` in the RPO. Move `a` up.
                    let idom = self.nodes[a.0].idom.expect("Unreachable basic block?");
                    a = (
                        layout.inst_ebb(idom).expect("Dangling idom instruction"),
                        idom,
                    );
                }
                Ordering::Equal => break,
            }
        }

        assert_eq!(a.0, b.0, "Unreachable block passed to common_dominator?");

        // We're in the same EBB. The common dominator is the earlier instruction.
        if layout.cmp(a.1, b.1) == Ordering::Less {
            a
        } else {
            b
        }
    }
}

impl DominatorTree {
    /// Allocate a new blank dominator tree. Use `compute` to compute the dominator tree for a
    /// function.
    pub fn new() -> DominatorTree {
        DominatorTree {
            nodes: EntityMap::new(),
            postorder: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// Allocate and compute a dominator tree.
    pub fn with_function(func: &Function, cfg: &ControlFlowGraph) -> DominatorTree {
        let mut domtree = DominatorTree::new();
        domtree.compute(func, cfg);
        domtree
    }

    /// Reset and compute a CFG post-order and dominator tree.
    pub fn compute(&mut self, func: &Function, cfg: &ControlFlowGraph) {
        self.compute_postorder(func, cfg);
        self.compute_domtree(func, cfg);
    }

    /// Clear the data structures used to represent the dominator tree. This will leave the tree in
    /// a state where `is_valid()` returns false.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.postorder.clear();
        assert!(self.stack.is_empty());
    }

    /// Check if the dominator tree is in a valid state.
    ///
    /// Note that this doesn't perform any kind of validity checks. It simply checks if the
    /// `compute()` method has been called since the last `clear()`. It does not check that the
    /// dominator tree is consistent with the CFG.
    pub fn is_valid(&self) -> bool {
        !self.nodes.is_empty()
    }

    /// Conveneince function to call `compute` if `compute` hasn't been called
    /// since the last `clear()`.
    pub fn ensure(&mut self, func: &Function, cfg: &ControlFlowGraph) {
        if !self.is_valid() {
            self.compute(func, cfg)
        }
    }

    /// Reset all internal data structures and compute a post-order for `cfg`.
    ///
    /// This leaves `rpo_number == 1` for all reachable EBBs, 0 for unreachable ones.
    fn compute_postorder(&mut self, func: &Function, cfg: &ControlFlowGraph) {
        self.clear();
        self.nodes.resize(func.dfg.num_ebbs());

        // During this algorithm only, use `rpo_number` to hold the following state:
        //
        // 0: EBB never reached.
        // 2: EBB has been pushed once, so it shouldn't be pushed again.
        // 1: EBB has already been popped once, and should be added to the post-order next time.
        const SEEN: u32 = 2;
        const DONE: u32 = 1;

        match func.layout.entry_block() {
            Some(ebb) => {
                self.nodes[ebb].rpo_number = SEEN;
                self.stack.push(ebb)
            }
            None => return,
        }

        while let Some(ebb) = self.stack.pop() {
            match self.nodes[ebb].rpo_number {
                // This is the first time we visit `ebb`, forming a pre-order.
                SEEN => {
                    // Mark it as done and re-queue it to be visited after its children.
                    self.nodes[ebb].rpo_number = DONE;
                    self.stack.push(ebb);
                    for &succ in cfg.get_successors(ebb) {
                        // Only push children that haven't been seen before.
                        if self.nodes[succ].rpo_number == 0 {
                            self.nodes[succ].rpo_number = SEEN;
                            self.stack.push(succ);
                        }
                    }
                }
                // This is the second time we popped `ebb`, so all its children have been visited.
                // This is the post-order.
                DONE => self.postorder.push(ebb),
                _ => panic!("Inconsistent stack rpo_number"),
            }
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
        let mut reachable_preds = cfg.get_predecessors(ebb).iter().cloned().filter(
            |&(pred, _)| {
                self.nodes[pred].rpo_number > 1
            },
        );

        // The RPO must visit at least one predecessor before this node.
        let mut idom = reachable_preds.next().expect(
            "EBB node must have one reachable predecessor",
        );

        for pred in reachable_preds {
            idom = self.common_dominator(idom, pred, layout);
        }

        idom.1
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
        let old_ebb_postorder_index =
            self.postorder
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
        for (&current_ebb, current_rpo) in
            self.postorder[0..ebb_postorder_index].iter().rev().zip(
                inserted_rpo_number +
                    1..,
            )
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

#[cfg(test)]
mod test {
    use cursor::{Cursor, FuncCursor};
    use flowgraph::ControlFlowGraph;
    use ir::{Function, InstBuilder, types};
    use super::*;
    use ir::types::*;
    use verifier::verify_context;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::with_function(&func);
        let dtree = DominatorTree::with_function(&func, &cfg);
        assert_eq!(0, dtree.nodes.keys().count());
        assert_eq!(dtree.cfg_postorder(), &[]);
    }

    #[test]
    fn non_zero_entry_block() {
        let mut func = Function::new();
        let ebb3 = func.dfg.make_ebb();
        let cond = func.dfg.append_ebb_arg(ebb3, types::I32);
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        let ebb0 = func.dfg.make_ebb();

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

        assert_eq!(dt.cfg_postorder(), &[ebb2, ebb0, ebb1, ebb3]);
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
        let trap = cur.ins().trap();

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
        let ebb0 = func.dfg.make_ebb();
        let ebb100 = func.dfg.make_ebb();

        let mut cur = FuncCursor::new(&mut func);

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
        verify_context(cur.func, &cfg, &dt, None).unwrap();
    }
}
