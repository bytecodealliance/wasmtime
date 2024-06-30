//! Implements the `wasi-nn` API for the WITX ("preview1") ABI.
//!
//! `wasi-nn` was never included in the official "preview1" snapshot, but this
//! module implements the ABI that is compatible with "preview1".
//!
//! The only export from this module is [`add_to_linker`]. To implement it, this
//! module proceeds in steps:
//! 1. generate all of the Wiggle glue code into a `gen::*` namespace
//! 2. wire up the `gen::*` glue to the context state, delegating actual
//!    computation to a `Backend`
//! 3. wrap up with some conversions, i.e., from `gen::*` types to this crate's
//!    [`types`].
//!
//! [`types`]: crate::wit::types

use crate::backend::BackendError;
use crate::backend::Id;
use crate::wit::GraphEncoding;
use crate::{Backend, ExecutionContext, Graph, Registry};
use std::collections::HashMap;
use std::hash::Hash;
use thiserror::Error;
use wiggle::{GuestError, GuestMemory, GuestPtr};

pub use gen::wasi_ephemeral_nn::add_to_linker;

pub(crate) type WasiNnResult<T> = std::result::Result<T, WasiNnError>;
type Result<T> = WasiNnResult<T>;
type GraphId = u32;
type GraphExecutionContextId = u32;

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

/// Generate the traits and types from the `wasi-nn` WITX specification.
mod gen {
    use super::*;
    wiggle::from_witx!({
        witx: ["$WASI_ROOT/wasi-nn.witx"],
        errors: { nn_errno => WasiNnError }
    });

    /// Additionally, we must let Wiggle know which of our error codes
    /// represents a successful operation.
    impl wiggle::GuestErrorType for types::NnErrno {
        fn success() -> Self {
            Self::Success
        }
    }

    /// Convert the host errors to their WITX-generated type.
    impl types::UserErrorConversion for WasiNnCtx {
        fn nn_errno_from_wasi_nn_error(
            &mut self,
            e: WasiNnError,
        ) -> anyhow::Result<types::NnErrno> {
            tracing::debug!("host error: {:?}", e);
            match e {
                WasiNnError::BackendError(_) => Ok(types::NnErrno::RuntimeError),
                WasiNnError::GuestError(_) => unimplemented!("guest error conversion"),
                WasiNnError::UsageError(_) => Ok(types::NnErrno::UnsupportedOperation),
                WasiNnError::NotEnoughMemory(_) => Ok(types::NnErrno::TooLarge),
            }
        }
    }
}

/// Wire up the WITX-generated trait to the `wasi-nn` host state.
impl gen::wasi_ephemeral_nn::WasiEphemeralNn for WasiNnCtx {
    fn load(
        &mut self,
        memory: &mut GuestMemory<'_>,
        builders: gen::types::GraphBuilderArray,
        encoding: gen::types::GraphEncoding,
        target: gen::types::ExecutionTarget,
    ) -> Result<gen::types::Graph> {
        let graph = if let Some(backend) = self.backends.get_mut(&encoding.into()) {
            // Retrieve all of the "builder lists" from the Wasm memory (see
            // $graph_builder_array) as slices for a backend to operate on.
            let mut slices = vec![];
            for builder in builders.iter() {
                let builder = memory.read(builder?)?;
                let slice = memory.as_slice(builder)?.expect(
                    "cannot use with shared memories; \
                     see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)",
                );
                slices.push(slice);
            }
            let slice_refs = slices.iter().map(|s| s.as_ref()).collect::<Vec<_>>();
            backend.load(&slice_refs, target.into())?
        } else {
            return Err(UsageError::InvalidEncoding(encoding.into()).into());
        };
        let graph_id = self.graphs.insert(graph);
        Ok(graph_id.into())
    }

    fn load_by_name(
        &mut self,
        memory: &mut GuestMemory<'_>,
        name: wiggle::GuestPtr<str>,
    ) -> Result<gen::types::Graph> {
        let name = memory.as_str(name)?.unwrap();
        if let Some(graph) = self.registry.get_mut(&name) {
            let graph_id = self.graphs.insert(graph.clone().into());
            Ok(graph_id.into())
        } else {
            return Err(UsageError::NotFound(name.to_string()).into());
        }
    }

    fn init_execution_context(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        graph_id: gen::types::Graph,
    ) -> Result<gen::types::GraphExecutionContext> {
        let exec_context = if let Some(graph) = self.graphs.get_mut(graph_id.into()) {
            graph.init_execution_context()?
        } else {
            return Err(UsageError::InvalidGraphHandle.into());
        };

        let exec_context_id = self.executions.insert(exec_context);
        Ok(exec_context_id.into())
    }

    fn set_input(
        &mut self,
        memory: &mut GuestMemory<'_>,
        exec_context_id: gen::types::GraphExecutionContext,
        index: u32,
        tensor: &gen::types::Tensor,
    ) -> Result<()> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id.into()) {
            let tensor = crate::wit::types::Tensor {
                dimensions: memory.to_vec(tensor.dimensions)?,
                ty: tensor.type_.into(),
                data: memory.to_vec(tensor.data)?,
            };
            Ok(exec_context.set_input(Id::Index(index), &tensor)?)
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }

    fn compute(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        exec_context_id: gen::types::GraphExecutionContext,
    ) -> Result<()> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id.into()) {
            Ok(exec_context.compute()?)
        } else {
            Err(UsageError::InvalidExecutionContextHandle.into())
        }
    }

    fn get_output(
        &mut self,
        memory: &mut GuestMemory<'_>,
        exec_context_id: gen::types::GraphExecutionContext,
        index: u32,
        out_buffer: GuestPtr<u8>,
        out_buffer_max_size: u32,
    ) -> Result<u32> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id.into()) {
            let tensor = exec_context.get_output(Id::Index(index))?;
            let destination = memory
                .as_slice_mut(out_buffer.as_array(out_buffer_max_size))?
                .expect(
                    "cannot use with shared memories; \
                     see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)",
                );
            if tensor.data.len() > destination.len() {
                Err(WasiNnError::NotEnoughMemory(tensor.data.len()))
            } else {
                destination[..tensor.data.len()].copy_from_slice(&tensor.data);
                Ok(tensor.data.len() as u32)
            }
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }
}

// Implement some conversion from `witx::types::*` to this crate's version.

impl From<gen::types::ExecutionTarget> for crate::wit::types::ExecutionTarget {
    fn from(value: gen::types::ExecutionTarget) -> Self {
        match value {
            gen::types::ExecutionTarget::Cpu => crate::wit::types::ExecutionTarget::Cpu,
            gen::types::ExecutionTarget::Gpu => crate::wit::types::ExecutionTarget::Gpu,
            gen::types::ExecutionTarget::Tpu => crate::wit::types::ExecutionTarget::Tpu,
        }
    }
}
impl From<gen::types::GraphEncoding> for crate::wit::types::GraphEncoding {
    fn from(value: gen::types::GraphEncoding) -> Self {
        match value {
            gen::types::GraphEncoding::Openvino => crate::wit::types::GraphEncoding::Openvino,
            gen::types::GraphEncoding::Onnx => crate::wit::types::GraphEncoding::Onnx,
            gen::types::GraphEncoding::Tensorflow => crate::wit::types::GraphEncoding::Tensorflow,
            gen::types::GraphEncoding::Pytorch => crate::wit::types::GraphEncoding::Pytorch,
            gen::types::GraphEncoding::Tensorflowlite => {
                crate::wit::types::GraphEncoding::Tensorflowlite
            }
            gen::types::GraphEncoding::Autodetect => crate::wit::types::GraphEncoding::Autodetect,
        }
    }
}
impl From<gen::types::TensorType> for crate::wit::types::TensorType {
    fn from(value: gen::types::TensorType) -> Self {
        match value {
            gen::types::TensorType::F16 => crate::wit::types::TensorType::Fp16,
            gen::types::TensorType::F32 => crate::wit::types::TensorType::Fp32,
            gen::types::TensorType::U8 => crate::wit::types::TensorType::U8,
            gen::types::TensorType::I32 => crate::wit::types::TensorType::I32,
            gen::types::TensorType::I64 => crate::wit::types::TensorType::I64,
            gen::types::TensorType::F64 => crate::wit::types::TensorType::Fp64,
        }
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
    #[error("not enough memory: requested {0} bytes")]
    NotEnoughMemory(usize),
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
