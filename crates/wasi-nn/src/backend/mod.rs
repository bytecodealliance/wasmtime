//! Define the Rust interface a backend must implement in order to be used by
//! this crate. The `Box<dyn ...>` types returned by these interfaces allow
//! implementations to maintain backend-specific state between calls.

pub mod openvino;

use self::openvino::OpenvinoBackend;
use crate::wit::types::{ExecutionTarget, GraphEncoding, Tensor};
use crate::{Backend, ExecutionContext, Graph};
use std::path::Path;
use thiserror::Error;
use wiggle::GuestError;

/// Return a list of all available backend frameworks.
pub fn list() -> Vec<crate::Backend> {
    vec![Backend::from(OpenvinoBackend::default())]
}

/// A [Backend] contains the necessary state to load [Graph]s.
pub trait BackendInner: Send + Sync {
    fn encoding(&self) -> GraphEncoding;
    fn load(&mut self, builders: &[&[u8]], target: ExecutionTarget) -> Result<Graph, BackendError>;
    fn as_dir_loadable<'a>(&'a mut self) -> Option<&'a mut dyn BackendFromDir>;
}

/// Some [Backend]s support loading a [Graph] from a directory on the
/// filesystem; this is not a general requirement for backends but is useful for
/// the Wasmtime CLI.
pub trait BackendFromDir: BackendInner {
    fn load_from_dir(
        &mut self,
        builders: &Path,
        target: ExecutionTarget,
    ) -> Result<Graph, BackendError>;
}

/// A [BackendGraph] can create [BackendExecutionContext]s; this is the backing
/// implementation for a [crate::witx::types::Graph].
pub trait BackendGraph: Send + Sync {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError>;
}

/// A [BackendExecutionContext] performs the actual inference; this is the
/// backing implementation for a [crate::witx::types::GraphExecutionContext].
pub trait BackendExecutionContext: Send + Sync {
    fn set_input(&mut self, index: u32, tensor: &Tensor) -> Result<(), BackendError>;
    fn compute(&mut self) -> Result<(), BackendError>;
    fn get_output(&mut self, index: u32, destination: &mut [u8]) -> Result<u32, BackendError>;
}

/// Errors returned by a backend; [BackendError::BackendAccess] is a catch-all
/// for failures interacting with the ML library.
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Failed while accessing backend")]
    BackendAccess(#[from] anyhow::Error),
    #[error("Failed while accessing guest module")]
    GuestAccess(#[from] GuestError),
    #[error("The backend expects {0} buffers, passed {1}")]
    InvalidNumberOfBuilders(usize, usize),
    #[error("Not enough memory to copy tensor data of size: {0}")]
    NotEnoughMemory(usize),
}
