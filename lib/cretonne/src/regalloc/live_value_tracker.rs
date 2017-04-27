//! Track which values are live in an EBB with instruction granularity.
//!
//! The `LiveValueTracker` keeps track of the set of live SSA values at each instruction in an EBB.
//! The sets of live values are computed on the fly as the tracker is moved from instruction to
//! instruction, starting at the EBB header.

use dominator_tree::DominatorTree;
use entity_list::{EntityList, ListPool};
use ir::instructions::BranchInfo;
use ir::{Inst, Ebb, Value, DataFlowGraph, ProgramOrder, ExpandedProgramPoint};
use partition_slice::partition_slice;
use regalloc::affinity::Affinity;
use regalloc::liveness::Liveness;

use std::collections::HashMap;

type ValueList = EntityList<Value>;

/// Compute and track live values throughout an EBB.
pub struct LiveValueTracker {
    /// The set of values that are live at the current program point.
    live: LiveValueVec,

    /// Saved set of live values for every jump and branch that can potentially be an immediate
    /// dominator of an EBB.
    ///
    /// This is the set of values that are live *before* the branch.
    idom_sets: HashMap<Inst, ValueList>,

    /// Memory pool for the live sets.
    idom_pool: ListPool<Value>,
}

/// Information about a value that is live at the current program point.
pub struct LiveValue {
    /// The live value.
    pub value: Value,

    /// The local ending point of the live range in the current EBB, as returned by
    /// `LiveRange::def_local_end()` or `LiveRange::livein_local_end()`.
    pub endpoint: Inst,

    /// The affinity of the value as represented in its `LiveRange`.
    ///
    /// This value is simply a copy of the affinity stored in the live range. We copy it because
    /// almost all users of `LiveValue` need to look at it.
    pub affinity: Affinity,
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
    fn new() -> LiveValueVec {
        LiveValueVec {
            values: Vec::new(),
            live_prefix: None,
        }
    }

    /// Add a new live value to `values`.
    fn push(&mut self, value: Value, endpoint: Inst, affinity: Affinity) {
        self.values
            .push(LiveValue {
                      value,
                      endpoint,
                      affinity,
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
}

impl LiveValueTracker {
    /// Create a new blank tracker.
    pub fn new() -> LiveValueTracker {
        LiveValueTracker {
            live: LiveValueVec::new(),
            idom_sets: HashMap::new(),
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

    /// Move the current position to the top of `ebb`.
    ///
    /// This depends on the stored live value set at `ebb`'s immediate dominator, so that must have
    /// been visited first.
    ///
    /// Returns `(liveins, args)` as a pair or slices. The first slice is the set of live-in values
    /// from the immediate dominator. The second slice is the set of `ebb` arguments that are live.
    /// Dead arguments with no uses are ignored and not added to the set.
    pub fn ebb_top<PO: ProgramOrder>(&mut self,
                                     ebb: Ebb,
                                     dfg: &DataFlowGraph,
                                     liveness: &Liveness,
                                     program_order: &PO,
                                     domtree: &DominatorTree)
                                     -> (&[LiveValue], &[LiveValue]) {
        // Start over, compute the set of live values at the top of the EBB from two sources:
        //
        // 1. Values that were live before `ebb`'s immediate dominator, filtered for those that are
        //    actually live-in.
        // 2. Arguments to `ebb` that are not dead.
        //
        self.live.clear();

        // Compute the live-in values. Start by filtering the set of values that were live before
        // the immediate dominator. Just use the empty set if there's no immediate dominator (i.e.,
        // the entry block or an unreachable block).
        if let Some(idom) = domtree.idom(ebb) {
            // If the immediate dominator exits, we must have a stored list for it. This is a
            // requirement to the order EBBs are visited: All dominators must have been processed
            // before the current EBB.
            let idom_live_list = self.idom_sets
                .get(&idom)
                .expect("No stored live set for dominator");
            // Get just the values that are live-in to `ebb`.
            for &value in idom_live_list.as_slice(&self.idom_pool) {
                let lr = liveness
                    .get(value)
                    .expect("Immediate dominator value has no live range");

                // Check if this value is live-in here.
                if let Some(endpoint) = lr.livein_local_end(ebb, program_order) {
                    self.live.push(value, endpoint, lr.affinity);
                }
            }
        }

        // Now add all the live arguments to `ebb`.
        let first_arg = self.live.values.len();
        for &value in dfg.ebb_args(ebb) {
            let lr = liveness
                .get(value)
                .expect("EBB argument value has no live range");
            assert_eq!(lr.def(), ebb.into());
            match lr.def_local_end().into() {
                ExpandedProgramPoint::Inst(endpoint) => {
                    self.live.push(value, endpoint, lr.affinity);
                }
                ExpandedProgramPoint::Ebb(local_ebb) => {
                    // This is a dead EBB argument which is not even live into the first
                    // instruction in the EBB. We can ignore it.
                    assert_eq!(local_ebb,
                               ebb,
                               "EBB argument live range ends at wrong EBB header");
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
    /// Returns `(kills, defs)` as a pair of slices. The `defs` slice is guaranteed to be in the
    /// same order as `inst`'s results, and includes dead defines. The order of `kills` is
    /// arbitrary.
    ///
    /// The `drop_dead()` method must be called next to actually remove the dead values from the
    /// tracked set after the two returned slices are no longer needed.
    pub fn process_inst(&mut self,
                        inst: Inst,
                        dfg: &DataFlowGraph,
                        liveness: &Liveness)
                        -> (&[LiveValue], &[LiveValue]) {
        // Save a copy of the live values before any branches or jumps that could be somebody's
        // immediate dominator.
        match dfg[inst].analyze_branch(&dfg.value_lists) {
            BranchInfo::NotABranch => {}
            _ => self.save_idom_live_set(inst),
        }

        // Move killed values to the end of the vector.
        // Don't remove them yet, `drop_dead()` will do that.
        let first_kill = self.live.live_after(inst);

        // Add the values defined by `inst`.
        let first_def = self.live.values.len();
        for &value in dfg.inst_results(inst) {
            let lr = match liveness.get(value) {
                Some(lr) => lr,
                None => panic!("{} result {} has no live range", dfg[inst].opcode(), value),
            };
            assert_eq!(lr.def(), inst.into());
            match lr.def_local_end().into() {
                ExpandedProgramPoint::Inst(endpoint) => {
                    self.live.push(value, endpoint, lr.affinity);
                }
                ExpandedProgramPoint::Ebb(ebb) => {
                    panic!("Instruction result live range can't end at {}", ebb);
                }
            }
        }

        (&self.live.values[first_kill..first_def], &self.live.values[first_def..])
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

    /// Save the current set of live values so it is associated with `idom`.
    fn save_idom_live_set(&mut self, idom: Inst) {
        let values = self.live.values.iter().map(|lv| lv.value);
        let pool = &mut self.idom_pool;
        // If there already is a set saved for `idom`, just keep it.
        self.idom_sets
            .entry(idom)
            .or_insert_with(|| {
                                let mut list = ValueList::default();
                                list.extend(values, pool);
                                list
                            });
    }
}
