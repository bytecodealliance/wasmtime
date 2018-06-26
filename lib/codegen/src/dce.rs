//! A Dead-Code Elimination (DCE) pass.
//!
//! Dead code here means instructions that have no side effects and have no
//! result values used by other instructions.

use cursor::{Cursor, FuncCursor};
use dominator_tree::DominatorTree;
use entity::EntityRef;
use ir::instructions::InstructionData;
use ir::{DataFlowGraph, Function, Inst, Opcode};
use std::vec::Vec;
use timing;

/// Test whether the given opcode is unsafe to even consider for DCE.
fn trivially_unsafe_for_dce(opcode: Opcode) -> bool {
    opcode.is_call()
        || opcode.is_branch()
        || opcode.is_terminator()
        || opcode.is_return()
        || opcode.can_trap()
        || opcode.other_side_effects()
        || opcode.can_store()
}

/// Preserve instructions with used result values.
fn any_inst_results_used(inst: Inst, live: &[bool], dfg: &DataFlowGraph) -> bool {
    dfg.inst_results(inst).iter().any(|v| live[v.index()])
}

/// Load instructions without the `notrap` flag are defined to trap when
/// operating on inaccessible memory, so we can't DCE them even if the
/// loaded value is unused.
fn is_load_with_defined_trapping(opcode: Opcode, data: &InstructionData) -> bool {
    if !opcode.can_load() {
        return false;
    }
    match *data {
        InstructionData::StackLoad { .. } => false,
        InstructionData::Load { flags, .. } => !flags.notrap(),
        _ => true,
    }
}

/// Perform DCE on `func`.
pub fn do_dce(func: &mut Function, domtree: &mut DominatorTree) {
    let _tt = timing::dce();
    debug_assert!(domtree.is_valid());

    let mut live = Vec::with_capacity(func.dfg.num_values());
    live.resize(func.dfg.num_values(), false);

    for &ebb in domtree.cfg_postorder() {
        let mut pos = FuncCursor::new(func).at_bottom(ebb);
        while let Some(inst) = pos.prev_inst() {
            {
                let data = &pos.func.dfg[inst];
                let opcode = data.opcode();
                if trivially_unsafe_for_dce(opcode)
                    || is_load_with_defined_trapping(opcode, &data)
                    || any_inst_results_used(inst, &live, &pos.func.dfg)
                {
                    for arg in pos.func.dfg.inst_args(inst) {
                        let v = pos.func.dfg.resolve_aliases(*arg);
                        live[v.index()] = true;
                    }
                    continue;
                }
            }
            pos.remove_inst();
        }
    }
}
