//! Implements the base structure (i.e. [WasiNnCtx]) that will provide the implementation of the
//! wasi-nn API.
use crate::r#impl::UsageError;
use crate::witx::types::{Graph, GraphExecutionContext};
use openvino::{InferenceError, SetupError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use thiserror::Error;
use wiggle::GuestError;

/// Possible errors while interacting with [WasiNnCtx].
#[derive(Debug, Error)]
pub enum WasiNnError {
    #[error("guest error")]
    GuestError(#[from] GuestError),
    #[error("openvino inference error")]
    OpenvinoInferenceError(#[from] InferenceError),
    #[error("openvino setup error")]
    OpenvinoSetupError(#[from] SetupError),
    #[error("usage error")]
    UsageError(#[from] UsageError),
}

pub(crate) type WasiNnResult<T> = std::result::Result<T, WasiNnError>;

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

pub struct ExecutionContext {
    pub(crate) graph: Graph,
    pub(crate) request: openvino::InferRequest,
}

impl ExecutionContext {
    pub(crate) fn new(graph: Graph, request: openvino::InferRequest) -> Self {
        Self { graph, request }
    }
}

/// Capture the state necessary for calling into `openvino`.
pub struct Ctx {
    pub(crate) core: Option<openvino::Core>,
    pub(crate) graphs: Table<Graph, (openvino::CNNNetwork, openvino::ExecutableNetwork)>,
    pub(crate) executions: Table<GraphExecutionContext, ExecutionContext>,
}

impl Ctx {
    /// Make a new `WasiNnCtx` with the default settings.
    pub fn new() -> WasiNnResult<Self> {
        Ok(Self {
            core: Option::default(),
            graphs: Table::default(),
            executions: Table::default(),
        })
    }
}

/// This structure provides the Rust-side context necessary for implementing the wasi-nn API. At the
/// moment, it is specialized for a single inference implementation (i.e. OpenVINO) but conceivably
/// this could support more than one backing implementation.
pub struct WasiNnCtx {
    pub(crate) ctx: RefCell<Ctx>,
}

impl WasiNnCtx {
    /// Make a new `WasiNnCtx` with the default settings.
    pub fn new() -> WasiNnResult<Self> {
        Ok(Self {
            ctx: RefCell::new(Ctx::new()?),
        })
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
