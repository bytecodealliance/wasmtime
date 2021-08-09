//! Define the Rust interface a backend must implement in order to be used by
//! this crate. the `Box<dyn ...>` types returned by these interfaces allow
//! implementations to maintain backend-specific state between calls.

use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor};
use thiserror::Error;
use wiggle::GuestError;

/// A [Backend] contains the necessary state to load [BackendGraph]s.
pub(crate) trait Backend {
    fn name(&self) -> &str;
    fn load(
        &mut self,
        builders: &GraphBuilderArray<'_>,
        target: ExecutionTarget,
    ) -> Result<Box<dyn BackendGraph>, BackendError>;
}

/// A [BackendGraph] can create [BackendExecutionContext]s; this is the backing
/// implementation for a [crate::witx::types::Graph].
pub(crate) trait BackendGraph {
    fn init_execution_context(&mut self) -> Result<Box<dyn BackendExecutionContext>, BackendError>;
}

/// A [BackendExecutionContext] performs the actual inference; this is the
/// backing implementation for a [crate::witx::types::GraphExecutionContext].
pub(crate) trait BackendExecutionContext {
    fn set_input(&mut self, index: u32, tensor: &Tensor<'_>) -> Result<(), BackendError>;
    fn compute(&mut self) -> Result<(), BackendError>;
    fn get_output(&mut self, index: u32, destination: &mut [u8]) -> Result<u32, BackendError>;
}

/// Errors returned by a backend; [BackendError::BackendAccess] is a catch-all
/// for failures interacting with the ML library.
#[derive(Debug, Error)]
pub(crate) enum BackendError {
    #[error("Failed while accessing backend")]
    BackendAccess(#[from] anyhow::Error),
    #[error("Failed while accessing guest module")]
    GuestAccess(#[from] GuestError),
    #[error("The backend expects {0} buffers, passed {1}")]
    InvalidNumberOfBuilders(u32, u32),
    #[error("Not enough memory to copy tensor data of size: {0}")]
    NotEnoughMemory(usize),
}
