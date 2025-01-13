//! Implements the `wasi-nn` API for the WIT ("preview2") ABI.
//!
//! Note that `wasi-nn` is not yet included in an official "preview2" world
//! (though it could be) so by "preview2" here we mean that this can be called
//! with the component model's canonical ABI.
//!
//! This module exports its [`types`] for use throughout the crate and the
//! [`ML`] object, which exposes [`ML::add_to_linker`]. To implement all of
//! this, this module proceeds in steps:
//! 1. generate all of the WIT glue code into a `generated::*` namespace
//! 2. wire up the `generated::*` glue to the context state, delegating actual
//!    computation to a [`Backend`]
//! 3. convert some types
//!
//! [`Backend`]: crate::Backend
//! [`types`]: crate::wit::types

use crate::backend::Id;
use crate::{Backend, Registry};
use anyhow::anyhow;
use std::collections::HashMap;
use std::hash::Hash;
use std::{fmt, str::FromStr};
use wasmtime::component::{Resource, ResourceTable};

/// Capture the state necessary for calling into the backend ML libraries.
pub struct WasiNnCtx {
    pub(crate) backends: HashMap<GraphEncoding, Backend>,
    pub(crate) registry: Registry,
}

impl WasiNnCtx {
    /// Make a new context from the default state.
    pub fn new(backends: impl IntoIterator<Item = Backend>, registry: Registry) -> Self {
        let backends = backends.into_iter().map(|b| (b.encoding(), b)).collect();
        Self { backends, registry }
    }
}

/// A wrapper capturing the needed internal wasi-nn state.
///
/// Unlike other WASI proposals (see `wasmtime-wasi`, `wasmtime-wasi-http`),
/// this wrapper is not a `trait` but rather holds the references directly. This
/// remove one layer of abstraction for simplicity only, and could be added back
/// in the future if embedders need more control here.
pub struct WasiNnView<'a> {
    ctx: &'a mut WasiNnCtx,
    table: &'a mut ResourceTable,
}

impl<'a> WasiNnView<'a> {
    /// Create a new view into the wasi-nn state.
    pub fn new(table: &'a mut ResourceTable, ctx: &'a mut WasiNnCtx) -> Self {
        Self { ctx, table }
    }
}

/// A wasi-nn error; this appears on the Wasm side as a component model
/// resource.
#[derive(Debug)]
pub struct Error {
    code: ErrorCode,
    data: anyhow::Error,
}

/// Construct an [`Error`] resource and immediately return it.
///
/// The WIT specification currently relies on "errors as resources;" this helper
/// macro hides some of that complexity. If [#75] is adopted ("errors as
/// records"), this macro is no longer necessary.
///
/// [#75]: https://github.com/WebAssembly/wasi-nn/pull/75
macro_rules! bail {
    ($self:ident, $code:expr, $data:expr) => {
        let e = Error {
            code: $code,
            data: $data.into(),
        };
        tracing::error!("failure: {e:?}");
        let r = $self.table.push(e)?;
        return Ok(Err(r));
    };
}

impl From<wasmtime::component::ResourceTableError> for Error {
    fn from(error: wasmtime::component::ResourceTableError) -> Self {
        Self {
            code: ErrorCode::Trap,
            data: error.into(),
        }
    }
}

/// The list of error codes available to the `wasi-nn` API; this should match
/// what is specified in WIT.
#[derive(Debug)]
pub enum ErrorCode {
    /// Caller module passed an invalid argument.
    InvalidArgument,
    /// Invalid encoding.
    InvalidEncoding,
    /// The operation timed out.
    Timeout,
    /// Runtime error.
    RuntimeError,
    /// Unsupported operation.
    UnsupportedOperation,
    /// Graph is too large.
    TooLarge,
    /// Graph not found.
    NotFound,
    /// A runtime error that Wasmtime should trap on; this will not appear in
    /// the WIT specification.
    Trap,
}

/// Generate the traits and types from the `wasi-nn` WIT specification.
mod generated_ {
    wasmtime::component::bindgen!({
        world: "ml",
        path: "wit/wasi-nn.wit",
        trappable_imports: true,
        with: {
            // Configure all WIT http resources to be defined types in this
            // crate to use the `ResourceTable` helper methods.
            "wasi:nn/graph/graph": crate::Graph,
            "wasi:nn/tensor/tensor": crate::Tensor,
            "wasi:nn/inference/graph-execution-context": crate::ExecutionContext,
            "wasi:nn/errors/error": super::Error,
        },
        trappable_error_type: {
            "wasi:nn/errors/error" => super::Error,
        },
    });
}
use generated_::wasi::nn::{self as generated}; // Shortcut to the module containing the types we need.

// Export the `types` used in this crate as well as `ML::add_to_linker`.
pub mod types {
    use super::generated;
    pub use generated::errors::Error;
    pub use generated::graph::{ExecutionTarget, Graph, GraphBuilder, GraphEncoding};
    pub use generated::inference::GraphExecutionContext;
    pub use generated::tensor::{Tensor, TensorType};
}
pub use generated::graph::{ExecutionTarget, Graph, GraphBuilder, GraphEncoding};
pub use generated::inference::GraphExecutionContext;
pub use generated::tensor::{Tensor, TensorData, TensorDimensions, TensorType};
pub use generated_::Ml as ML;

/// Add the WIT-based version of the `wasi-nn` API to a
/// [`wasmtime::component::Linker`].
pub fn add_to_linker<T>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl Fn(&mut T) -> WasiNnView<'_> + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    generated::graph::add_to_linker_get_host(l, f)?;
    generated::tensor::add_to_linker_get_host(l, f)?;
    generated::inference::add_to_linker_get_host(l, f)?;
    generated::errors::add_to_linker_get_host(l, f)?;
    Ok(())
}

impl generated::graph::Host for WasiNnView<'_> {
    fn load(
        &mut self,
        builders: Vec<GraphBuilder>,
        encoding: GraphEncoding,
        target: ExecutionTarget,
    ) -> wasmtime::Result<Result<Resource<Graph>, Resource<Error>>> {
        tracing::debug!("load {encoding:?} {target:?}");
        if let Some(backend) = self.ctx.backends.get_mut(&encoding) {
            let slices = builders.iter().map(|s| s.as_slice()).collect::<Vec<_>>();
            match backend.load(&slices, target.into()) {
                Ok(graph) => {
                    let graph = self.table.push(graph)?;
                    Ok(Ok(graph))
                }
                Err(error) => {
                    bail!(self, ErrorCode::RuntimeError, error);
                }
            }
        } else {
            bail!(
                self,
                ErrorCode::InvalidEncoding,
                anyhow!("unable to find a backend for this encoding")
            );
        }
    }

    fn load_by_name(
        &mut self,
        name: String,
    ) -> wasmtime::Result<Result<Resource<Graph>, Resource<Error>>> {
        use core::result::Result::*;
        tracing::debug!("load by name {name:?}");
        let registry = &self.ctx.registry;
        if let Some(graph) = registry.get(&name) {
            let graph = graph.clone();
            let graph = self.table.push(graph)?;
            Ok(Ok(graph))
        } else {
            bail!(
                self,
                ErrorCode::NotFound,
                anyhow!("failed to find graph with name: {name}")
            );
        }
    }
}

impl generated::graph::HostGraph for WasiNnView<'_> {
    fn init_execution_context(
        &mut self,
        graph: Resource<Graph>,
    ) -> wasmtime::Result<Result<Resource<GraphExecutionContext>, Resource<Error>>> {
        use core::result::Result::*;
        tracing::debug!("initialize execution context");
        let graph = self.table.get(&graph)?;
        match graph.init_execution_context() {
            Ok(exec_context) => {
                let exec_context = self.table.push(exec_context)?;
                Ok(Ok(exec_context))
            }
            Err(error) => {
                bail!(self, ErrorCode::RuntimeError, error);
            }
        }
    }

    fn drop(&mut self, graph: Resource<Graph>) -> wasmtime::Result<()> {
        self.table.delete(graph)?;
        Ok(())
    }
}

impl generated::inference::HostGraphExecutionContext for WasiNnView<'_> {
    fn set_input(
        &mut self,
        exec_context: Resource<GraphExecutionContext>,
        name: String,
        tensor: Resource<Tensor>,
    ) -> wasmtime::Result<Result<(), Resource<Error>>> {
        let tensor = self.table.get(&tensor)?;
        tracing::debug!("set input {name:?}: {tensor:?}");
        let tensor = tensor.clone(); // TODO: avoid copying the tensor
        let exec_context = self.table.get_mut(&exec_context)?;
        if let Err(error) = exec_context.set_input(Id::Name(name), &tensor) {
            bail!(self, ErrorCode::InvalidArgument, error);
        } else {
            Ok(Ok(()))
        }
    }

    fn compute(
        &mut self,
        exec_context: Resource<GraphExecutionContext>,
    ) -> wasmtime::Result<Result<(), Resource<Error>>> {
        let exec_context = &mut self.table.get_mut(&exec_context)?;
        tracing::debug!("compute");
        match exec_context.compute() {
            Ok(()) => Ok(Ok(())),
            Err(error) => {
                bail!(self, ErrorCode::RuntimeError, error);
            }
        }
    }

    fn get_output(
        &mut self,
        exec_context: Resource<GraphExecutionContext>,
        name: String,
    ) -> wasmtime::Result<Result<Resource<Tensor>, Resource<Error>>> {
        let exec_context = self.table.get_mut(&exec_context)?;
        tracing::debug!("get output {name:?}");
        match exec_context.get_output(Id::Name(name)) {
            Ok(tensor) => {
                let tensor = self.table.push(tensor)?;
                Ok(Ok(tensor))
            }
            Err(error) => {
                bail!(self, ErrorCode::RuntimeError, error);
            }
        }
    }

    fn drop(&mut self, exec_context: Resource<GraphExecutionContext>) -> wasmtime::Result<()> {
        self.table.delete(exec_context)?;
        Ok(())
    }
}

impl generated::tensor::HostTensor for WasiNnView<'_> {
    fn new(
        &mut self,
        dimensions: TensorDimensions,
        ty: TensorType,
        data: TensorData,
    ) -> wasmtime::Result<Resource<Tensor>> {
        let tensor = Tensor {
            dimensions,
            ty,
            data,
        };
        let tensor = self.table.push(tensor)?;
        Ok(tensor)
    }

    fn dimensions(&mut self, tensor: Resource<Tensor>) -> wasmtime::Result<TensorDimensions> {
        let tensor = self.table.get(&tensor)?;
        Ok(tensor.dimensions.clone())
    }

    fn ty(&mut self, tensor: Resource<Tensor>) -> wasmtime::Result<TensorType> {
        let tensor = self.table.get(&tensor)?;
        Ok(tensor.ty)
    }

    fn data(&mut self, tensor: Resource<Tensor>) -> wasmtime::Result<TensorData> {
        let tensor = self.table.get(&tensor)?;
        Ok(tensor.data.clone())
    }

    fn drop(&mut self, tensor: Resource<Tensor>) -> wasmtime::Result<()> {
        self.table.delete(tensor)?;
        Ok(())
    }
}

impl generated::errors::HostError for WasiNnView<'_> {
    fn code(&mut self, error: Resource<Error>) -> wasmtime::Result<generated::errors::ErrorCode> {
        let error = self.table.get(&error)?;
        match error.code {
            ErrorCode::InvalidArgument => Ok(generated::errors::ErrorCode::InvalidArgument),
            ErrorCode::InvalidEncoding => Ok(generated::errors::ErrorCode::InvalidEncoding),
            ErrorCode::Timeout => Ok(generated::errors::ErrorCode::Timeout),
            ErrorCode::RuntimeError => Ok(generated::errors::ErrorCode::RuntimeError),
            ErrorCode::UnsupportedOperation => {
                Ok(generated::errors::ErrorCode::UnsupportedOperation)
            }
            ErrorCode::TooLarge => Ok(generated::errors::ErrorCode::TooLarge),
            ErrorCode::NotFound => Ok(generated::errors::ErrorCode::NotFound),
            ErrorCode::Trap => Err(anyhow!(error.data.to_string())),
        }
    }

    fn data(&mut self, error: Resource<Error>) -> wasmtime::Result<String> {
        let error = self.table.get(&error)?;
        Ok(error.data.to_string())
    }

    fn drop(&mut self, error: Resource<Error>) -> wasmtime::Result<()> {
        self.table.delete(error)?;
        Ok(())
    }
}

impl generated::errors::Host for WasiNnView<'_> {
    fn convert_error(&mut self, err: Error) -> wasmtime::Result<Error> {
        if matches!(err.code, ErrorCode::Trap) {
            Err(err.data)
        } else {
            Ok(err)
        }
    }
}

impl generated::tensor::Host for WasiNnView<'_> {}
impl generated::inference::Host for WasiNnView<'_> {}

impl Hash for generated::graph::GraphEncoding {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_string().hash(state)
    }
}

impl fmt::Display for generated::graph::GraphEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use generated::graph::GraphEncoding::*;
        match self {
            Openvino => write!(f, "openvino"),
            Onnx => write!(f, "onnx"),
            Pytorch => write!(f, "pytorch"),
            Tensorflow => write!(f, "tensorflow"),
            Tensorflowlite => write!(f, "tensorflowlite"),
            Autodetect => write!(f, "autodetect"),
            Ggml => write!(f, "ggml"),
        }
    }
}

impl FromStr for generated::graph::GraphEncoding {
    type Err = GraphEncodingParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openvino" => Ok(generated::graph::GraphEncoding::Openvino),
            "onnx" => Ok(generated::graph::GraphEncoding::Onnx),
            "pytorch" => Ok(generated::graph::GraphEncoding::Pytorch),
            "tensorflow" => Ok(generated::graph::GraphEncoding::Tensorflow),
            "tensorflowlite" => Ok(generated::graph::GraphEncoding::Tensorflowlite),
            "autodetect" => Ok(generated::graph::GraphEncoding::Autodetect),
            _ => Err(GraphEncodingParseError(s.into())),
        }
    }
}
#[derive(Debug)]
pub struct GraphEncodingParseError(String);
impl fmt::Display for GraphEncodingParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown graph encoding: {}", self.0)
    }
}
impl std::error::Error for GraphEncodingParseError {}
