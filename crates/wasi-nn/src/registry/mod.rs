//! Define the registry API.
//!
//! A [`GraphRegistry`] is place to store backend graphs so they can be loaded
//! by name. This API does not mandate how a graph is loaded or how it must be
//! stored--it could be stored remotely and rematerialized when needed, e.g. A
//! naive in-memory implementation, [`InMemoryRegistry`] is provided for use
//! with the Wasmtime CLI.

mod in_memory;

use crate::Graph;
pub use in_memory::InMemoryRegistry;

pub trait GraphRegistry: Send + Sync {
    fn get(&self, name: &str) -> Option<&Graph>;
    fn get_mut(&mut self, name: &str) -> Option<&mut Graph>;
}
