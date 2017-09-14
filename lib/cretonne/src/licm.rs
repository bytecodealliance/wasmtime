//! A Loop Invariant Code Motion optimization pass

use ir::{Function, Ebb, Inst, Value, Cursor, CursorBase, Type, InstBuilder, Layout};
use flowgraph::ControlFlowGraph;
use std::collections::HashSet;
use dominator_tree::DominatorTree;
use entity::{EntityList, ListPool};
use loop_analysis::{Loop, LoopAnalysis};

/// Performs the LICM pass by detecting loops within the CFG and moving
/// loop-invariant instructions out of them.
/// Changes the CFG and domtree in-place during the operation.
pub fn do_licm(
    func: &mut Function,
    cfg: &mut ControlFlowGraph,
    domtree: &mut DominatorTree,
    loop_analysis: &mut LoopAnalysis,
) {
    debug_assert!(cfg.is_valid());
    debug_assert!(domtree.is_valid());
    debug_assert!(loop_analysis.is_valid());

    for lp in loop_analysis.loops() {
        // For each loop that we want to optimize we determine the set of loop-invariant
        // instructions
        let invariant_inst = remove_loop_invariant_instructions(lp, func, cfg, loop_analysis);
        // Then we create the loop's pre-header and fill it with the invariant instructions
        // Then we remove the invariant instructions from the loop body
        if !invariant_inst.is_empty() {
            // If the loop has a natural pre-header we use it, otherwise we create it.
            let mut pos;
            match has_pre_header(&func.layout, cfg, domtree, loop_analysis.loop_header(lp)) {
                None => {
                    let pre_header =
                        create_pre_header(loop_analysis.loop_header(lp), func, cfg, domtree);
                    pos = Cursor::new(&mut func.layout);
                    pos.goto_bottom(pre_header);
                    pos.prev_inst();
                }
                // If there is a natural pre-header we insert new instructions just before the
                // related jumping instruction (which is not necessarily at the end).
                Some((_, last_inst)) => {
                    pos = Cursor::new(&mut func.layout);
                    pos.goto_inst(last_inst);
                }
            };
            // The last instruction of the pre-header is the termination instruction (usually
            // a jump) so we need to insert just before this.
            for inst in invariant_inst {
                pos.insert_inst(inst);
            }
        }
    }
    // We have to recompute the domtree to account for the changes
    cfg.compute(func);
    domtree.compute(func, cfg);
}

// Insert a pre-header before the header, modifying the function layout and CFG to reflect it.
// A jump instruction to the header is placed at the end of the pre-header.
fn create_pre_header(
    header: Ebb,
    func: &mut Function,
    cfg: &mut ControlFlowGraph,
    domtree: &DominatorTree,
) -> Ebb {
    let pool = &mut ListPool::<Value>::new();
    let header_args_values: Vec<Value> = func.dfg.ebb_args(header).into_iter().cloned().collect();
    let header_args_types: Vec<Type> = header_args_values
        .clone()
        .into_iter()
        .map(|val| func.dfg.value_type(val))
        .collect();
    let pre_header = func.dfg.make_ebb();
    let mut pre_header_args_value: EntityList<Value> = EntityList::new();
    for typ in header_args_types {
        pre_header_args_value.push(func.dfg.append_ebb_arg(pre_header, typ), pool);
    }
    for &(_, last_inst) in cfg.get_predecessors(header) {
        // We only follow normal edges (not the back edges)
        if !domtree.dominates(header, last_inst, &func.layout) {
            change_branch_jump_destination(last_inst, pre_header, func);
        }
    }
    {
        let mut pos = Cursor::new(&mut func.layout);
        pos.goto_top(header);
        // Inserts the pre-header at the right place in the layout.
        pos.insert_ebb(pre_header);
        pos.next_inst();
        func.dfg.ins(&mut pos).jump(
            header,
            pre_header_args_value.as_slice(pool),
        );
    }
    pre_header
}

// Detects if a loop header has a natural pre-header.
//
// A loop header has a pre-header if there is only one predecessor that the header doesn't
// dominate.
// Returns the pre-header Ebb and the instruction jumping to the header.
fn has_pre_header(
    layout: &Layout,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    header: Ebb,
) -> Option<(Ebb, Inst)> {
    let mut result = None;
    let mut found = false;
    for &(pred_ebb, last_inst) in cfg.get_predecessors(header) {
        // We only count normal edges (not the back edges)
        if !domtree.dominates(header, last_inst, layout) {
            if found {
                // We have already found one, there are more than one
                return None;
            } else {
                result = Some((pred_ebb, last_inst));
                found = true;
            }
        }
    }
    result
}


// Change the destination of a jump or branch instruction. Does nothing if called with a non-jump
// or non-branch instruction.
fn change_branch_jump_destination(inst: Inst, new_ebb: Ebb, func: &mut Function) {
    match func.dfg[inst].branch_destination_mut() {
        None => (),
        Some(instruction_dest) => *instruction_dest = new_ebb,
    }
}

// Traverses a loop in reverse post-order from a header EBB and identify loop-invariant
// instructions. These loop-invariant instructions are then removed from the code and returned
// (in reverse post-order) for later use.
fn remove_loop_invariant_instructions(
    lp: Loop,
    func: &mut Function,
    cfg: &ControlFlowGraph,
    loop_analysis: &LoopAnalysis,
) -> Vec<Inst> {
    let mut loop_values: HashSet<Value> = HashSet::new();
    let mut invariant_inst: Vec<Inst> = Vec::new();
    let mut pos = Cursor::new(&mut func.layout);
    // We traverse the loop EBB in reverse post-order.
    for ebb in postorder_ebbs_loop(loop_analysis, cfg, lp).iter().rev() {
        // Arguments of the EBB are loop values
        for val in func.dfg.ebb_args(*ebb) {
            loop_values.insert(*val);
        }
        pos.goto_top(*ebb);
        while let Some(inst) = pos.next_inst() {
            if func.dfg.has_results(inst) &&
                func.dfg.inst_args(inst).into_iter().all(|arg| {
                    !loop_values.contains(arg)
                })
            {
                // If all the instruction's argument are defined outside the loop
                // then this instruction is loop-invariant
                invariant_inst.push(inst);
                // We remove it from the loop
                pos.remove_inst_and_step_back();
            } else {
                // If the instruction is not loop-invariant we push its results in the set of
                // loop values
                for out in func.dfg.inst_results(inst) {
                    loop_values.insert(*out);
                }
            }
        }
    }
    invariant_inst
}

/// Return ebbs from a loop in post-order, starting from an entry point in the block.
fn postorder_ebbs_loop(loop_analysis: &LoopAnalysis, cfg: &ControlFlowGraph, lp: Loop) -> Vec<Ebb> {
    let mut grey = HashSet::new();
    let mut black = HashSet::new();
    let mut stack = vec![loop_analysis.loop_header(lp)];
    let mut postorder = Vec::new();

    while !stack.is_empty() {
        let node = stack.pop().unwrap();
        if !grey.contains(&node) {
            // This is a white node. Mark it as gray.
            grey.insert(node);
            stack.push(node);
            // Get any children we've never seen before.
            for child in cfg.get_successors(node) {
                if loop_analysis.is_in_loop(*child, lp) && !grey.contains(child) {
                    stack.push(*child);
                }
            }
        } else if !black.contains(&node) {
            postorder.push(node);
            black.insert(node);
        }
    }
    postorder
}
