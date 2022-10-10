//! Last-store tracking via alias analysis.
//!
//! We partition memory state into several *disjoint pieces* of
//! "abstract state". There are a finite number of such pieces:
//! currently, we call them "heap", "table", "vmctx", and "other". Any
//! given address in memory belongs to exactly one disjoint piece.
//!
//! One never tracks which piece a concrete address belongs to at
//! runtime; this is a purely static concept. Instead, all
//! memory-accessing instructions (loads and stores) are labeled with
//! one of these four categories in the `MemFlags`. It is forbidden
//! for a load or store to access memory under one category and a
//! later load or store to access the same memory under a different
//! category. This is ensured to be true by construction during
//! frontend translation into CLIF and during legalization.
//!
//! Given that this non-aliasing property is ensured by the producer
//! of CLIF, we can compute a *may-alias* property: one load or store
//! may-alias another load or store if both access the same category
//! of abstract state.
//!
//! The "last store" pass helps to compute this aliasing: as we scan
//! the input CLIF to produce the egraph, we track the last
//! instruction that *might have* written to a given part of abstract
//! state. We also track the block containing this store. When we
//! enter a block no longer dominated by that block, we clear the info
//! to "unknown". (We could do a fixpoint analysis instead and resolve
//! merges this way, but in practice when iterating over a
//! structured-code CFG in RPO, our approach will do just as well.)
//!
//! We can't say for sure that the "last store" *did* actually write
//! that state, but we know for sure that no instruction *later* than
//! it (up to the current instruction) did. However, we can get a
//! must-alias property from this: if at a given load or store, we
//! look backward to the "last store", *AND* we find that it has
//! exactly the same address expression and type, then we know that
//! the current instruction's access *must* be to the same memory
//! location.
//!
//! To get this must-alias property, we leverage the node
//! hashconsing. We design the Eq/Hash (node identity relation
//! definition) of the `Node` struct so that all loads with (i) the
//! same "last store", and (ii) the same address expression, and (iii)
//! the same opcode-and-offset, will deduplicate (the first will be
//! computed, and the later ones will use the same value). Furthermore
//! we have a rewrite rule (in opts/store_to_load.isle) that rewrites
//! a load into the stored value of the last store *if* the last store
//! has the same address expression and constant offset.
//!
//! This gives us two optimizations, "redundant load elimination" and
//! "store-to-load forwarding".
//!
//! In theory we could also do *dead-store elimination*, where if a
//! store overwrites a value earlier written by another store, *and*
//! if no other load/store to the abstract state category occurred,
//! *and* no other trapping instruction occurred (at which point we
//! need an up-to-date memory state because post-trap-termination
//! memory state can be observed), *and* we can prove the original
//! store could not have trapped, then we can eliminate the original
//! store. Because this is so complex, and the conditions for doing it
//! correctly when post-trap state must be correct likely reduce the
//! potential benefit, we don't yet do this.

use crate::flowgraph::ControlFlowGraph;
use crate::fx::{FxHashMap, FxHashSet};
use crate::inst_predicates::has_memory_fence_semantics;
use crate::ir::{Block, Function, Inst, Opcode};
use cranelift_entity::EntityRef;
use cranelift_entity::SecondaryMap;
use smallvec::{smallvec, SmallVec};

/// For a given program point, the vector of last-store instruction
/// indices for each disjoint category of abstract state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LastStores {
    heap: MemoryState,
    table: MemoryState,
    vmctx: MemoryState,
    other: MemoryState,
}

/// State of memory seen by a load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum MemoryState {
    /// State at function entry: nothing is known (but it is one
    /// consistent value, so two loads from "entry" state at the same
    /// address will still provide the same result).
    #[default]
    Entry,
    /// State just after a store by the given instruction. The
    /// instruction is a store from which we can forward.
    Store(Inst),
    /// State just before the given instruction. Used for abstract
    /// value merges at merge-points when we cannot name a single
    /// producing site.
    BeforeInst(Inst),
    /// State just after the given instruction. Used when the
    /// instruction may update the associated state, but is not a
    /// store whose value we can cleanly forward. (E.g., perhaps a
    /// barrier of some sort.)
    AfterInst(Inst),
}

impl LastStores {
    fn update(&mut self, func: &Function, inst: Inst) {
        let opcode = func.dfg[inst].opcode();
        if has_memory_fence_semantics(opcode) {
            self.heap = MemoryState::AfterInst(inst);
            self.table = MemoryState::AfterInst(inst);
            self.vmctx = MemoryState::AfterInst(inst);
            self.other = MemoryState::AfterInst(inst);
        } else if opcode.can_store() {
            if let Some(memflags) = func.dfg[inst].memflags() {
                if memflags.heap() {
                    self.heap = MemoryState::Store(inst);
                } else if memflags.table() {
                    self.table = MemoryState::Store(inst);
                } else if memflags.vmctx() {
                    self.vmctx = MemoryState::Store(inst);
                } else {
                    self.other = MemoryState::Store(inst);
                }
            } else {
                self.heap = MemoryState::AfterInst(inst);
                self.table = MemoryState::AfterInst(inst);
                self.vmctx = MemoryState::AfterInst(inst);
                self.other = MemoryState::AfterInst(inst);
            }
        }
    }

    fn get_load_input_state(&self, func: &Function, inst: Inst) -> MemoryState {
        if let Some(memflags) = func.dfg[inst].memflags() {
            if memflags.heap() {
                self.heap
            } else if memflags.table() {
                self.table
            } else if memflags.vmctx() {
                self.vmctx
            } else {
                self.other
            }
        } else {
            MemoryState::AfterInst(inst)
        }
    }

    fn meet_from(&mut self, other: &LastStores, loc: Inst) {
        let meet = |a: MemoryState, b: MemoryState| -> MemoryState {
            match (a, b) {
                (a, b) if a == b => a,
                _ => MemoryState::BeforeInst(loc),
            }
        };

        self.heap = meet(self.heap, other.heap);
        self.table = meet(self.table, other.table);
        self.vmctx = meet(self.vmctx, other.vmctx);
        self.other = meet(self.other, other.other);
    }
}

/// An alias-analysis pass.
pub struct AliasAnalysis {
    /// Last-store instruction (or none) for a given load.
    load_mem_state: FxHashMap<Inst, MemoryState>,
}

impl AliasAnalysis {
    /// Perform an alias analysis pass.
    pub fn new(func: &Function, cfg: &ControlFlowGraph) -> AliasAnalysis {
        log::trace!("alias analysis: input is:\n{:?}", func);
        let mut analysis = AliasAnalysis {
            load_mem_state: FxHashMap::default(),
        };

        let block_input = analysis.compute_block_input_states(func, cfg);
        analysis.compute_load_last_stores(func, block_input);
        analysis
    }

    #[inline(never)]
    fn compute_block_input_states(
        &mut self,
        func: &Function,
        cfg: &ControlFlowGraph,
    ) -> SecondaryMap<Block, Option<LastStores>> {
        let mut block_input = SecondaryMap::with_default(None);
        block_input.resize(func.dfg.num_blocks());
        let mut queue: SmallVec<[Block; 8]> = smallvec![];
        let mut queue_set = FxHashSet::default();
        let entry = func.layout.entry_block().unwrap();
        queue.push(entry);
        queue_set.insert(entry);
        block_input[entry] = Some(LastStores::default());

        while let Some(block) = queue.pop() {
            queue_set.remove(&block);
            let mut state = block_input[block].clone().unwrap();

            log::trace!(
                "alias analysis: input to block{} is {:?}",
                block.index(),
                state
            );

            for inst in func.layout.block_insts(block) {
                state.update(func, inst);
                log::trace!("after inst{}: state is {:?}", inst.index(), state);
            }

            for succ in cfg.succ_iter(block) {
                let succ_first_inst = func.layout.block_insts(succ).into_iter().next().unwrap();
                let succ_state = &mut block_input[succ];
                let old = succ_state.clone();
                succ_state
                    .get_or_insert_with(|| LastStores::default())
                    .meet_from(&state, succ_first_inst);
                let updated = *succ_state != old;

                if updated && queue_set.insert(succ) {
                    queue.push(succ);
                }
            }
        }

        block_input
    }

    #[inline(never)]
    fn compute_load_last_stores(
        &mut self,
        func: &Function,
        block_input: SecondaryMap<Block, Option<LastStores>>,
    ) {
        for block in func.layout.blocks() {
            let mut state = block_input[block].clone().unwrap();

            for inst in func.layout.block_insts(block) {
                log::trace!(
                    "alias analysis: scanning at inst{} with state {:?} ({:?})",
                    inst.index(),
                    state,
                    func.dfg[inst],
                );

                let opcode = func.dfg[inst].opcode();
                if opcode == Opcode::Load {
                    let mem_state = state.get_load_input_state(func, inst);
                    log::trace!(
                        "alias analysis: at inst{}: load with mem_state {:?}",
                        inst.index(),
                        mem_state,
                    );

                    self.load_mem_state.insert(inst, mem_state);
                }

                state.update(func, inst);
            }
        }
    }

    /// Get the state seen by a load, if any.
    pub fn get_state_for_load(&self, inst: Inst) -> Option<MemoryState> {
        self.load_mem_state.get(&inst).cloned()
    }
}
