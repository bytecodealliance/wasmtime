//! Fallible, OOM-handling collections.

mod entity_set;
mod hash_set;
mod primary_map;
mod secondary_map;

pub use entity_set::EntitySet;
pub use hash_set::HashSet;
pub use primary_map::PrimaryMap;
pub use secondary_map::SecondaryMap;
pub use wasmtime_core::alloc::{TryNew, Vec, try_new};
