//! A union-find (disjoint-set) data structure.
//!
//! Beyond the usual `union`/`find`, this supports:
//!
//! * `set_members`: iterate one set in time proportional to its size, via a
//!   circular linked list, which also supports merging these sets in constant
//!   time in `union`.
//!
//! * `set_min`: get a set's least member in amortized constant time.
//!
//! * `elems`: iterate only the elements that have ever been passed to
//!   `union`. All others are implicit singletons that this structure never
//!   allocates for or visits.

use crate::prelude::*;
use core::fmt;
use cranelift_entity::{
    EntityRef, EntitySet, SecondaryMap,
    packed_option::{PackedOption, ReservedValue},
};

/// A union-find over entity references of type `K`.
///
/// Every element of `K`'s index space starts out in its own singleton set;
/// only elements passed to `union` are materialized.
#[derive(Clone)]
pub struct UnionFind<K: EntityRef + ReservedValue> {
    /// Parent pointers, `None` at a set's root.
    parent: SecondaryMap<K, PackedOption<K>>,

    /// Circular linked lists threading together each set's members. `None`
    /// for an untouched element, whose set is just itself.
    next: SecondaryMap<K, PackedOption<K>>,

    /// Set sizes, for union-by-size. Only meaningful at roots.
    size: SecondaryMap<K, u32>,

    /// Least member of each set, by entity index. Only meaningful for roots.
    /// `None` for untouched elements, which are their own least members.
    min: SecondaryMap<K, PackedOption<K>>,

    /// The elements that have been passed to `union`.
    touched: EntitySet<K>,

    /// The same elements as `touched`, in first-touched order, so that the
    /// runtime of `elems` is proportional to the number of touched elements,
    /// not the size of the entity space.
    order: Vec<K>,

    /// Scratch space for path compression in `find`.
    stack: Vec<K>,
}

impl<K: EntityRef + ReservedValue> Default for UnionFind<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: EntityRef + ReservedValue + fmt::Debug> fmt::Debug for UnionFind<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_set();
        for root in self.roots() {
            f.entry(&self.set_members(root).collect::<Vec<_>>());
        }
        f.finish()
    }
}

impl<K: EntityRef + ReservedValue> UnionFind<K> {
    /// Create a new union-find where every element is a singleton.
    pub fn new() -> Self {
        Self {
            parent: SecondaryMap::new(),
            next: SecondaryMap::new(),
            size: SecondaryMap::new(),
            min: SecondaryMap::new(),
            touched: EntitySet::new(),
            order: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// The number of elements that have participated in a `union`.
    ///
    /// This is neither the number of sets nor the size of the key space:
    /// untouched singletons are not counted.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Has `k` participated in a `union`? If not, it is an implicit singleton.
    pub fn contains(&self, k: K) -> bool {
        self.touched.contains(k)
    }

    /// Iterate over the elements that have participated in a `union`, in
    /// first-touched order.
    pub fn elems(&self) -> impl ExactSizeIterator<Item = K> + '_ {
        self.order.iter().copied()
    }

    /// Iterate over the root of each set of touched elements.
    ///
    /// Combine with `set_members` to visit every member of every such set.
    /// Untouched elements, being implicit singletons, do not appear.
    pub fn roots(&self) -> impl Iterator<Item = K> + '_ {
        self.elems()
            .filter(|k| self.find_without_path_compression(*k) == *k)
    }

    /// Find the representative of `k`'s set without mutating `self`.
    ///
    /// Prefer `find` when you have mutable access; it also compresses the path
    /// to the root, making subsequent lookups fast.
    pub fn find_without_path_compression(&self, k: K) -> K {
        let mut k = k;
        while let Some(p) = self.parent[k].expand() {
            if p == k {
                break;
            }
            k = p;
        }
        k
    }

    /// Find the representative of `k`'s set, compressing the path to the root
    /// along the way.
    pub fn find(&mut self, k: K) -> K {
        let mut root = k;
        while let Some(p) = self.parent[root].expand() {
            if p == root {
                break;
            }
            self.stack.push(root);
            root = p;
        }
        for e in self.stack.drain(..) {
            self.parent[e] = root.into();
        }
        root
    }

    /// Are `a` and `b` in the same set?
    #[cfg(test)]
    pub fn same_set(&self, a: K, b: K) -> bool {
        self.find_without_path_compression(a) == self.find_without_path_compression(b)
    }

    /// Merge the sets containing `a` and `b`, returning the merged set's root.
    ///
    /// Which element ends up as the root is an implementation detail of the
    /// union-by-size balancing and may change as sets merge.
    pub fn union(&mut self, a: K, b: K) -> K {
        self.touch(a);
        self.touch(b);

        let a = self.find(a);
        let b = self.find(b);
        if a == b {
            return a;
        }

        // Keep trees shallow by hanging the smaller set off of the larger
        // set's root.
        let (root, child) = if self.size[a] >= self.size[b] {
            (a, b)
        } else {
            (b, a)
        };
        self.parent[child] = root.into();
        self.size[root] += self.size[child];

        let child_min = self.min[child].unwrap();
        let root_min = self.min[root].unwrap();
        if child_min.index() < root_min.index() {
            self.min[root] = child_min.into();
        }

        // Splice the sets together by swapping their successors.
        let root_next = self.next[root].unwrap();
        let child_next = self.next[child].unwrap();
        self.next[root] = child_next.into();
        self.next[child] = root_next.into();

        root
    }

    /// Iterate over every member of `k`'s set, including `k` itself.
    ///
    /// Takes time proportional to the size of the set, not the size of
    /// `self.len()` or the whole entity space.
    ///
    /// Yields `k` first; the rest of the order is unspecified.
    pub fn set_members(&self, k: K) -> impl Iterator<Item = K> + '_ {
        let mut cursor = Some(k);
        core::iter::from_fn(move || {
            let this = cursor?;
            cursor = match self.next[this].expand() {
                // Walking all the way around to where we started means we're
                // done and there's no next member.
                Some(n) if n == k => None,
                // Otherwise, we have a next member.
                Some(n) => Some(n),
                // `None` means `this` is an untouched singleton, has no next.
                None => None,
            };
            Some(this)
        })
    }

    /// The least member of `k`'s set, by entity index.
    ///
    /// This is independent of the size of the set, unlike walking
    /// `set_members` and taking the minimum, so it is affordable for every
    /// element of a large set.
    pub fn set_min(&self, k: K) -> K {
        let root = self.find_without_path_compression(k);
        // An untouched element is a singleton, so it is its own least member.
        self.min[root].expand().unwrap_or(root)
    }

    /// Materialize `k` as a singleton set, if it is not already present.
    fn touch(&mut self, k: K) {
        if self.touched.insert(k) {
            self.parent[k] = None.into();
            self.next[k] = k.into();
            self.size[k] = 1;
            self.min[k] = k.into();
            self.order.push(k);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property_check;
    use cranelift_entity::entity_impl;
    use mutatis::{Mutate, check::CheckResult, mutators as m};
    use std::collections::BTreeSet;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
    struct E(u32);
    entity_impl!(E);

    fn e(i: u8) -> E {
        E::from_u32(u32::from(i))
    }

    /// The size of the key space that the property tests union over.
    const N: u8 = 16;

    /// An obviously-correct partition to check the real implementation against.
    ///
    /// Every element carries a group label; unioning relabels one whole group.
    struct Reference {
        labels: [u8; N as usize],
        touched: BTreeSet<u8>,
    }

    impl Reference {
        fn new() -> Self {
            let mut labels = [0; N as usize];
            for (i, l) in labels.iter_mut().enumerate() {
                *l = u8::try_from(i).unwrap();
            }
            Reference {
                labels,
                touched: BTreeSet::new(),
            }
        }

        fn union(&mut self, a: u8, b: u8) {
            self.touched.insert(a);
            self.touched.insert(b);
            let a = self.labels[usize::from(a)];
            let b = self.labels[usize::from(b)];
            for l in self.labels.iter_mut() {
                if *l == b {
                    *l = a;
                }
            }
        }

        fn same_set(&self, a: u8, b: u8) -> bool {
            self.labels[usize::from(a)] == self.labels[usize::from(b)]
        }

        fn set_min(&self, a: u8) -> E {
            *self.set_members(a).first().unwrap()
        }

        fn set_members(&self, a: u8) -> BTreeSet<E> {
            (0..N)
                .filter(|b| self.same_set(a, *b))
                .map(e)
                .collect::<BTreeSet<_>>()
        }
    }

    #[test]
    fn untouched_elements_are_singletons() {
        let uf = UnionFind::<E>::new();
        assert_eq!(uf.len(), 0);
        for i in 0..N {
            assert!(!uf.contains(e(i)));
            assert_eq!(uf.find_without_path_compression(e(i)), e(i));
            assert_eq!(uf.set_members(e(i)).collect::<Vec<_>>(), vec![e(i)]);
            assert_eq!(uf.set_min(e(i)), e(i));
            for j in 0..N {
                assert_eq!(uf.same_set(e(i), e(j)), i == j);
            }
        }
        assert_eq!(uf.roots().count(), 0);
    }

    #[test]
    fn basic_unions() {
        let mut uf = UnionFind::new();

        // Self-union is a no-op, other than touching the element.
        assert_eq!(uf.union(e(0), e(0)), e(0));
        assert!(uf.contains(e(0)));
        assert_eq!(uf.set_members(e(0)).collect::<Vec<_>>(), vec![e(0)]);

        let ab = uf.union(e(1), e(2));
        assert!(uf.same_set(e(1), e(2)));
        assert_eq!(uf.find(e(1)), ab);
        assert_eq!(uf.find(e(2)), ab);
        assert_eq!(
            uf.set_members(e(1)).collect::<BTreeSet<_>>(),
            BTreeSet::from([e(1), e(2)])
        );
        assert_eq!(
            uf.set_members(e(2)).collect::<BTreeSet<_>>(),
            BTreeSet::from([e(1), e(2)])
        );

        assert_eq!(uf.set_min(e(1)), e(1));
        assert_eq!(uf.set_min(e(2)), e(1));

        uf.union(e(3), e(2));
        assert!(uf.same_set(e(1), e(3)));
        assert_eq!(
            uf.set_members(e(3)).collect::<BTreeSet<_>>(),
            BTreeSet::from([e(1), e(2), e(3)])
        );

        // Element 0's set is unaffected by all of that.
        assert!(!uf.same_set(e(0), e(1)));
        assert_eq!(uf.set_members(e(0)).collect::<Vec<_>>(), vec![e(0)]);
        assert_eq!(uf.set_min(e(0)), e(0));

        // Merging two multi-element sets splices both member lists.
        uf.union(e(4), e(5));
        assert_eq!(uf.set_min(e(5)), e(4));
        uf.union(e(5), e(1));
        assert_eq!(
            uf.set_members(e(4)).collect::<BTreeSet<_>>(),
            BTreeSet::from([e(1), e(2), e(3), e(4), e(5)])
        );
        // Merging in a set whose least member is larger must not raise the
        // result's least member.
        for i in 1..=5 {
            assert_eq!(uf.set_min(e(i)), e(1));
        }
    }

    /// `set_members` is proportional to the set and not the key space, so it
    /// must work for elements far beyond any that were ever unioned.
    #[test]
    fn sparse_keys() {
        const HIGH: E = E(if cfg!(miri) { 1_000 } else { 1_000_000 });
        const LOW: E = E(HIGH.0 / 1_000);

        let mut uf = UnionFind::new();
        uf.union(LOW, HIGH);
        assert!(uf.same_set(LOW, HIGH));
        assert_eq!(
            uf.set_members(LOW).collect::<BTreeSet<_>>(),
            BTreeSet::from([LOW, HIGH])
        );
        assert_eq!(uf.set_min(HIGH), LOW);
        assert!(!uf.contains(E(7)));
        assert_eq!(uf.set_members(E(7)).collect::<Vec<_>>(), vec![E(7)]);
        assert_eq!(uf.set_min(E(7)), E(7));
    }

    /// Drive random `union` sequences and check every query against the
    /// reference partition.
    #[test]
    fn matches_reference_partition() -> CheckResult<Vec<(u8, u8)>> {
        let mutator = m::default::<Vec<(u8, u8)>>().map(|_ctx, unions| {
            unions.truncate(2 * usize::from(N));
            for (a, b) in unions {
                *a %= N;
                *b %= N;
            }
            Ok(())
        });

        property_check().run_with(
            mutator,
            [Vec::new()],
            |unions: &Vec<(u8, u8)>| -> Result<(), String> {
                let mut uf = UnionFind::new();
                let mut reference = Reference::new();

                for (a, b) in unions {
                    let root = uf.union(e(*a), e(*b));
                    reference.union(*a, *b);

                    // `union` returns the root of both arguments.
                    assert_eq!(uf.find(e(*a)), root);
                    assert_eq!(uf.find(e(*b)), root);
                    assert_eq!(uf.find(root), root, "find is idempotent at a root");

                    for i in 0..N {
                        // Path compression must not change any answer.
                        let no_compress = uf.find_without_path_compression(e(i));
                        assert_eq!(uf.find(e(i)), no_compress);

                        assert_eq!(
                            uf.contains(e(i)),
                            reference.touched.contains(&i),
                            "contains() agrees about which elements were touched"
                        );
                        assert_eq!(
                            uf.set_members(e(i)).collect::<BTreeSet<_>>(),
                            reference.set_members(i),
                            "set_members() agrees with the reference partition"
                        );
                        assert_eq!(
                            uf.set_min(e(i)),
                            reference.set_min(i),
                            "set_min() agrees with the reference partition"
                        );

                        for j in 0..N {
                            assert_eq!(
                                uf.same_set(e(i), e(j)),
                                reference.same_set(i, j),
                                "same_set({i}, {j}) agrees with the reference partition"
                            );
                        }
                    }
                }

                // `elems` yields exactly the touched elements, and no
                // duplicates.
                let elems = uf.elems().collect::<BTreeSet<_>>();
                assert_eq!(elems.len(), uf.len());
                assert_eq!(
                    elems,
                    reference
                        .touched
                        .iter()
                        .copied()
                        .map(e)
                        .collect::<BTreeSet<_>>()
                );

                // The sets rooted at `roots` partition `elems`.
                let mut covered = BTreeSet::new();
                for root in uf.roots() {
                    for member in uf.set_members(root) {
                        assert!(covered.insert(member), "sets are disjoint");
                    }
                }
                assert_eq!(covered, elems, "roots cover every touched element");

                Ok(())
            },
        )
    }
}
