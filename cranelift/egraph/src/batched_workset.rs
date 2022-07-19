//! Data structure: a set of pending items to process, with batched
//! processing. Holds a second set and does swaps in a way such that
//! allocations are avoided altogether in the steady state (in a sort
//! of double-buffering way).

use fxhash::FxHashSet;
use std::hash::Hash;

/// A workset of items to process, allowing processing in batches.
pub struct BatchedWorkset<T: Hash + Eq> {
    set: FxHashSet<T>,
    buf: FxHashSet<T>,
}

impl<T: Hash + Eq> std::default::Default for BatchedWorkset<T> {
    fn default() -> Self {
        Self {
            set: FxHashSet::default(),
            buf: FxHashSet::default(),
        }
    }
}

impl<T: Hash + Eq> BatchedWorkset<T> {
    /// Add an item to the workset. Duplicates are not added
    /// again. Returns `true` if actually added (i.e. no duplicate
    /// present already in set).
    pub fn add(&mut self, t: T) -> bool {
        self.set.insert(t)
    }

    /// Take a batch of worklist items, clearing this workset.
    pub fn take_batch(&mut self) -> Batch<T> {
        std::mem::swap(&mut self.set, &mut self.buf);
        debug_assert!(self.set.is_empty());
        Batch {
            set: std::mem::take(&mut self.buf),
        }
    }

    /// Reuse the memory from a batch.
    pub fn reuse(&mut self, mut batch: Batch<T>) {
        batch.set.clear();
        self.buf = batch.set;
    }

    /// Is the workset empty?
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}

/// A batch of items to process.
pub struct Batch<T: Hash + Eq> {
    set: FxHashSet<T>,
}

impl<T: Hash + Eq> Batch<T> {
    /// Returns an iterator that drains the batch, returning each item
    /// once.
    pub fn batch<'a>(&'a mut self) -> impl Iterator<Item = T> + 'a {
        self.set.drain()
    }
}
