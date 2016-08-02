//! A control flow graph represented as mappings of extended basic blocks to their predecessors
//! and successors. Successors are represented as extended basic blocks while predecessors are
//! represented by basic blocks.
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

use ir::Function;
use ir::entities::{Inst, Ebb};
use ir::instructions::BranchInfo;
use entity_map::{EntityMap, Keys};
use std::collections::HashSet;

/// A basic block denoted by its enclosing Ebb and last instruction.
pub type BasicBlock = (Ebb, Inst);

/// A container for the successors and predecessors of some Ebb.
#[derive(Debug, Clone, Default)]
pub struct CFGNode {
    pub successors: Vec<Ebb>,
    pub predecessors: Vec<BasicBlock>,
}

impl CFGNode {
    pub fn new() -> CFGNode {
        CFGNode {
            successors: Vec::new(),
            predecessors: Vec::new(),
        }
    }
}

/// The Control Flow Graph maintains a mapping of ebbs to their predecessors
/// and successors where predecessors are basic blocks and successors are
/// extended basic blocks.
#[derive(Debug)]
pub struct ControlFlowGraph {
    entry_block: Option<Ebb>,
    data: EntityMap<Ebb, CFGNode>,
}

impl ControlFlowGraph {
    /// During initialization mappings will be generated for any existing
    /// blocks within the CFG's associated function.
    pub fn new(func: &Function) -> ControlFlowGraph {

        let mut cfg = ControlFlowGraph {
            data: EntityMap::with_capacity(func.dfg.num_ebbs()),
            entry_block: func.layout.entry_block(),
        };

        for ebb in &func.layout {
            for inst in func.layout.ebb_insts(ebb) {
                match func.dfg[inst].analyze_branch() {
                    BranchInfo::SingleDest(dest, _) => {
                        cfg.add_edge((ebb, inst), dest);
                    }
                    BranchInfo::Table(jt) => {
                        for (_, dest) in func.jump_tables[jt].entries() {
                            cfg.add_edge((ebb, inst), dest);
                        }
                    }
                    BranchInfo::NotABranch => {}
                }
            }
        }
        cfg
    }

    fn add_edge(&mut self, from: BasicBlock, to: Ebb) {
        self.data[from.0].successors.push(to);
        self.data[to].predecessors.push(from);
    }

    pub fn get_predecessors(&self, ebb: Ebb) -> &Vec<BasicBlock> {
        &self.data[ebb].predecessors
    }

    pub fn get_successors(&self, ebb: Ebb) -> &Vec<Ebb> {
        &self.data[ebb].successors
    }

    /// Return [reachable] ebbs in postorder.
    pub fn postorder_ebbs(&self) -> Vec<Ebb> {
        let entry_block = match self.entry_block {
            None => {
                return Vec::new();
            }
            Some(eb) => eb,
        };

        let mut grey = HashSet::new();
        let mut black = HashSet::new();
        let mut stack = vec![entry_block.clone()];
        let mut postorder = Vec::new();

        while !stack.is_empty() {
            let node = stack.pop().unwrap();
            if !grey.contains(&node) {
                // This is a white node. Mark it as gray.
                grey.insert(node);
                stack.push(node);
                // Get any children we’ve never seen before.
                for child in self.get_successors(node) {
                    if !grey.contains(child) {
                        stack.push(child.clone());
                    }
                }
            } else if !black.contains(&node) {
                postorder.push(node.clone());
                black.insert(node.clone());
            }
        }
        postorder
    }

    /// An iterator across all of the ebbs stored in the cfg.
    pub fn ebbs(&self) -> Keys<Ebb> {
        self.data.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::Function;

    use test_utils::make_inst;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::new(&func);
        assert_eq!(None, cfg.ebbs().next());
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
        let nodes = cfg.ebbs().collect::<Vec<_>>();
        assert_eq!(nodes.len(), 3);

        let mut fun_ebbs = func.layout.ebbs();
        for ebb in nodes {
            assert_eq!(ebb, fun_ebbs.next().unwrap());
            assert_eq!(cfg.get_predecessors(ebb).len(), 0);
            assert_eq!(cfg.get_successors(ebb).len(), 0);
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

        let ebb0_successors = cfg.get_successors(ebb0);
        let ebb1_successors = cfg.get_successors(ebb1);
        let ebb2_successors = cfg.get_successors(ebb2);

        assert_eq!(ebb0_predecessors.len(), 0);
        assert_eq!(ebb1_predecessors.len(), 2);
        assert_eq!(ebb2_predecessors.len(), 2);

        assert_eq!(ebb1_predecessors.contains(&(ebb0, jmp_ebb0_ebb1)), true);
        assert_eq!(ebb1_predecessors.contains(&(ebb1, br_ebb1_ebb1)), true);
        assert_eq!(ebb2_predecessors.contains(&(ebb0, br_ebb0_ebb2)), true);
        assert_eq!(ebb2_predecessors.contains(&(ebb1, jmp_ebb1_ebb2)), true);

        assert_eq!(ebb0_successors.len(), 2);
        assert_eq!(ebb1_successors.len(), 2);
        assert_eq!(ebb2_successors.len(), 0);

        assert_eq!(ebb0_successors.contains(&ebb1), true);
        assert_eq!(ebb0_successors.contains(&ebb2), true);
        assert_eq!(ebb1_successors.contains(&ebb1), true);
        assert_eq!(ebb1_successors.contains(&ebb2), true);
    }
}
