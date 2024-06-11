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
pub use gen::wasi_ephemeral_nn::add_to_linker;
use wiggle::{GuestMemory, GuestPtr};
pub use crate::ctx::kserve_registry;
use crate::ctx::{UsageError, WasiNnCtx, WasiNnError, WasiNnResult as Result};

/// Generate the traits and types from the `wasi-nn` WITX specification.
mod gen {
    use wiggle::GuestError;

    use crate::backend::BackendError;

    use super::*;

    wiggle::from_witx!({
        witx: ["$WASI_ROOT/wasi-nn.witx"],
        errors: { nn_errno => WasiNnError },
        async: { wasi_ephemeral_nn::compute, wasi_ephemeral_nn::load_by_name, wasi_ephemeral_nn::init_execution_context },
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

            anyhow::Result::Ok(match e {
                WasiNnError::BackendError(e) => match e {
                    BackendError::BackendAccess(d) => {
                        eprintln!("{}", d.to_string());
                        gen::types::NnErrno::RuntimeError
                    }
                    BackendError::GuestAccess(d) => {
                        eprintln!("{}", d.to_string());
                        gen::types::NnErrno::RuntimeError
                    }
                    BackendError::InvalidNumberOfBuilders(expected, actual) => {
                        eprintln!("Expected {} builds received {}", expected, actual);
                        gen::types::NnErrno::InvalidArgument
                    }
                    BackendError::NotEnoughMemory(d) => {
                        eprintln!("Unable to allocation {} bytes of memory.", d);
                        gen::types::NnErrno::MissingMemory
                    }
                    BackendError::UnsupportedOperation(d) => {
                        eprintln!("Unsupported operation: {}", d);
                        gen::types::NnErrno::InvalidArgument
                    }
                },
                WasiNnError::GuestError(e) => match e {
                    GuestError::InvalidFlagValue(d) => {
                        eprintln!("Invalid flag value {}", d);
                        gen::types::NnErrno::InvalidArgument
                    }
                    GuestError::InvalidEnumValue(d) => {
                        eprintln!("Invalid enum value {}", d);
                        gen::types::NnErrno::InvalidArgument
                    }
                    GuestError::PtrOverflow => {
                        eprintln!("Pointer Overflow");
                        gen::types::NnErrno::RuntimeError
                    }
                    GuestError::PtrOutOfBounds(_) => {
                        eprintln!("Pointer out of bounds");
                        gen::types::NnErrno::RuntimeError
                    }
                    GuestError::PtrNotAligned(_, _) => {
                        eprintln!("Pointer not aligned");
                        gen::types::NnErrno::RuntimeError
                    }
                    GuestError::PtrBorrowed(_) => {
                        eprintln!("Pointer borrowed");
                        gen::types::NnErrno::RuntimeError
                    }
                    GuestError::BorrowCheckerOutOfHandles => {
                        eprintln!("Borrow checker out of handles");
                        gen::types::NnErrno::RuntimeError
                    }
                    GuestError::SliceLengthsDiffer => {
                        eprintln!("Slice lengths differe");
                        gen::types::NnErrno::RuntimeError
                    }
                    GuestError::InFunc { .. } => {
                        eprintln!("In func?");
                        gen::types::NnErrno::RuntimeError
                    }
                    GuestError::InvalidUtf8(_) => {
                        eprintln!("Invalid UTF 8");
                        gen::types::NnErrno::RuntimeError
                    }
                    GuestError::TryFromIntError(_) => {
                        eprintln!("Try from int error.");
                        gen::types::NnErrno::RuntimeError
                    }
                },
                WasiNnError::UsageError(e) => {
                    eprintln!("Usage error {:?}", e);
                    gen::types::NnErrno::RuntimeError
                }
            })
        }
    }
}

#[wiggle::async_trait]
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

    async fn load_by_name(
        &mut self,
        memory: &mut GuestMemory<'_>,
        name: wiggle::GuestPtr<str>,
    ) -> Result<gen::types::Graph> {
        let name = memory.as_str(name)?.unwrap();
        if let Some(graph) = self.registry.get_mut(&name).await.unwrap() {
            let graph_id = self.graphs.insert(graph.clone().into());
            Ok(graph_id.into())
        } else {
            return Err(UsageError::NotFound(name.to_string()).into());
        }
    }

    async fn init_execution_context(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        graph_id: gen::types::Graph,
    ) -> Result<gen::types::GraphExecutionContext> {
        let exec_context = if let Some(graph) = self.graphs.get_mut(graph_id.into()) {
            graph.init_execution_context().await?
        } else {
            return Err(WasiNnError::UsageError(UsageError::InvalidGraphHandle));
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
                tensor_type: tensor.type_.into(),
                data: memory.to_vec(tensor.data)?,
            };
            Ok(exec_context.set_input(index, &tensor)?)
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }

    async fn compute(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        exec_context_id: gen::types::GraphExecutionContext,
    ) -> Result<()> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id.into()) {
            Ok(exec_context.compute().await?)
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
            let mut destination = memory
                .as_slice_mut(out_buffer.as_array(out_buffer_max_size))?
                .expect(
                    "cannot use with shared memories; \
                     see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)",
                );
            Ok(exec_context.get_output(index, &mut destination)?)
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }
}

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
