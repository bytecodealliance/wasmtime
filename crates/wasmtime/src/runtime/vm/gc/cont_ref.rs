//! Side table for continuation references stored in the GC heap.
//!
//! Continuation references are sixteen bytes values, while every
//! reference field in the GC heap is four bytes. GC objects therefore
//! store an ID into this table rather than storing a `VMContObj`
//! directly.

use crate::{Result, bail_bug, hash_map::HashMap, vm::VMContObj};
use wasmtime_core::{
    alloc::PanicOnOom,
    slab::{Id, Slab},
};

/// Side table mapping the IDs stored in GC fields to continuation
/// objects.
///
/// Raw ID zero is reserved for a null continuation reference.
/// Non-null IDs are one greater than their underlying slab IDs.
#[derive(Default)]
pub struct ContRefTable {
    interned: HashMap<VMContObj, u32>,
    slab: Slab<VMContObj>,
}

impl ContRefTable {
    /// Intern a continuation object and return the ID to store in the
    /// GC heap.  Null continuation references use the reserved ID
    /// zero.
    ///
    /// # Safety
    ///
    /// A non-null continuation's `VMContRef` pointer must remain valid for the
    /// duration of this table's lifetime.
    pub unsafe fn intern(&mut self, contobj: Option<VMContObj>) -> u32 {
        let Some(contobj) = contobj else {
            return 0;
        };

        *self.interned.entry(contobj).or_insert_with(|| {
            // TODO(dhil): Handle allocation failure here rather than panicking.
            let id = self.slab.alloc(contobj).panic_on_oom().into_raw();
            id.checked_add(1).unwrap()
        })
    }

    /// Resolve an ID loaded from the GC heap.
    pub fn get(&self, raw: u32) -> Result<Option<VMContObj>> {
        if raw == 0 {
            return Ok(None);
        }

        let id = Id::from_raw(raw - 1);
        match self.slab.get(id).copied() {
            Some(contobj) => Ok(Some(contobj)),
            None => bail_bug!("bad continuation-reference table ID"),
        }
    }
}
