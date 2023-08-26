//! Define the Rust interface a backend must implement in order to be used by
//! this crate. The `Box<dyn ...>` types returned by these interfaces allow
//! implementations to maintain backend-specific state between calls.

mod kserve;
mod openvino;

use self::openvino::OpenvinoBackend;
use crate::backend::kserve::{KServeBackend, KServeClient};
use crate::wit::types::{ExecutionTarget, Tensor};
use crate::{ExecutionContext, Graph, GraphRegistry};
use std::{error::Error, fmt, path::Path, str::FromStr};
use thiserror::Error;
use wiggle::async_trait_crate::async_trait;
use wiggle::GuestError;

/// Return a list of all available backend frameworks.
pub fn list() -> Vec<(BackendKind, Box<dyn Backend>)> {
    vec![
        (BackendKind::OpenVINO, Box::new(OpenvinoBackend::default())),
        (BackendKind::KServe, Box::new(KServeBackend::default())),
    ]
}

pub fn build_kserve_registry(server_url: &String) -> Box<dyn GraphRegistry> {
    Box::new(KServeBackend { server_url: server_url.clone(), ..Default::default() })
}

/// A [Backend] contains the necessary state to load [Graph]s.
pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    fn load(&mut self, builders: &[&[u8]], target: ExecutionTarget) -> Result<Graph, BackendError>;
    fn as_dir_loadable<'a>(&'a mut self) -> Option<&'a mut dyn BackendFromDir>;
}

/// Some [Backend]s support loading a [Graph] from a directory on the
/// filesystem; this is not a general requirement for backends but is useful for
/// the Wasmtime CLI.
pub trait BackendFromDir: Backend {
    fn load_from_dir(
        &mut self,
        builders: &Path,
        target: ExecutionTarget,
    ) -> Result<Graph, BackendError>;
}

#[async_trait]
/// A [BackendGraph] can create [BackendExecutionContext]s; this is the backing
/// implementation for a [crate::witx::types::Graph].
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

#[derive(Hash, PartialEq, Debug, Eq, Clone, Copy)]
pub enum BackendKind {
    OpenVINO,
    KServe,
}

impl FromStr for BackendKind {
    type Err = BackendKindParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openvino" => Ok(BackendKind::OpenVINO),
            _ => Err(BackendKindParseError(s.into())),
        }
    }
}

#[derive(Debug)]
pub struct BackendKindParseError(String);

impl fmt::Display for BackendKindParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown backend: {}", self.0)
    }
}

impl Error for BackendKindParseError {}
