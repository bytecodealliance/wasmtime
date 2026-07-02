//! Alias analysis, consisting of a "last store" pass and a "memory
//! values" pass. These two passes operate as one fused pass, and so
//! are implemented together here.
//!
//! We partition memory state into several *disjoint regions* of
//! "abstract state". These regions are defined by `ir::AliasRegion`
//! and may correspond to distinct linear memories in Wasm, different
//! types (or fields) that cannot alias each other (known as
//! type-based alias analysis, or TBAA), unique stack slots,
//! etc... Any given address in memory belongs to at most one region.
//!
//! We never track which piece a concrete address belongs to at
//! runtime; this is a purely static concept. Instead, all
//! memory-accessing instructions (loads and stores) are tagged with
//! one of these regions in their `ir::MemFlagsData`. It is forbidden
//! for one instruction tagged with region `R` to access a memory
//! location `L` and then for another instruction tagged with region
//! `S` to access the same memory location `L`. This invariant must be
//! provided by the CLIF-producing frontend.
//!
//! Given that this non-aliasing property is provided by the CLIF
//! producer, we can compute a *may-alias* property: one load or store
//! may-alias another load or store if both access the same region.
//!
//! The "last store" pass helps to compute this aliasing: it scans the
//! code, finding at each program point the last instruction that
//! *might have* written to a given region.
//!
//! We can't say for sure that the "last store" *did* actually write
//! that region, but we know for sure that no instruction *later* than
//! it (up to the current instruction) did. However, we can derive a
//! *must-alias* property from this: if at a given load or store, we
//! look backward to the "last store", *AND* we find that it has
//! exactly the same address expression and type, then we know that
//! the current instruction's access *must* be to the same memory
//! location.
//!
//! To get this must-alias property, we compute a sparse table of
//! "memory values": these are known equivalences between SSA `Value`s
//! and particular locations in memory. The memory-values table is a
//! mapping from (last store, address expression, type) to SSA
//! value. At a store, we can insert into this table directly. At a
//! load, we can also insert, if we don't already have a value (from
//! the store that produced the load's value).
//!
//! Then we do a few optimizations at once given this table:
//!
//! * If a load accesses a location identified by a (last store,
//!   address, type) key already in the table, we replace it with the
//!   SSA value for that memory location. This is usually known as
//!   "redundant load elimination" if the value came from an earlier
//!   load of the same location, or "store-to-load forwarding" if the
//!   value came from an earlier store to the same location.
//!
//! * If a store writes the same value that is already in the table
//!   for its memory location, then we can elide this store because it
//!   doesn't actually modify memory. We call this "idempotent-store
//!   elimination".
//!
//! * If a store overwrites a key in the table, *and* if this
//!   overwriting store always executes after the original store
//!   (i.e. this store post-dominates the original), *and* if no other
//!   instruction has "observed" the associated alias region in
//!   between, then we can eliminate the original store. This is
//!   called "dead-store elimination". Note that observing an alias
//!   region is not just loading from it, all potentially-trapping
//!   instructions must be treated as observing all regions because we
//!   must preserve post-trap memory state.

use crate::cursor::CursorPosition;
use crate::{FxHashMap, FxHashSet};
use crate::{
    cursor::{Cursor, FuncCursor},
    dominator_tree::DominatorTree,
    flowgraph::ControlFlowGraph,
    inst_predicates::{inst_addr_offset_type, inst_store_data, visit_block_succs},
    ir::{AliasRegion, Block, Function, Inst, Opcode, Type, Value, immediates::Offset32},
    post_dominator_tree::PostDominatorTree,
    trace,
};
use core::cmp::Ordering;
use cranelift_entity::{EntityRef, SecondaryMap, packed_option::PackedOption};

/// Determine whether this opcode behaves as a memory fence, i.e.,
/// prohibits any moving of memory accesses across it.
fn has_memory_fence_semantics(op: Opcode) -> bool {
    match op {
        Opcode::AtomicRmw
        | Opcode::AtomicCas
        | Opcode::AtomicLoad
        | Opcode::AtomicStore
        | Opcode::Fence
        | Opcode::Debugtrap
        | Opcode::SequencePoint => true,
        Opcode::Call | Opcode::CallIndirect | Opcode::TryCall | Opcode::TryCallIndirect => true,
        _ => false,
    }
}

/// A description of which alias region(s) can an instruction observe.
enum AliasRegionsObserved {
    /// All alias regions.
    All,
    /// Just the given alias region.
    Just(AliasRegion),
    /// Just the "other" / missing alias region.
    Other,
    /// No alias regions observed.
    None,
}

/// Which alias region(s) can an instruction observe?
fn alias_regions_observed(func: &Function, inst: Inst, opcode: Opcode) -> AliasRegionsObserved {
    debug_assert_eq!(func.dfg.insts[inst].opcode(), opcode);
    if opcode.is_return()
        || opcode.is_call()
        || opcode.can_trap()
        // NB: the `opcode.can_trap()` check above only covers explicitly
        // trapping instructions (like `trap` and `trapz`), not loads/stores
        // that can implicitly trap; we check those via their memflags.
        || func.dfg.insts[inst]
            .memflags_data(&func.dfg)
            .and_then(|flags| flags.trap_code())
            .is_some()
    {
        return AliasRegionsObserved::All;
    }

    if opcode.can_load() {
        if let Some(region) = func.dfg.insts[inst].alias_region(&func.dfg) {
            AliasRegionsObserved::Just(region)
        } else {
            AliasRegionsObserved::Other
        }
    } else {
        AliasRegionsObserved::None
    }
}

/// The last-store state for a single named alias region: the last store to the
/// region (if any) and whether that region has been observed since.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RegionState {
    /// Last store to this region.
    last_store: PackedOption<Inst>,

    /// Whether this region has been observed since it was last stored to.
    observed: bool,
}

/// For a given program point, the vector of last-store instruction
/// indices for each disjoint category of abstract state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LastStores {
    /// Last store (and whether it has been observed) for each named alias
    /// region.
    regions: SecondaryMap<AliasRegion, RegionState>,

    /// Last store for memory accesses with no alias region.
    other: PackedOption<Inst>,

    /// Whether the other/missing alias region has been observed since it was
    /// last stored to.
    observed_other: bool,

    /// Last instruction with fence semantics. This applies to ALL regions,
    /// including ones not yet in the `regions` map.
    last_fence: PackedOption<Inst>,

    /// Whether all alias regions have been observed since they were last stored
    /// to (e.g. via a memory fence or a call).
    observed_all: bool,
}

impl LastStores {
    pub(crate) fn update(&mut self, func: &Function, inst: Inst) {
        let opcode = func.dfg.insts[inst].opcode();

        if has_memory_fence_semantics(opcode) {
            self.fence(inst);
        }
        // Explicitly trapping instructions (`trap`, `trapz`, `udiv`,
        // `uadd_overflow_trap`, etc... but not loads/stores that can implicitly
        // trap): allow store-to-load forwarding across these instructions, but
        // do not eliminate dead stores across them, as that would change the
        // state of memory on trap. We do this by marking every region with a
        // last-store as observed, but not clearing its last-store information.
        else if opcode.can_trap() {
            self.observe_others(None);
        }
        // Store instructions: update the last-store information for this
        // instruction's alias region, or, if it has no alias region, treat it
        // as a fence.
        else if opcode.can_store() {
            if let Some(memflags) = func.dfg.insts[inst].memflags() {
                match func.dfg.mem_flags[memflags].alias_region() {
                    Some(region) => {
                        self.regions[region] = RegionState {
                            last_store: inst.into(),
                            observed: false,
                        };

                        // And if this store can trap, then we need to observe
                        // all other alias regions, to ensure that their state
                        // is preserved in the case that this store traps
                        // (similar to the `can_trap()` handling above).
                        //
                        // This prevents removing the first store in the
                        // following snippet, for example:
                        //
                        //     store notrap region0 v0, v3+8
                        //     store user42 region1 v1, v4+16
                        //     store notrap region0 v2, v3+8
                        //
                        // Removing it would be invalid because it drops a
                        // memory store to `v3+8` that would otherwise have been
                        // performed when writing to `v4+16` traps.
                        //
                        // On the other hand, if it cannot trap, then we need to
                        // observe all the regions whose last-store *can* trap
                        // so that we don't allow a non-trapping store to
                        // effectively be moved ahead of a trapping store:
                        //
                        //     store user42 region0 v0, v3+8
                        //     store notrap region1 v1, v4+16
                        //     store user42 region0 v2, v3+8
                        //
                        // In this case, removing the first store would mean
                        // that when writing to `v3+8` traps, we would
                        // incorrectly store to `v4+16`, when we otherwise
                        // wouldn't have.
                        if func.dfg.mem_flags[memflags].trap_code().is_some() {
                            self.observe_others(Some(region));
                        } else {
                            self.observe_trapping_others(region, func);
                        }
                    }
                    None => {
                        // A store with no alias region may alias any region, so
                        // treat it like a fence.
                        self.fence(inst);
                    }
                }
            } else {
                // Store with no memflags (and therefore no region):
                // treat it like a fence.
                self.fence(inst);
            }
        }
        // Everything else: determine which, if any, alias regions this
        // instruction observes.
        else {
            match alias_regions_observed(func, inst, opcode) {
                AliasRegionsObserved::All => {
                    self.observed_all = true;
                }
                AliasRegionsObserved::Just(region) => {
                    self.regions[region].observed = true;
                    // NB: Because stores without regions may alias any other
                    // region, we have also observed the last-store in
                    // `self.other`.
                    self.observed_other = true;
                }
                AliasRegionsObserved::Other => {
                    self.observed_other = true;
                }
                AliasRegionsObserved::None => {}
            }
        }
    }

    /// Mark all regions except for `excluding` (if given) as observed.
    fn observe_others(&mut self, excluding: Option<AliasRegion>) {
        for (region, state) in self.regions.iter_mut() {
            if state.last_store.is_some() && excluding.is_none_or(|r| r != region) {
                state.observed = true;
            }
        }
        self.observed_other = true;
    }

    /// Mark all regions whose last-store can trap as observed, except for
    /// `excluding`.
    fn observe_trapping_others(&mut self, excluding: AliasRegion, func: &Function) {
        for (region, state) in self.regions.iter_mut() {
            if region != excluding
                && state
                    .last_store
                    .expand()
                    .is_some_and(|s| func.dfg.insts[s].memflags_trap_code(&func.dfg).is_some())
            {
                state.observed = true;
            }
        }

        self.observed_other |= self
            .other
            .expand()
            .is_some_and(|s| func.dfg.insts[s].memflags_trap_code(&func.dfg).is_some());
    }

    /// Handle memory fence-like instructions by clearing all analysis data.
    fn fence(&mut self, inst: Inst) {
        self.regions.clear();
        self.other = inst.into();
        self.observed_other = false;
        self.last_fence = inst.into();
        self.observed_all = false;
    }

    /// Get the last-store instruction for the given `inst`'s alias region, if
    /// any, and whether that alias region has been observed or not.
    fn get_last_store(&self, func: &Function, inst: Inst) -> (PackedOption<Inst>, bool) {
        if let Some(memflags) = func.dfg.insts[inst].memflags() {
            match func.dfg.mem_flags[memflags].alias_region() {
                None => return (self.other, self.observed_all || self.observed_other),
                Some(region) => {
                    let region_state = self.regions[region];
                    // If the region has never been explicitly stored to,
                    // fall back to the last fence (which affects all regions).
                    if region_state.last_store.is_none() {
                        return (self.last_fence, self.observed_all);
                    } else {
                        return (
                            region_state.last_store,
                            self.observed_all || region_state.observed,
                        );
                    }
                }
            }
        }

        let opcode = func.dfg.insts[inst].opcode();
        if opcode.can_load() || opcode.can_store() {
            (
                inst.into(),
                opcode.can_load() || self.observed_all || self.observed_other,
            )
        } else {
            (None.into(), true)
        }
    }

    /// Meet `self` with `rhs`, placing the result in `self`.
    ///
    /// Returns `true` if `self` changed, `false` otherwise.
    fn meet_from(&mut self, rhs: &LastStores, loc: Inst) -> bool {
        // NB: Destructure to make sure we don't accidentally forget a
        // field.
        let LastStores {
            regions,
            other,
            observed_other,
            last_fence,
            observed_all,
        } = self;

        let meet = |a: &mut PackedOption<Inst>, b: PackedOption<Inst>| -> bool {
            let old = a.expand();
            let new = match (old, b.expand()) {
                (None, None) => None,
                (Some(a), Some(b)) if a == b => Some(a),
                _ => Some(loc),
            };
            *a = new.into();
            old != new
        };

        let union_bool = |a: &mut bool, b: bool| -> bool {
            let old = *a;
            *a = *a || b;
            *a != old
        };

        let mut changed = false;

        let max_len = core::cmp::max(regions.keys().len(), rhs.regions.keys().len());
        for i in 0..max_len {
            let region = AliasRegion::new(i);
            let rhs_state = rhs.regions[region];
            let state = &mut regions[region];
            changed |= meet(&mut state.last_store, rhs_state.last_store);
            // Union the observed bit: a region is observed after the meet if it
            // was observed on either incoming path.
            changed |= union_bool(&mut state.observed, rhs_state.observed);
        }

        changed |= meet(other, rhs.other);
        changed |= union_bool(observed_other, rhs.observed_other);

        changed |= meet(last_fence, rhs.last_fence);
        changed |= union_bool(observed_all, rhs.observed_all);

        changed
    }
}

/// A key identifying a unique memory location.
///
/// For the result of a load to be equivalent to the result of another
/// load, or the store data from a store, we need for (i) the
/// "version" of memory (here ensured by having the same last store
/// instruction to touch the disjoint category of abstract state we're
/// accessing); (ii) the address must be the same (here ensured by
/// having the same SSA value, which doesn't change after computed);
/// (iii) the offset must be the same; and (iv) the accessed type and
/// extension mode (e.g., 8-to-32, signed) must be the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct MemoryLoc {
    last_store: PackedOption<Inst>,
    address: Value,
    offset: Offset32,
    ty: Type,
    /// We keep the *opcode* of the instruction that produced the
    /// value we record at this key if the opcode is anything other
    /// than an ordinary load or store. This is needed when we
    /// consider loads that extend the value: e.g., an 8-to-32
    /// sign-extending load will produce a 32-bit value from an 8-bit
    /// value in memory, so we can only reuse that (as part of RLE)
    /// for another load with the same extending opcode.
    ///
    /// We could improve the transform to insert explicit extend ops
    /// in place of extending loads when we know the memory value, but
    /// we haven't yet done this.
    extending_opcode: Option<Opcode>,
}

/// The result of processing an instruction through alias analysis.
pub enum OptResult {
    /// No optimization applied.
    None,
    /// A redundant load; alias its result to this value.
    AliasedLoad(Value),
    /// An idempotent store; remove it.
    IdempotentStore,
    /// We determined that an instruction is a dead store and its memory write
    /// cannot be observed.
    DeadStore {
        /// The store instruction that is dead.
        dead: Inst,
        /// The other store instruction that makes the previous instruction
        /// dead.
        overwriter: Inst,
    },
}

/// An alias-analysis pass.
pub struct AliasAnalysis<'a> {
    /// The domtree for the function.
    domtree: &'a DominatorTree,

    /// The post-dominator tree for the function.
    ///
    /// This is computed lazily, on the first cross-block dead-store candidate,
    /// because it is only ever consulted by dead-store elimination and only for
    /// candidates whose two stores live in different blocks (because we can
    /// easily test post-domination without building a post-dominator tree when
    /// the two instructions are in the same block). The majority of functions
    /// have no such dead-store candidates, so building it eagerly for every
    /// function is a waste.
    post_dom_tree: Option<PostDominatorTree>,

    /// Input state to a basic block.
    block_input: FxHashMap<Block, LastStores>,

    /// Known memory-value equivalences. This is the result of the
    /// analysis. This is a mapping from (last store, address
    /// expression, offset, type) to SSA `Value`.
    ///
    /// We keep the defining inst around for quick dominance checks.
    mem_values: FxHashMap<MemoryLoc, (Inst, Value)>,
}

impl<'a> AliasAnalysis<'a> {
    /// Perform an alias analysis pass.
    pub fn new(func: &Function, domtree: &'a DominatorTree) -> AliasAnalysis<'a> {
        trace!("alias analysis: input is:\n{:?}", func);
        assert!(domtree.is_valid());
        let mut analysis = AliasAnalysis {
            domtree,
            post_dom_tree: None,
            block_input: FxHashMap::default(),
            mem_values: FxHashMap::default(),
        };

        analysis.compute_block_input_states(func);
        analysis
    }

    /// Does `overwriter` post-dominate `maybe_dead`?
    ///
    /// That is, does every path from `maybe_dead` out of the function pass
    /// through `overwriter`? Used as part of deciding whether `maybe_dead` is
    /// truly a dead store.
    fn post_dominates_maybe_dead_store(
        &mut self,
        func: &Function,
        cfg: &ControlFlowGraph,
        overwriter: Inst,
        maybe_dead: Inst,
    ) -> bool {
        let overwriter_block = func.layout.inst_block(overwriter).unwrap();
        let maybe_dead_block = func.layout.inst_block(maybe_dead).unwrap();

        // When our instructions are in the same block, we do not need to
        // force computation of the whole post-dominator tree: `overwriter`
        // post-dominates `maybe_dead` iff `overwriter` is at or after
        // `maybe_dead`.
        if overwriter_block == maybe_dead_block {
            return func.layout.pp_cmp(overwriter, maybe_dead) != Ordering::Less;
        }

        self.post_dom_tree
            .get_or_insert_with(|| PostDominatorTree::with_cfg(cfg))
            .post_dominates(overwriter, maybe_dead, &func.layout)
    }

    fn compute_block_input_states(&mut self, func: &Function) {
        let mut queue = vec![];
        let mut queue_set = FxHashSet::default();
        let entry = func.layout.entry_block().unwrap();
        queue.push(entry);
        queue_set.insert(entry);

        while let Some(block) = queue.pop() {
            queue_set.remove(&block);
            let mut state = self
                .block_input
                .entry(block)
                .or_insert_with(|| LastStores::default())
                .clone();

            trace!(
                "alias analysis: input to block{} is {:?}",
                block.index(),
                state
            );

            for inst in func.layout.block_insts(block) {
                state.update(func, inst);
                trace!("after inst{}: state is {:?}", inst.index(), state);
            }

            visit_block_succs(func, block, |_inst, succ, _from_table| {
                let succ_first_inst = func.layout.block_insts(succ).next().unwrap();
                let updated = match self.block_input.get_mut(&succ) {
                    Some(succ_state) => succ_state.meet_from(&state, succ_first_inst),
                    None => {
                        self.block_input.insert(succ, state.clone());
                        true
                    }
                };

                if updated && queue_set.insert(succ) {
                    queue.push(succ);
                }
            });
        }
    }

    /// Get the starting state for a block.
    pub fn block_starting_state(&self, block: Block) -> LastStores {
        self.block_input
            .get(&block)
            .cloned()
            .unwrap_or_else(|| LastStores::default())
    }

    /// Process one instruction. Meant to be invoked in program order
    /// within a block, and ideally in RPO or at least some domtree
    /// preorder for maximal reuse.
    pub fn process_inst(
        &mut self,
        func: &mut Function,
        cfg: &ControlFlowGraph,
        state: &mut LastStores,
        inst: Inst,
    ) -> OptResult {
        trace!(
            "process_inst: {inst}: {}\n\twith last stores: {state:?}\n\twith mem values = {:?}",
            func.dfg.display_inst(inst),
            self.mem_values,
        );

        let result = if let Some((address, offset, ty)) = inst_addr_offset_type(func, inst) {
            let address = func.dfg.resolve_aliases(address);
            let opcode = func.dfg.insts[inst].opcode();

            if opcode.can_store() {
                let store_data = inst_store_data(func, inst).unwrap();
                let store_data = func.dfg.resolve_aliases(store_data);

                let (last_store, observed_last_store) = state.get_last_store(func, inst);

                // Check whether this store makes the last store dead.
                if let Some(last_store) = last_store.expand() {
                    // A store can only be dead if we haven't observed
                    // its alias region yet.
                    if !observed_last_store
                        // This instruction doesn't make the last
                        // store dead if it itself is the last store.
                        && inst != last_store
                        // The last store isn't dead if this
                        // instruction is a fetch-add or something
                        // like that, as these instructions first load
                        // from (and therefore observe) memory before
                        // storing to it.
                        && !func.dfg.insts[inst].opcode().can_load()
                        // `last_store` must really be a store that
                        // writes exactly the bytes this store
                        // overwrites (same region, address, offset,
                        // type, and width).
                        && fully_overwrites(func, last_store, inst, address, offset, ty)
                        // We can't remove dead stores if they've
                        // already been removed, and `post_dominates`
                        // requires that `last_store` is in the
                        // layout.
                        && func.layout.inst_block(last_store).is_some()
                        // The last store is only dead if all paths
                        // out of the function from it go through this
                        // instruction.
                        && self.post_dominates_maybe_dead_store(func, cfg, inst, last_store)
                    {
                        trace!(
                            "  --> discovered dead store at {last_store}: {}",
                            func.dfg.display_inst(last_store)
                        );
                        return OptResult::DeadStore {
                            dead: last_store,
                            overwriter: inst,
                        };
                    }
                }

                let check_loc = MemoryLoc {
                    last_store,
                    address,
                    offset,
                    ty,
                    extending_opcode: get_ext_opcode(opcode),
                };
                if let Some((def_inst, known_value)) = self.mem_values.get(&check_loc).cloned() {
                    // Check for idempotent stores, where we are
                    // storing the exact same value back to a location
                    // that already has that value.
                    if known_value == store_data
                        // We cannot remove an idempotent store if we already
                        // removed the original store instruction (perhaps
                        // because this instruction made it dead).
                        && func.layout.inst_block(def_inst).is_some()
                        // We cannot remove this store unless all control-flow
                        // paths leading to it go through the original store
                        // instruction.
                        && self.domtree.dominates(def_inst, inst, &func.layout)
                    {
                        trace!("  --> idempotent store of {store_data} to loc {check_loc:?}");
                        return OptResult::IdempotentStore;
                    }
                }

                // Otherwise, update our state to reflect this store.
                let mem_loc = MemoryLoc {
                    last_store: inst.into(),
                    address,
                    offset,
                    ty,
                    extending_opcode: get_ext_opcode(opcode),
                };
                trace!("  --> updating known values in memory: {mem_loc:?} = {store_data}");
                self.mem_values.insert(mem_loc, (inst, store_data));

                OptResult::None
            } else if opcode.can_load() {
                let (last_store, _observed_last_store) = state.get_last_store(func, inst);
                let load_result = func.dfg.inst_results(inst)[0];
                let mem_loc = MemoryLoc {
                    last_store,
                    address,
                    offset,
                    ty,
                    extending_opcode: get_ext_opcode(opcode),
                };
                trace!("  load with last_store at loc {mem_loc:?}");

                // Is there a Value already known to be stored
                // at this specific memory location?  If so,
                // we can alias the load result to this
                // already-known Value.
                //
                // Check if the definition dominates this
                // location; it might not, if it comes from a
                // load (stores will always dominate though if
                // their `last_store` survives through
                // meet-points to this use-site).
                let aliased = if let Some((def_inst, value)) =
                    self.mem_values.get(&mem_loc).cloned()
                {
                    trace!("  see known value {value} from {def_inst}");
                    if self.domtree.dominates(def_inst, inst, &func.layout) {
                        trace!(
                            "  --> dominates; inserting value equivalence from {load_result} to {value}",
                        );
                        Some(value)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Otherwise, we can keep *this* load around
                // as a new equivalent value.
                if aliased.is_none() {
                    trace!("  --> inserting load result {load_result} at loc {mem_loc:?}");
                    self.mem_values.insert(mem_loc, (inst, load_result));
                }

                match aliased {
                    Some(value) => {
                        // NB: Early return to skip the `state.update` below --
                        // store-to-load forwarding does not observe the store
                        // and its region and should not prevent the store from
                        // being dead-store eliminated.
                        return OptResult::AliasedLoad(value);
                    }
                    None => OptResult::None,
                }
            } else {
                OptResult::None
            }
        } else {
            OptResult::None
        };

        state.update(func, inst);

        result
    }

    /// Make a pass and update known-redundant loads to aliased
    /// values. We interleave the updates with the memory-location
    /// tracking because resolving some aliases may expose others
    /// (e.g. in cases of double-indirection with two separate chains
    /// of loads).
    pub fn compute_and_update_aliases(&mut self, func: &mut Function, cfg: &ControlFlowGraph) {
        let mut pos = FuncCursor::new(func);

        while let Some(block) = pos.next_block() {
            let mut state = self.block_starting_state(block);
            while let Some(inst) = pos.next_inst() {
                match self.process_inst(pos.func, cfg, &mut state, inst) {
                    OptResult::None => {}
                    OptResult::AliasedLoad(replaced_result) => {
                        let result = pos.func.dfg.inst_results(inst)[0];
                        pos.func.dfg.clear_results(inst);
                        pos.func.dfg.change_to_alias(result, replaced_result);
                        pos.remove_inst_and_step_back();
                    }
                    OptResult::IdempotentStore => {
                        pos.remove_inst_and_step_back();
                    }
                    OptResult::DeadStore {
                        dead,
                        overwriter: _,
                    } => {
                        assert!(
                            !matches!(pos.position(), CursorPosition::At(other) if dead == other)
                        );
                        pos.func.layout.remove_inst(dead);
                    }
                }
            }
        }
    }
}

fn get_ext_opcode(op: Opcode) -> Option<Opcode> {
    debug_assert!(op.can_load() || op.can_store());
    match op {
        Opcode::Load | Opcode::Store => None,
        _ => Some(op),
    }
}

/// Can `overwriter` make `maybe_dead` a dead store?
///
/// Only if `maybe_dead` is itself a store that writes exactly the
/// bytes `overwriter` overwrites: the same alias region, address,
/// offset, type, and store width (i.e. extending/truncating
/// opcode). Otherwise some (or all) of `maybe_dead`'s bytes may
/// remain observable after `overwriter` runs, and removing
/// `maybe_dead` would change the program's behavior.
///
/// `overwriter_addr` must already have had its value-aliases resolved
/// (as the caller does for the overwriter's address).
fn fully_overwrites(
    func: &Function,
    maybe_dead: Inst,
    overwriter: Inst,
    overwriter_addr: Value,
    overwriter_offset: Offset32,
    overwriter_ty: Type,
) -> bool {
    debug_assert!(!func.dfg.value_is_alias(overwriter_addr));

    let maybe_dead_opcode = func.dfg.insts[maybe_dead].opcode();

    // `maybe_dead` must really be a store (this rejects the `last_fence` and merge
    // fallbacks that point at calls/fences/atomics or unrelated instructions).
    if !maybe_dead_opcode.can_store() {
        return false;
    }

    // Both must write the same number of bytes: e.g. `istore8` and `store`
    // write different widths even when their value types are equal.
    let overwriter_opcode = func.dfg.insts[overwriter].opcode();
    if get_ext_opcode(maybe_dead_opcode) != get_ext_opcode(overwriter_opcode) {
        return false;
    }

    // Both must target the same alias region; a store to one region cannot make
    // a store to a disjoint region dead.
    if func.dfg.insts[maybe_dead].alias_region(&func.dfg)
        != func.dfg.insts[overwriter].alias_region(&func.dfg)
    {
        return false;
    }

    // Both must have the same trap code, if any. Otherwise, removing
    // `maybe_dead` could change which code an execution traps with.
    if func.dfg.insts[maybe_dead]
        .memflags()
        .and_then(|f| func.dfg.mem_flags[f].trap_code())
        != func.dfg.insts[overwriter]
            .memflags()
            .and_then(|f| func.dfg.mem_flags[f].trap_code())
    {
        return false;
    }

    // Both must write the same address, offset, and type.
    match inst_addr_offset_type(func, maybe_dead) {
        Some((addr, offset, ty)) => {
            func.dfg.resolve_aliases(addr) == overwriter_addr
                && offset == overwriter_offset
                && ty == overwriter_ty
        }
        None => false,
    }
}
