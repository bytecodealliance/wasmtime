//! Topological order of EBBs, according to the dominator tree.

use dominator_tree::DominatorTree;
use entity::SparseSet;
use ir::{Ebb, Layout};
use std::vec::Vec;

/// Present EBBs in a topological order such that all dominating EBBs are guaranteed to be visited
/// before the current EBB.
///
/// There are many topological orders of the EBBs in a function, so it is possible to provide a
/// preferred order, and the `TopoOrder` will present EBBs in an order that is as close as possible
/// to the preferred order.
pub struct TopoOrder {
    /// Preferred order of EBBs to visit.
    preferred: Vec<Ebb>,

    /// Next entry to get from `preferred`.
    next: usize,

    /// Set of visited EBBs.
    visited: SparseSet<Ebb>,

    /// Stack of EBBs to be visited next, already in `visited`.
    stack: Vec<Ebb>,
}

impl TopoOrder {
    /// Create a new empty topological order.
    pub fn new() -> Self {
        Self {
            preferred: Vec::new(),
            next: 0,
            visited: SparseSet::new(),
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

    /// Reset and initialize with a preferred sequence of EBBs. The resulting topological order is
    /// guaranteed to contain all of the EBBs in `preferred` as well as any dominators.
    pub fn reset<Ebbs>(&mut self, preferred: Ebbs)
    where
        Ebbs: IntoIterator<Item = Ebb>,
    {
        self.preferred.clear();
        self.preferred.extend(preferred);
        self.next = 0;
        self.visited.clear();
        self.stack.clear();
    }

    /// Get the next EBB in the topological order.
    ///
    /// Two things are guaranteed about the EBBs returned by this function:
    ///
    /// - All EBBs in the `preferred` iterator given to `reset` will be returned.
    /// - All dominators are visited before the EBB returned.
    pub fn next(&mut self, layout: &Layout, domtree: &DominatorTree) -> Option<Ebb> {
        // Any entries in `stack` should be returned immediately. They have already been added to
        // `visited`.
        while self.stack.is_empty() {
            match self.preferred.get(self.next).cloned() {
                None => return None,
                Some(mut ebb) => {
                    // We have the next EBB in the preferred order.
                    self.next += 1;
                    // Push it along with any non-visited dominators.
                    while self.visited.insert(ebb).is_none() {
                        self.stack.push(ebb);
                        match domtree.idom(ebb) {
                            Some(idom) => ebb = layout.inst_ebb(idom).expect("idom not in layout"),
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
    use cursor::{Cursor, FuncCursor};
    use dominator_tree::DominatorTree;
    use flowgraph::ControlFlowGraph;
    use ir::{Function, InstBuilder};
    use std::iter;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let mut topo = TopoOrder::new();

        assert_eq!(topo.next(&func.layout, &domtree), None);
        topo.reset(func.layout.ebbs());
        assert_eq!(topo.next(&func.layout, &domtree), None);
    }

    #[test]
    fn simple() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();

        {
            let mut cur = FuncCursor::new(&mut func);

            cur.insert_ebb(ebb0);
            cur.ins().jump(ebb1, &[]);
            cur.insert_ebb(ebb1);
            cur.ins().jump(ebb1, &[]);
        }

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let mut topo = TopoOrder::new();

        topo.reset(iter::once(ebb1));
        assert_eq!(topo.next(&func.layout, &domtree), Some(ebb0));
        assert_eq!(topo.next(&func.layout, &domtree), Some(ebb1));
        assert_eq!(topo.next(&func.layout, &domtree), None);
    }
}
