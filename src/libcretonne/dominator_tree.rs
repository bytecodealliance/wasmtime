/// ! A Dominator Tree represented as mappings of Ebbs to their immediate dominator.

use cfg::*;
use ir::entities::Ebb;
use entity_map::EntityMap;

pub struct DominatorTree {
    data: EntityMap<Ebb, Option<Ebb>>,
}

impl DominatorTree {
    pub fn new(cfg: &ControlFlowGraph) -> DominatorTree {
        let mut dt = DominatorTree { data: EntityMap::new() };
        dt.build(cfg);
        dt
    }

    pub fn build(&mut self, cfg: &ControlFlowGraph) {
        let reverse_postorder_map = cfg.reverse_postorder_ebbs();
        let ebbs = reverse_postorder_map.keys().collect::<Vec<Ebb>>();
        let len = reverse_postorder_map.len();

        for (i, ebb) in ebbs.iter().enumerate() {
            if i > 0 {
                self.data.push(None);
            } else {
                self.data.push(Some(ebb.clone()));
            }
        }

        let mut changed = len > 0;

        while changed {
            changed = false;
            for i in 1..len {
                let ebb = ebbs[i];
                let preds = cfg.get_predecessors(ebb);
                let mut new_idom = None;

                for &(p, _) in preds {
                    if new_idom == None {
                        new_idom = Some(p);
                        continue;
                    }
                    if let Some(_) = self.data[p] {
                        new_idom =
                            Some(self.intersect(&reverse_postorder_map, p, new_idom.unwrap()));
                    }
                }
                match self.data[ebb] {
                    None => {
                        self.data[ebb] = new_idom;
                        changed = true;
                    }
                    Some(idom) => {
                        // Old idom != New idom
                        if idom != new_idom.unwrap() {
                            self.data[ebb] = new_idom;
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    fn intersect(&self, ordering: &EntityMap<Ebb, usize>, first: Ebb, second: Ebb) -> Ebb {
        println!("A {} B {}", first, second);
        let mut a = first;
        let mut b = second;
        while a != b {
            while ordering[a] < ordering[b] {
                a = self.data[a].unwrap();
            }
            while ordering[b] < ordering[a] {
                b = self.data[b].unwrap();
            }
        }
        a
    }

    pub fn idom(&self, ebb: Ebb) -> Option<Ebb> {
        self.data[ebb].clone()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ir::Function;
    use cfg::ControlFlowGraph;
    use test_utils::make_inst;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::new(&func);
        let dtree = DominatorTree::new(&cfg);
        assert_eq!(dtree.len(), 0);
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
        assert_eq!(dt.len(), cfg.len());
        assert_eq!(dt.idom(ebb3).unwrap(), ebb3);
        assert_eq!(dt.idom(ebb1).unwrap(), ebb3);
        assert_eq!(dt.idom(ebb2).unwrap(), ebb1);
        assert_eq!(dt.idom(ebb0).unwrap(), ebb1);
    }
}
