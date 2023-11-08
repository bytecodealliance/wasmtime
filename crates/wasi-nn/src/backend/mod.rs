//! Define the Rust interface a backend must implement in order to be used by
//! this crate. The `Box<dyn ...>` types returned by these interfaces allow
//! implementations to maintain backend-specific state between calls.

mod kserve;
mod openvino;

use self::openvino::OpenvinoBackend;
use crate::backend::kserve::KServeBackend;
use crate::wit::types::{ExecutionTarget, GraphEncoding, Tensor};
use crate::{Backend, ExecutionContext, Graph, Registry};
use std::path::Path;

use thiserror::Error;
use wiggle::async_trait_crate::async_trait;
use wiggle::GuestError;

/// Return a list of all available backend frameworks.
pub fn list() -> Vec<Backend> {
    vec![
        (Backend::from(OpenvinoBackend::default())),
        (Backend::from(KServeBackend::default())),
    ]
}

pub fn build_kserve_registry(server_url: &String) -> Registry {
    Registry::from(KServeBackend {
        server_url: server_url.clone(),
        ..Default::default()
    })
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

#[async_trait]
/// A [BackendGraph] can create [BackendExecutionContext]s; this is the backing
/// implementation for the user-facing graph.
pub trait BackendGraph: Send + Sync {
    async fn init_execution_context(&self) -> Result<ExecutionContext, BackendError>;
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
