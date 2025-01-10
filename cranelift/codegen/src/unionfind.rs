//! Simple union-find data structure.

use crate::trace;
use cranelift_entity::{EntityRef, SecondaryMap, packed_option::ReservedValue};
use std::hash::Hash;
use std::mem::swap;

/// A union-find data structure. The data structure can allocate
/// `Idx`s, indicating eclasses, and can merge eclasses together.
///
/// Running `union(a, b)` will change the canonical `Idx` of `a` or `b`.
/// Usually, this is chosen based on what will minimize path lengths,
/// but it is also possible to _pin_ an eclass, such that its canonical `Idx`
/// won't change unless it gets unioned with another pinned eclass.
///
/// In the context of the egraph pass, merging two pinned eclasses
/// is very unlikely to happen – we do not know a single concrete test case
/// where it does. The only situation where it might happen looks as follows:
///
/// 1. We encounter terms `A` and `B`, and the optimizer does not find any
///    reason to union them together.
/// 2. We encounter a term `C`, and we rewrite `C -> A`, and separately, `C -> B`.
///
/// Unless `C` somehow includes some crucial hint without which it is hard to
/// notice that `A = B`, there's probably a rewrite rule that we should add.
///
/// Worst case, if we do merge two pinned eclasses, some nodes will essentially
/// disappear from the GVN map, which only affects the quality of the generated
/// code.
#[derive(Clone, Debug, PartialEq)]
pub struct UnionFind<Idx: EntityRef> {
    parent: SecondaryMap<Idx, Val<Idx>>,
    /// The `rank` table is used to perform the union operations optimally,
    /// without creating unnecessarily long paths. Pins are represented by
    /// eclasses with a rank of `u8::MAX`.
    ///
    /// `rank[x]` is the upper bound on the height of the subtree rooted at `x`.
    /// The subtree is guaranteed to have at least `2**rank[x]` elements,
    /// unless `rank` has been artificially inflated by pinning.
    rank: SecondaryMap<Idx, u8>,

    pub(crate) pinned_union_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct Val<Idx>(Idx);

impl<Idx: EntityRef + ReservedValue> Default for Val<Idx> {
    fn default() -> Self {
        Self(Idx::reserved_value())
    }
}

impl<Idx: EntityRef + Hash + std::fmt::Display + Ord + ReservedValue> UnionFind<Idx> {
    /// Create a new `UnionFind` with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        UnionFind {
            parent: SecondaryMap::with_capacity(cap),
            rank: SecondaryMap::with_capacity(cap),
            pinned_union_count: 0,
        }
    }

    /// Add an `Idx` to the `UnionFind`, with its own equivalence class
    /// initially. All `Idx`s must be added before being queried or
    /// unioned.
    pub fn add(&mut self, id: Idx) {
        debug_assert!(id != Idx::reserved_value());
        self.parent[id] = Val(id);
    }

    /// Find the canonical `Idx` of a given `Idx`.
    pub fn find(&self, mut node: Idx) -> Idx {
        while node != self.parent[node].0 {
            node = self.parent[node].0;
        }
        node
    }

    /// Find the canonical `Idx` of a given `Idx`, updating the data
    /// structure in the process so that future queries for this `Idx`
    /// (and others in its chain up to the root of the equivalence
    /// class) will be faster.
    pub fn find_and_update(&mut self, mut node: Idx) -> Idx {
        // "Path halving" mutating find (Tarjan and Van Leeuwen).
        debug_assert!(node != Idx::reserved_value());
        while node != self.parent[node].0 {
            let next = self.parent[self.parent[node].0].0;
            debug_assert!(next != Idx::reserved_value());
            self.parent[node] = Val(next);
            node = next;
        }
        debug_assert!(node != Idx::reserved_value());
        node
    }

    /// Request a stable identifier for `node`.
    ///
    /// After an `union` operation, the canonical representative of one
    /// of the eclasses being merged together necessarily changes. If a pinned
    /// eclass is merged with a non-pinned eclass, it'll be the other eclass
    /// whose representative will change.
    ///
    /// If two pinned eclasses are unioned, one of the pins gets broken,
    /// which is reported in the statistics for the pass. No concrete test case
    /// which triggers this is known.
    pub fn pin_index(&mut self, mut node: Idx) -> Idx {
        node = self.find_and_update(node);
        self.rank[node] = u8::MAX;
        node
    }

    /// Merge the equivalence classes of the two `Idx`s.
    pub fn union(&mut self, a: Idx, b: Idx) {
        let mut a = self.find_and_update(a);
        let mut b = self.find_and_update(b);

        if a == b {
            return;
        }

        if self.rank[a] < self.rank[b] {
            swap(&mut a, &mut b);
        } else if self.rank[a] == self.rank[b] {
            self.rank[a] = self.rank[a].checked_add(1).unwrap_or_else(
                #[cold]
                || {
                    // Both `a` and `b` are pinned.
                    //
                    // This should only occur if we rewrite X -> Y and X -> Z,
                    // yet neither Y -> Z nor Z -> Y can be established without
                    // the "hint" provided by X. This probably means we're
                    // missing an optimization rule.
                    self.pinned_union_count += 1;
                    u8::MAX
                },
            );
        }

        self.parent[b] = Val(a);
        trace!("union: {}, {}", a, b);
    }
}
