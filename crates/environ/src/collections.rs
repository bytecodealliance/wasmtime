//! Fallible, OOM-handling collections.

mod entity_set;

pub use entity_set::EntitySet;
pub use wasmtime_core::alloc::{TryNew, try_new};
