//! A control flow graph represented as mappings of extended basic blocks to their predecessors
//! and successors.
//!
//! Successors are represented as extended basic blocks while predecessors are represented by basic
//! blocks. Basic blocks are denoted by tuples of EBB and branch/jump instructions. Each
//! predecessor tuple corresponds to the end of a basic block.
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
//! Here `Ebb1` and `Ebb2` would each have a single predecessor denoted as `(Ebb0, brz)`
//! and `(Ebb0, jmp Ebb2)` respectively.

use ir::{Function, Inst, Ebb};
use ir::instructions::BranchInfo;
use entity_map::{EntityMap, Keys};
use std::collections::HashSet;

/// A basic block denoted by its enclosing Ebb and last instruction.
pub type BasicBlock = (Ebb, Inst);

/// A container for the successors and predecessors of some Ebb.
#[derive(Debug, Clone, Default)]
pub struct CFGNode {
    /// EBBs that are the targets of branches and jumps in this EBB.
    pub successors: Vec<Ebb>,
    /// Basic blocks that can branch or jump to this EBB.
    pub predecessors: Vec<BasicBlock>,
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
    /// Allocate a new blank control flow graph.
    pub fn new() -> ControlFlowGraph {
        ControlFlowGraph {
            entry_block: None,
            data: EntityMap::new(),
        }
    }

    /// Allocate and compute the control flow graph for `func`.
    pub fn with_function(func: &Function) -> ControlFlowGraph {
        let mut cfg = ControlFlowGraph::new();
        cfg.compute(func);
        cfg
    }

    /// Compute the control flow graph of `func`.
    ///
    /// This will clear and overwrite any information already stored in this data structure.
    pub fn compute(&mut self, func: &Function) {
        self.entry_block = func.layout.entry_block();
        self.data.clear();
        self.data.resize(func.dfg.num_ebbs());

        for ebb in &func.layout {
            for inst in func.layout.ebb_insts(ebb) {
                match func.dfg[inst].analyze_branch(&func.dfg.value_lists) {
                    BranchInfo::SingleDest(dest, _) => {
                        self.add_edge((ebb, inst), dest);
                    }
                    BranchInfo::Table(jt) => {
                        for (_, dest) in func.jump_tables[jt].entries() {
                            self.add_edge((ebb, inst), dest);
                        }
                    }
                    BranchInfo::NotABranch => {}
                }
            }
        }
    }

    fn add_edge(&mut self, from: BasicBlock, to: Ebb) {
        self.data[from.0].successors.push(to);
        self.data[to].predecessors.push(from);
    }

    /// Get the CFG predecessor basic blocks to `ebb`.
    pub fn get_predecessors(&self, ebb: Ebb) -> &[BasicBlock] {
        &self.data[ebb].predecessors
    }

    /// Get the CFG successors to `ebb`.
    pub fn get_successors(&self, ebb: Ebb) -> &[Ebb] {
        &self.data[ebb].successors
    }

    /// Return [reachable] ebbs in post-order.
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
                // Get any children we've never seen before.
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

    /// An iterator across all of the ebbs stored in the CFG.
    pub fn ebbs(&self) -> Keys<Ebb> {
        self.data.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::{Function, InstBuilder, Cursor, types};

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::with_function(&func);
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

        let cfg = ControlFlowGraph::with_function(&func);
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
        let cond = func.dfg.append_ebb_arg(ebb0, types::I32);
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();

        let br_ebb0_ebb2;
        let br_ebb1_ebb1;
        let jmp_ebb0_ebb1;
        let jmp_ebb1_ebb2;

        {
            let dfg = &mut func.dfg;
            let cur = &mut Cursor::new(&mut func.layout);

            cur.insert_ebb(ebb0);
            br_ebb0_ebb2 = dfg.ins(cur).brnz(cond, ebb2, &[]);
            jmp_ebb0_ebb1 = dfg.ins(cur).jump(ebb1, &[]);

            cur.insert_ebb(ebb1);
            br_ebb1_ebb1 = dfg.ins(cur).brnz(cond, ebb1, &[]);
            jmp_ebb1_ebb2 = dfg.ins(cur).jump(ebb2, &[]);

            cur.insert_ebb(ebb2);
        }

        let cfg = ControlFlowGraph::with_function(&func);

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
