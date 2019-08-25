//! This module implements a late-stage redundant-reload remover, which runs after registers have
//! been allocated and stack slots have been given specific offsets.

use crate::cursor::{Cursor, CursorPosition, EncCursor, FuncCursor};
use crate::entity::EntitySet;
use crate::flowgraph::ControlFlowGraph;
use crate::ir::dfg::DataFlowGraph;
use crate::ir::instructions::BranchInfo;
use crate::ir::stackslot::{StackSlotKind, StackSlots};
use crate::ir::{
    Ebb, Function, Inst, InstBuilder, InstructionData, Opcode, StackSlotData, Type, Value, ValueLoc,
};
use crate::isa::{RegInfo, RegUnit, TargetIsa};
use crate::regalloc::RegDiversions;
use core::convert::TryInto;
use cranelift_entity::{PrimaryMap, SecondaryMap};
use std::vec::Vec;

// =============================================================================================
// A description of the redundant-fill-removal algorithm
//
//
// The algorithm works forwards through each Ebb.  It carries along and updates a table,
// AvailEnv, with which it tracks registers that are known to have the same value as some stack
// slot.  The actions on encountering an instruction depend on the instruction, as follows:
//
// ss1 = spill r0: update the AvailEnv so as to note that slot `ss1` and register `r0`
//                 have the same value.
//
// r1 = fill ss0: look in the AvailEnv.  If it tells us that register `r1` and slot `ss0`
//                have the same value, then delete the instruction by converting it to a
//                `fill_nop`.
//
//                If it tells us that some other register `r2` has the same value as
//                slot `ss0`, convert the instruction into a copy from `r2` to `r1`.
//
// any other insn: remove from the AvailEnv, any bindings associated with registers
//                 written by this instruction, since they will be invalidated by it.
//
// Tracking the effects of `copy` instructions in AvailEnv for the case when both source and
// destination are registers does not cause any more fills to be removed or converted to copies.
// It's not clear why.
//
// There are various other instruction-handling cases in `visit_inst`, which are documented
// in-line, and do not change the core algorithm, so are not described here.
//
// The registers tracked by AvailEnv are the post-diversion registers that are really used by the
// code; they are not the pre-diversion names associated with each SSA `Value`.  The second
// `fill` case above opportunistically copies values from registers that may have been diversion
// targets in some predecessor block, and so are no longer associated with any specific SSA-level
// name at the point the copy is made.  Hence those copies (from `r2` to `r1`) cannot be done
// with an ordinary `copy` instruction.  Instead they have to be done using a new `copy_to_ssa`
// instruction, which copies from an arbitrary register to a register-resident `Value` (that is,
// "back to" SSA-world).
//
// That completes the description of the core algorithm.
//
// In the case where a block `A` jumps to `B` and `A` is the only predecessor of `B`, the
// AvailEnv at the end of `A` will still be valid at the entry to `B`.  In such a case, we can
// profitably transform `B` using the AvailEnv "inherited" from `A`.  In order to take full
// advantage of this, this module partitions the function's CFG into tree-shaped groups of
// blocks, and processes each tree as described above.  So the AvailEnv is only initialised to
// empty at the start of blocks that form the root of each tree; that is, for blocks which have
// two or more predecessors.

// =============================================================================================
// Top level algorithm structure
//
// The overall algorithm, for a function, starts like this:
//
// * (once per function): finds Ebbs that have two or more predecessors, since they will be the
//   roots of Ebb trees.  Also, the entry node for the function is considered to be a root.
//
// It then continues with a loop that first finds a tree of Ebbs ("discovery") and then removes
// redundant fills as described above ("processing"):
//
// * (discovery; once per tree): for each root, performs a depth first search to find all the Ebbs
//   in the tree, guided by RedundantReloadRemover::discovery_stack.
//
// * (processing; once per tree): the just-discovered tree is then processed as described above,
//   guided by RedundantReloadRemover::processing_stack.
//
// In this way, all Ebbs reachable from the function's entry point are eventually processed.  Note
// that each tree is processed as soon as it has been discovered, so the algorithm never creates a
// list of trees for the function.
//
// The running state is stored in `RedundantReloadRemover`.  This is allocated once and can be
// reused for multiple functions so as to minimise heap turnover.  The fields are, roughly:
//
//   num_regunits -- constant for the whole function; used by the tree processing phase
//   num_preds_per_ebb -- constant for the whole function; used by the tree discovery process
//
//   discovery_stack -- used to guide the tree discovery process
//   nodes_in_tree -- the discovered nodes are recorded here
//
//   processing_stack -- used to guide the tree processing process
//   nodes_already_visited -- used to ensure the tree processing logic terminates in the case
//                            where a tree has a branch back to its root node.
//
// There is further documentation in line below, as appropriate.

// =============================================================================================
// A side note on register choice heuristics

// The core algorithm opportunistically replaces fill instructions when it knows of a register
// that already holds the required value.  How effective this is largely depends on how long
// reloaded values happen to stay alive before the relevant register is overwritten.  And that
// depends on the register allocator's register choice heuristics.  The worst case is, when the
// register allocator reuses registers as soon as possible after they become free.  Unfortunately
// that was indeed the selection scheme, prior to development of this pass.
//
// As part of this work, the register selection scheme has been changed as follows: for registers
// written by any instruction other than a fill, use the lowest numbered available register.  But
// for registers written by a fill instruction, use the highest numbered available register.  The
// aim is to try and keep reload- and non-reload registers disjoint to the extent possible.
// Several other schemes were tried, but this one is simple and can be worth an extra 2% of
// performance in some cases.
//
// The relevant change is more or less a one-line change in the solver.

// =============================================================================================
// Data structures used for discovery of trees

// `ZeroOneOrMany` is used to record the number of predecessors an Ebb block has.  The `Zero` case
// is included so as to cleanly handle the case where the incoming graph has unreachable Ebbs.

#[derive(Clone, PartialEq)]
enum ZeroOneOrMany {
    Zero,
    One,
    Many,
}

// =============================================================================================
// Data structures used for processing of trees

// `SlotInfo` describes a spill slot in the obvious way.  Note that it doesn't indicate which
// register(s) are currently associated with the slot.  That job is done by `AvailEnv` instead.
//
// In the CL framework, stack slots are partitioned into disjoint sets, one for each
// `StackSlotKind`.  The offset and size only give a unique identity within any particular
// `StackSlotKind`.  So, to uniquely identify a stack slot, all three fields are necessary.

#[derive(Clone, Copy)]
struct SlotInfo {
    kind: StackSlotKind,
    offset: i32,
    size: u32,
}

// `AvailEnv` maps each possible register to a stack slot that holds the same value.  The index
// space of `AvailEnv::map` is exactly the set of registers available on the current target.  If
// (as is mostly the case) a register is not known to have the same value as a stack slot, then
// its entry is `None` rather than `Some(..)`.
//
// Invariants for AvailEnv:
//
// AvailEnv may have multiple different registers bound to the same stack slot -- that is, `(kind,
// offset, size)` triple.  That's OK, and reflects the reality that those two registers contain
// the same value.  This could happen, for example, in the case
//
//   ss1 = spill r0
//   ..
//   r2 = fill ss1
//
// Then both `r0` and `r2` will have the same value as `ss1`, provided that ".." doesn't write to
// `r1`.
//
// To say that two different registers may be bound to the same stack slot is the same as saying
// that it is allowed to have two different entries in AvailEnv with the same `(kind, offset,
// size)` triple.  What is *not* allowed is to have partial overlaps.  That is, if two SlotInfos
// have the same `kind` field and have `offset` and `size` fields that overlap, then their
// `offset` and `size` fields must be identical.  This is so as to make the algorithm safe against
// situations where, for example, a 64 bit register is spilled, but then only the bottom 32 bits
// are reloaded from the slot.
//
// Although in such a case it seems likely that the Cranelift IR would be ill-typed, and so this
// case could probably not occur in practice.

#[derive(Clone)]
struct AvailEnv {
    map: Vec<Option<SlotInfo>>,
}

// `ProcessingStackElem` combines AvailEnv with contextual information needed to "navigate" within
// an Ebb.
//
// A ProcessingStackElem conceptually has the lifetime of exactly one Ebb: once the current Ebb is
// completed, the ProcessingStackElem will be abandoned.  In practice the top level state,
// RedundantReloadRemover, caches them, so as to avoid heap turnover.
//
// Note that ProcessingStackElem must contain a CursorPosition.  The CursorPosition, which
// indicates where we are in the current Ebb, cannot be implicitly maintained by looping over all
// the instructions in an Ebb in turn, because we may choose to suspend processing the current Ebb
// at a side exit, continue by processing the subtree reached via the side exit, and only later
// resume the current Ebb.

struct ProcessingStackElem {
    /// Indicates the AvailEnv at the current point in the Ebb.
    avail_env: AvailEnv,

    /// Shows where we currently are inside the Ebb.
    cursor: CursorPosition,

    /// Indicates the currently active register diversions at the current point.
    diversions: RegDiversions,
}

// =============================================================================================
// The top level data structure

// `RedundantReloadRemover` contains data structures for the two passes: discovery of tree shaped
// regions, and processing of them.  These are allocated once and stay alive for the entire
// function, even though they are cleared out for each new tree shaped region.  It also caches
// `num_regunits` and `num_preds_per_ebb`, which are computed at the start of each function and
// then remain constant.

/// The redundant reload remover's state.
pub struct RedundantReloadRemover {
    /// The total number of RegUnits available on this architecture.  This is unknown when the
    /// RedundantReloadRemover is created.  It becomes known at the beginning of processing of a
    /// function.
    num_regunits: Option<u16>,

    /// This stores, for each Ebb, a characterisation of the number of predecessors it has.
    num_preds_per_ebb: PrimaryMap<Ebb, ZeroOneOrMany>,

    /// The stack used for the first phase (discovery).  There is one element on the discovery
    /// stack for each currently unexplored Ebb in the tree being searched.
    discovery_stack: Vec<Ebb>,

    /// The nodes in the discovered tree are inserted here.
    nodes_in_tree: EntitySet<Ebb>,

    /// The stack used during the second phase (transformation).  There is one element on the
    /// processing stack for each currently-open node in the tree being transformed.
    processing_stack: Vec<ProcessingStackElem>,

    /// Used in the second phase to avoid visiting nodes more than once.
    nodes_already_visited: EntitySet<Ebb>,
}

// =============================================================================================
// Miscellaneous small helper functions

// Is this a kind of stack slot that is safe to track in AvailEnv?  This is probably overly
// conservative, but tracking only the SpillSlot and IncomingArgument kinds catches almost all
// available redundancy in practice.
fn is_slot_kind_tracked(kind: StackSlotKind) -> bool {
    match kind {
        StackSlotKind::SpillSlot | StackSlotKind::IncomingArg => true,
        _ => false,
    }
}

// Find out if the range `[offset, +size)` overlaps with the range in `si`.
fn overlaps(si: &SlotInfo, offset: i32, size: u32) -> bool {
    let a_offset = si.offset as i64;
    let a_size = si.size as i64;
    let b_offset = offset as i64;
    let b_size = size as i64;
    let no_overlap = a_offset + a_size <= b_offset || b_offset + b_size <= a_offset;
    !no_overlap
}

// Find, in `reginfo`, the register bank that `reg` lives in, and return the lower limit and size
// of the bank.  This is so the caller can conveniently iterate over all RegUnits in the bank that
// `reg` lives in.
fn find_bank_limits(reginfo: &RegInfo, reg: RegUnit) -> (RegUnit, u16) {
    if let Some(bank) = reginfo.bank_containing_regunit(reg) {
        return (bank.first_unit, bank.units);
    }
    // We should never get here, since `reg` must come from *some* RegBank.
    panic!("find_regclass_limits: reg not found");
}

// Returns the register that `v` is allocated to.  Assumes that `v` actually resides in a
// register.
fn reg_of_value(locations: &SecondaryMap<Value, ValueLoc>, v: Value) -> RegUnit {
    match locations[v] {
        ValueLoc::Reg(ru) => ru,
        _ => panic!("reg_of_value: value isn't in a reg"),
    }
}

// Returns the stack slot that `v` is allocated to.  Assumes that `v` actually resides in a stack
// slot.
fn slot_of_value<'s>(
    locations: &SecondaryMap<Value, ValueLoc>,
    stack_slots: &'s StackSlots,
    v: Value,
) -> &'s StackSlotData {
    match locations[v] {
        ValueLoc::Stack(slot) => &stack_slots[slot],
        _ => panic!("slot_of_value: value isn't in a stack slot"),
    }
}

// =============================================================================================
// Top level: discovery of tree shaped regions

impl RedundantReloadRemover {
    // A helper for `add_nodes_to_tree` below.
    fn discovery_stack_push_successors_of(&mut self, cfg: &ControlFlowGraph, node: Ebb) {
        for successor in cfg.succ_iter(node) {
            self.discovery_stack.push(successor);
        }
    }

    // Visit the tree of Ebbs rooted at `starting_point` and add them to `self.nodes_in_tree`.
    // `self.num_preds_per_ebb` guides the process, ensuring we don't leave the tree-ish region
    // and indirectly ensuring that the process will terminate in the presence of cycles in the
    // graph.  `self.discovery_stack` holds the search state in this function.
    fn add_nodes_to_tree(&mut self, cfg: &ControlFlowGraph, starting_point: Ebb) {
        // One might well ask why this doesn't loop forever when it encounters cycles in the
        // control flow graph.  The reason is that any cycle in the graph that is reachable from
        // anywhere outside the cycle -- in particular, that is reachable from the function's
        // entry node -- must have at least one node that has two or more predecessors.  So the
        // logic below won't follow into it, because it regards any such node as the root of some
        // other tree.
        debug_assert!(self.discovery_stack.is_empty());
        debug_assert!(self.nodes_in_tree.is_empty());

        self.nodes_in_tree.insert(starting_point);
        self.discovery_stack_push_successors_of(cfg, starting_point);

        while let Some(node) = self.discovery_stack.pop() {
            match self.num_preds_per_ebb[node] {
                // We arrived at a node with multiple predecessors, so it's a new root.  Ignore it.
                ZeroOneOrMany::Many => {}
                // This node has just one predecessor, so we should incorporate it in the tree and
                // immediately transition into searching from it instead.
                ZeroOneOrMany::One => {
                    self.nodes_in_tree.insert(node);
                    self.discovery_stack_push_successors_of(cfg, node);
                }
                // This is meaningless.  We arrived at a node that doesn't point back at where we
                // came from.
                ZeroOneOrMany::Zero => panic!("add_nodes_to_tree: inconsistent graph"),
            }
        }
    }
}

// =============================================================================================
// Operations relating to `AvailEnv`

impl AvailEnv {
    // Create a new one.
    fn new(size: usize) -> Self {
        let mut env = AvailEnv {
            map: Vec::<Option<SlotInfo>>::new(),
        };
        env.map.resize(size, None);
        env
    }

    // Debug only: checks (some of) the required AvailEnv invariants.
    #[cfg(debug_assertions)]
    fn check_invariants(&self) -> bool {
        // Check that any overlapping entries overlap exactly.  This is super lame (quadratic),
        // but it's only used in debug builds.
        for i in 0..self.map.len() {
            if let Some(si) = self.map[i] {
                for j in i + 1..self.map.len() {
                    if let Some(sj) = self.map[j] {
                        // "si and sj overlap, but not exactly"
                        if si.kind == sj.kind
                            && overlaps(&si, sj.offset, sj.size)
                            && !(si.offset == sj.offset && si.size == sj.size)
                        {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    // Invalidates the binding associated with `reg`.  Note that by construction of AvailEnv,
    // `reg` can only be associated with one binding at once.
    fn invalidate_by_reg(&mut self, reg: RegUnit) {
        self.map[reg as usize] = None;
    }

    // Invalidates any binding that has any overlap with `(kind, offset, size)`.
    fn invalidate_by_offset(&mut self, kind: StackSlotKind, offset: i32, size: u32) {
        debug_assert!(is_slot_kind_tracked(kind));
        for i in 0..self.map.len() {
            if let Some(si) = &self.map[i] {
                if si.kind == kind && overlaps(&si, offset, size) {
                    self.map[i] = None;
                }
            }
        }
    }

    // Invalidates all bindings.
    fn invalidate_all(&mut self) {
        for i in 0..self.map.len() {
            self.map[i] = None;
        }
    }

    // Updates AvailEnv to track the effect of a `regmove` instruction.
    fn copy_reg(&mut self, src: RegUnit, dst: RegUnit) {
        self.map[dst as usize] = self.map[src as usize];
    }

    // Does `env` have the exact binding characterised by `(reg, kind, offset, size)` ?
    fn has_exact_binding(&self, reg: RegUnit, kind: StackSlotKind, offset: i32, size: u32) -> bool {
        debug_assert!(is_slot_kind_tracked(kind));
        if let Some(si) = &self.map[reg as usize] {
            return si.kind == kind && si.offset == offset && si.size == size;
        }
        // No such binding.
        false
    }

    // Does `env` have a binding characterised by `(kind, offset, size)` but to a register, let's
    // call it `other_reg`, that isn't `reg`?  If so, return `other_reg`.  Note that `other_reg`
    // will have the same bank as `reg`.  It is a checked error to call this function with a
    // binding matching all four of `(reg, kind, offset, size)`.
    fn has_inexact_binding(
        &self,
        reginfo: &RegInfo,
        reg: RegUnit,
        kind: StackSlotKind,
        offset: i32,
        size: u32,
    ) -> Option<RegUnit> {
        debug_assert!(is_slot_kind_tracked(kind));
        // Find the range of RegUnit numbers for the bank that contains `reg`, and use that as our
        // search space.  This is so as to guarantee that any match is restricted to the same bank
        // as `reg`.
        let (first_unit, num_units) = find_bank_limits(reginfo, reg);
        for other_reg in first_unit..first_unit + num_units {
            if let Some(si) = &self.map[other_reg as usize] {
                if si.kind == kind && si.offset == offset && si.size == size {
                    if other_reg == reg {
                        panic!("has_inexact_binding: binding *is* exact!");
                    }
                    return Some(other_reg);
                }
            }
        }
        // No such binding.
        None
    }

    // Create the binding `(reg, kind, offset, size)` in `env`, and throw away any previous
    // binding associated with either `reg` or the `(kind, offset, size)` triple.
    fn bind(&mut self, reg: RegUnit, kind: StackSlotKind, offset: i32, size: u32) {
        debug_assert!(is_slot_kind_tracked(kind));
        self.invalidate_by_offset(kind, offset, size);
        self.map[reg as usize] = Some(SlotInfo { kind, offset, size });
    }
}

// Invalidates in `avail_env`, any binding associated with a regunit that is written by `inst`.
fn invalidate_regs_written_by_inst(
    locations: &SecondaryMap<Value, ValueLoc>,
    diversions: &RegDiversions,
    dfg: &DataFlowGraph,
    avail_env: &mut AvailEnv,
    inst: Inst,
) {
    for v in dfg.inst_results(inst).iter() {
        if let ValueLoc::Reg(ru) = locations[*v] {
            // This must be true.  It would be meaningless for an SSA value to be diverted before
            // the point where it is defined.
            debug_assert!(diversions.reg(*v, locations) == ru);
            avail_env.invalidate_by_reg(ru);
        }
    }
}

// =============================================================================================
// Processing of individual instructions

impl RedundantReloadRemover {
    // Process `inst`, possibly changing it into a different instruction, and possibly changing
    // `self.avail_env` and `func.dfg`.
    fn visit_inst(
        &mut self,
        func: &mut Function,
        reginfo: &RegInfo,
        isa: &dyn TargetIsa,
        inst: Inst,
    ) {
        // Get hold of the top-of-stack work item.  This is the state that we will mutate during
        // processing of this instruction.
        debug_assert!(!self.processing_stack.is_empty());
        let ProcessingStackElem {
            avail_env,
            cursor: _,
            diversions,
        } = &mut self.processing_stack.last_mut().unwrap();

        #[cfg(debug_assertions)]
        debug_assert!(
            avail_env.check_invariants(),
            "visit_inst: env invariants not ok"
        );

        let dfg = &mut func.dfg;
        let locations = &func.locations;
        let stack_slots = &func.stack_slots;

        // To avoid difficulties with the borrow checker, do this in two stages.  First, examine
        // the instruction to see if it can be deleted or modified, and park the relevant
        // information in `transform`.  Update `self.avail_env` too.  Later, use `transform` to
        // actually do the transformation if necessary.
        enum Transform {
            NoChange,
            ChangeToNopFill(Value),           // delete this insn entirely
            ChangeToCopyToSSA(Type, RegUnit), // change it into a copy from the specified reg
        }
        let mut transform = Transform::NoChange;

        // In this match { .. } statement, either we must treat the instruction specially, or we
        // must call `invalidate_regs_written_by_inst` on it.
        match &dfg[inst] {
            InstructionData::Unary {
                opcode: Opcode::Spill,
                arg: src_value,
            } => {
                // Extract: (src_reg, kind, offset, size)
                // Invalidate: (kind, offset, size)
                // Add new binding: {src_reg -> (kind, offset, size)}
                // Don't forget that src_value might be diverted, so we have to deref it.
                let slot = slot_of_value(locations, stack_slots, dfg.inst_results(inst)[0]);
                let src_reg = diversions.reg(*src_value, locations);
                let kind = slot.kind;
                if is_slot_kind_tracked(kind) {
                    let offset = slot.offset.expect("visit_inst: spill with no offset");
                    let size = slot.size;
                    avail_env.bind(src_reg, kind, offset, size);
                } else {
                    // We don't expect this insn to write any regs.  But to be consistent with the
                    // rule above, do this anyway.
                    invalidate_regs_written_by_inst(locations, diversions, dfg, avail_env, inst);
                }
            }
            InstructionData::Unary {
                opcode: Opcode::Fill,
                arg: src_value,
            } => {
                // Extract: (dst_reg, kind, offset, size)
                // Invalidate: (kind, offset, size)
                // Add new: {dst_reg -> (dst_value, kind, offset, size)}
                let slot = slot_of_value(locations, stack_slots, *src_value);
                let dst_value = dfg.inst_results(inst)[0];
                let dst_reg = reg_of_value(locations, dst_value);
                // This must be true.  It would be meaningless for an SSA value to be diverted
                // before it was defined.
                debug_assert!(dst_reg == diversions.reg(dst_value, locations));
                let kind = slot.kind;
                if is_slot_kind_tracked(kind) {
                    let offset = slot.offset.expect("visit_inst: fill with no offset");
                    let size = slot.size;
                    if avail_env.has_exact_binding(dst_reg, kind, offset, size) {
                        // This instruction is an exact copy of a fill we saw earlier, and the
                        // loaded value is still valid.  So we'll schedule this instruction for
                        // deletion (below).  No need to make any changes to `avail_env`.
                        transform = Transform::ChangeToNopFill(*src_value);
                    } else if let Some(other_reg) =
                        avail_env.has_inexact_binding(reginfo, dst_reg, kind, offset, size)
                    {
                        // This fill is from the required slot, but into a different register
                        // `other_reg`.  So replace it with a copy from `other_reg` to `dst_reg`
                        // and update `dst_reg`s binding to make it the same as `other_reg`'s, so
                        // as to maximise the chances of future matches after this instruction.
                        debug_assert!(other_reg != dst_reg);
                        transform =
                            Transform::ChangeToCopyToSSA(dfg.value_type(dst_value), other_reg);
                        avail_env.copy_reg(other_reg, dst_reg);
                    } else {
                        // This fill creates some new binding we don't know about.  Update
                        // `avail_env` to track it.
                        avail_env.bind(dst_reg, kind, offset, size);
                    }
                } else {
                    // Else it's "just another instruction that writes a reg", so we'd better
                    // treat it as such, just as we do below for instructions that we don't handle
                    // specially.
                    invalidate_regs_written_by_inst(locations, diversions, dfg, avail_env, inst);
                }
            }
            InstructionData::RegMove {
                opcode: _,
                arg: _,
                src,
                dst,
            } => {
                // These happen relatively rarely, but just frequently enough that it's worth
                // tracking the copy (at the machine level, it's really a copy) in `avail_env`.
                avail_env.copy_reg(*src, *dst);
            }
            InstructionData::RegSpill { .. }
            | InstructionData::RegFill { .. }
            | InstructionData::Call { .. }
            | InstructionData::CallIndirect { .. }
            | InstructionData::StackLoad { .. }
            | InstructionData::StackStore { .. }
            | InstructionData::Unary {
                opcode: Opcode::AdjustSpDown,
                ..
            }
            | InstructionData::UnaryImm {
                opcode: Opcode::AdjustSpUpImm,
                ..
            }
            | InstructionData::UnaryImm {
                opcode: Opcode::AdjustSpDownImm,
                ..
            } => {
                // All of these change, or might change, the memory-register bindings tracked in
                // `avail_env` in some way we don't know about, or at least, we might be able to
                // track, but for which the effort-to-benefit ratio seems too low to bother.  So
                // play safe: forget everything we know.
                //
                // For Call/CallIndirect, we could do better when compiling for calling
                // conventions that have callee-saved registers, since bindings for them would
                // remain valid across the call.
                avail_env.invalidate_all();
            }
            _ => {
                // Invalidate: any `avail_env` entry associated with a reg written by `inst`.
                invalidate_regs_written_by_inst(locations, diversions, dfg, avail_env, inst);
            }
        }

        // Actually do the transformation.
        match transform {
            Transform::NoChange => {}
            Transform::ChangeToNopFill(arg) => {
                // Load is completely redundant.  Convert it to a no-op.
                dfg.replace(inst).fill_nop(arg);
                let ok = func.update_encoding(inst, isa).is_ok();
                debug_assert!(ok, "fill_nop encoding missing for this type");
            }
            Transform::ChangeToCopyToSSA(ty, reg) => {
                // We already have the relevant value in some other register.  Convert the
                // load into a reg-reg copy.
                dfg.replace(inst).copy_to_ssa(ty, reg);
                let ok = func.update_encoding(inst, isa).is_ok();
                debug_assert!(ok, "copy_to_ssa encoding missing for type {}", ty);
            }
        }
    }
}

// =============================================================================================
// Top level: processing of tree shaped regions

impl RedundantReloadRemover {
    // Push a clone of the top-of-stack ProcessingStackElem.  This will be used to process exactly
    // one Ebb.  The diversions are created new, rather than cloned, to reflect the fact
    // that diversions are local to each Ebb.
    fn processing_stack_push(&mut self, cursor: CursorPosition) {
        let avail_env = if let Some(stack_top) = self.processing_stack.last() {
            stack_top.avail_env.clone()
        } else {
            AvailEnv::new(
                self.num_regunits
                    .expect("processing_stack_push: num_regunits unknown!")
                    as usize,
            )
        };
        self.processing_stack.push(ProcessingStackElem {
            avail_env,
            cursor,
            diversions: RegDiversions::new(),
        });
    }

    // This pushes the node `dst` onto the processing stack, and sets up the new
    // ProcessingStackElem accordingly.  But it does all that only if `dst` is part of the current
    // tree *and* we haven't yet visited it.
    fn processing_stack_maybe_push(&mut self, dst: Ebb) {
        if self.nodes_in_tree.contains(dst) && !self.nodes_already_visited.contains(dst) {
            if !self.processing_stack.is_empty() {
                // If this isn't the outermost node in the tree (that is, the root), then it must
                // have exactly one predecessor.  Nodes with no predecessors are dead and not
                // incorporated in any tree.  Nodes with two or more predecessors are the root of
                // some other tree, and visiting them as if they were part of the current tree
                // would be a serious error.
                debug_assert!(self.num_preds_per_ebb[dst] == ZeroOneOrMany::One);
            }
            self.processing_stack_push(CursorPosition::Before(dst));
            self.nodes_already_visited.insert(dst);
        }
    }

    // Perform redundant-reload removal on the tree shaped region of graph defined by `root` and
    // `self.nodes_in_tree`.  The following state is modified: `self.processing_stack`,
    // `self.nodes_already_visited`, and `func.dfg`.
    fn process_tree(
        &mut self,
        func: &mut Function,
        reginfo: &RegInfo,
        isa: &dyn TargetIsa,
        root: Ebb,
    ) {
        debug_assert!(self.nodes_in_tree.contains(root));
        debug_assert!(self.processing_stack.is_empty());
        debug_assert!(self.nodes_already_visited.is_empty());

        // Create the initial work item
        self.processing_stack_maybe_push(root);

        while !self.processing_stack.is_empty() {
            // It seems somewhat ridiculous to construct a whole new FuncCursor just so we can do
            // next_inst() on it once, and then copy the resulting position back out.  But use of
            // a function-global FuncCursor, or of the EncCursor in struct Context, leads to
            // borrow checker problems, as does including FuncCursor directly in
            // ProcessingStackElem.  In any case this is not as bad as it looks, since profiling
            // shows that the build-insert-step-extract work is reduced to just 8 machine
            // instructions in an optimised x86_64 build, presumably because rustc can inline and
            // then optimise out almost all the work.
            let tos = self.processing_stack.len() - 1;
            let mut pos = FuncCursor::new(func).at_position(self.processing_stack[tos].cursor);
            let maybe_inst = pos.next_inst();
            self.processing_stack[tos].cursor = pos.position();

            if let Some(inst) = maybe_inst {
                // Deal with this insn, possibly changing it, possibly updating the top item of
                // `self.processing_stack`.
                self.visit_inst(func, reginfo, isa, inst);

                // Update diversions after the insn.
                self.processing_stack[tos].diversions.apply(&func.dfg[inst]);

                // If the insn can branch outside this Ebb, push work items on the stack for all
                // target Ebbs that are part of the same tree and that we haven't yet visited.
                // The next iteration of this instruction-processing loop will immediately start
                // work on the most recently pushed Ebb, and will eventually continue in this Ebb
                // when those new items have been removed from the stack.
                match func.dfg.analyze_branch(inst) {
                    BranchInfo::NotABranch => (),
                    BranchInfo::SingleDest(dst, _) => {
                        self.processing_stack_maybe_push(dst);
                    }
                    BranchInfo::Table(jt, default) => {
                        func.jump_tables[jt]
                            .iter()
                            .for_each(|dst| self.processing_stack_maybe_push(*dst));
                        if let Some(dst) = default {
                            self.processing_stack_maybe_push(dst);
                        }
                    }
                }
            } else {
                // We've come to the end of the current work-item (Ebb).  We'll already have
                // processed the fallthrough/continuation/whatever for it using the logic above.
                // Pop it off the stack and resume work on its parent.
                self.processing_stack.pop();
            }
        }
    }
}

// =============================================================================================
// Top level: perform redundant fill removal for a complete function

impl RedundantReloadRemover {
    /// Create a new remover state.
    pub fn new() -> Self {
        Self {
            num_regunits: None,
            num_preds_per_ebb: PrimaryMap::<Ebb, ZeroOneOrMany>::with_capacity(8),
            discovery_stack: Vec::<Ebb>::with_capacity(16),
            nodes_in_tree: EntitySet::<Ebb>::new(),
            processing_stack: Vec::<ProcessingStackElem>::with_capacity(8),
            nodes_already_visited: EntitySet::<Ebb>::new(),
        }
    }

    /// Clear the state of the remover.
    pub fn clear(&mut self) {
        self.clear_for_new_function();
    }

    fn clear_for_new_function(&mut self) {
        self.num_preds_per_ebb.clear();
        self.clear_for_new_tree();
    }

    fn clear_for_new_tree(&mut self) {
        self.discovery_stack.clear();
        self.nodes_in_tree.clear();
        self.processing_stack.clear();
        self.nodes_already_visited.clear();
    }

    #[inline(never)]
    fn do_redundant_fill_removal_on_function(
        &mut self,
        func: &mut Function,
        reginfo: &RegInfo,
        isa: &dyn TargetIsa,
        cfg: &ControlFlowGraph,
    ) {
        // Fail in an obvious way if there are more than (2^32)-1 Ebbs in this function.
        let num_ebbs: u32 = func.dfg.num_ebbs().try_into().unwrap();

        // Clear out per-tree state.
        self.clear_for_new_function();

        // Create a PrimaryMap that summarises the number of predecessors for each block, as 0, 1
        // or "many", and that also claims the entry block as having "many" predecessors.
        self.num_preds_per_ebb.clear();
        self.num_preds_per_ebb.reserve(num_ebbs as usize);

        for i in 0..num_ebbs {
            let mut pi = cfg.pred_iter(Ebb::from_u32(i));
            let mut n_pi = ZeroOneOrMany::Zero;
            if let Some(_) = pi.next() {
                n_pi = ZeroOneOrMany::One;
                if let Some(_) = pi.next() {
                    n_pi = ZeroOneOrMany::Many;
                    // We don't care if there are more than two preds, so stop counting now.
                }
            }
            self.num_preds_per_ebb.push(n_pi);
        }
        debug_assert!(self.num_preds_per_ebb.len() == num_ebbs as usize);

        // The entry block must be the root of some tree, so set up the state to reflect that.
        let entry_ebb = func
            .layout
            .entry_block()
            .expect("do_redundant_fill_removal_on_function: entry ebb unknown");
        debug_assert!(self.num_preds_per_ebb[entry_ebb] == ZeroOneOrMany::Zero);
        self.num_preds_per_ebb[entry_ebb] = ZeroOneOrMany::Many;

        // Now build and process trees.
        for root_ix in 0..self.num_preds_per_ebb.len() {
            let root = Ebb::from_u32(root_ix as u32);

            // Build a tree for each node that has two or more preds, and ignore all other nodes.
            if self.num_preds_per_ebb[root] != ZeroOneOrMany::Many {
                continue;
            }

            // Clear out per-tree state.
            self.clear_for_new_tree();

            // Discovery phase: build the tree, as `root` and `self.nodes_in_tree`.
            self.add_nodes_to_tree(cfg, root);
            debug_assert!(self.nodes_in_tree.cardinality() > 0);
            debug_assert!(self.num_preds_per_ebb[root] == ZeroOneOrMany::Many);

            // Processing phase: do redundant-reload-removal.
            self.process_tree(func, reginfo, isa, root);
            debug_assert!(
                self.nodes_in_tree.cardinality() == self.nodes_already_visited.cardinality()
            );
        }
    }
}

// =============================================================================================
// Top level: the external interface

struct Context<'a> {
    // Current instruction as well as reference to function and ISA.
    cur: EncCursor<'a>,

    // Cached ISA information.  We save it here to avoid frequent virtual function calls on the
    // `TargetIsa` trait object.
    reginfo: RegInfo,

    // References to contextual data structures we need.
    cfg: &'a ControlFlowGraph,

    // The running state.
    state: &'a mut RedundantReloadRemover,
}

impl RedundantReloadRemover {
    /// Run the remover.
    pub fn run(&mut self, isa: &dyn TargetIsa, func: &mut Function, cfg: &ControlFlowGraph) {
        let ctx = Context {
            cur: EncCursor::new(func, isa),
            reginfo: isa.register_info(),
            cfg: cfg,
            state: &mut RedundantReloadRemover::new(),
        };
        let mut total_regunits = 0;
        for rb in isa.register_info().banks {
            total_regunits += rb.units;
        }
        ctx.state.num_regunits = Some(total_regunits);
        ctx.state.do_redundant_fill_removal_on_function(
            ctx.cur.func,
            &ctx.reginfo,
            ctx.cur.isa,
            &ctx.cfg,
        );
    }
}
