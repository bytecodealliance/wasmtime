//! Fallible, OOM-handling collections.

mod entity_set;
mod hash_set;
mod primary_map;
mod secondary_map;

pub use entity_set::EntitySet;
pub use hash_set::HashSet;
pub use primary_map::PrimaryMap;
pub use secondary_map::SecondaryMap;
pub use wasmtime_core::alloc::{TryClone, TryNew, Vec, try_new};

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
