//! Implements the `wasi-nn` API for a "preview2" ABI.
//!
//! Note that `wasi-nn` is not yet included in an official "preview2" world
//! (though it could be) so by "preview2" here we mean that this can be called
//! with the component model's canonical ABI.
//!
//! The only export from this module is the [`ML`] object, which exposes
//! [`ML::add_to_linker`]. To implement it, this module proceeds in steps:
//! 1. generate all of the WIT glue code into a `wit::*` namespace
//! 2. wire up the `wit::*` glue to the context state, delegating actual
//!    computation to a `Backend`
//! 3. wrap up with some conversions, i.e., from `wit::*` types to this crate's
//!    [`types`].
//!
//! [`Backend`]: crate::backend::Backend
//! [`types`]: crate::types

use crate::{backend::BackendKind, ctx::UsageError, WasiNnCtx};

pub use wit_::Ml as ML;

/// Generate the traits and types from the `wasi-nn` WIT specification.
mod wit_ {
    wasmtime::component::bindgen!("ml");
}
use wit_::wasi::nn as wit; // Shortcut to the module containing the types we need.

impl wit::inference::Host for WasiNnCtx {
    /// Load an opaque sequence of bytes to use for inference.
    fn load(
        &mut self,
        builders: wit::types::GraphBuilderArray,
        encoding: wit::types::GraphEncoding,
        target: wit::types::ExecutionTarget,
    ) -> wasmtime::Result<Result<wit::types::Graph, wit::types::Error>> {
        let backend_kind: BackendKind = encoding.try_into()?;
        let graph = if let Some(backend) = self.backends.get_mut(&backend_kind) {
            let slices = builders.iter().map(|s| s.as_slice()).collect::<Vec<_>>();
            backend.load(&slices, target.into())?
        } else {
            return Err(UsageError::InvalidEncoding(encoding.into()).into());
        };
        let graph_id = self.graphs.insert(graph);
        Ok(Ok(graph_id))
    }

    /// Create an execution instance of a loaded graph.
    ///
    /// TODO: remove completely?
    fn init_execution_context(
        &mut self,
        graph_id: wit::types::Graph,
    ) -> wasmtime::Result<Result<wit::types::GraphExecutionContext, wit::types::Error>> {
        let exec_context = if let Some(graph) = self.graphs.get_mut(graph_id) {
            graph.init_execution_context()?
        } else {
            return Err(UsageError::InvalidGraphHandle.into());
        };

        let exec_context_id = self.executions.insert(exec_context);
        Ok(Ok(exec_context_id))
    }

    /// Define the inputs to use for inference.
    fn set_input(
        &mut self,
        exec_context_id: wit::types::GraphExecutionContext,
        index: u32,
        tensor: wit::types::Tensor,
    ) -> wasmtime::Result<Result<(), wit::types::Error>> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id) {
            let dims = &tensor
                .dimensions
                .iter()
                .map(|d| *d as usize)
                .collect::<Vec<_>>();
            let ty = tensor.tensor_type.into();
            let data = tensor.data.as_slice();
            exec_context.set_input(index, &crate::types::Tensor { dims, ty, data })?;
            Ok(Ok(()))
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }

    /// Compute the inference on the given inputs.
    ///
    /// TODO: refactor to compute(list<tensor>) -> result<list<tensor>, error>
    fn compute(
        &mut self,
        exec_context_id: wit::types::GraphExecutionContext,
    ) -> wasmtime::Result<Result<(), wit::types::Error>> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id) {
            exec_context.compute()?;
            Ok(Ok(()))
        } else {
            Err(UsageError::InvalidExecutionContextHandle.into())
        }
    }

    /// Extract the outputs after inference.
    fn get_output(
        &mut self,
        exec_context_id: wit::types::GraphExecutionContext,
        index: u32,
    ) -> wasmtime::Result<Result<wit::types::TensorData, wit::types::Error>> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id) {
            // Read the output bytes. TODO: this involves a hard-coded upper
            // limit on the tensor size that is necessary because there is no
            // way to introspect the graph outputs
            // (https://github.com/WebAssembly/wasi-nn/issues/37).
            let mut destination = vec![0; 1024 * 1024];
            let bytes_read = exec_context.get_output(index, &mut destination)?;
            destination.truncate(bytes_read as usize);
            Ok(Ok(destination))
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }
}

impl From<wit::types::GraphEncoding> for crate::types::GraphEncoding {
    fn from(value: wit::types::GraphEncoding) -> Self {
        match value {
            wit::types::GraphEncoding::Openvino => crate::types::GraphEncoding::OpenVINO,
            wit::types::GraphEncoding::Onnx => crate::types::GraphEncoding::ONNX,
            wit::types::GraphEncoding::Tensorflow => crate::types::GraphEncoding::Tensorflow,
            wit::types::GraphEncoding::Pytorch => crate::types::GraphEncoding::PyTorch,
            wit::types::GraphEncoding::Tensorflowlite => {
                crate::types::GraphEncoding::TensorflowLite
            }
        }
    }
}

impl TryFrom<wit::types::GraphEncoding> for crate::backend::BackendKind {
    type Error = UsageError;
    fn try_from(value: wit::types::GraphEncoding) -> Result<Self, Self::Error> {
        match value {
            wit::types::GraphEncoding::Openvino => Ok(crate::backend::BackendKind::OpenVINO),
            _ => Err(UsageError::InvalidEncoding(value.into())),
        }
    }
}

impl From<wit::types::ExecutionTarget> for crate::types::ExecutionTarget {
    fn from(value: wit::types::ExecutionTarget) -> Self {
        match value {
            wit::types::ExecutionTarget::Cpu => crate::types::ExecutionTarget::CPU,
            wit::types::ExecutionTarget::Gpu => crate::types::ExecutionTarget::GPU,
            wit::types::ExecutionTarget::Tpu => crate::types::ExecutionTarget::TPU,
        }
    }
}

impl From<wit::types::TensorType> for crate::types::TensorType {
    fn from(value: wit::types::TensorType) -> Self {
        match value {
            wit::types::TensorType::Fp16 => crate::types::TensorType::F16,
            wit::types::TensorType::Fp32 => crate::types::TensorType::F32,
            wit::types::TensorType::U8 => crate::types::TensorType::U8,
            wit::types::TensorType::I32 => crate::types::TensorType::I32,
        }
    }
}
