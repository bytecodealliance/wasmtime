/// ! A Dominator Tree represented as mappings of Ebbs to their immediate dominator.

use cfg::*;
use ir::Ebb;
use ir::entities::NO_INST;
use entity_map::{EntityMap, Keys};

pub struct DominatorTree {
    data: EntityMap<Ebb, Option<BasicBlock>>,
}

impl DominatorTree {
    /// Build a dominator tree from a control flow graph using Keith D. Cooper's
    /// "Simple, Fast Dominator Algorithm."
    pub fn new(cfg: &ControlFlowGraph) -> DominatorTree {
        let mut ebbs = cfg.postorder_ebbs();
        ebbs.reverse();

        let len = ebbs.len();

        // The mappings which designate the dominator tree.
        let mut data = EntityMap::with_capacity(len);

        let mut postorder_map = EntityMap::with_capacity(len);
        for (i, ebb) in ebbs.iter().enumerate() {
            postorder_map[ebb.clone()] = len - i;
        }

        let mut changed = false;

        if len > 0 {
            data[ebbs[0]] = Some((ebbs[0], NO_INST));
            changed = true;
        }

        while changed {
            changed = false;
            for i in 1..len {
                let ebb = ebbs[i];
                let preds = cfg.get_predecessors(ebb);
                let mut new_idom = None;

                for &(p_ebb, _) in preds {
                    if new_idom == None {
                        new_idom = Some((p_ebb, NO_INST));
                        continue;
                    }
                    // If this predecessor has an idom available find its common
                    // ancestor with the current value of new_idom.
                    if let Some(_) = data[p_ebb] {
                        new_idom = match new_idom {
                            Some(cur_idom) => {
                                Some((DominatorTree::intersect(&mut data,
                                                               &postorder_map,
                                                               p_ebb,
                                                               cur_idom.0),
                                      NO_INST))
                            }
                            None => panic!("A 'current idom' should have been set!"),
                        }
                    }
                }
                match data[ebb] {
                    None => {
                        data[ebb] = new_idom;
                        changed = true;
                    }
                    Some(idom) => {
                        // Old idom != New idom
                        if idom != new_idom.unwrap() {
                            data[ebb] = new_idom;
                            changed = true;
                        }
                    }
                }
            }
        }

        // At this point the basic blocks in the tree are incomplete
        // since they have all been set with NO_INST. Here we add instructions
        // by iterating through each Ebb -> BasicBlock mapping in the dominator
        // tree and replacing the basic block with a corresponding predecessor
        // from the Ebb (on the left hand side).
        //
        // The predecessor chosen should have the lowest instruction number and
        // an Ebb which matches the Ebb from the dummy basic block. Because
        // extended basic blocks have a single entry point this will always
        // result in the correct basic block being chosen.
        for lhs_ebb in ebbs {
            let rhs_bb = data[lhs_ebb].unwrap();
            for pred_bb in cfg.get_predecessors(lhs_ebb) {
                if rhs_bb.0 == pred_bb.0 {
                    // Predecessors are added in order while iterating through
                    // instructions from lowest to highest. Because of this,
                    // the first match we encounter will have the lowest instruction
                    // number.
                    data[lhs_ebb] = Some(pred_bb.clone());
                    break;
                }
            }
        }

        DominatorTree { data: data }
    }

    /// Find the common dominator of two ebbs.
    fn intersect(data: &EntityMap<Ebb, Option<BasicBlock>>,
                 ordering: &EntityMap<Ebb, usize>,
                 first: Ebb,
                 second: Ebb)
                 -> Ebb {
        let mut a = first;
        let mut b = second;

        // Here we use 'ordering', a mapping of ebbs to their postorder
        // visitation number, to ensure that we move upward through the tree.
        // Walking upward means that we may always expect self.data[a] and
        // self.data[b] to contain non-None entries.
        while a != b {
            while ordering[a] < ordering[b] {
                a = data[a].unwrap().0;
            }
            while ordering[b] < ordering[a] {
                b = data[b].unwrap().0;
            }
        }
        a
    }

    /// Returns the immediate dominator of some ebb or None if the
    /// node is unreachable.
    pub fn idom(&self, ebb: Ebb) -> Option<BasicBlock> {
        self.data[ebb].clone()
    }

    /// An iterator across all of the ebbs stored in the tree.
    pub fn ebbs(&self) -> Keys<Ebb> {
        self.data.keys()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ir::Function;
    use ir::entities::NO_INST;
    use cfg::ControlFlowGraph;
    use test_utils::make_inst;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::new(&func);
        let dtree = DominatorTree::new(&cfg);
        assert_eq!(None, dtree.ebbs().next());
    }

    #[test]
    fn non_zero_entry_block() {
        let mut func = Function::new();
        let ebb3 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        let ebb0 = func.dfg.make_ebb();
        func.layout.append_ebb(ebb3);
        func.layout.append_ebb(ebb1);
        func.layout.append_ebb(ebb2);
        func.layout.append_ebb(ebb0);

        let jmp_ebb3_ebb1 = make_inst::jump(&mut func, ebb1);
        let br_ebb1_ebb0 = make_inst::branch(&mut func, ebb0);
        let jmp_ebb1_ebb2 = make_inst::jump(&mut func, ebb2);
        let jmp_ebb2_ebb0 = make_inst::jump(&mut func, ebb0);

        func.layout.append_inst(br_ebb1_ebb0, ebb1);
        func.layout.append_inst(jmp_ebb1_ebb2, ebb1);
        func.layout.append_inst(jmp_ebb2_ebb0, ebb2);
        func.layout.append_inst(jmp_ebb3_ebb1, ebb3);

        let cfg = ControlFlowGraph::new(&func);
        let dt = DominatorTree::new(&cfg);

        assert_eq!(func.layout.entry_block().unwrap(), ebb3);
        assert_eq!(dt.idom(ebb3).unwrap(), (ebb3, NO_INST));
        assert_eq!(dt.idom(ebb1).unwrap(), (ebb3, jmp_ebb3_ebb1));
        assert_eq!(dt.idom(ebb2).unwrap(), (ebb1, jmp_ebb1_ebb2));
        assert_eq!(dt.idom(ebb0).unwrap(), (ebb1, br_ebb1_ebb0));
    }
}
