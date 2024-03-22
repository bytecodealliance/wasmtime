//! Define the registry API.
//!
//! A [`GraphRegistry`] is place to store backend graphs so they can be loaded
//! by name. This API does not mandate how a graph is loaded or how it must be
//! stored--it could be stored remotely and rematerialized when needed, e.g. A
//! naive in-memory implementation, [`InMemoryRegistry`] is provided for use
//! with the Wasmtime CLI.

mod in_memory;

use crate::backend::BackendError;
use crate::Graph;
pub use in_memory::InMemoryRegistry;
use wiggle::async_trait;

#[async_trait]
pub trait GraphRegistry: Send + Sync {
    async fn get_mut(&mut self, name: &str) -> Result<Option<&mut Graph>, BackendError>;
}
