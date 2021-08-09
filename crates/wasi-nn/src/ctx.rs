//! Implements the base structure (i.e. [WasiNnCtx]) that will provide the
//! implementation of the wasi-nn API.
use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::openvino::OpenvinoBackend;
use crate::r#impl::UsageError;
use crate::witx::types::{Graph, GraphEncoding, GraphExecutionContext};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use thiserror::Error;
use wiggle::GuestError;

/// Capture the state necessary for calling into the backend ML libraries.
pub struct Ctx {
    pub(crate) backends: HashMap<u8, Box<dyn Backend>>,
    pub(crate) graphs: Table<Graph, Box<dyn BackendGraph>>,
    pub(crate) executions: Table<GraphExecutionContext, Box<dyn BackendExecutionContext>>,
}

impl Ctx {
    /// Make a new context from the default state.
    pub fn new() -> WasiNnResult<Self> {
        let mut backends = HashMap::new();
        backends.insert(
            // This is necessary because Wiggle's variant types do not derive
            // `Hash` and `Eq`.
            GraphEncoding::Openvino.into(),
            Box::new(OpenvinoBackend::default()) as Box<dyn Backend>,
        );
        Ok(Self {
            backends,
            graphs: Table::default(),
            executions: Table::default(),
        })
    }
}

/// This struct solely wraps [Ctx] in a `RefCell`.
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
