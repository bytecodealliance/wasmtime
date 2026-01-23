//! Fallible, OOM-handling collections.

mod entity_set;
mod primary_map;

pub use entity_set::EntitySet;
pub use primary_map::PrimaryMap;
pub use wasmtime_core::alloc::{TryNew, try_new};
