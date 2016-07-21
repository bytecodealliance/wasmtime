//! A control flow graph represented as mappings of extended basic blocks to their predecessors.
//! BasicBlocks are denoted by tuples of EBB and branch/jump instructions. Each predecessor
//! tuple corresponds to the end of a basic block.
//!
//! ```c
//!     Ebb0:
//!         ...          ; beginning of basic block
//!
//!         ...
//!
//!         brz vx, Ebb1 ; end of basic block
//!
//!         ...          ; beginning of basic block
//!
//!         ...
//!
//!         jmp Ebb2     ; end of basic block
//! ```
//!
//! Here Ebb1 and Ebb2 would each have a single predecessor denoted as (Ebb0, `brz vx, Ebb1`)
//! and (Ebb0, `jmp Ebb2`) respectively.

use repr::Function;
use repr::entities::{Inst, Ebb};
use repr::instructions::InstructionData;
use entity_map::EntityMap;
use std::collections::BTreeSet;

/// A basic block denoted by its enclosing Ebb and last instruction.
pub type BasicBlock = (Ebb, Inst);

/// Storing predecessors in a BTreeSet ensures that their ordering is
/// stable with no duplicates.
pub type BasicBlockSet = BTreeSet<BasicBlock>;

/// The Control Flow Graph maintains a mapping of ebbs to their predecessors
/// where predecessors are basic blocks.
#[derive(Debug)]
pub struct ControlFlowGraph {
    data: EntityMap<Ebb, BasicBlockSet>,
}

impl ControlFlowGraph {
    /// During initialization mappings will be generated for any existing
    /// blocks within the CFG's associated function. Basic sanity checks will
    /// also be performed to ensure that the blocks are well formed.
    pub fn new(func: &Function) -> ControlFlowGraph {
        let mut cfg = ControlFlowGraph { data: EntityMap::new() };

        // Even ebbs without predecessors should show up in the CFG, albeit
        // with no entires.
        for _ in &func.layout {
            cfg.push_ebb();
        }

        for ebb in &func.layout {
            // Flips to true when a terminating instruction is seen. So that if additional
            // instructions occur an error may be returned.
            for inst in func.layout.ebb_insts(ebb) {
                match func.dfg[inst] {
                    InstructionData::Branch { ty: _, opcode: _, ref data } => {
                        cfg.add_predecessor(data.destination, (ebb, inst));
                    }
                    InstructionData::Jump { ty: _, opcode: _, ref data } => {
                        cfg.add_predecessor(data.destination, (ebb, inst));
                    }
                    _ => (),
                }
            }
        }
        cfg
    }

    pub fn push_ebb(&mut self) {
        self.data.push(BTreeSet::new());
    }

    pub fn add_predecessor(&mut self, ebb: Ebb, predecessor: BasicBlock) {
        self.data[ebb].insert(predecessor);
    }

    /// Returns all of the predecessors for some ebb, if it has an entry.
    pub fn get_predecessors(&self, ebb: Ebb) -> &BasicBlockSet {
        &self.data[ebb]
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> CFGIter {
        CFGIter {
            cur: 0,
            cfg: &self,
        }
    }
}

pub struct CFGIter<'a> {
    cfg: &'a ControlFlowGraph,
    cur: usize,
}

impl<'a> Iterator for CFGIter<'a> {
    type Item = (Ebb, &'a BasicBlockSet);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.cfg.len() {
            let ebb = Ebb::with_number(self.cur as u32).unwrap();
            let bbs = self.cfg.get_predecessors(ebb);
            self.cur += 1;
            Some((ebb, bbs))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repr::Function;

    use test_utils::make_inst;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::new(&func);
        assert_eq!(None, cfg.iter().next());
    }

    #[test]
    fn no_predecessors() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        func.layout.append_ebb(ebb0);
        func.layout.append_ebb(ebb1);
        func.layout.append_ebb(ebb2);

        let cfg = ControlFlowGraph::new(&func);
        let nodes = cfg.iter().collect::<Vec<_>>();
        assert_eq!(nodes.len(), 3);

        let mut fun_ebbs = func.layout.ebbs();
        for (ebb, predecessors) in nodes {
            assert_eq!(ebb, fun_ebbs.next().unwrap());
            assert_eq!(predecessors.len(), 0);
        }
    }

    #[test]
    fn branches_and_jumps() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        func.layout.append_ebb(ebb0);
        func.layout.append_ebb(ebb1);
        func.layout.append_ebb(ebb2);

        let br_ebb0_ebb2 = make_inst::branch(&mut func, ebb2);
        func.layout.append_inst(br_ebb0_ebb2, ebb0);

        let jmp_ebb0_ebb1 = make_inst::jump(&mut func, ebb1);
        func.layout.append_inst(jmp_ebb0_ebb1, ebb0);

        let br_ebb1_ebb1 = make_inst::branch(&mut func, ebb1);
        func.layout.append_inst(br_ebb1_ebb1, ebb1);

        let jmp_ebb1_ebb2 = make_inst::jump(&mut func, ebb2);
        func.layout.append_inst(jmp_ebb1_ebb2, ebb1);

        let cfg = ControlFlowGraph::new(&func);
        let ebb0_predecessors = cfg.get_predecessors(ebb0);
        let ebb1_predecessors = cfg.get_predecessors(ebb1);
        let ebb2_predecessors = cfg.get_predecessors(ebb2);
        assert_eq!(ebb0_predecessors.len(), 0);
        assert_eq!(ebb1_predecessors.len(), 2);
        assert_eq!(ebb2_predecessors.len(), 2);

        assert_eq!(ebb1_predecessors.contains(&(ebb0, jmp_ebb0_ebb1)), true);
        assert_eq!(ebb1_predecessors.contains(&(ebb1, br_ebb1_ebb1)), true);
        assert_eq!(ebb2_predecessors.contains(&(ebb0, br_ebb0_ebb2)), true);
        assert_eq!(ebb2_predecessors.contains(&(ebb1, jmp_ebb1_ebb2)), true);
    }
}
