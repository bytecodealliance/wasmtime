//! A simple GVN pass.

use cursor::{Cursor, FuncCursor};
use dominator_tree::DominatorTree;
use ir::{Function, Inst, InstructionData, Opcode, Type};
use scoped_hash_map::ScopedHashMap;
use std::vec::Vec;
use timing;

/// Test whether the given opcode is unsafe to even consider for GVN.
fn trivially_unsafe_for_gvn(opcode: Opcode) -> bool {
    opcode.is_call() || opcode.is_branch() || opcode.is_terminator() || opcode.is_return()
        || opcode.can_trap() || opcode.other_side_effects() || opcode.can_store()
        || opcode.can_load() || opcode.writes_cpu_flags()
}

/// Perform simple GVN on `func`.
///
pub fn do_simple_gvn(func: &mut Function, domtree: &mut DominatorTree) {
    let _tt = timing::gvn();
    debug_assert!(domtree.is_valid());

    let mut visible_values: ScopedHashMap<(InstructionData, Type), Inst> = ScopedHashMap::new();
    let mut scope_stack: Vec<Inst> = Vec::new();

    // Visit EBBs in a reverse post-order.
    let mut pos = FuncCursor::new(func);

    for &ebb in domtree.cfg_postorder().iter().rev() {
        // Pop any scopes that we just exited.
        loop {
            if let Some(current) = scope_stack.last() {
                if domtree.dominates(*current, ebb, &pos.func.layout) {
                    break;
                }
            } else {
                break;
            }
            scope_stack.pop();
            visible_values.decrement_depth();
        }

        // Push a scope for the current block.
        scope_stack.push(pos.func.layout.first_inst(ebb).unwrap());
        visible_values.increment_depth();

        pos.goto_top(ebb);
        while let Some(inst) = pos.next_inst() {
            // Resolve aliases, particularly aliases we created earlier.
            pos.func.dfg.resolve_aliases_in_arguments(inst);

            let opcode = pos.func.dfg[inst].opcode();
            if opcode.is_branch() && !opcode.is_terminator() {
                scope_stack.push(pos.func.layout.next_inst(inst).unwrap());
                visible_values.increment_depth();
            }
            if trivially_unsafe_for_gvn(opcode) {
                continue;
            }

            let ctrl_typevar = pos.func.dfg.ctrl_typevar(inst);
            let key = (pos.func.dfg[inst].clone(), ctrl_typevar);
            let entry = visible_values.entry(key);
            use scoped_hash_map::Entry::*;
            match entry {
                Occupied(entry) => {
                    debug_assert!(domtree.dominates(*entry.get(), inst, &pos.func.layout));
                    // If the redundant instruction is representing the current
                    // scope, pick a new representative.
                    let old = scope_stack.last_mut().unwrap();
                    if *old == inst {
                        *old = pos.func.layout.next_inst(inst).unwrap();
                    }
                    // Replace the redundant instruction and remove it.
                    pos.func.dfg.replace_with_aliases(inst, *entry.get());
                    pos.remove_inst_and_step_back();
                }
                Vacant(entry) => {
                    entry.insert(inst);
                }
            }
        }
    }
}
