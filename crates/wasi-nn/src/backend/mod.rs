//! Define the Rust interface a backend must implement in order to be used by
//! this crate. The `Box<dyn ...>` types returned by these interfaces allow
//! implementations to maintain backend-specific state between calls.


mod kserve;
#[cfg(feature = "onnx")]
pub mod onnxruntime;
#[cfg(feature = "openvino")]
pub mod openvino;
#[cfg(all(feature = "winml", target_os = "windows"))]
pub mod winml;


#[cfg(feature = "onnx")]
use self::onnxruntime::OnnxBackend;
#[cfg(feature = "openvino")]
use self::openvino::OpenvinoBackend;

use crate::backend::kserve::KServeBackend;
use crate::{Backend, ExecutionContext, Graph, Registry};

#[cfg(all(feature = "winml", target_os = "windows"))]
use self::winml::WinMLBackend;
use std::fs::File;
use std::io::Read;

use std::path::Path;

use thiserror::Error;
use wiggle::async_trait_crate::async_trait;
use wiggle::GuestError;
use crate::wit::types::{ExecutionTarget, GraphEncoding, Tensor};

/// Return a list of all available backend frameworks.
pub fn list() -> Vec<Backend> {
    let mut backends = vec![];
    #[cfg(feature = "openvino")]
    {
        backends.push(Backend::from(OpenvinoBackend::default()));
    }
    #[cfg(all(feature = "winml", target_os = "windows"))]
    {
        backends.push(Backend::from(WinMLBackend::default()));
    }
    #[cfg(feature = "onnx")]
    {
        backends.push(Backend::from(OnnxBackend::default()));
    }
    #[cfg(feature = "kserve")]
    {
        backend.push(Backend::from(KServeBackend::default()));
    }
    backends
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

/// Read a file into a byte vector.
fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = vec![];
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
