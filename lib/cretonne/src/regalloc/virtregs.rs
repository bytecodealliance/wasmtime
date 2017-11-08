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

use entity::{EntityList, ListPool};
use entity::{PrimaryMap, EntityMap, Keys};
use ir::Value;
use packed_option::PackedOption;
use ref_slice::ref_slice;

/// A virtual register reference.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
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
    ///
    /// The list of values ion a virtual register is kept sorted according to the dominator tree's
    /// RPO of the value defs.
    vregs: PrimaryMap<VirtReg, ValueList>,

    /// Each value belongs to at most one virtual register.
    value_vregs: EntityMap<Value, PackedOption<VirtReg>>,
}

#[allow(dead_code)]
impl VirtRegs {
    /// Create a new virtual register collection.
    pub fn new() -> Self {
        Self {
            pool: ListPool::new(),
            vregs: PrimaryMap::new(),
            value_vregs: EntityMap::new(),
        }
    }

    /// Clear all virtual registers.
    pub fn clear(&mut self) {
        self.vregs.clear();
        self.value_vregs.clear();
        self.pool.clear();
    }

    /// Get the virtual register containing `value`, if any.
    pub fn get(&self, value: Value) -> Option<VirtReg> {
        self.value_vregs[value].into()
    }

    /// Get the list of values in `vreg`. The values are ordered according to `DomTree::rpo_cmp` of
    /// their definition points.
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
    pub fn congruence_class<'a, 'b>(&'a self, value: &'b Value) -> &'b [Value]
    where
        'a: 'b,
    {
        self.get(*value).map(|vr| self.values(vr)).unwrap_or(
            ref_slice(value),
        )
    }

    /// Check if `a` and `b` belong to the same congruence class.
    pub fn same_class(&self, a: Value, b: Value) -> bool {
        match (self.get(a), self.get(b)) {
            (Some(va), Some(vb)) => va == vb,
            _ => a == b,
        }
    }

    /// Unify `values` into a single virtual register.
    ///
    /// The values in the slice can be singletons or they can belong to a virtual register already.
    /// If a value belongs to a virtual register, all of the values in that register must be
    /// present.
    ///
    /// The values are assumed to already be in RPO order.
    pub fn unify(&mut self, values: &[Value]) -> VirtReg {
        // Start by clearing all virtual registers involved.
        // Pick a virtual register to reuse (the smallest number) or allocate a new one.
        let mut singletons = 0;
        let mut cleared = 0;
        let vreg = values
            .iter()
            .filter_map(|&v| {
                let vr = self.get(v);
                match vr {
                    None => singletons += 1,
                    Some(vr) => {
                        if !self.vregs[vr].is_empty() {
                            cleared += self.vregs[vr].len(&self.pool);
                            self.vregs[vr].clear(&mut self.pool);
                        }
                    }
                }
                vr
            })
            .min()
            .unwrap_or_else(|| self.vregs.push(Default::default()));

        assert_eq!(
            values.len(),
            singletons + cleared,
            "Can't unify partial virtual registers"
        );

        self.vregs[vreg].extend(values.iter().cloned(), &mut self.pool);
        for &v in values {
            self.value_vregs[v] = vreg.into();
        }

        vreg
    }
}
