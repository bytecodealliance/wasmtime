//! Define the Rust interface a backend must implement in order to be used by
//! this crate. The `Box<dyn ...>` types returned by these interfaces allow
//! implementations to maintain backend-specific state between calls.

mod openvino;
mod kserve;


use self::openvino::OpenvinoBackend;
use crate::wit::types::{ExecutionTarget, Tensor};
use crate::{ExecutionContext, Graph};
use thiserror::Error;
use wiggle::async_trait_crate::async_trait;
use wiggle::GuestError;
use crate::backend::kserve::KServeBackend;

/// Return a list of all available backend frameworks.
pub fn list() -> Vec<(BackendKind, Box<dyn Backend>)> {
    vec![
        (BackendKind::OpenVINO, Box::new(OpenvinoBackend::default())),
        (BackendKind::KServe, Box::new(KServeBackend::default())),
    ]
}

/// A [Backend] contains the necessary state to load [BackendGraph]s.
pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    fn load(&mut self, builders: &[&[u8]], target: ExecutionTarget) -> Result<Graph, BackendError>;
}

/// A [BackendGraph] can create [BackendExecutionContext]s; this is the backing
/// implementation for a [crate::witx::types::Graph].
pub trait BackendGraph: Send + Sync {
    fn init_execution_context(&mut self) -> Result<ExecutionContext, BackendError>;
}

/// A [BackendExecutionContext] performs the actual inference; this is the
/// backing implementation for a [crate::witx::types::GraphExecutionContext].
#[async_trait]
pub trait BackendExecutionContext: Send + Sync {
    fn set_input(&mut self, index: u32, tensor: &Tensor) -> Result<(), BackendError>;
    async fn compute(&mut self) -> Result<(), BackendError>;
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
    #[error("Unsupoprted operation: {0}")]
    UnsupportedOperation(&'static str),
}

#[derive(Hash, PartialEq, Debug, Eq, Clone, Copy)]
pub enum BackendKind {
    OpenVINO,
    KServe,
}
