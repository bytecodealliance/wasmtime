//! Implements the `wasi-nn` API for a "preview1" ABI.
//!
//! Note that `wasi-nn` was never included in the official "preview1" snapshot,
//! but the naming here means that the `wasi-nn` imports can be called with the
//! original "preview1" ABI.
//!
//! The only export from this module is [`add_to_linker`]. To implement it,
//! this module proceeds in steps:
//! 1. generate all of the Wiggle glue code into a `witx::*` namespace
//! 2. wire up the `witx::*` glue to the context state, delegating actual
//!    computation to a `Backend`
//! 3. wrap up with some conversions, i.e., from `witx::*` types to this crate's
//!    [`types`].
//!
//! [`types`]: crate::types

use crate::ctx::{UsageError, WasiNnCtx, WasiNnError, WasiNnResult as Result};
use wiggle::GuestPtr;

pub use witx::wasi_ephemeral_nn::add_to_linker;

/// Generate the traits and types from the `wasi-nn` WITX specification.
mod witx {
    use super::*;
    wiggle::from_witx!({
        witx: ["$WASI_ROOT/phases/ephemeral/witx/wasi_ephemeral_nn.witx"],
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
    impl<'a> types::UserErrorConversion for WasiNnCtx {
        fn nn_errno_from_wasi_nn_error(
            &mut self,
            e: WasiNnError,
        ) -> anyhow::Result<types::NnErrno> {
            eprintln!("Host error: {:?}", e);
            match e {
                WasiNnError::BackendError(_) => unimplemented!(),
                WasiNnError::GuestError(_) => unimplemented!(),
                WasiNnError::UsageError(_) => unimplemented!(),
            }
        }
    }
}

/// Wire up the WITX-generated trait to the `wasi-nn` host state.
impl<'a> witx::wasi_ephemeral_nn::WasiEphemeralNn for WasiNnCtx {
    fn load<'b>(
        &mut self,
        builders: &witx::types::GraphBuilderArray<'_>,
        encoding: witx::types::GraphEncoding,
        target: witx::types::ExecutionTarget,
    ) -> Result<witx::types::Graph> {
        let encoding_id: u8 = encoding.into();
        let graph = if let Some(backend) = self.backends.get_mut(&encoding_id.into()) {
            // Retrieve all of the "builder lists" from the Wasm memory (see
            // $graph_builder_array) as slices for a backend to operate on.
            let mut slices = vec![];
            for builder in builders.iter() {
                let slice = builder?
                    .read()?
                    .as_slice()?
                    .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");
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

    fn init_execution_context(
        &mut self,
        graph_id: witx::types::Graph,
    ) -> Result<witx::types::GraphExecutionContext> {
        let exec_context = if let Some(graph) = self.graphs.get_mut(graph_id.into()) {
            graph.init_execution_context()?
        } else {
            return Err(UsageError::InvalidGraphHandle.into());
        };

        let exec_context_id = self.executions.insert(exec_context);
        Ok(exec_context_id.into())
    }

    fn set_input<'b>(
        &mut self,
        exec_context_id: witx::types::GraphExecutionContext,
        index: u32,
        tensor: &witx::types::Tensor<'b>,
    ) -> Result<()> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id.into()) {
            let mut dims = vec![];
            for d in tensor.dimensions.iter() {
                dims.push(d?.read()? as usize);
            }
            let ty = tensor.type_.into();
            let data_ = tensor.data
                .as_slice()?
                .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");
            let data = data_.as_ref();
            let dims = &dims;
            Ok(exec_context.set_input(index, &crate::types::Tensor { dims, ty, data })?)
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }

    fn compute(&mut self, exec_context_id: witx::types::GraphExecutionContext) -> Result<()> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id.into()) {
            Ok(exec_context.compute()?)
        } else {
            Err(UsageError::InvalidExecutionContextHandle.into())
        }
    }

    fn get_output<'b>(
        &mut self,
        exec_context_id: witx::types::GraphExecutionContext,
        index: u32,
        out_buffer: &GuestPtr<'_, u8>,
        out_buffer_max_size: u32,
    ) -> Result<u32> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id.into()) {
            let mut destination = out_buffer
                .as_array(out_buffer_max_size)
                .as_slice_mut()?
                .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");
            Ok(exec_context.get_output(index, &mut destination)?)
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }
}

// Implement some conversion from `witx::types::*` to this crate's version.

impl From<witx::types::ExecutionTarget> for crate::types::ExecutionTarget {
    fn from(value: witx::types::ExecutionTarget) -> Self {
        match value {
            witx::types::ExecutionTarget::Cpu => crate::types::ExecutionTarget::CPU,
            witx::types::ExecutionTarget::Gpu => crate::types::ExecutionTarget::GPU,
            witx::types::ExecutionTarget::Tpu => crate::types::ExecutionTarget::TPU,
        }
    }
}
impl From<witx::types::GraphEncoding> for crate::types::GraphEncoding {
    fn from(value: witx::types::GraphEncoding) -> Self {
        match value {
            witx::types::GraphEncoding::Openvino => crate::types::GraphEncoding::OpenVINO,
        }
    }
}
impl From<witx::types::TensorType> for crate::types::TensorType {
    fn from(value: witx::types::TensorType) -> Self {
        match value {
            witx::types::TensorType::F16 => crate::types::TensorType::F16,
            witx::types::TensorType::F32 => crate::types::TensorType::F32,
            witx::types::TensorType::U8 => crate::types::TensorType::U8,
            witx::types::TensorType::I32 => crate::types::TensorType::I32,
        }
    }
}
