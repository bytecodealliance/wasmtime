//! Implements the `wasi-nn` API for the WIT ("preview2") ABI.
//!
//! Note that `wasi-nn` is not yet included in an official "preview2" world
//! (though it could be) so by "preview2" here we mean that this can be called
//! with the component model's canonical ABI.
//!
//! The only export from this module is the [`ML`] object, which exposes
//! [`ML::add_to_linker`]. To implement it, this module proceeds in steps:
//! 1. generate all of the WIT glue code into a `gen::*` namespace
//! 2. wire up the `gen::*` glue to the context state, delegating actual
//!    computation to a `Backend`
//! 3. wrap up with some conversions, i.e., from `gen::*` types to this crate's
//!    [`types`].
//!
//! [`Backend`]: crate::backend::Backend
//! [`types`]: crate::types

use crate::{backend::BackendKind, ctx::UsageError, WasiNnCtx};

pub use gen_::Ml as ML;

/// Generate the traits and types from the `wasi-nn` WIT specification.
mod gen_ {
    wasmtime::component::bindgen!("ml");
}
use gen_::wasi::nn as gen; // Shortcut to the module containing the types we need.

impl gen::inference::Host for WasiNnCtx {
    /// Load an opaque sequence of bytes to use for inference.
    fn load(
        &mut self,
        builders: gen::types::GraphBuilderArray,
        encoding: gen::types::GraphEncoding,
        target: gen::types::ExecutionTarget,
    ) -> wasmtime::Result<Result<gen::types::Graph, gen::types::Error>> {
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
        graph_id: gen::types::Graph,
    ) -> wasmtime::Result<Result<gen::types::GraphExecutionContext, gen::types::Error>> {
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
        exec_context_id: gen::types::GraphExecutionContext,
        index: u32,
        tensor: gen::types::Tensor,
    ) -> wasmtime::Result<Result<(), gen::types::Error>> {
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
        exec_context_id: gen::types::GraphExecutionContext,
    ) -> wasmtime::Result<Result<(), gen::types::Error>> {
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
        exec_context_id: gen::types::GraphExecutionContext,
        index: u32,
    ) -> wasmtime::Result<Result<gen::types::TensorData, gen::types::Error>> {
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

impl From<gen::types::GraphEncoding> for crate::types::GraphEncoding {
    fn from(value: gen::types::GraphEncoding) -> Self {
        match value {
            gen::types::GraphEncoding::Openvino => crate::types::GraphEncoding::OpenVINO,
            gen::types::GraphEncoding::Onnx => crate::types::GraphEncoding::ONNX,
            gen::types::GraphEncoding::Tensorflow => crate::types::GraphEncoding::Tensorflow,
            gen::types::GraphEncoding::Pytorch => crate::types::GraphEncoding::PyTorch,
            gen::types::GraphEncoding::Tensorflowlite => {
                crate::types::GraphEncoding::TensorflowLite
            }
        }
    }
}

impl TryFrom<gen::types::GraphEncoding> for crate::backend::BackendKind {
    type Error = UsageError;
    fn try_from(value: gen::types::GraphEncoding) -> Result<Self, Self::Error> {
        match value {
            gen::types::GraphEncoding::Openvino => Ok(crate::backend::BackendKind::OpenVINO),
            _ => Err(UsageError::InvalidEncoding(value.into())),
        }
    }
}

impl From<gen::types::ExecutionTarget> for crate::types::ExecutionTarget {
    fn from(value: gen::types::ExecutionTarget) -> Self {
        match value {
            gen::types::ExecutionTarget::Cpu => crate::types::ExecutionTarget::CPU,
            gen::types::ExecutionTarget::Gpu => crate::types::ExecutionTarget::GPU,
            gen::types::ExecutionTarget::Tpu => crate::types::ExecutionTarget::TPU,
        }
    }
}

impl From<gen::types::TensorType> for crate::types::TensorType {
    fn from(value: gen::types::TensorType) -> Self {
        match value {
            gen::types::TensorType::Fp16 => crate::types::TensorType::F16,
            gen::types::TensorType::Fp32 => crate::types::TensorType::F32,
            gen::types::TensorType::U8 => crate::types::TensorType::U8,
            gen::types::TensorType::I32 => crate::types::TensorType::I32,
        }
    }
}
