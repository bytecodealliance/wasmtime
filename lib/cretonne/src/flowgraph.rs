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
use entity_map::EntityMap;
use std::mem;

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
            self.compute_ebb(func, ebb);
        }
    }

    fn compute_ebb(&mut self, func: &Function, ebb: Ebb) {
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

    fn invalidate_ebb_successors(&mut self, ebb: Ebb) {
        // Temporarily take ownership because we need mutable access to self.data inside the loop.
        // Unfortunately borrowck cannot see that our mut accesses to predecessors don't alias
        // our iteration over successors.
        let mut successors = mem::replace(&mut self.data[ebb].successors, Vec::new());
        for suc in successors.iter().cloned() {
            self.data[suc].predecessors.retain(|&(e, _)| e != ebb);
        }
        successors.clear();
        self.data[ebb].successors = successors;
    }

    /// Recompute the control flow graph of `ebb`.
    ///
    /// This is for use after modifying instructions within a specific EBB. It recomputes all edges
    /// from `ebb` while leaving edges to `ebb` intact. Its functionality a subset of that of the
    /// more expensive `compute`, and should be used when we know we don't need to recompute the CFG
    /// from scratch, but rather that our changes have been restricted to specific EBBs.
    pub fn recompute_ebb(&mut self, func: &Function, ebb: Ebb) {
        self.invalidate_ebb_successors(ebb);
        self.compute_ebb(func, ebb);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::{Function, InstBuilder, Cursor, types};

    #[test]
    fn empty() {
        let func = Function::new();
        ControlFlowGraph::with_function(&func);
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

        let mut fun_ebbs = func.layout.ebbs();
        for ebb in func.layout.ebbs() {
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

        let mut cfg = ControlFlowGraph::with_function(&func);

        {
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

        // Change some instructions and recompute ebb0
        func.dfg.replace(br_ebb0_ebb2).brnz(cond, ebb1, &[]);
        func.dfg.replace(jmp_ebb0_ebb1).return_(&[]);
        cfg.recompute_ebb(&mut func, ebb0);
        let br_ebb0_ebb1 = br_ebb0_ebb2;

        {
            let ebb0_predecessors = cfg.get_predecessors(ebb0);
            let ebb1_predecessors = cfg.get_predecessors(ebb1);
            let ebb2_predecessors = cfg.get_predecessors(ebb2);

            let ebb0_successors = cfg.get_successors(ebb0);
            let ebb1_successors = cfg.get_successors(ebb1);
            let ebb2_successors = cfg.get_successors(ebb2);

            assert_eq!(ebb0_predecessors.len(), 0);
            assert_eq!(ebb1_predecessors.len(), 2);
            assert_eq!(ebb2_predecessors.len(), 1);

            assert_eq!(ebb1_predecessors.contains(&(ebb0, br_ebb0_ebb1)), true);
            assert_eq!(ebb1_predecessors.contains(&(ebb1, br_ebb1_ebb1)), true);
            assert_eq!(ebb2_predecessors.contains(&(ebb0, br_ebb0_ebb2)), false);
            assert_eq!(ebb2_predecessors.contains(&(ebb1, jmp_ebb1_ebb2)), true);

            assert_eq!(ebb0_successors.len(), 1);
            assert_eq!(ebb1_successors.len(), 2);
            assert_eq!(ebb2_successors.len(), 0);

            assert_eq!(ebb0_successors.contains(&ebb1), true);
            assert_eq!(ebb0_successors.contains(&ebb2), false);
            assert_eq!(ebb1_successors.contains(&ebb1), true);
            assert_eq!(ebb1_successors.contains(&ebb2), true);
        }
    }
}
