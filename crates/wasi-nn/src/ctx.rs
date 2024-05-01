//! Implements the host state for the `wasi-nn` API: [WasiNnCtx].

use crate::backend::{self, BackendError};
use crate::wit::types::GraphEncoding;
use crate::{Backend, ExecutionContext, Graph, InMemoryRegistry, Registry};
use anyhow::anyhow;
use std::{collections::HashMap, hash::Hash, path::Path};
use thiserror::Error;
use wiggle::GuestError;

type GraphId = u32;
type GraphExecutionContextId = u32;
type BackendName = String;
type GraphDirectory = String;

/// Construct an in-memory registry from the available backends and a list of
/// `(<backend name>, <graph directory>)`. This assumes graphs can be loaded
/// from a local directory, which is a safe assumption currently for the current
/// model types.
pub fn preload(
    preload_graphs: &[(BackendName, GraphDirectory)],
) -> anyhow::Result<(impl IntoIterator<Item = Backend>, Registry)> {
    let mut backends = backend::list();
    let mut registry = InMemoryRegistry::new();
    for (kind, path) in preload_graphs {
        let kind_ = kind.parse()?;
        let backend = backends
            .iter_mut()
            .find(|b| b.encoding() == kind_)
            .ok_or(anyhow!("unsupported backend: {}", kind))?
            .as_dir_loadable()
            .ok_or(anyhow!("{} does not support directory loading", kind))?;
        registry.load(backend, Path::new(path))?;
    }
    Ok((backends, Registry::from(registry)))
}

/// Capture the state necessary for calling into the backend ML libraries.
pub struct WasiNnCtx {
    pub(crate) backends: HashMap<GraphEncoding, Backend>,
    pub(crate) registry: Registry,
    pub(crate) graphs: Table<GraphId, Graph>,
    pub(crate) executions: Table<GraphExecutionContextId, ExecutionContext>,
}

impl WasiNnCtx {
    /// Make a new context from the default state.
    pub fn new(backends: impl IntoIterator<Item = Backend>, registry: Registry) -> Self {
        let backends = backends.into_iter().map(|b| (b.encoding(), b)).collect();
        Self {
            backends,
            registry,
            graphs: Table::default(),
            executions: Table::default(),
        }
    }
}

pub trait WasiNnView {
    fn ctx(&mut self) -> &mut WasiNnCtx;
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
    #[error("Only OpenVINO's IR is currently supported, passed encoding: {0:?}")]
    InvalidEncoding(GraphEncoding),
    #[error("Invalid graph handle; has it been loaded?")]
    InvalidGraphHandle,
    #[error("Invalid execution context handle; has it been initialized?")]
    InvalidExecutionContextHandle,
    #[error("No graph found with name: {0}")]
    NotFound(String),
}

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

    pub fn get(&self, key: K) -> Option<&V> {
        self.entries.get(&key)
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
    use crate::registry::GraphRegistry;

    #[test]
    fn example() {
        struct FakeRegistry;
        impl GraphRegistry for FakeRegistry {
            fn get_mut(&mut self, _: &str) -> Option<&mut Graph> {
                None
            }
        }

        let _ctx = WasiNnCtx::new([], Registry::from(FakeRegistry));
    }
}
