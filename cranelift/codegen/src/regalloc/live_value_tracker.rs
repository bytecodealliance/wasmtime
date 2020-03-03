//! Track which values are live in a block with instruction granularity.
//!
//! The `LiveValueTracker` keeps track of the set of live SSA values at each instruction in a block.
//! The sets of live values are computed on the fly as the tracker is moved from instruction to
//! instruction, starting at the block header.

use crate::dominator_tree::DominatorTree;
use crate::entity::{EntityList, ListPool};
use crate::fx::FxHashMap;
use crate::ir::{Block, DataFlowGraph, ExpandedProgramPoint, Inst, Layout, Value};
use crate::partition_slice::partition_slice;
use crate::regalloc::affinity::Affinity;
use crate::regalloc::liveness::Liveness;
use crate::regalloc::liverange::LiveRange;
use alloc::vec::Vec;

type ValueList = EntityList<Value>;

/// Compute and track live values throughout a block.
pub struct LiveValueTracker {
    /// The set of values that are live at the current program point.
    live: LiveValueVec,

    /// Saved set of live values for every jump and branch that can potentially be an immediate
    /// dominator of a block.
    ///
    /// This is the set of values that are live *before* the branch.
    idom_sets: FxHashMap<Inst, ValueList>,

    /// Memory pool for the live sets.
    idom_pool: ListPool<Value>,
}

/// Information about a value that is live at the current program point.
#[derive(Debug)]
pub struct LiveValue {
    /// The live value.
    pub value: Value,

    /// The local ending point of the live range in the current block, as returned by
    /// `LiveRange::def_local_end()` or `LiveRange::livein_local_end()`.
    pub endpoint: Inst,

    /// The affinity of the value as represented in its `LiveRange`.
    ///
    /// This value is simply a copy of the affinity stored in the live range. We copy it because
    /// almost all users of `LiveValue` need to look at it.
    pub affinity: Affinity,

    /// The live range for this value never leaves its block.
    pub is_local: bool,

    /// This value is dead - the live range ends immediately.
    pub is_dead: bool,
}

struct LiveValueVec {
    /// The set of values that are live at the current program point.
    values: Vec<LiveValue>,

    /// How many values at the front of `values` are known to be live after `inst`?
    ///
    /// This is used to pass a much smaller slice to `partition_slice` when its called a second
    /// time for the same instruction.
    live_prefix: Option<(Inst, usize)>,
}

impl LiveValueVec {
    fn new() -> Self {
        Self {
            values: Vec::new(),
            live_prefix: None,
        }
    }

    /// Add a new live value to `values`. Copy some properties from `lr`.
    fn push(&mut self, value: Value, endpoint: Inst, lr: &LiveRange) {
        self.values.push(LiveValue {
            value,
            endpoint,
            affinity: lr.affinity,
            is_local: lr.is_local(),
            is_dead: lr.is_dead(),
        });
    }

    /// Remove all elements.
    fn clear(&mut self) {
        self.values.clear();
        self.live_prefix = None;
    }

    /// Make sure that the values killed by `next_inst` are moved to the end of the `values`
    /// vector.
    ///
    /// Returns the number of values that will be live after `next_inst`.
    fn live_after(&mut self, next_inst: Inst) -> usize {
        // How many values at the front of the vector are already known to survive `next_inst`?
        // We don't need to pass this prefix to `partition_slice()`
        let keep = match self.live_prefix {
            Some((i, prefix)) if i == next_inst => prefix,
            _ => 0,
        };

        // Move the remaining surviving values to the front partition of the vector.
        let prefix = keep + partition_slice(&mut self.values[keep..], |v| v.endpoint != next_inst);

        // Remember the new prefix length in case we get called again for the same `next_inst`.
        self.live_prefix = Some((next_inst, prefix));
        prefix
    }

    /// Remove the values killed by `next_inst`.
    fn remove_kill_values(&mut self, next_inst: Inst) {
        let keep = self.live_after(next_inst);
        self.values.truncate(keep);
    }

    /// Remove any dead values.
    fn remove_dead_values(&mut self) {
        self.values.retain(|v| !v.is_dead);
        self.live_prefix = None;
    }
}

impl LiveValueTracker {
    /// Create a new blank tracker.
    pub fn new() -> Self {
        Self {
            live: LiveValueVec::new(),
            idom_sets: FxHashMap(),
            idom_pool: ListPool::new(),
        }
    }

    /// Clear all cached information.
    pub fn clear(&mut self) {
        self.live.clear();
        self.idom_sets.clear();
        self.idom_pool.clear();
    }

    /// Get the set of currently live values.
    ///
    /// Between calls to `process_inst()` and `drop_dead()`, this includes both values killed and
    /// defined by the current instruction.
    pub fn live(&self) -> &[LiveValue] {
        &self.live.values
    }

    /// Get a mutable set of currently live values.
    ///
    /// Use with care and don't move entries around.
    pub fn live_mut(&mut self) -> &mut [LiveValue] {
        &mut self.live.values
    }

    /// Move the current position to the top of `block`.
    ///
    /// This depends on the stored live value set at `block`'s immediate dominator, so that must have
    /// been visited first.
    ///
    /// Returns `(liveins, args)` as a pair of slices. The first slice is the set of live-in values
    /// from the immediate dominator. The second slice is the set of `block` parameters.
    ///
    /// Dead parameters with no uses are included in `args`. Call `drop_dead_args()` to remove them.
    pub fn block_top(
        &mut self,
        block: Block,
        dfg: &DataFlowGraph,
        liveness: &Liveness,
        layout: &Layout,
        domtree: &DominatorTree,
    ) -> (&[LiveValue], &[LiveValue]) {
        // Start over, compute the set of live values at the top of the block from two sources:
        //
        // 1. Values that were live before `block`'s immediate dominator, filtered for those that are
        //    actually live-in.
        // 2. Arguments to `block` that are not dead.
        //
        self.live.clear();

        // Compute the live-in values. Start by filtering the set of values that were live before
        // the immediate dominator. Just use the empty set if there's no immediate dominator (i.e.,
        // the entry block or an unreachable block).
        if let Some(idom) = domtree.idom(block) {
            // If the immediate dominator exits, we must have a stored list for it. This is a
            // requirement to the order blocks are visited: All dominators must have been processed
            // before the current block.
            let idom_live_list = self
                .idom_sets
                .get(&idom)
                .expect("No stored live set for dominator");
            // Get just the values that are live-in to `block`.
            for &value in idom_live_list.as_slice(&self.idom_pool) {
                let lr = liveness
                    .get(value)
                    .expect("Immediate dominator value has no live range");

                // Check if this value is live-in here.
                if let Some(endpoint) = lr.livein_local_end(block, layout) {
                    self.live.push(value, endpoint, lr);
                }
            }
        }

        // Now add all the live parameters to `block`.
        let first_arg = self.live.values.len();
        for &value in dfg.block_params(block) {
            let lr = &liveness[value];
            debug_assert_eq!(lr.def(), block.into());
            match lr.def_local_end().into() {
                ExpandedProgramPoint::Inst(endpoint) => {
                    self.live.push(value, endpoint, lr);
                }
                ExpandedProgramPoint::Block(local_block) => {
                    // This is a dead block parameter which is not even live into the first
                    // instruction in the block.
                    debug_assert_eq!(
                        local_block, block,
                        "block parameter live range ends at wrong block header"
                    );
                    // Give this value a fake endpoint that is the first instruction in the block.
                    // We expect it to be removed by calling `drop_dead_args()`.
                    self.live
                        .push(value, layout.first_inst(block).expect("Empty block"), lr);
                }
            }
        }

        self.live.values.split_at(first_arg)
    }

    /// Prepare to move past `inst`.
    ///
    /// Determine the set of already live values that are killed by `inst`, and add the new defined
    /// values to the tracked set.
    ///
    /// Returns `(throughs, kills, defs)` as a tuple of slices:
    ///
    /// 1. The `throughs` slice is the set of live-through values that are neither defined nor
    ///    killed by the instruction.
    /// 2. The `kills` slice is the set of values that were live before the instruction and are
    ///    killed at the instruction. This does not include dead defs.
    /// 3. The `defs` slice is guaranteed to be in the same order as `inst`'s results, and includes
    ///    dead defines.
    ///
    /// The order of `throughs` and `kills` is arbitrary.
    ///
    /// The `drop_dead()` method must be called next to actually remove the dead values from the
    /// tracked set after the two returned slices are no longer needed.
    pub fn process_inst(
        &mut self,
        inst: Inst,
        dfg: &DataFlowGraph,
        liveness: &Liveness,
    ) -> (&[LiveValue], &[LiveValue], &[LiveValue]) {
        // Save a copy of the live values before any branches or jumps that could be somebody's
        // immediate dominator.
        if dfg[inst].opcode().is_branch() {
            self.save_idom_live_set(inst);
        }

        // Move killed values to the end of the vector.
        // Don't remove them yet, `drop_dead()` will do that.
        let first_kill = self.live.live_after(inst);

        // Add the values defined by `inst`.
        let first_def = self.live.values.len();
        for &value in dfg.inst_results(inst) {
            let lr = &liveness[value];
            debug_assert_eq!(lr.def(), inst.into());
            match lr.def_local_end().into() {
                ExpandedProgramPoint::Inst(endpoint) => {
                    self.live.push(value, endpoint, lr);
                }
                ExpandedProgramPoint::Block(block) => {
                    panic!("Instruction result live range can't end at {}", block);
                }
            }
        }

        (
            &self.live.values[0..first_kill],
            &self.live.values[first_kill..first_def],
            &self.live.values[first_def..],
        )
    }

    /// Prepare to move past a ghost instruction.
    ///
    /// This is like `process_inst`, except any defs are ignored.
    ///
    /// Returns `(throughs, kills)`.
    pub fn process_ghost(&mut self, inst: Inst) -> (&[LiveValue], &[LiveValue]) {
        let first_kill = self.live.live_after(inst);
        self.live.values.as_slice().split_at(first_kill)
    }

    /// Drop the values that are now dead after moving past `inst`.
    ///
    /// This removes both live values that were killed by `inst` and dead defines on `inst` itself.
    ///
    /// This must be called after `process_inst(inst)` and before proceeding to the next
    /// instruction.
    pub fn drop_dead(&mut self, inst: Inst) {
        // Remove both live values that were killed by `inst` and dead defines from `inst`.
        self.live.remove_kill_values(inst);
    }

    /// Drop any values that are marked as `is_dead`.
    ///
    /// Use this after calling `block_top` to clean out dead block parameters.
    pub fn drop_dead_params(&mut self) {
        self.live.remove_dead_values();
    }

    /// Process new spills.
    ///
    /// Any values where `f` returns true are spilled and will be treated as if their affinity was
    /// `Stack`.
    pub fn process_spills<F>(&mut self, mut f: F)
    where
        F: FnMut(Value) -> bool,
    {
        for lv in &mut self.live.values {
            if f(lv.value) {
                lv.affinity = Affinity::Stack;
            }
        }
    }

    /// Save the current set of live values so it is associated with `idom`.
    fn save_idom_live_set(&mut self, idom: Inst) {
        let values = self.live.values.iter().map(|lv| lv.value);
        let pool = &mut self.idom_pool;
        // If there already is a set saved for `idom`, just keep it.
        self.idom_sets.entry(idom).or_insert_with(|| {
            let mut list = ValueList::default();
            list.extend(values, pool);
            list
        });
    }
}
