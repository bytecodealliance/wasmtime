//! Index/slot allocator policies for the pooling allocator.

use super::PoolingAllocationStrategy;
use crate::CompiledModuleId;
use rand::Rng;
use std::collections::HashMap;

/// A slot index. The job of this allocator is to hand out these
/// indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotId(pub usize);
impl SlotId {
    /// The index of this slot.
    pub fn index(self) -> usize {
        self.0
    }
}

/// An index in the global freelist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalFreeListIndex(usize);
impl GlobalFreeListIndex {
    /// The index of this slot.
    fn index(self) -> usize {
        self.0
    }
}

/// An index in a per-module freelist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerModuleFreeListIndex(usize);
impl PerModuleFreeListIndex {
    /// The index of this slot.
    fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug)]
pub(crate) enum PoolingAllocationState {
    NextAvailable(Vec<SlotId>),
    Random(Vec<SlotId>),
    /// Reuse-affinity policy state.
    ///
    /// The data structures here deserve a little explanation:
    ///
    /// - free_list: this is a vec of slot indices that are free, no
    ///   matter their affinities (or no affinity at all).
    /// - per_module: this is a hashmap of vecs of slot indices that
    ///   are free, with affinity for particular module IDs. A slot may
    ///   appear in zero or one of these lists.
    /// - slot_state: indicates what state each slot is in: allocated
    ///   (Taken), only in free_list (Empty), or in free_list and a
    ///   per_module list (Affinity).
    ///
    /// The slot state tracks a slot's index in the global and
    /// per-module freelists, so it can be efficiently removed from
    /// both. We take some care to keep these up-to-date as well.
    ///
    /// On allocation, we first try to find a slot with affinity for
    /// the given module ID, if any. If not, we pick a random slot
    /// ID. This random choice is unbiased across all free slots.
    ReuseAffinity {
        /// Free-list of all slots. We use this to pick a victim when
        /// we don't have an appropriate slot with the preferred
        /// affinity.
        free_list: Vec<SlotId>,
        /// Invariant: any module ID in this hashmap must have a
        /// non-empty list of free slots (otherwise we remove it). We
        /// remove a module's freelist when we have no more slots with
        /// affinity for that module.
        per_module: HashMap<CompiledModuleId, Vec<SlotId>>,
        /// The state of any given slot. Records indices in the above
        /// list (empty) or two lists (with affinity), and these
        /// indices are kept up-to-date to allow fast removal.
        slot_state: Vec<SlotState>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum SlotState {
    /// Currently allocated.
    ///
    /// Invariant: no slot in this state has its index in either
    /// `free_list` or any list in `per_module`.
    Taken(Option<CompiledModuleId>),
    /// Currently free. A free slot is able to be allocated for any
    /// request, but may have affinity to a certain module that we
    /// prefer to use it for.
    ///
    /// Invariant: every slot in this state has its index in at least
    /// `free_list`, and possibly a `per_module` free-list; see
    /// FreeSlotState.
    Free(FreeSlotState),
}

impl SlotState {
    fn unwrap_free(&self) -> &FreeSlotState {
        match self {
            &Self::Free(ref free) => free,
            _ => panic!("Slot not free"),
        }
    }

    fn unwrap_free_mut(&mut self) -> &mut FreeSlotState {
        match self {
            &mut Self::Free(ref mut free) => free,
            _ => panic!("Slot not free"),
        }
    }

    fn unwrap_module_id(&self) -> Option<CompiledModuleId> {
        match self {
            &Self::Taken(module_id) => module_id,
            _ => panic!("Slot not in Taken state"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum FreeSlotState {
    /// The slot is free, and has no affinity.
    ///
    /// Invariant: every slot in this state has its index in
    /// `free_list`. No slot in this state has its index in any other
    /// (per-module) free-list.
    NoAffinity {
        /// Index in the global free list.
        ///
        /// Invariant: free_list[slot_state[i].free_list_index] == i.
        free_list_index: GlobalFreeListIndex,
    },
    /// The slot is free, and has an affinity for some module. This
    /// means we prefer to choose this slot (or some other one with
    /// the same affinity) given a request to allocate a slot for this
    /// module. It can, however, still be used for any other module if
    /// needed.
    ///
    /// Invariant: every slot in this state has its index in both
    /// `free_list` *and* exactly one list in `per_module`.
    Affinity {
        module: CompiledModuleId,
        /// Index in the global free list.
        ///
        /// Invariant: free_list[slot_state[i].free_list_index] == i.
        free_list_index: GlobalFreeListIndex,
        /// Index in a per-module free list.
        ///
        /// Invariant: per_module[slot_state[i].module][slot_state[i].per_module_index]
        /// == i.
        per_module_index: PerModuleFreeListIndex,
    },
}

impl FreeSlotState {
    /// Get the index of this slot in the global free list.
    fn free_list_index(&self) -> GlobalFreeListIndex {
        match self {
            &Self::NoAffinity { free_list_index }
            | &Self::Affinity {
                free_list_index, ..
            } => free_list_index,
        }
    }

    /// Update the index of this slot in the global free list.
    fn update_free_list_index(&mut self, index: GlobalFreeListIndex) {
        match self {
            &mut Self::NoAffinity {
                ref mut free_list_index,
            }
            | &mut Self::Affinity {
                ref mut free_list_index,
                ..
            } => {
                *free_list_index = index;
            }
        }
    }

    /// Get the index of this slot in its per-module free list.
    fn per_module_index(&self) -> PerModuleFreeListIndex {
        match self {
            &Self::Affinity {
                per_module_index, ..
            } => per_module_index,
            _ => panic!("per_module_index on slot with no affinity"),
        }
    }

    /// Update the index of this slot in its per-module free list.
    fn update_per_module_index(&mut self, index: PerModuleFreeListIndex) {
        match self {
            &mut Self::Affinity {
                ref mut per_module_index,
                ..
            } => {
                *per_module_index = index;
            }
            _ => panic!("per_module_index on slot with no affinity"),
        }
    }
}

/// Internal: remove a slot-index from the global free list.
fn remove_global_free_list_item(
    slot_state: &mut Vec<SlotState>,
    free_list: &mut Vec<SlotId>,
    index: SlotId,
) {
    let free_list_index = slot_state[index.index()].unwrap_free().free_list_index();
    assert_eq!(index, free_list.swap_remove(free_list_index.index()));
    if free_list_index.index() < free_list.len() {
        let replaced = free_list[free_list_index.index()];
        slot_state[replaced.index()]
            .unwrap_free_mut()
            .update_free_list_index(free_list_index);
    }
}

/// Internal: remove a slot-index from a per-module free list.
fn remove_module_free_list_item(
    slot_state: &mut Vec<SlotState>,
    per_module: &mut HashMap<CompiledModuleId, Vec<SlotId>>,
    id: CompiledModuleId,
    index: SlotId,
) {
    debug_assert!(
        per_module.contains_key(&id),
        "per_module list for given module should not be empty"
    );

    let per_module_list = per_module.get_mut(&id).unwrap();
    debug_assert!(!per_module_list.is_empty());

    let per_module_index = slot_state[index.index()].unwrap_free().per_module_index();
    assert_eq!(index, per_module_list.swap_remove(per_module_index.index()));
    if per_module_index.index() < per_module_list.len() {
        let replaced = per_module_list[per_module_index.index()];
        slot_state[replaced.index()]
            .unwrap_free_mut()
            .update_per_module_index(per_module_index);
    }
    if per_module_list.is_empty() {
        per_module.remove(&id);
    }
}

impl PoolingAllocationState {
    /// Create the default state for this strategy.
    pub(crate) fn new(strategy: PoolingAllocationStrategy, max_instances: usize) -> Self {
        let ids = (0..max_instances).map(|i| SlotId(i)).collect::<Vec<_>>();
        match strategy {
            PoolingAllocationStrategy::NextAvailable => PoolingAllocationState::NextAvailable(ids),
            PoolingAllocationStrategy::Random => PoolingAllocationState::Random(ids),
            PoolingAllocationStrategy::ReuseAffinity => PoolingAllocationState::ReuseAffinity {
                free_list: ids,
                per_module: HashMap::new(),
                slot_state: (0..max_instances)
                    .map(|i| {
                        SlotState::Free(FreeSlotState::NoAffinity {
                            free_list_index: GlobalFreeListIndex(i),
                        })
                    })
                    .collect(),
            },
        }
    }

    /// Are any slots left, or is this allocator empty?
    pub(crate) fn is_empty(&self) -> bool {
        match self {
            &PoolingAllocationState::NextAvailable(ref free_list)
            | &PoolingAllocationState::Random(ref free_list) => free_list.is_empty(),
            &PoolingAllocationState::ReuseAffinity { ref free_list, .. } => free_list.is_empty(),
        }
    }

    /// Allocate a new slot.
    pub(crate) fn alloc(&mut self, id: Option<CompiledModuleId>) -> SlotId {
        match self {
            &mut PoolingAllocationState::NextAvailable(ref mut free_list) => {
                debug_assert!(free_list.len() > 0);
                free_list.pop().unwrap()
            }
            &mut PoolingAllocationState::Random(ref mut free_list) => {
                debug_assert!(free_list.len() > 0);
                let id = rand::thread_rng().gen_range(0..free_list.len());
                free_list.swap_remove(id)
            }
            &mut PoolingAllocationState::ReuseAffinity {
                ref mut free_list,
                ref mut per_module,
                ref mut slot_state,
                ..
            } => {
                if let Some(this_module) = id.and_then(|id| per_module.get_mut(&id)) {
                    // There is a freelist of slots with affinity for
                    // the requested module-ID. Pick the last one; any
                    // will do, no need for randomness here.
                    assert!(!this_module.is_empty());
                    let slot_id = this_module.pop().expect("List should never be empty");
                    if this_module.is_empty() {
                        per_module.remove(&id.unwrap());
                    }
                    // Make sure to remove from the global
                    // freelist. We already removed from the
                    // per-module list above.
                    remove_global_free_list_item(slot_state, free_list, slot_id);
                    slot_state[slot_id.index()] = SlotState::Taken(id);
                    slot_id
                } else {
                    // Pick a random free slot ID. Note that we do
                    // this, rather than pick a victim module first,
                    // to maintain an unbiased stealing distribution:
                    // we want the likelihood of our taking a slot
                    // from some other module's freelist to be
                    // proportional to that module's freelist
                    // length. Or in other words, every *slot* should
                    // be equally likely to be stolen. The
                    // alternative, where we pick the victim module
                    // freelist first, means that either a module with
                    // an affinity freelist of one slot has the same
                    // chances of losing that slot as one with a
                    // hundred slots; or else we need a weighted
                    // random choice among modules, which is just as
                    // complex as this process.
                    //
                    // We don't bother picking an empty slot (no
                    // established affinity) before a random slot,
                    // because this is more complex, and in the steady
                    // state, all slots will see at least one
                    // instantiation very quickly, so there will never
                    // (past an initial phase) be a slot with no
                    // affinity.
                    let free_list_index = rand::thread_rng().gen_range(0..free_list.len());
                    let slot_id = free_list[free_list_index];
                    // Remove from both the global freelist and
                    // per-module freelist, if any.
                    remove_global_free_list_item(slot_state, free_list, slot_id);
                    if let &SlotState::Free(FreeSlotState::Affinity { module, .. }) =
                        &slot_state[slot_id.index()]
                    {
                        remove_module_free_list_item(slot_state, per_module, module, slot_id);
                    }
                    slot_state[slot_id.index()] = SlotState::Taken(id);

                    slot_id
                }
            }
        }
    }

    pub(crate) fn free(&mut self, index: SlotId) {
        match self {
            &mut PoolingAllocationState::NextAvailable(ref mut free_list)
            | &mut PoolingAllocationState::Random(ref mut free_list) => {
                free_list.push(index);
            }
            &mut PoolingAllocationState::ReuseAffinity {
                ref mut per_module,
                ref mut free_list,
                ref mut slot_state,
            } => {
                let module_id = slot_state[index.index()].unwrap_module_id();

                let free_list_index = GlobalFreeListIndex(free_list.len());
                free_list.push(index);
                if let Some(id) = module_id {
                    let per_module_list = per_module
                        .entry(id)
                        .or_insert_with(|| Vec::with_capacity(1));
                    let per_module_index = PerModuleFreeListIndex(per_module_list.len());
                    per_module_list.push(index);
                    slot_state[index.index()] = SlotState::Free(FreeSlotState::Affinity {
                        module: id,
                        free_list_index,
                        per_module_index,
                    });
                } else {
                    slot_state[index.index()] =
                        SlotState::Free(FreeSlotState::NoAffinity { free_list_index });
                }
            }
        }
    }

    /// For testing only, we want to be able to assert what is on the
    /// single freelist, for the policies that keep just one.
    #[cfg(test)]
    pub(crate) fn testing_freelist(&self) -> &[SlotId] {
        match self {
            &PoolingAllocationState::NextAvailable(ref free_list)
            | &PoolingAllocationState::Random(ref free_list) => &free_list[..],
            _ => panic!("Wrong kind of state"),
        }
    }

    /// For testing only, get the list of all modules with at least
    /// one slot with affinity for that module.
    #[cfg(test)]
    pub(crate) fn testing_module_affinity_list(&self) -> Vec<CompiledModuleId> {
        match self {
            &PoolingAllocationState::NextAvailable(..) | &PoolingAllocationState::Random(..) => {
                panic!("Wrong kind of state")
            }
            &PoolingAllocationState::ReuseAffinity { ref per_module, .. } => {
                let mut ret = vec![];
                for (module, list) in per_module {
                    assert!(!list.is_empty());
                    ret.push(*module);
                }
                ret
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{PoolingAllocationState, SlotId};
    use crate::CompiledModuleIdAllocator;
    use crate::PoolingAllocationStrategy;

    #[test]
    fn test_next_available_allocation_strategy() {
        let strat = PoolingAllocationStrategy::NextAvailable;
        let mut state = PoolingAllocationState::new(strat, 10);
        assert_eq!(state.alloc(None).index(), 9);
        let mut state = PoolingAllocationState::new(strat, 5);
        assert_eq!(state.alloc(None).index(), 4);
        let mut state = PoolingAllocationState::new(strat, 1);
        assert_eq!(state.alloc(None).index(), 0);
    }

    #[test]
    fn test_random_allocation_strategy() {
        let strat = PoolingAllocationStrategy::Random;
        let mut state = PoolingAllocationState::new(strat, 100);
        assert!(state.alloc(None).index() < 100);
        let mut state = PoolingAllocationState::new(strat, 1);
        assert_eq!(state.alloc(None).index(), 0);
    }

    #[test]
    fn test_affinity_allocation_strategy() {
        let strat = PoolingAllocationStrategy::ReuseAffinity;
        let id_alloc = CompiledModuleIdAllocator::new();
        let id1 = id_alloc.alloc();
        let id2 = id_alloc.alloc();
        let mut state = PoolingAllocationState::new(strat, 100);

        let index1 = state.alloc(Some(id1));
        assert!(index1.index() < 100);
        let index2 = state.alloc(Some(id2));
        assert!(index2.index() < 100);
        assert_ne!(index1, index2);

        state.free(index1);
        let index3 = state.alloc(Some(id1));
        assert_eq!(index3, index1);
        state.free(index3);

        state.free(index2);

        // Both id1 and id2 should have some slots with affinity.
        let affinity_modules = state.testing_module_affinity_list();
        assert_eq!(2, affinity_modules.len());
        assert!(affinity_modules.contains(&id1));
        assert!(affinity_modules.contains(&id2));

        // Now there is 1 free instance for id2 and 1 free instance
        // for id1, and 98 empty. Allocate 100 for id2. The first
        // should be equal to the one we know was previously used for
        // id2. The next 99 are arbitrary.

        let mut indices = vec![];
        for _ in 0..100 {
            assert!(!state.is_empty());
            indices.push(state.alloc(Some(id2)));
        }
        assert!(state.is_empty());
        assert_eq!(indices[0], index2);

        for i in indices {
            state.free(i);
        }

        // Now there should be no slots left with affinity for id1.
        let affinity_modules = state.testing_module_affinity_list();
        assert_eq!(1, affinity_modules.len());
        assert!(affinity_modules.contains(&id2));

        // Allocate an index we know previously had an instance but
        // now does not (list ran empty).
        let index = state.alloc(Some(id1));
        state.free(index);
    }

    #[test]
    fn test_affinity_allocation_strategy_random() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let strat = PoolingAllocationStrategy::ReuseAffinity;
        let id_alloc = CompiledModuleIdAllocator::new();
        let ids = std::iter::repeat_with(|| id_alloc.alloc())
            .take(10)
            .collect::<Vec<_>>();
        let mut state = PoolingAllocationState::new(strat, 1000);
        let mut allocated: Vec<SlotId> = vec![];
        let mut last_id = vec![None; 1000];

        let mut hits = 0;
        for _ in 0..100_000 {
            if !allocated.is_empty() && (state.is_empty() || rng.gen_bool(0.5)) {
                let i = rng.gen_range(0..allocated.len());
                let to_free_idx = allocated.swap_remove(i);
                state.free(to_free_idx);
            } else {
                assert!(!state.is_empty());
                let id = ids[rng.gen_range(0..ids.len())];
                let index = state.alloc(Some(id));
                if last_id[index.index()] == Some(id) {
                    hits += 1;
                }
                last_id[index.index()] = Some(id);
                allocated.push(index);
            }
        }

        // 10% reuse would be random chance (because we have 10 module
        // IDs). Check for at least double that to ensure some sort of
        // affinity is occurring.
        assert!(
            hits > 20000,
            "expected at least 20000 (20%) ID-reuses but got {}",
            hits
        );
    }
}
