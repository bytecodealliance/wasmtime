//! Split the outgoing edges of conditional branches that pass parameters.
//!
//! One of the reason for splitting edges is to be able to insert `copy` and `regmove` instructions
//! between a conditional branch and the following terminator.
#![cfg(feature = "basic-blocks")]

use std::vec::Vec;

use crate::cursor::{Cursor, EncCursor};
use crate::dominator_tree::DominatorTree;
use crate::flowgraph::ControlFlowGraph;
use crate::ir::{Ebb, Function, Inst, InstBuilder, InstructionData, Opcode, ValueList};
use crate::isa::TargetIsa;
use crate::topo_order::TopoOrder;

pub fn run(
    isa: &dyn TargetIsa,
    func: &mut Function,
    cfg: &mut ControlFlowGraph,
    domtree: &mut DominatorTree,
    topo: &mut TopoOrder,
) {
    let mut ctx = Context {
        has_new_blocks: false,
        has_fallthrough_return: None,
        cur: EncCursor::new(func, isa),
        domtree,
        topo,
        cfg,
    };
    ctx.run()
}

struct Context<'a> {
    /// True if new blocks were inserted.
    has_new_blocks: bool,

    /// Record whether newly inserted empty blocks should be inserted last, or before the last, to
    /// avoid disturbing the expected control flow of `fallthroug_return` statements.
    ///
    /// This value is computed when needed. The Option wraps the computed value if any.
    has_fallthrough_return: Option<bool>,

    /// Current instruction as well as reference to function and ISA.
    cur: EncCursor<'a>,

    /// References to contextual data structures we need.
    domtree: &'a mut DominatorTree,
    topo: &'a mut TopoOrder,
    cfg: &'a mut ControlFlowGraph,
}

impl<'a> Context<'a> {
    fn run(&mut self) {
        // Any ebb order will do.
        self.topo.reset(self.cur.func.layout.ebbs());
        while let Some(ebb) = self.topo.next(&self.cur.func.layout, self.domtree) {
            // Branches can only be at the last or second to last position in an extended basic
            // block.
            self.cur.goto_last_inst(ebb);
            let terminator_inst = self.cur.current_inst().expect("terminator");
            if let Some(inst) = self.cur.prev_inst() {
                let opcode = self.cur.func.dfg[inst].opcode();
                if opcode.is_branch() {
                    self.visit_conditional_branch(inst, opcode);
                    self.cur.goto_inst(terminator_inst);
                    self.visit_terminator_branch(terminator_inst);
                }
            }
        }

        // If blocks were added the cfg and domtree are inconsistent and must be recomputed.
        if self.has_new_blocks {
            self.cfg.compute(&self.cur.func);
            self.domtree.compute(&self.cur.func, self.cfg);
        }
    }

    fn visit_conditional_branch(&mut self, branch: Inst, opcode: Opcode) {
        // TODO: target = dfg[branch].branch_destination().expect("conditional branch");
        let target = match self.cur.func.dfg[branch] {
            InstructionData::Branch { destination, .. }
            | InstructionData::BranchIcmp { destination, .. }
            | InstructionData::BranchInt { destination, .. }
            | InstructionData::BranchFloat { destination, .. } => destination,
            _ => panic!("Unexpected instruction in visit_conditional_branch"),
        };

        // If there are any parameters, split the edge.
        if self.should_split_edge(target) {
            // Create the block the branch will jump to.
            let new_ebb = self.make_empty_ebb();

            // Extract the arguments of the branch instruction, split the Ebb parameters and the
            // branch arguments
            let num_fixed = opcode.constraints().num_fixed_value_arguments();
            let dfg = &mut self.cur.func.dfg;
            let old_args: Vec<_> = {
                let args = dfg[branch].take_value_list().expect("ebb parameters");
                args.as_slice(&dfg.value_lists).iter().map(|x| *x).collect()
            };
            let (branch_args, ebb_params) = old_args.split_at(num_fixed);

            // Replace the branch destination by the new Ebb created with no parameters, and restore
            // the branch arguments, without the original Ebb parameters.
            {
                let branch_args = ValueList::from_slice(branch_args, &mut dfg.value_lists);
                let data = &mut dfg[branch];
                *data.branch_destination_mut().expect("branch") = new_ebb;
                data.put_value_list(branch_args);
            }
            let ok = self.cur.func.update_encoding(branch, self.cur.isa).is_ok();
            debug_assert!(ok);

            // Insert a jump to the original target with its arguments into the new block.
            self.cur.goto_first_insertion_point(new_ebb);
            self.cur.ins().jump(target, ebb_params);

            // Reset the cursor to point to the branch.
            self.cur.goto_inst(branch);
        }
    }

    fn visit_terminator_branch(&mut self, inst: Inst) {
        let inst_data = &self.cur.func.dfg[inst];
        let opcode = inst_data.opcode();
        if opcode != Opcode::Jump && opcode != Opcode::Fallthrough {
            // This opcode is ignored as it does not have any EBB parameters.
            if opcode != Opcode::IndirectJumpTableBr {
                debug_assert!(!opcode.is_branch())
            }
            return;
        }

        let target = match inst_data {
            InstructionData::Jump { destination, .. } => destination,
            _ => panic!(
                "Unexpected instruction {} in visit_terminator_branch",
                self.cur.display_inst(inst)
            ),
        };
        debug_assert!(self.cur.func.dfg[inst].opcode().is_terminator());

        // If there are any parameters, split the edge.
        if self.should_split_edge(*target) {
            // Create the block the branch will jump to.
            let new_ebb = self.cur.func.dfg.make_ebb();
            self.has_new_blocks = true;

            // Split the current block before its terminator, and insert a new jump instruction to
            // jump to it.
            let jump = self.cur.ins().jump(new_ebb, &[]);
            self.cur.insert_ebb(new_ebb);

            // Reset the cursor to point to new terminator of the old ebb.
            self.cur.goto_inst(jump);
        }
    }

    // A new ebb must be inserted before the last ebb because the last ebb may have a
    // fallthrough_return and can't have anything after it.
    fn make_empty_ebb(&mut self) -> Ebb {
        let last_ebb = self.cur.layout().last_ebb().unwrap();
        if self.has_fallthrough_return == None {
            let last_inst = self.cur.layout().last_inst(last_ebb).unwrap();
            self.has_fallthrough_return =
                Some(self.cur.func.dfg[last_inst].opcode() == Opcode::FallthroughReturn);
        }
        let new_ebb = self.cur.func.dfg.make_ebb();
        if self.has_fallthrough_return == Some(true) {
            // Insert before the last block which has a fallthrough_return
            // instruction.
            self.cur.layout_mut().insert_ebb(new_ebb, last_ebb);
        } else {
            // Insert after the last block.
            self.cur.layout_mut().insert_ebb_after(new_ebb, last_ebb);
        }
        self.has_new_blocks = true;
        new_ebb
    }

    /// Returns whether we should introduce a new branch.
    fn should_split_edge(&self, target: Ebb) -> bool {
        // We should split the edge if the target has any parameters.
        if self.cur.func.dfg.ebb_params(target).len() > 0 {
            return true;
        };

        // Or, if the target has more than one block reaching it.
        debug_assert!(self.cfg.pred_iter(target).next() != None);
        if let Some(_) = self.cfg.pred_iter(target).skip(1).next() {
            return true;
        };

        false
    }
}
