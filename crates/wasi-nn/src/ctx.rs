//! Implements the host state for the `wasi-nn` API: [WasiNnCtx].

use crate::backend::{
    self, Backend, BackendError, BackendExecutionContext, BackendGraph, BackendKind,
};
use crate::wit::types::GraphEncoding;
use std::collections::HashMap;
use std::hash::Hash;
use thiserror::Error;
use wiggle::GuestError;

type GraphId = u32;
type GraphExecutionContextId = u32;

/// Capture the state necessary for calling into the backend ML libraries.
pub struct WasiNnCtx {
    pub(crate) backends: HashMap<BackendKind, Box<dyn Backend>>,
    pub(crate) graphs: Table<GraphId, Box<dyn BackendGraph>>,
    pub(crate) executions: Table<GraphExecutionContextId, Box<dyn BackendExecutionContext>>,
}

impl WasiNnCtx {
    /// Make a new context from the default state.
    pub fn new() -> WasiNnResult<Self> {
        let mut backends = HashMap::new();
        for (kind, backend) in backend::list() {
            backends.insert(kind, backend);
        }
        Ok(Self {
            backends,
            graphs: Table::default(),
            executions: Table::default(),
        })
    }
}

/// Possible errors while interacting with [WasiNnCtx].
#[derive(Debug, Error)]
pub enum WasiNnError {
    #[error("backend error")]
    BackendError(#[from] BackendError),
    #[error("guest error")]
    GuestError(#[from] GuestError),
    #[error("usage error")]
    UsageError(#[from] UsageError),
}

#[derive(Debug, Error)]
pub enum UsageError {
    #[error("Invalid context; has the load function been called?")]
    InvalidContext,
    #[error("Only OpenVINO's IR is currently supported, passed encoding: {0:?}")]
    InvalidEncoding(GraphEncoding),
    #[error("OpenVINO expects only two buffers (i.e. [ir, weights]), passed: {0}")]
    InvalidNumberOfBuilders(u32),
    #[error("Invalid graph handle; has it been loaded?")]
    InvalidGraphHandle,
    #[error("Invalid execution context handle; has it been initialized?")]
    InvalidExecutionContextHandle,
    #[error("Not enough memory to copy tensor data of size: {0}")]
    NotEnoughMemory(u32),
}

pub(crate) type WasiNnResult<T> = std::result::Result<T, WasiNnError>;

/// Record handle entries in a table.
pub struct Table<K, V> {
    entries: HashMap<K, V>,
    next_key: u32,
}

impl<K, V> Default for Table<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            next_key: 0,
        }
    }
}

impl<K, V> Table<K, V>
where
    K: Eq + Hash + From<u32> + Copy,
{
    pub fn insert(&mut self, value: V) -> K {
        let key = self.use_next_key();
        self.entries.insert(key, value);
        key
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.entries.get_mut(&key)
    }

    fn use_next_key(&mut self) -> K {
        let current = self.next_key;
        self.next_key += 1;
        K::from(current)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn instantiate() {
        WasiNnCtx::new().unwrap();
    }
}
