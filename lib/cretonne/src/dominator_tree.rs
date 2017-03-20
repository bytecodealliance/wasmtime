//! A Dominator Tree represented as mappings of Ebbs to their immediate dominator.

use cfg::{ControlFlowGraph, BasicBlock};
use ir::{Ebb, Inst, Function, Layout, ProgramOrder};
use entity_map::EntityMap;
use packed_option::PackedOption;

use std::cmp::Ordering;

// Dominator tree node. We keep one of these per EBB.
#[derive(Clone, Default)]
struct DomNode {
    // Number of this node in a reverse post-order traversal of the CFG, starting from 1.
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
}

/// Methods for querying the dominator tree.
impl DominatorTree {
    /// Is `ebb` reachable from the entry block?
    pub fn is_reachable(&self, ebb: Ebb) -> bool {
        self.nodes[ebb].rpo_number != 0
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

    /// Compare two EBBs relative to a reverse post-order traversal of the control-flow graph.
    ///
    /// Return `Ordering::Less` if `a` comes before `b` in the RPO.
    pub fn rpo_cmp(&self, a: Ebb, b: Ebb) -> Ordering {
        self.nodes[a].rpo_number.cmp(&self.nodes[b].rpo_number)
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
    pub fn dominates(&self, a: Inst, b: Inst, layout: &Layout) -> bool {
        let ebb_a = layout.inst_ebb(a).expect("Instruction not in layout.");
        self.ebb_dominates(ebb_a, b, layout) && layout.cmp(a, b) != Ordering::Greater
    }

    /// Returns `true` if `ebb_a` dominates `b`.
    ///
    /// This means that every control-flow path from the function entry to `b` must go through
    /// `ebb_a`.
    ///
    /// Dominance is ill defined for unreachable blocks. This function can always determine
    /// dominance for instructions in the same EBB, but otherwise returns `false` if either block
    /// is unreachable.
    pub fn ebb_dominates(&self, ebb_a: Ebb, mut b: Inst, layout: &Layout) -> bool {
        let mut ebb_b = layout.inst_ebb(b).expect("Instruction not in layout.");
        let rpo_a = self.nodes[ebb_a].rpo_number;

        // Run a finger up the dominator tree from b until we see a.
        // Do nothing if b is unreachable.
        while rpo_a < self.nodes[ebb_b].rpo_number {
            b = self.idom(ebb_b).expect("Shouldn't meet unreachable here.");
            ebb_b = layout.inst_ebb(b).expect("Dominator got removed.");
        }

        ebb_a == ebb_b
    }

    /// Compute the common dominator of two basic blocks.
    ///
    /// Both basic blocks are assumed to be reachable.
    pub fn common_dominator(&self,
                            mut a: BasicBlock,
                            mut b: BasicBlock,
                            layout: &Layout)
                            -> BasicBlock {
        loop {
            match self.rpo_cmp(a.0, b.0) {
                Ordering::Less => {
                    // `a` comes before `b` in the RPO. Move `b` up.
                    let idom = self.nodes[b.0].idom.expect("Unreachable basic block?");
                    b = (layout.inst_ebb(idom).expect("Dangling idom instruction"), idom);
                }
                Ordering::Greater => {
                    // `b` comes before `a` in the RPO. Move `a` up.
                    let idom = self.nodes[a.0].idom.expect("Unreachable basic block?");
                    a = (layout.inst_ebb(idom).expect("Dangling idom instruction"), idom);
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
        DominatorTree { nodes: EntityMap::new() }
    }

    /// Allocate and compute a dominator tree.
    pub fn with_function(func: &Function, cfg: &ControlFlowGraph) -> DominatorTree {
        let mut domtree = DominatorTree::new();
        domtree.compute(func, cfg);
        domtree
    }

    /// Build a dominator tree from a control flow graph using Keith D. Cooper's
    /// "Simple, Fast Dominator Algorithm."
    pub fn compute(&mut self, func: &Function, cfg: &ControlFlowGraph) {
        self.nodes.clear();
        self.nodes.resize(func.dfg.num_ebbs());

        // We'll be iterating over a reverse post-order of the CFG.
        // This vector only contains reachable EBBs.
        let mut postorder = cfg.postorder_ebbs();

        // Remove the entry block, and abort if the function is empty.
        // The last block visited in a post-order traversal must be the entry block.
        let entry_block = match postorder.pop() {
            Some(ebb) => ebb,
            None => return,
        };
        assert_eq!(Some(entry_block), func.layout.entry_block());

        // Do a first pass where we assign RPO numbers to all reachable nodes.
        self.nodes[entry_block].rpo_number = 1;
        for (rpo_idx, &ebb) in postorder.iter().rev().enumerate() {
            // Update the current node and give it an RPO number.
            // The entry block got 1, the rest start at 2.
            //
            // Nodes do not appear as reachable until the have an assigned RPO number, and
            // `compute_idom` will only look at reachable nodes. This means that the function will
            // never see an uninitialized predecessor.
            //
            // Due to the nature of the post-order traversal, every node we visit will have at
            // least one predecessor that has previously been visited during this RPO.
            self.nodes[ebb] = DomNode {
                idom: self.compute_idom(ebb, cfg, &func.layout).into(),
                rpo_number: rpo_idx as u32 + 2,
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
        // Get an iterator with just the reachable predecessors to `ebb`.
        // Note that during the first pass, `is_reachable` returns false for blocks that haven't
        // been visited yet.
        let mut reachable_preds = cfg.get_predecessors(ebb)
            .iter()
            .cloned()
            .filter(|&(ebb, _)| self.is_reachable(ebb));

        // The RPO must visit at least one predecessor before this node.
        let mut idom =
            reachable_preds.next().expect("EBB node must have one reachable predecessor");

        for pred in reachable_preds {
            idom = self.common_dominator(idom, pred, layout);
        }

        idom.1
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ir::{Function, InstBuilder, Cursor, types};
    use cfg::ControlFlowGraph;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::with_function(&func);
        let dtree = DominatorTree::with_function(&func, &cfg);
        assert_eq!(0, dtree.nodes.keys().count());
    }

    #[test]
    fn non_zero_entry_block() {
        let mut func = Function::new();
        let ebb3 = func.dfg.make_ebb();
        let cond = func.dfg.append_ebb_arg(ebb3, types::I32);
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        let ebb0 = func.dfg.make_ebb();

        let jmp_ebb3_ebb1;
        let br_ebb1_ebb0;
        let jmp_ebb1_ebb2;

        {
            let dfg = &mut func.dfg;
            let cur = &mut Cursor::new(&mut func.layout);

            cur.insert_ebb(ebb3);
            jmp_ebb3_ebb1 = dfg.ins(cur).jump(ebb1, &[]);

            cur.insert_ebb(ebb1);
            br_ebb1_ebb0 = dfg.ins(cur).brnz(cond, ebb0, &[]);
            jmp_ebb1_ebb2 = dfg.ins(cur).jump(ebb2, &[]);

            cur.insert_ebb(ebb2);
            dfg.ins(cur).jump(ebb0, &[]);

            cur.insert_ebb(ebb0);
        }

        let cfg = ControlFlowGraph::with_function(&func);
        let dt = DominatorTree::with_function(&func, &cfg);

        assert_eq!(func.layout.entry_block().unwrap(), ebb3);
        assert_eq!(dt.idom(ebb3), None);
        assert_eq!(dt.idom(ebb1).unwrap(), jmp_ebb3_ebb1);
        assert_eq!(dt.idom(ebb2).unwrap(), jmp_ebb1_ebb2);
        assert_eq!(dt.idom(ebb0).unwrap(), br_ebb1_ebb0);

        assert!(dt.dominates(br_ebb1_ebb0, br_ebb1_ebb0, &func.layout));
        assert!(!dt.dominates(br_ebb1_ebb0, jmp_ebb3_ebb1, &func.layout));
        assert!(dt.dominates(jmp_ebb3_ebb1, br_ebb1_ebb0, &func.layout));
    }
}
