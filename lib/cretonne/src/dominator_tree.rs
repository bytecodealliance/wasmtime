/// ! A Dominator Tree represented as mappings of Ebbs to their immediate dominator.

use cfg::*;
use ir::Ebb;
use ir::entities::NO_INST;
use entity_map::EntityMap;

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

                for pred in preds {
                    if new_idom == None {
                        new_idom = Some(pred.clone());
                        continue;
                    }
                    // If this predecessor has an idom available find its common
                    // ancestor with the current value of new_idom.
                    if let Some(_) = data[pred.0] {
                        new_idom = match new_idom {
                            Some(cur_idom) => {
                                Some((DominatorTree::intersect(&mut data,
                                                               &postorder_map,
                                                               *pred,
                                                               cur_idom)))
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
                        if idom.0 != new_idom.unwrap().0 {
                            data[ebb] = new_idom;
                            changed = true;
                        }
                    }
                }
            }
        }

        DominatorTree { data: data }
    }

    /// Find the common dominator of two ebbs.
    fn intersect(data: &EntityMap<Ebb, Option<BasicBlock>>,
                 ordering: &EntityMap<Ebb, usize>,
                 first: BasicBlock,
                 second: BasicBlock)
                 -> BasicBlock {
        let mut a = first;
        let mut b = second;

        // Here we use 'ordering', a mapping of ebbs to their postorder
        // visitation number, to ensure that we move upward through the tree.
        // Walking upward means that we may always expect self.data[a] and
        // self.data[b] to contain non-None entries.
        while a.0 != b.0 {
            while ordering[a.0] < ordering[b.0] {
                a = data[a.0].unwrap();
            }
            while ordering[b.0] < ordering[a.0] {
                b = data[b.0].unwrap();
            }
        }

        // TODO: we can't rely on instruction numbers to always be ordered
        // from lowest to highest. Given that, it will be necessary to create
        // an abolute mapping to determine the instruction order in the future.
        if a.1 == NO_INST || a.1 < b.1 { a } else { b }
    }

    /// Returns the immediate dominator of some ebb or None if the
    /// node is unreachable.
    pub fn idom(&self, ebb: Ebb) -> Option<BasicBlock> {
        self.data[ebb].clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ir::{Function, InstBuilder, Cursor, VariableArgs, types};
    use ir::entities::NO_INST;
    use cfg::ControlFlowGraph;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::new(&func);
        let dtree = DominatorTree::new(&cfg);
        assert_eq!(0, dtree.data.keys().count());
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
            jmp_ebb3_ebb1 = dfg.ins(cur).jump(ebb1, VariableArgs::new());

            cur.insert_ebb(ebb1);
            br_ebb1_ebb0 = dfg.ins(cur).brnz(cond, ebb0, VariableArgs::new());
            jmp_ebb1_ebb2 = dfg.ins(cur).jump(ebb2, VariableArgs::new());

            cur.insert_ebb(ebb2);
            dfg.ins(cur).jump(ebb0, VariableArgs::new());

            cur.insert_ebb(ebb0);
        }

        let cfg = ControlFlowGraph::new(&func);
        let dt = DominatorTree::new(&cfg);

        assert_eq!(func.layout.entry_block().unwrap(), ebb3);
        assert_eq!(dt.idom(ebb3).unwrap(), (ebb3, NO_INST));
        assert_eq!(dt.idom(ebb1).unwrap(), (ebb3, jmp_ebb3_ebb1));
        assert_eq!(dt.idom(ebb2).unwrap(), (ebb1, jmp_ebb1_ebb2));
        assert_eq!(dt.idom(ebb0).unwrap(), (ebb1, br_ebb1_ebb0));
    }
}
