//! Virtual registers.
//!
//! A virtual register is a set of related SSA values whose live ranges don't interfere. If all the
//! values in a virtual register are assigned to the same location, fewer copies will result in the
//! output.
//!
//! A virtual register is typically built by merging together SSA values that are "phi-related" -
//! that is, one value is passed as an EBB argument to a branch and the other is the EBB parameter
//! value itself.
//!
//! If any values in a virtual register are spilled, they will use the same stack slot. This avoids
//! memory-to-memory copies when a spilled value is passed as an EBB argument.

use dbg::DisplayList;
use dominator_tree::DominatorTreePreorder;
use entity::EntityRef;
use entity::{EntityList, ListPool};
use entity::{Keys, PrimaryMap, SecondaryMap};
use ir::{Function, Value};
use packed_option::PackedOption;
use ref_slice::ref_slice;
use std::cmp::Ordering;
use std::fmt;
use std::vec::Vec;

/// A virtual register reference.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VirtReg(u32);
entity_impl!(VirtReg, "vreg");

type ValueList = EntityList<Value>;

/// Collection of virtual registers.
///
/// Each virtual register is a list of values. Also maintain a map from values to their unique
/// virtual register, if any.
pub struct VirtRegs {
    /// Memory pool for the value lists.
    pool: ListPool<Value>,

    /// The primary table of virtual registers.
    vregs: PrimaryMap<VirtReg, ValueList>,

    /// Allocated virtual register numbers that are no longer in use.
    unused_vregs: Vec<VirtReg>,

    /// Each value belongs to at most one virtual register.
    value_vregs: SecondaryMap<Value, PackedOption<VirtReg>>,

    /// Table used during the union-find phase while `vregs` is empty.
    union_find: SecondaryMap<Value, i32>,

    /// Values that have been activated in the `union_find` table, but not yet added to any virtual
    /// registers by the `finish_union_find()` function.
    pending_values: Vec<Value>,
}

impl VirtRegs {
    /// Create a new virtual register collection.
    pub fn new() -> Self {
        Self {
            pool: ListPool::new(),
            vregs: PrimaryMap::new(),
            unused_vregs: Vec::new(),
            value_vregs: SecondaryMap::new(),
            union_find: SecondaryMap::new(),
            pending_values: Vec::new(),
        }
    }

    /// Clear all virtual registers.
    pub fn clear(&mut self) {
        self.vregs.clear();
        self.unused_vregs.clear();
        self.value_vregs.clear();
        self.pool.clear();
        self.union_find.clear();
        self.pending_values.clear();
    }

    /// Get the virtual register containing `value`, if any.
    pub fn get(&self, value: Value) -> Option<VirtReg> {
        self.value_vregs[value].into()
    }

    /// Get the list of values in `vreg`.
    pub fn values(&self, vreg: VirtReg) -> &[Value] {
        self.vregs[vreg].as_slice(&self.pool)
    }

    /// Get an iterator over all virtual registers.
    pub fn all_virtregs(&self) -> Keys<VirtReg> {
        self.vregs.keys()
    }

    /// Get the congruence class of `value`.
    ///
    /// If `value` belongs to a virtual register, the congruence class is the values of the virtual
    /// register. Otherwise it is just the value itself.
    #[cfg_attr(feature = "cargo-clippy", allow(trivially_copy_pass_by_ref))]
    pub fn congruence_class<'a, 'b>(&'a self, value: &'b Value) -> &'b [Value]
    where
        'a: 'b,
    {
        self.get(*value)
            .map_or_else(|| ref_slice(value), |vr| self.values(vr))
    }

    /// Check if `a` and `b` belong to the same congruence class.
    pub fn same_class(&self, a: Value, b: Value) -> bool {
        match (self.get(a), self.get(b)) {
            (Some(va), Some(vb)) => va == vb,
            _ => a == b,
        }
    }

    /// Sort the values in `vreg` according to the dominator tree pre-order.
    ///
    /// Returns the slice of sorted values which `values(vreg)` will also return from now on.
    pub fn sort_values(
        &mut self,
        vreg: VirtReg,
        func: &Function,
        preorder: &DominatorTreePreorder,
    ) -> &[Value] {
        let s = self.vregs[vreg].as_mut_slice(&mut self.pool);
        s.sort_unstable_by(|&a, &b| preorder.pre_cmp_def(a, b, func));
        s
    }

    /// Insert a single value into a sorted virtual register.
    ///
    /// It is assumed that the virtual register containing `big` is already sorted by
    /// `sort_values()`, and that `single` does not already belong to a virtual register.
    ///
    /// If `big` is not part of a virtual register, one will be created.
    pub fn insert_single(
        &mut self,
        big: Value,
        single: Value,
        func: &Function,
        preorder: &DominatorTreePreorder,
    ) -> VirtReg {
        debug_assert_eq!(self.get(single), None, "Expected singleton {}", single);

        // Make sure `big` has a vreg.
        let vreg = self.get(big).unwrap_or_else(|| {
            let vr = self.alloc();
            self.vregs[vr].push(big, &mut self.pool);
            self.value_vregs[big] = vr.into();
            vr
        });

        // Determine the insertion position for `single`.
        let index = match self
            .values(vreg)
            .binary_search_by(|&v| preorder.pre_cmp_def(v, single, func))
        {
            Ok(_) => panic!("{} already in {}", single, vreg),
            Err(i) => i,
        };
        self.vregs[vreg].insert(index, single, &mut self.pool);
        self.value_vregs[single] = vreg.into();
        vreg
    }

    /// Remove a virtual register.
    ///
    /// The values in `vreg` become singletons, and the virtual register number may be reused in
    /// the future.
    pub fn remove(&mut self, vreg: VirtReg) {
        // Start by reassigning all the values.
        for &v in self.vregs[vreg].as_slice(&self.pool) {
            let old = self.value_vregs[v].take();
            debug_assert_eq!(old, Some(vreg));
        }

        self.vregs[vreg].clear(&mut self.pool);
        self.unused_vregs.push(vreg);
    }

    /// Allocate a new empty virtual register.
    fn alloc(&mut self) -> VirtReg {
        self.unused_vregs
            .pop()
            .unwrap_or_else(|| self.vregs.push(Default::default()))
    }

    /// Unify `values` into a single virtual register.
    ///
    /// The values in the slice can be singletons or they can belong to a virtual register already.
    /// If a value belongs to a virtual register, all of the values in that register must be
    /// present.
    ///
    /// The values are assumed to already be in topological order.
    pub fn unify(&mut self, values: &[Value]) -> VirtReg {
        // Start by clearing all virtual registers involved.
        let mut singletons = 0;
        let mut cleared = 0;
        for &val in values {
            match self.get(val) {
                None => singletons += 1,
                Some(vreg) => {
                    if !self.vregs[vreg].is_empty() {
                        cleared += self.vregs[vreg].len(&self.pool);
                        self.vregs[vreg].clear(&mut self.pool);
                        self.unused_vregs.push(vreg);
                    }
                }
            }
        }

        debug_assert_eq!(
            values.len(),
            singletons + cleared,
            "Can't unify partial virtual registers"
        );

        let vreg = self.alloc();
        self.vregs[vreg].extend(values.iter().cloned(), &mut self.pool);
        for &v in values {
            self.value_vregs[v] = vreg.into();
        }

        vreg
    }
}

impl fmt::Display for VirtRegs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for vreg in self.all_virtregs() {
            write!(f, "\n{} = {}", vreg, DisplayList(self.values(vreg)))?;
        }
        Ok(())
    }
}

/// Expanded version of a union-find table entry.
enum UFEntry {
    /// This value is a a set leader. The embedded number is the set's rank.
    Rank(u32),

    /// This value belongs to the same set as the linked value.
    Link(Value),
}

/// The `union_find` table contains `i32` entries that are interpreted as follows:
///
/// x = 0: The value belongs to its own singleton set.
/// x > 0: The value is the leader of a set with rank x.
/// x < 0: The value belongs to the same set as the value numbered !x.
///
/// The rank of a set is an upper bound on the number of links that must be followed from a member
/// of the set to the set leader.
///
/// A singleton set is the same as a set with rank 0. It contains only the leader value.
impl UFEntry {
    /// Decode a table entry.
    fn decode(x: i32) -> Self {
        if x < 0 {
            UFEntry::Link(Value::new((!x) as usize))
        } else {
            UFEntry::Rank(x as u32)
        }
    }

    /// Encode a link entry.
    fn encode_link(v: Value) -> i32 {
        !(v.index() as i32)
    }
}

/// Union-find algorithm for building virtual registers.
///
/// Before values are added to virtual registers, it is possible to use a union-find algorithm to
/// construct virtual registers efficiently. This support implemented here is used as follows:
///
/// 1. Repeatedly call the `union(a, b)` method to request that `a` and `b` are placed in the same
///    virtual register.
/// 2. When done, call `finish_union_find()` to construct the virtual register sets based on the
///    `union()` calls.
///
/// The values that were passed to `union(a, b)` must not belong to any existing virtual registers
/// by the time `finish_union_find()` is called.
///
/// For more information on the algorithm implemented here, see Chapter 21 "Data Structures for
/// Disjoint Sets" of Cormen, Leiserson, Rivest, Stein, "Introduction to algorithms", 3rd Ed.
///
/// The [Wikipedia entry on disjoint-set data
/// structures](https://en.wikipedia.org/wiki/Disjoint-set_data_structure) is also good.
impl VirtRegs {
    /// Find the leader value and rank of the set containing `v`.
    /// Compress the path if needed.
    fn find(&mut self, val: Value) -> (Value, u32) {
        match UFEntry::decode(self.union_find[val]) {
            UFEntry::Rank(rank) => (val, rank),
            UFEntry::Link(parent) => {
                // TODO: This recursion would be more efficient as an iteration that pushes
                // elements onto a SmallVector.
                let found = self.find(parent);
                // Compress the path if needed.
                if found.0 != parent {
                    self.union_find[val] = UFEntry::encode_link(found.0);
                }
                found
            }
        }
    }

    /// Union the two sets containing `a` and `b`.
    ///
    /// This ensures that `a` and `b` will belong to the same virtual register after calling
    /// `finish_union_find()`.
    pub fn union(&mut self, a: Value, b: Value) {
        let (leader_a, rank_a) = self.find(a);
        let (leader_b, rank_b) = self.find(b);

        if leader_a == leader_b {
            return;
        }

        // The first time we see a value, its rank will be 0. Add it to the list of pending values.
        if rank_a == 0 {
            debug_assert_eq!(a, leader_a);
            self.pending_values.push(a);
        }
        if rank_b == 0 {
            debug_assert_eq!(b, leader_b);
            self.pending_values.push(b);
        }

        // Merge into the set with the greater rank. This preserves the invariant that the rank is
        // an upper bound on the number of links to the leader.
        match rank_a.cmp(&rank_b) {
            Ordering::Less => {
                self.union_find[leader_a] = UFEntry::encode_link(leader_b);
            }
            Ordering::Greater => {
                self.union_find[leader_b] = UFEntry::encode_link(leader_a);
            }
            Ordering::Equal => {
                // When the two sets have the same rank, we arbitrarily pick the a-set to preserve.
                // We need to increase the rank by one since the elements in the b-set are now one
                // link further away from the leader.
                self.union_find[leader_a] += 1;
                self.union_find[leader_b] = UFEntry::encode_link(leader_a);
            }
        }
    }

    /// Compute virtual registers based on previous calls to `union(a, b)`.
    ///
    /// This terminates the union-find algorithm, so the next time `union()` is called, it is for a
    /// new independent batch of values.
    ///
    /// The values in each virtual register will be ordered according to when they were first
    /// passed to `union()`, but backwards. It is expected that `sort_values()` will be used to
    /// create a more sensible value order.
    ///
    /// The new virtual registers will be appended to `new_vregs`, if present.
    pub fn finish_union_find(&mut self, mut new_vregs: Option<&mut Vec<VirtReg>>) {
        debug_assert_eq!(
            self.pending_values.iter().find(|&&v| self.get(v).is_some()),
            None,
            "Values participating in union-find must not belong to existing virtual registers"
        );

        while let Some(val) = self.pending_values.pop() {
            let (leader, _) = self.find(val);

            // Get the vreg for `leader`, or create it.
            let vreg = self.get(leader).unwrap_or_else(|| {
                // Allocate a vreg for `leader`, but leave it empty.
                let vr = self.alloc();
                if let Some(ref mut vec) = new_vregs {
                    vec.push(vr);
                }
                self.value_vregs[leader] = vr.into();
                vr
            });

            // Push values in `pending_values` order, including when `v == leader`.
            self.vregs[vreg].push(val, &mut self.pool);
            self.value_vregs[val] = vreg.into();

            // Clear the entry in the union-find table. The `find(val)` call may still look at this
            // entry in a future iteration, but that it ok. It will return a rank 0 leader that has
            // already been assigned to the correct virtual register.
            self.union_find[val] = 0;
        }

        // We do *not* call `union_find.clear()` table here because re-initializing the table for
        // sparse use takes time linear in the number of values in the function. Instead we reset
        // the entries that are known to be non-zero in the loop above.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use entity::EntityRef;
    use ir::Value;

    #[test]
    fn empty_union_find() {
        let mut vregs = VirtRegs::new();
        vregs.finish_union_find(None);
        assert_eq!(vregs.all_virtregs().count(), 0);
    }

    #[test]
    fn union_self() {
        let mut vregs = VirtRegs::new();
        let v1 = Value::new(1);
        vregs.union(v1, v1);
        vregs.finish_union_find(None);
        assert_eq!(vregs.get(v1), None);
        assert_eq!(vregs.all_virtregs().count(), 0);
    }

    #[test]
    fn union_pair() {
        let mut vregs = VirtRegs::new();
        let v1 = Value::new(1);
        let v2 = Value::new(2);
        vregs.union(v1, v2);
        vregs.finish_union_find(None);
        assert_eq!(vregs.congruence_class(&v1), &[v2, v1]);
        assert_eq!(vregs.congruence_class(&v2), &[v2, v1]);
        assert_eq!(vregs.all_virtregs().count(), 1);
    }

    #[test]
    fn union_pair_backwards() {
        let mut vregs = VirtRegs::new();
        let v1 = Value::new(1);
        let v2 = Value::new(2);
        vregs.union(v2, v1);
        vregs.finish_union_find(None);
        assert_eq!(vregs.congruence_class(&v1), &[v1, v2]);
        assert_eq!(vregs.congruence_class(&v2), &[v1, v2]);
        assert_eq!(vregs.all_virtregs().count(), 1);
    }

    #[test]
    fn union_tree() {
        let mut vregs = VirtRegs::new();
        let v1 = Value::new(1);
        let v2 = Value::new(2);
        let v3 = Value::new(3);
        let v4 = Value::new(4);

        vregs.union(v2, v4);
        vregs.union(v3, v1);
        // Leaders: v2, v3
        vregs.union(v4, v1);
        vregs.finish_union_find(None);
        assert_eq!(vregs.congruence_class(&v1), &[v1, v3, v4, v2]);
        assert_eq!(vregs.congruence_class(&v2), &[v1, v3, v4, v2]);
        assert_eq!(vregs.congruence_class(&v3), &[v1, v3, v4, v2]);
        assert_eq!(vregs.congruence_class(&v4), &[v1, v3, v4, v2]);
        assert_eq!(vregs.all_virtregs().count(), 1);
    }

    #[test]
    fn union_two() {
        let mut vregs = VirtRegs::new();
        let v1 = Value::new(1);
        let v2 = Value::new(2);
        let v3 = Value::new(3);
        let v4 = Value::new(4);

        vregs.union(v2, v4);
        vregs.union(v3, v1);
        // Leaders: v2, v3
        vregs.finish_union_find(None);
        assert_eq!(vregs.congruence_class(&v1), &[v1, v3]);
        assert_eq!(vregs.congruence_class(&v2), &[v4, v2]);
        assert_eq!(vregs.congruence_class(&v3), &[v1, v3]);
        assert_eq!(vregs.congruence_class(&v4), &[v4, v2]);
        assert_eq!(vregs.all_virtregs().count(), 2);
    }

    #[test]
    fn union_uneven() {
        let mut vregs = VirtRegs::new();
        let v1 = Value::new(1);
        let v2 = Value::new(2);
        let v3 = Value::new(3);
        let v4 = Value::new(4);

        vregs.union(v2, v4); // Rank 0-0
        vregs.union(v3, v2); // Rank 0-1
        vregs.union(v2, v1); // Rank 1-0
        vregs.finish_union_find(None);
        assert_eq!(vregs.congruence_class(&v1), &[v1, v3, v4, v2]);
        assert_eq!(vregs.congruence_class(&v2), &[v1, v3, v4, v2]);
        assert_eq!(vregs.congruence_class(&v3), &[v1, v3, v4, v2]);
        assert_eq!(vregs.congruence_class(&v4), &[v1, v3, v4, v2]);
        assert_eq!(vregs.all_virtregs().count(), 1);
    }
}
