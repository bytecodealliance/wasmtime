//! A simple GVN pass.

use cursor::{Cursor, FuncCursor};
use dominator_tree::DominatorTree;
use ir::{Function, Inst, InstructionData, Opcode, Type};
use scoped_hash_map::ScopedHashMap;
use std::cell::{Ref, RefCell};
use std::hash::{Hash, Hasher};
use std::vec::Vec;
use timing;

/// Test whether the given opcode is unsafe to even consider for GVN.
fn trivially_unsafe_for_gvn(opcode: Opcode) -> bool {
    opcode.is_call()
        || opcode.is_branch()
        || opcode.is_terminator()
        || opcode.is_return()
        || opcode.can_trap()
        || opcode.other_side_effects()
        || opcode.can_store()
        || opcode.can_load()
        || opcode.writes_cpu_flags()
}

/// Wrapper around `InstructionData` which implements `Eq` and `Hash`
#[derive(Clone)]
struct HashKey<'a, 'f: 'a> {
    inst: InstructionData,
    ty: Type,
    pos: &'a RefCell<FuncCursor<'f>>,
}
impl<'a, 'f: 'a> Hash for HashKey<'a, 'f> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let pool = &self.pos.borrow().func.dfg.value_lists;
        self.inst.hash(state, pool);
        self.ty.hash(state);
    }
}
impl<'a, 'f: 'a> PartialEq for HashKey<'a, 'f> {
    fn eq(&self, other: &Self) -> bool {
        let pool = &self.pos.borrow().func.dfg.value_lists;
        self.inst.eq(&other.inst, pool) && self.ty == other.ty
    }
}
impl<'a, 'f: 'a> Eq for HashKey<'a, 'f> {}

/// Perform simple GVN on `func`.
///
pub fn do_simple_gvn(func: &mut Function, domtree: &mut DominatorTree) {
    let _tt = timing::gvn();
    debug_assert!(domtree.is_valid());

    // Visit EBBs in a reverse post-order.
    //
    // The RefCell here is a bit ugly since the HashKeys in the ScopedHashMap
    // need a reference to the function.
    let pos = RefCell::new(FuncCursor::new(func));

    let mut visible_values: ScopedHashMap<HashKey, Inst> = ScopedHashMap::new();
    let mut scope_stack: Vec<Inst> = Vec::new();

    for &ebb in domtree.cfg_postorder().iter().rev() {
        {
            // Pop any scopes that we just exited.
            let layout = &pos.borrow().func.layout;
            loop {
                if let Some(current) = scope_stack.last() {
                    if domtree.dominates(*current, ebb, layout) {
                        break;
                    }
                } else {
                    break;
                }
                scope_stack.pop();
                visible_values.decrement_depth();
            }

            // Push a scope for the current block.
            scope_stack.push(layout.first_inst(ebb).unwrap());
            visible_values.increment_depth();
        }

        pos.borrow_mut().goto_top(ebb);
        while let Some(inst) = {
            let mut pos = pos.borrow_mut();
            pos.next_inst()
        } {
            // Resolve aliases, particularly aliases we created earlier.
            pos.borrow_mut().func.dfg.resolve_aliases_in_arguments(inst);

            let func = Ref::map(pos.borrow(), |pos| &pos.func);

            let opcode = func.dfg[inst].opcode();
            if opcode.is_branch() && !opcode.is_terminator() {
                scope_stack.push(func.layout.next_inst(inst).unwrap());
                visible_values.increment_depth();
            }
            if trivially_unsafe_for_gvn(opcode) {
                continue;
            }

            let ctrl_typevar = func.dfg.ctrl_typevar(inst);
            let key = HashKey {
                inst: func.dfg[inst].clone(),
                ty: ctrl_typevar,
                pos: &pos,
            };
            let entry = visible_values.entry(key);
            use scoped_hash_map::Entry::*;
            match entry {
                Occupied(entry) => {
                    debug_assert!(domtree.dominates(*entry.get(), inst, &func.layout));
                    // If the redundant instruction is representing the current
                    // scope, pick a new representative.
                    let old = scope_stack.last_mut().unwrap();
                    if *old == inst {
                        *old = func.layout.next_inst(inst).unwrap();
                    }
                    // Replace the redundant instruction and remove it.
                    drop(func);
                    let mut pos = pos.borrow_mut();
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
