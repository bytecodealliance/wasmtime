//! Implements the `wasi-nn` API for the WIT ("preview2") ABI.
//!
//! Note that `wasi-nn` is not yet included in an official "preview2" world
//! (though it could be) so by "preview2" here we mean that this can be called
//! with the component model's canonical ABI.
//!
//! This module exports its [`types`] for use throughout the crate and the
//! [`ML`] object, which exposes [`ML::add_to_linker`]. To implement all of
//! this, this module proceeds in steps:
//! 1. generate all of the WIT glue code into a `gen::*` namespace
//! 2. wire up the `gen::*` glue to the context state, delegating actual
//!    computation to a [`Backend`]
//! 3. convert some types
//!
//! [`Backend`]: crate::Backend
//! [`types`]: crate::wit::types

use crate::backend::Id;
use crate::{Backend, Registry};
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

pub enum Error {
    /// Caller module passed an invalid argument.
    InvalidArgument,
    /// Invalid encoding.
    InvalidEncoding,
    /// The operation timed out.
    Timeout,
    /// Runtime Error.
    RuntimeError,
    /// Unsupported operation.
    UnsupportedOperation,
    /// Graph is too large.
    TooLarge,
    /// Graph not found.
    NotFound,
    /// A runtime error occurred that we should trap on; see `StreamError`.
    Trap(anyhow::Error),
}

impl From<wasmtime::component::ResourceTableError> for Error {
    fn from(error: wasmtime::component::ResourceTableError) -> Self {
        Self::Trap(error.into())
    }
}

/// Generate the traits and types from the `wasi-nn` WIT specification.
mod gen_ {
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
        },
        trappable_error_type: {
            "wasi:nn/errors/error" => super::Error,
        },
    });
}
use gen_::wasi::nn::{self as gen}; // Shortcut to the module containing the types we need.

// Export the `types` used in this crate as well as `ML::add_to_linker`.
pub mod types {
    use super::gen;
    pub use gen::errors::Error;
    pub use gen::graph::{ExecutionTarget, Graph, GraphBuilder, GraphEncoding};
    pub use gen::inference::GraphExecutionContext;
    pub use gen::tensor::{Tensor, TensorType};
}
pub use gen::graph::{ExecutionTarget, Graph, GraphBuilder, GraphEncoding};
pub use gen::inference::GraphExecutionContext;
pub use gen::tensor::{Tensor, TensorData, TensorDimensions, TensorType};
pub use gen_::Ml as ML;

/// Add the WIT-based version of the `wasi-nn` API to a
/// [`wasmtime::component::Linker`].
pub fn add_to_linker<T>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl Fn(&mut T) -> WasiNnView<'_> + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    gen::graph::add_to_linker_get_host(l, f)?;
    gen::tensor::add_to_linker_get_host(l, f)?;
    gen::inference::add_to_linker_get_host(l, f)?;
    gen::errors::add_to_linker_get_host(l, f)?;
    Ok(())
}

impl gen::graph::Host for WasiNnView<'_> {
    fn load(
        &mut self,
        builders: Vec<GraphBuilder>,
        encoding: GraphEncoding,
        target: ExecutionTarget,
    ) -> Result<Resource<crate::Graph>, Error> {
        tracing::debug!("load {encoding:?} {target:?}");
        if let Some(backend) = self.ctx.backends.get_mut(&encoding) {
            let slices = builders.iter().map(|s| s.as_slice()).collect::<Vec<_>>();
            match backend.load(&slices, target.into()) {
                Ok(graph) => {
                    let graph = self.table.push(graph)?;
                    Ok(graph)
                }
                Err(error) => {
                    tracing::error!("failed to load graph: {error:?}");
                    Err(Error::RuntimeError)
                }
            }
        } else {
            Err(Error::InvalidEncoding)
        }
    }

    fn load_by_name(&mut self, name: String) -> Result<Resource<Graph>, Error> {
        use core::result::Result::*;
        tracing::debug!("load by name {name:?}");
        let registry = &self.ctx.registry;
        if let Some(graph) = registry.get(&name) {
            let graph = graph.clone();
            let graph = self.table.push(graph)?;
            Ok(graph)
        } else {
            tracing::error!("failed to find graph with name: {name}");
            Err(Error::NotFound)
        }
    }
}

impl gen::graph::HostGraph for WasiNnView<'_> {
    fn init_execution_context(
        &mut self,
        graph: Resource<Graph>,
    ) -> Result<Resource<GraphExecutionContext>, Error> {
        use core::result::Result::*;
        tracing::debug!("initialize execution context");
        let graph = self.table.get(&graph)?;
        match graph.init_execution_context() {
            Ok(exec_context) => {
                let exec_context = self.table.push(exec_context)?;
                Ok(exec_context)
            }
            Err(error) => {
                tracing::error!("failed to initialize execution context: {error:?}");
                Err(Error::RuntimeError)
            }
        }
    }

    fn drop(&mut self, graph: Resource<Graph>) -> wasmtime::Result<()> {
        self.table.delete(graph)?;
        Ok(())
    }
}

impl gen::inference::HostGraphExecutionContext for WasiNnView<'_> {
    fn set_input(
        &mut self,
        exec_context: Resource<GraphExecutionContext>,
        name: String,
        tensor: Resource<Tensor>,
    ) -> Result<(), Error> {
        let tensor = self.table.get(&tensor)?;
        tracing::debug!("set input {name:?}: {tensor:?}");
        let tensor = tensor.clone(); // TODO: avoid copying the tensor
        let exec_context = self.table.get_mut(&exec_context)?;
        if let Err(e) = exec_context.set_input(Id::Name(name), &tensor) {
            tracing::error!("failed to set input: {e:?}");
            Err(Error::InvalidArgument)
        } else {
            Ok(())
        }
    }

    fn compute(&mut self, exec_context: Resource<GraphExecutionContext>) -> Result<(), Error> {
        let exec_context = &mut self.table.get_mut(&exec_context)?;
        tracing::debug!("compute");
        match exec_context.compute() {
            Ok(()) => Ok(()),
            Err(error) => {
                tracing::error!("failed to compute: {error:?}");
                Err(Error::RuntimeError)
            }
        }
    }

    #[doc = r" Extract the outputs after inference."]
    fn get_output(
        &mut self,
        exec_context: Resource<GraphExecutionContext>,
        name: String,
    ) -> Result<Resource<Tensor>, Error> {
        let exec_context = self.table.get_mut(&exec_context)?;
        tracing::debug!("get output {name:?}");
        match exec_context.get_output(Id::Name(name)) {
            Ok(tensor) => {
                let tensor = self.table.push(tensor)?;
                Ok(tensor)
            }
            Err(error) => {
                tracing::error!("failed to get output: {error:?}");
                Err(Error::RuntimeError)
            }
        }
    }

    fn drop(&mut self, exec_context: Resource<GraphExecutionContext>) -> wasmtime::Result<()> {
        self.table.delete(exec_context)?;
        Ok(())
    }
}

impl gen::tensor::HostTensor for WasiNnView<'_> {
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

impl gen::tensor::Host for WasiNnView<'_> {}
impl gen::errors::Host for WasiNnView<'_> {
    fn convert_error(&mut self, err: Error) -> wasmtime::Result<gen::errors::Error> {
        match err {
            Error::InvalidArgument => Ok(gen::errors::Error::InvalidArgument),
            Error::InvalidEncoding => Ok(gen::errors::Error::InvalidEncoding),
            Error::Timeout => Ok(gen::errors::Error::Timeout),
            Error::RuntimeError => Ok(gen::errors::Error::RuntimeError),
            Error::UnsupportedOperation => Ok(gen::errors::Error::UnsupportedOperation),
            Error::TooLarge => Ok(gen::errors::Error::TooLarge),
            Error::NotFound => Ok(gen::errors::Error::NotFound),
            Error::Trap(e) => Err(e),
        }
    }
}
impl gen::inference::Host for WasiNnView<'_> {}

impl Hash for gen::graph::GraphEncoding {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_string().hash(state)
    }
}

impl fmt::Display for gen::graph::GraphEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use gen::graph::GraphEncoding::*;
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

impl FromStr for gen::graph::GraphEncoding {
    type Err = GraphEncodingParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openvino" => Ok(gen::graph::GraphEncoding::Openvino),
            "onnx" => Ok(gen::graph::GraphEncoding::Onnx),
            "pytorch" => Ok(gen::graph::GraphEncoding::Pytorch),
            "tensorflow" => Ok(gen::graph::GraphEncoding::Tensorflow),
            "tensorflowlite" => Ok(gen::graph::GraphEncoding::Tensorflowlite),
            "autodetect" => Ok(gen::graph::GraphEncoding::Autodetect),
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
