//! Instruction predicates/properties, shared by various analyses.

use crate::ir::{DataFlowGraph, Function, Inst, InstructionData, Opcode};
use cranelift_entity::EntityRef;

/// Preserve instructions with used result values.
pub fn any_inst_results_used(inst: Inst, live: &[bool], dfg: &DataFlowGraph) -> bool {
    dfg.inst_results(inst).iter().any(|v| live[v.index()])
}

/// Test whether the given opcode is unsafe to even consider as side-effect-free.
fn trivially_has_side_effects(opcode: Opcode) -> bool {
    opcode.is_call()
        || opcode.is_branch()
        || opcode.is_terminator()
        || opcode.is_return()
        || opcode.can_trap()
        || opcode.other_side_effects()
        || opcode.can_store()
}

/// Load instructions without the `notrap` flag are defined to trap when
/// operating on inaccessible memory, so we can't treat them as side-effect-free even if the loaded
/// value is unused.
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

/// Does the given instruction have any side-effect that would preclude it from being removed when
/// its value is unused?
pub fn has_side_effect(func: &Function, inst: Inst) -> bool {
    let data = &func.dfg[inst];
    let opcode = data.opcode();
    trivially_has_side_effects(opcode) || is_load_with_defined_trapping(opcode, data)
}
