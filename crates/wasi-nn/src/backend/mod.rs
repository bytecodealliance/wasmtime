//! Define the Rust interface a backend must implement in order to be used by
//! this crate. The `Box<dyn ...>` types returned by these interfaces allow
//! implementations to maintain backend-specific state between calls.

#[cfg(feature = "onnx")]
pub mod onnx;
#[cfg(feature = "openvino")]
pub mod openvino;
#[cfg(all(feature = "winml", target_os = "windows"))]
pub mod winml;

#[cfg(feature = "onnx")]
use self::onnx::OnnxBackend;
#[cfg(feature = "openvino")]
use self::openvino::OpenvinoBackend;
#[cfg(all(feature = "winml", target_os = "windows"))]
use self::winml::WinMLBackend;

use crate::wit::{ExecutionTarget, GraphEncoding, Tensor};
use crate::{Backend, ExecutionContext, Graph};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;
use wiggle::GuestError;

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
    backends
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
/// implementation for the user-facing graph.
pub trait BackendGraph: Send + Sync {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError>;
}

/// A [BackendExecutionContext] performs the actual inference; this is the
/// backing implementation for a user-facing execution context.
pub trait BackendExecutionContext: Send + Sync {
    fn set_input(&mut self, id: Id, tensor: &Tensor) -> Result<(), BackendError>;
    fn compute(&mut self) -> Result<(), BackendError>;
    fn get_output(&mut self, id: Id) -> Result<Tensor, BackendError>;
}

/// An identifier for a tensor in a [Graph].
#[derive(Debug)]
pub enum Id {
    Index(u32),
    Name(String),
}
impl Id {
    pub fn index(&self) -> Option<u32> {
        match self {
            Id::Index(i) => Some(*i),
            Id::Name(_) => None,
        }
    }
    pub fn name(&self) -> Option<&str> {
        match self {
            Id::Index(_) => None,
            Id::Name(n) => Some(n),
        }
    }
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

/// Read a file into a byte vector.
fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = vec![];
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
