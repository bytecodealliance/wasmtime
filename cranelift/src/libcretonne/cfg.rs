//! A control flow graph represented as mappings of extended basic blocks to their predecessors.
//! Predecessors are denoted by tuples of EBB and branch/jump instructions. Each predecessor
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
use entities::{Inst, Ebb};
use instructions::InstructionData;
use std::collections::{BTreeSet, BTreeMap, btree_map};

/// A basic block denoted by its enclosing Ebb and last instruction.
pub type Predecessor = (Ebb, Inst);

/// Storing predecessors in a BTreeSet ensures that their ordering is
/// stable with no duplicates.
pub type PredecessorSet = BTreeSet<Predecessor>;

/// The Control Flow Graph maintains a mapping of ebbs to their predecessors
/// where predecessors are basic blocks.
#[derive(Debug)]
pub struct ControlFlowGraph {
    data: BTreeMap<Ebb, PredecessorSet>,
}

impl ControlFlowGraph {
    /// During initialization mappings will be generated for any existing
    /// blocks within the CFG's associated function. Basic sanity checks will
    /// also be performed to ensure that the blocks are well formed.
    pub fn new(func: &Function) -> Result<ControlFlowGraph, String> {
        let mut cfg = ControlFlowGraph { data: BTreeMap::new() };

        // Even ebbs without predecessors should show up in the CFG, albeit
        // with no entires.
        for ebb in func.ebbs_numerically() {
            try!(cfg.init_ebb(ebb));
        }

        for ebb in func.ebbs_numerically() {
            // Flips to true when a terminating instruction is seen. So that if additional
            // instructions occur an error may be returned.
            let mut terminated = false;
            for inst in func.ebb_insts(ebb) {
                if terminated {
                    return Err(format!("{} contains unreachable instructions.", ebb));
                }

                match func[inst] {
                    InstructionData::Branch { ty: _, opcode: _, ref data } => {
                        try!(cfg.add_predecessor(data.destination, (ebb, inst)));
                    }
                    InstructionData::Jump { ty: _, opcode: _, ref data } => {
                        try!(cfg.add_predecessor(data.destination, (ebb, inst)));
                        terminated = true;
                    }
                    InstructionData::Return { ty: _, opcode: _, data: _ } => {
                        terminated = true;
                    }
                    InstructionData::Nullary { ty: _, opcode: _ } => {
                        terminated = true;
                    }
                    _ => (),
                }
            }
        }
        Ok(cfg)
    }

    /// Initializes a predecessor set for some ebb. If an ebb already has an
    /// entry it will be clobbered.
    pub fn init_ebb(&mut self, ebb: Ebb) -> Result<&mut PredecessorSet, &'static str> {
        self.data.insert(ebb, BTreeSet::new());
        match self.data.get_mut(&ebb) {
            Some(predecessors) => Ok(predecessors),
            None => Err("Ebb initialization failed."),
        }
    }

    /// Attempts to add a predecessor for some ebb, attempting to initialize
    /// any ebb which has no entry.
    pub fn add_predecessor(&mut self,
                           ebb: Ebb,
                           predecessor: Predecessor)
                           -> Result<(), &'static str> {
        let success = match self.data.get_mut(&ebb) {
            Some(predecessors) => predecessors.insert(predecessor),
            None => false,
        };

        if success {
            Ok(())
        } else {
            let mut predecessors = try!(self.init_ebb(ebb));
            if predecessors.insert(predecessor) {
                return Ok(());
            }
            Err("Predecessor insertion failed.")
        }

    }

    /// Returns all of the predecessors for some ebb, if it has an entry.
    pub fn get_predecessors(&self, ebb: Ebb) -> Option<&PredecessorSet> {
        self.data.get(&ebb)
    }

    /// An iterator over all of the ebb to predecessor mappings in the CFG.
    pub fn iter<'a>(&'a self) -> btree_map::Iter<'a, Ebb, PredecessorSet> {
        self.data.iter()
    }
}

#[cfg(test)]
mod tests {
    use instructions::*;
    use entities::{Ebb, Inst, NO_VALUE};
    use repr::Function;
    use super::*;
    use types;

    // Some instructions will be re-used in several tests.

    fn nullary(func: &mut Function) -> Inst {
        func.make_inst(InstructionData::Nullary {
            opcode: Opcode::Iconst,
            ty: types::I32,
        })
    }

    fn jump(func: &mut Function, dest: Ebb) -> Inst {
        func.make_inst(InstructionData::Jump {
            opcode: Opcode::Jump,
            ty: types::VOID,
            data: Box::new(JumpData {
                destination: dest,
                arguments: VariableArgs::new(),
            }),
        })
    }

    fn branch(func: &mut Function, dest: Ebb) -> Inst {
        func.make_inst(InstructionData::Branch {
            opcode: Opcode::Brz,
            ty: types::VOID,
            data: Box::new(BranchData {
                arg: NO_VALUE,
                destination: dest,
                arguments: VariableArgs::new(),
            }),
        })
    }

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::new(&func).unwrap();
        assert_eq!(None, cfg.iter().next());
    }

    #[test]
    fn no_predecessors() {
        let mut func = Function::new();
        func.make_ebb();
        func.make_ebb();
        func.make_ebb();
        let cfg = ControlFlowGraph::new(&func).unwrap();
        let nodes = cfg.iter().collect::<Vec<_>>();
        assert_eq!(nodes.len(), 3);

        let mut fun_ebbs = func.ebbs_numerically();
        for (ebb, predecessors) in nodes {
            assert_eq!(ebb.index(), fun_ebbs.next().unwrap().index());
            assert_eq!(predecessors.len(), 0);
        }
    }

    #[test]
    #[should_panic(expected = "instructions")]
    fn nullable_before_branch() {
        // Ensure that branching after a nullary, within an ebb, triggers an error.
        let mut func = Function::new();
        let ebb0 = func.make_ebb();
        let ebb1_malformed = func.make_ebb();
        let ebb2 = func.make_ebb();

        let nullary_inst = nullary(&mut func);
        func.append_inst(ebb1_malformed, nullary_inst);

        // This jump should not be recorded since a nullary takes place
        // before it appears.
        let jmp_ebb1_ebb0 = jump(&mut func, ebb0);
        func.append_inst(ebb1_malformed, jmp_ebb1_ebb0);

        let jmp_ebb0_ebb2 = jump(&mut func, ebb2);
        func.append_inst(ebb0, jmp_ebb0_ebb2);

        ControlFlowGraph::new(&func).unwrap();
    }

    #[test]
    #[should_panic(expected = "instructions")]
    fn jump_before_branch() {
        // Ensure that branching after a jump, within an ebb, triggers an error.
        let mut func = Function::new();
        let ebb0 = func.make_ebb();
        let ebb1_malformed = func.make_ebb();
        let ebb2 = func.make_ebb();

        let jmp_ebb0_ebb1 = jump(&mut func, ebb2);
        func.append_inst(ebb0, jmp_ebb0_ebb1);

        let jmp_ebb1_ebb2 = jump(&mut func, ebb2);
        func.append_inst(ebb1_malformed, jmp_ebb1_ebb2);

        // This branch should not be recorded since a jump takes place
        // before it appears.
        let br_ebb1_ebb0 = branch(&mut func, ebb0);
        func.append_inst(ebb1_malformed, br_ebb1_ebb0);

        ControlFlowGraph::new(&func).unwrap();
    }

    #[test]
    fn branches_and_jumps() {
        let mut func = Function::new();
        let ebb0 = func.make_ebb();
        let ebb1 = func.make_ebb();
        let ebb2 = func.make_ebb();

        let br_ebb0_ebb2 = branch(&mut func, ebb2);
        func.append_inst(ebb0, br_ebb0_ebb2);

        let jmp_ebb0_ebb1 = jump(&mut func, ebb1);
        func.append_inst(ebb0, jmp_ebb0_ebb1);

        let br_ebb1_ebb1 = branch(&mut func, ebb1);
        func.append_inst(ebb1, br_ebb1_ebb1);

        let jmp_ebb1_ebb2 = jump(&mut func, ebb2);
        func.append_inst(ebb1, jmp_ebb1_ebb2);

        let cfg = ControlFlowGraph::new(&func).unwrap();
        let ebb0_predecessors = cfg.get_predecessors(ebb0).unwrap();
        let ebb1_predecessors = cfg.get_predecessors(ebb1).unwrap();
        let ebb2_predecessors = cfg.get_predecessors(ebb2).unwrap();
        assert_eq!(ebb0_predecessors.len(), 0);
        assert_eq!(ebb1_predecessors.len(), 2);
        assert_eq!(ebb2_predecessors.len(), 2);

        assert_eq!(ebb1_predecessors.contains(&(ebb0, jmp_ebb0_ebb1)), true);
        assert_eq!(ebb1_predecessors.contains(&(ebb1, br_ebb1_ebb1)), true);
        assert_eq!(ebb2_predecessors.contains(&(ebb0, br_ebb0_ebb2)), true);
        assert_eq!(ebb2_predecessors.contains(&(ebb1, jmp_ebb1_ebb2)), true);
    }

}
