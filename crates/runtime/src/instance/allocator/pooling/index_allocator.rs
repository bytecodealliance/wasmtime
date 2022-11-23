//! Index/slot allocator policies for the pooling allocator.

use super::PoolingAllocationStrategy;
use crate::CompiledModuleId;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::sync::Mutex;

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

#[derive(Debug)]
pub struct IndexAllocator(Mutex<Inner>);

#[derive(Debug)]
struct Inner {
    strategy: PoolingAllocationStrategy,
    rng: SmallRng,

    /// Free-list of all slots.
    ///
    /// We use this to pick a victim when we don't have an appropriate slot with
    /// the preferred affinity.
    free_list: Vec<SlotId>,

    /// Affine slot management which tracks which slots are free and were last
    /// used with the specified `CompiledModuleId`.
    ///
    /// Invariant: any module ID in this hashmap must have a non-empty list of
    /// free slots (otherwise we remove it). We remove a module's freelist when
    /// we have no more slots with affinity for that module.
    per_module: HashMap<CompiledModuleId, Vec<SlotId>>,

    /// The state of any given slot.
    ///
    /// Records indices in the above list (empty) or two lists (with affinity),
    /// and these indices are kept up-to-date to allow fast removal.
    slot_state: Vec<SlotState>,
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

impl IndexAllocator {
    /// Create the default state for this strategy.
    pub fn new(strategy: PoolingAllocationStrategy, max_instances: usize) -> Self {
        let ids = (0..max_instances).map(|i| SlotId(i)).collect::<Vec<_>>();
        // Use a deterministic seed during fuzzing to improve reproducibility of
        // test cases, but otherwise outside of fuzzing use a random seed to
        // shake things up.
        let seed = if cfg!(fuzzing) {
            [0; 32]
        } else {
            rand::thread_rng().gen()
        };
        let rng = SmallRng::from_seed(seed);
        IndexAllocator(Mutex::new(Inner {
            rng,
            strategy,
            free_list: ids,
            per_module: HashMap::new(),
            slot_state: (0..max_instances)
                .map(|i| {
                    SlotState::Free(FreeSlotState::NoAffinity {
                        free_list_index: GlobalFreeListIndex(i),
                    })
                })
                .collect(),
        }))
    }

    /// Allocate a new index from this allocator optionally using `id` as an
    /// affinity request if the allocation strategy supports it.
    ///
    /// Returns `None` if no more slots are available.
    pub fn alloc(&self, id: Option<CompiledModuleId>) -> Option<SlotId> {
        self._alloc(id, false)
    }

    /// Attempts to allocate a guaranteed-affine slot to the module `id`
    /// specified.
    ///
    /// Returns `None` if there are no slots affine to `id`. The allocation of
    /// this slot will not record the affinity to `id`, instead simply listing
    /// it as taken. This is intended to be used for clearing out all affine
    /// slots to a module.
    pub fn alloc_affine_and_clear_affinity(&self, id: CompiledModuleId) -> Option<SlotId> {
        self._alloc(Some(id), true)
    }

    fn _alloc(
        &self,
        id: Option<CompiledModuleId>,
        force_affine_and_clear_affinity: bool,
    ) -> Option<SlotId> {
        let mut inner = self.0.lock().unwrap();
        let inner = &mut *inner;

        let slot_id = match inner.strategy {
            PoolingAllocationStrategy::NextAvailable => *inner.free_list.last()?,
            PoolingAllocationStrategy::Random => inner.alloc_random()?,
            PoolingAllocationStrategy::ReuseAffinity => {
                // First attempt an affine allocation where the slot returned
                // was previously used by `id`, but if that fails pick a random
                // free slot ID.
                //
                // Note that we do this to maintain an unbiased stealing
                // distribution: we want the likelihood of our taking a slot
                // from some other module's freelist to be proportional to that
                // module's freelist length. Or in other words, every *slot*
                // should be equally likely to be stolen. The alternative,
                // where we pick the victim module freelist first, means that
                // either a module with an affinity freelist of one slot has
                // the same chances of losing that slot as one with a hundred
                // slots; or else we need a weighted random choice among
                // modules, which is just as complex as this process.
                //
                // We don't bother picking an empty slot (no established
                // affinity) before a random slot, because this is more
                // complex, and in the steady state, all slots will see at
                // least one instantiation very quickly, so there will never
                // (past an initial phase) be a slot with no affinity.
                inner.alloc_affine(id).or_else(|| {
                    if force_affine_and_clear_affinity {
                        None
                    } else {
                        inner.alloc_random()
                    }
                })?
            }
        };

        // Update internal metadata bout the allocation of `slot_id` to `id`,
        // meaning that it's removed from the per-module freelist if it was
        // previously affine and additionally it's removed from the global
        // freelist.
        inner.remove_global_free_list_item(slot_id);
        if let &SlotState::Free(FreeSlotState::Affinity { module, .. }) =
            &inner.slot_state[slot_id.index()]
        {
            inner.remove_module_free_list_item(module, slot_id);
        }
        inner.slot_state[slot_id.index()] = SlotState::Taken(if force_affine_and_clear_affinity {
            None
        } else {
            id
        });

        Some(slot_id)
    }

    pub(crate) fn free(&self, index: SlotId) {
        let mut inner = self.0.lock().unwrap();
        let free_list_index = GlobalFreeListIndex(inner.free_list.len());
        inner.free_list.push(index);
        let module_id = inner.slot_state[index.index()].unwrap_module_id();
        inner.slot_state[index.index()] = if let Some(id) = module_id {
            let per_module_list = inner
                .per_module
                .entry(id)
                .or_insert_with(|| Vec::with_capacity(1));
            let per_module_index = PerModuleFreeListIndex(per_module_list.len());
            per_module_list.push(index);
            SlotState::Free(FreeSlotState::Affinity {
                module: id,
                free_list_index,
                per_module_index,
            })
        } else {
            SlotState::Free(FreeSlotState::NoAffinity { free_list_index })
        };
    }

    /// For testing only, we want to be able to assert what is on the
    /// single freelist, for the policies that keep just one.
    #[cfg(test)]
    pub(crate) fn testing_freelist(&self) -> Vec<SlotId> {
        let inner = self.0.lock().unwrap();
        inner.free_list.clone()
    }

    /// For testing only, get the list of all modules with at least
    /// one slot with affinity for that module.
    #[cfg(test)]
    pub(crate) fn testing_module_affinity_list(&self) -> Vec<CompiledModuleId> {
        let inner = self.0.lock().unwrap();
        let mut ret = vec![];
        for (module, list) in inner.per_module.iter() {
            assert!(!list.is_empty());
            ret.push(*module);
        }
        ret
    }
}

impl Inner {
    /// Attempts to allocate a slot already affine to `id`, returning `None` if
    /// `id` is `None` or if there are no affine slots.
    fn alloc_affine(&self, id: Option<CompiledModuleId>) -> Option<SlotId> {
        let free = self.per_module.get(&id?)?;
        free.last().copied()
    }

    fn alloc_random(&mut self) -> Option<SlotId> {
        if self.free_list.len() == 0 {
            return None;
        }
        let i = self.rng.gen_range(0..self.free_list.len());
        Some(self.free_list[i])
    }

    /// Remove a slot-index from the global free list.
    fn remove_global_free_list_item(&mut self, index: SlotId) {
        let free_list_index = self.slot_state[index.index()]
            .unwrap_free()
            .free_list_index();
        assert_eq!(index, self.free_list.swap_remove(free_list_index.index()));
        if free_list_index.index() < self.free_list.len() {
            let replaced = self.free_list[free_list_index.index()];
            self.slot_state[replaced.index()]
                .unwrap_free_mut()
                .update_free_list_index(free_list_index);
        }
    }

    /// Remove a slot-index from a per-module free list.
    fn remove_module_free_list_item(&mut self, id: CompiledModuleId, index: SlotId) {
        debug_assert!(
            self.per_module.contains_key(&id),
            "per_module list for given module should not be empty"
        );

        let per_module_list = self.per_module.get_mut(&id).unwrap();
        debug_assert!(!per_module_list.is_empty());

        let per_module_index = self.slot_state[index.index()]
            .unwrap_free()
            .per_module_index();
        assert_eq!(index, per_module_list.swap_remove(per_module_index.index()));
        if per_module_index.index() < per_module_list.len() {
            let replaced = per_module_list[per_module_index.index()];
            self.slot_state[replaced.index()]
                .unwrap_free_mut()
                .update_per_module_index(per_module_index);
        }
        if per_module_list.is_empty() {
            self.per_module.remove(&id);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{IndexAllocator, SlotId};
    use crate::CompiledModuleIdAllocator;
    use crate::PoolingAllocationStrategy;

    #[test]
    fn test_next_available_allocation_strategy() {
        let strat = PoolingAllocationStrategy::NextAvailable;

        for size in 0..20 {
            let state = IndexAllocator::new(strat, size);
            for i in 0..size {
                assert_eq!(state.alloc(None).unwrap().index(), size - i - 1);
            }
            assert!(state.alloc(None).is_none());
        }
    }

    #[test]
    fn test_random_allocation_strategy() {
        let strat = PoolingAllocationStrategy::Random;

        for size in 0..20 {
            let state = IndexAllocator::new(strat, size);
            for _ in 0..size {
                assert!(state.alloc(None).unwrap().index() < size);
            }
            assert!(state.alloc(None).is_none());
        }
    }

    #[test]
    fn test_affinity_allocation_strategy() {
        let strat = PoolingAllocationStrategy::ReuseAffinity;
        let id_alloc = CompiledModuleIdAllocator::new();
        let id1 = id_alloc.alloc();
        let id2 = id_alloc.alloc();
        let state = IndexAllocator::new(strat, 100);

        let index1 = state.alloc(Some(id1)).unwrap();
        assert!(index1.index() < 100);
        let index2 = state.alloc(Some(id2)).unwrap();
        assert!(index2.index() < 100);
        assert_ne!(index1, index2);

        state.free(index1);
        let index3 = state.alloc(Some(id1)).unwrap();
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
            indices.push(state.alloc(Some(id2)).unwrap());
        }
        assert!(state.alloc(None).is_none());
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
        let index = state.alloc(Some(id1)).unwrap();
        state.free(index);
    }

    #[test]
    fn clear_affine() {
        let strat = PoolingAllocationStrategy::ReuseAffinity;
        let id_alloc = CompiledModuleIdAllocator::new();
        let id = id_alloc.alloc();
        let state = IndexAllocator::new(strat, 100);

        let index1 = state.alloc(Some(id)).unwrap();
        let index2 = state.alloc(Some(id)).unwrap();
        state.free(index2);
        state.free(index1);
        assert!(state.alloc_affine_and_clear_affinity(id).is_some());
        assert!(state.alloc_affine_and_clear_affinity(id).is_some());
        assert_eq!(state.alloc_affine_and_clear_affinity(id), None);
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
        let state = IndexAllocator::new(strat, 1000);
        let mut allocated: Vec<SlotId> = vec![];
        let mut last_id = vec![None; 1000];

        let mut hits = 0;
        for _ in 0..100_000 {
            loop {
                if !allocated.is_empty() && rng.gen_bool(0.5) {
                    let i = rng.gen_range(0..allocated.len());
                    let to_free_idx = allocated.swap_remove(i);
                    state.free(to_free_idx);
                } else {
                    let id = ids[rng.gen_range(0..ids.len())];
                    let index = match state.alloc(Some(id)) {
                        Some(id) => id,
                        None => continue,
                    };
                    if last_id[index.index()] == Some(id) {
                        hits += 1;
                    }
                    last_id[index.index()] = Some(id);
                    allocated.push(index);
                }
                break;
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
