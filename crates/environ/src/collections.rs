//! Fallible, OOM-handling collections.

mod entity_set;
mod primary_map;
mod secondary_map;

pub use entity_set::EntitySet;
pub use primary_map::PrimaryMap;
pub use secondary_map::SecondaryMap;
pub use wasmtime_core::alloc::{TryNew, Vec, try_new};
