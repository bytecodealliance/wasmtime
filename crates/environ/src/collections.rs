//! Fallible, OOM-handling collections.

pub mod btree_map;
mod entity_set;
mod hash_map;
mod hash_set;
mod index_map;
mod primary_map;
mod secondary_map;

pub use btree_map::TryBTreeMap;
pub use entity_set::TryEntitySet;
pub use hash_map::TryHashMap;
pub use hash_set::TryHashSet;
pub use index_map::TryIndexMap;
pub use primary_map::TryPrimaryMap;
pub use secondary_map::TrySecondaryMap;
pub use wasmtime_core::{
    alloc::{
        TryClone, TryCollect, TryCow, TryExtend, TryFromIterator, TryNew, TryString, TryToOwned,
        TryVec, try_new,
    },
    try_vec,
};

/// Collections which abort on OOM.
//
// FIXME(#12069) this is just here for Wasmtime at this time. Ideally
// collections would only be fallible in this module and would handle OOM. We're
// in a bit of a transition period though.
pub mod oom_abort {
    #[cfg(not(feature = "std"))]
    pub use hashbrown::{hash_map, hash_set};
    #[cfg(feature = "std")]
    pub use std::collections::{hash_map, hash_set};

    pub use self::hash_map::HashMap;
    pub use self::hash_set::HashSet;
}
