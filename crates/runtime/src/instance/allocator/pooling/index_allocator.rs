//! Index/slot allocator policies for the pooling allocator.

use super::PoolingAllocationStrategy;
use crate::CompiledModuleId;
use rand::Rng;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub(crate) enum PoolingAllocationState {
    NextAvailable(Vec<usize>),
    Random(Vec<usize>),
    /// Reuse-affinity policy state.
    ///
    /// The data structures here deserve a little explanation:
    ///
    /// - free_list: this is a vec of slot indices that are free, no
    ///   matter their affinities.
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
        // Free-list of all slots. We use this to pick a victim when
        // we don't have an appropriate slot with the preferred
        // affinity.
        free_list: Vec<usize>,
        // Invariant: any module ID in this hashmap must have a
        // non-empty list of free slots (otherwise we remove it).
        per_module: HashMap<CompiledModuleId, Vec<usize>>,
        // The state of any given slot. Records indices in the above
        // list (empty) or two lists (with affinity), and these
        // indices are kept up-to-date to allow fast removal.
        slot_state: Vec<SlotState>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum SlotState {
    Taken,
    Empty {
        /// Index in the global free list. Invariant:
        /// free_list[slot_state[i].free_list_index] == i.
        free_list_index: usize,
    },
    Affinity {
        module: CompiledModuleId,
        /// Index in the global free list. Invariant:
        /// free_list[slot_state[i].free_list_index] == i.
        free_list_index: usize,
        /// Index in a per-module free list. Invariant:
        /// per_module[slot_state[i].module][slot_state[i].per_module_index]
        /// == i.
        per_module_index: usize,
    },
}

impl SlotState {
    /// Get the index of this slot in the global free list.
    fn free_list_index(&self) -> usize {
        match self {
            &Self::Empty { free_list_index }
            | &Self::Affinity {
                free_list_index, ..
            } => free_list_index,
            _ => unreachable!(),
        }
    }

    /// Update the index of this slot in the global free list.
    fn update_free_list_index(&mut self, index: usize) {
        match self {
            &mut Self::Empty {
                ref mut free_list_index,
            }
            | &mut Self::Affinity {
                ref mut free_list_index,
                ..
            } => {
                *free_list_index = index;
            }
            _ => panic!("Taken in free list"),
        }
    }

    /// Get the index of this slot in its per-module free list.
    fn per_module_index(&self) -> usize {
        match self {
            &Self::Affinity {
                per_module_index, ..
            } => per_module_index,
            _ => unreachable!(),
        }
    }

    /// Update the index of this slot in its per-module free list.
    fn update_per_module_index(&mut self, index: usize) {
        match self {
            &mut Self::Affinity {
                ref mut per_module_index,
                ..
            } => {
                *per_module_index = index;
            }
            _ => panic!("Taken in per-module free list"),
        }
    }
}

impl PoolingAllocationState {
    /// Create the default state for this strategy.
    pub(crate) fn new(strategy: PoolingAllocationStrategy, max_instances: usize) -> Self {
        let ids = (0..max_instances).collect::<Vec<_>>();
        match strategy {
            PoolingAllocationStrategy::NextAvailable => PoolingAllocationState::NextAvailable(ids),
            PoolingAllocationStrategy::Random => PoolingAllocationState::Random(ids),
            PoolingAllocationStrategy::ReuseAffinity => PoolingAllocationState::ReuseAffinity {
                free_list: ids,
                per_module: HashMap::new(),
                slot_state: (0..max_instances)
                    .map(|i| SlotState::Empty { free_list_index: i })
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

    /// Internal: remove a slot-index from the global free list.
    fn remove_free_list_item(
        slot_state: &mut Vec<SlotState>,
        free_list: &mut Vec<usize>,
        index: usize,
    ) {
        let free_list_index = slot_state[index].free_list_index();
        assert_eq!(index, free_list.swap_remove(free_list_index));
        if free_list_index < free_list.len() {
            let replaced = free_list[free_list_index];
            slot_state[replaced].update_free_list_index(free_list_index);
        }
    }

    /// Internal: remove a slot-index from a per-module free list.
    fn remove_module_free_list_item(
        slot_state: &mut Vec<SlotState>,
        per_module: &mut HashMap<CompiledModuleId, Vec<usize>>,
        id: CompiledModuleId,
        index: usize,
    ) {
        let per_module_list = per_module.get_mut(&id).unwrap();
        let per_module_index = slot_state[index].per_module_index();
        assert_eq!(index, per_module_list.swap_remove(per_module_index));
        if per_module_index < per_module_list.len() {
            let replaced = per_module_list[per_module_index];
            slot_state[replaced].update_per_module_index(per_module_index);
        }
        if per_module_list.is_empty() {
            per_module.remove(&id);
        }
    }

    /// Allocate a new slot.
    pub(crate) fn alloc(&mut self, id: Option<CompiledModuleId>) -> usize {
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
                    let new_id = this_module.pop().expect("List should never be empty");
                    if this_module.is_empty() {
                        per_module.remove(&id.unwrap());
                    }
                    // Make sure to remove from the global
                    // freelist. We already removed from the
                    // per-module list above.
                    Self::remove_free_list_item(slot_state, free_list, new_id);
                    slot_state[new_id] = SlotState::Taken;
                    new_id
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
                    let new_id = free_list[free_list_index];
                    // Remove from both the global freelist and
                    // per-module freelist, if any.
                    Self::remove_free_list_item(slot_state, free_list, new_id);
                    if let &SlotState::Affinity { module, .. } = &slot_state[new_id] {
                        Self::remove_module_free_list_item(slot_state, per_module, module, new_id);
                    }
                    slot_state[new_id] = SlotState::Taken;

                    new_id
                }
            }
        }
    }

    pub(crate) fn free(&mut self, index: usize, id: Option<CompiledModuleId>) {
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
                let free_list_index = free_list.len();
                free_list.push(index);
                if let Some(id) = id {
                    let per_module_list = per_module.entry(id).or_insert_with(|| vec![]);
                    let per_module_index = per_module_list.len();
                    per_module_list.push(index);
                    slot_state[index] = SlotState::Affinity {
                        module: id,
                        free_list_index,
                        per_module_index,
                    };
                } else {
                    slot_state[index] = SlotState::Empty { free_list_index };
                }
            }
        }
    }

    /// For testing only, we want to be able to assert what is on the
    /// single freelist, for the policies that keep just one.
    #[cfg(test)]
    pub(crate) fn testing_freelist(&self) -> &[usize] {
        match self {
            &PoolingAllocationState::NextAvailable(ref free_list)
            | &PoolingAllocationState::Random(ref free_list) => &free_list[..],
            _ => panic!("Wrong kind of state"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::PoolingAllocationState;
    use crate::CompiledModuleIdAllocator;
    use crate::PoolingAllocationStrategy;

    #[test]
    fn test_next_available_allocation_strategy() {
        let strat = PoolingAllocationStrategy::NextAvailable;
        let mut state = PoolingAllocationState::new(strat, 10);
        assert_eq!(state.alloc(None), 9);
        let mut state = PoolingAllocationState::new(strat, 5);
        assert_eq!(state.alloc(None), 4);
        let mut state = PoolingAllocationState::new(strat, 1);
        assert_eq!(state.alloc(None), 0);
    }

    #[test]
    fn test_random_allocation_strategy() {
        let strat = PoolingAllocationStrategy::Random;
        let mut state = PoolingAllocationState::new(strat, 100);
        assert!(state.alloc(None) < 100);
        let mut state = PoolingAllocationState::new(strat, 1);
        assert_eq!(state.alloc(None), 0);
    }

    #[test]
    fn test_affinity_allocation_strategy() {
        let strat = PoolingAllocationStrategy::ReuseAffinity;
        let id_alloc = CompiledModuleIdAllocator::new();
        let id1 = id_alloc.alloc();
        let id2 = id_alloc.alloc();
        let mut state = PoolingAllocationState::new(strat, 100);

        let index1 = state.alloc(Some(id1));
        assert!(index1 < 100);
        let index2 = state.alloc(Some(id2));
        assert!(index2 < 100);
        assert_ne!(index1, index2);

        state.free(index1, Some(id1));
        let index3 = state.alloc(Some(id1));
        assert_eq!(index3, index1);
        state.free(index3, Some(id1));

        state.free(index2, Some(id2));

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
            state.free(i, Some(id2));
        }

        // Allocate an index we know previously had an instance but
        // now does not (list ran empty).
        let index = state.alloc(Some(id1));
        state.free(index, Some(id1));
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
        let mut allocated = vec![];
        let mut last_id = vec![None; 1000];

        let mut hits = 0;
        for _ in 0..100_000 {
            if !allocated.is_empty() && (state.is_empty() || rng.gen_bool(0.5)) {
                let i = rng.gen_range(0..allocated.len());
                let (to_free_idx, to_free_id) = allocated.swap_remove(i);
                let to_free_id = if rng.gen_bool(0.1) {
                    None
                } else {
                    Some(to_free_id)
                };
                state.free(to_free_idx, to_free_id);
            } else {
                assert!(!state.is_empty());
                let id = ids[rng.gen_range(0..ids.len())];
                let index = state.alloc(Some(id));
                if last_id[index] == Some(id) {
                    hits += 1;
                }
                last_id[index] = Some(id);
                allocated.push((index, id));
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
