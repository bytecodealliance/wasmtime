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

/// A trait for modifying internal wasi-nn state.
///
/// This follows the pattern used by other WASI proposals (see `wasmtime-wasi`,
/// `wasmtime-wasi-http`).
pub trait WasiNnView {
    fn ctx(&mut self) -> &mut WasiNnCtx;
    fn table(&mut self) -> &mut ResourceTable;
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
pub use gen::errors::Error;
pub use gen::graph::{ExecutionTarget, Graph, GraphBuilder, GraphEncoding};
pub use gen::inference::GraphExecutionContext;
pub use gen::tensor::{Tensor, TensorData, TensorDimensions, TensorType};
pub use gen_::Ml as ML;

pub fn add_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiNnView,
{
    let closure = type_annotate_nn::<T, _>(|t| WasiNnImpl(t));
    gen::graph::add_to_linker_get_host(l, closure)?;
    gen::tensor::add_to_linker_get_host(l, closure)?;
    gen::inference::add_to_linker_get_host(l, closure)?;
    gen::errors::add_to_linker_get_host(l, closure)?;
    Ok(())
}

pub struct WasiNnImpl<T>(pub T);

fn type_annotate_nn<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> WasiNnImpl<&mut T>,
{
    val
}

impl<T: WasiNnView> WasiNnView for WasiNnImpl<&mut T> {
    fn ctx(&mut self) -> &mut WasiNnCtx {
        self.0.ctx()
    }
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        self.0.table()
    }
}

impl<T: WasiNnView> gen::graph::Host for WasiNnImpl<&mut T> {
    fn load(
        &mut self,
        builders: Vec<GraphBuilder>,
        encoding: GraphEncoding,
        target: ExecutionTarget,
    ) -> wasmtime::Result<Result<Resource<crate::Graph>, Error>> {
        use core::result::Result::*;
        tracing::debug!("load {encoding:?} {target:?}");
        let result = if let Some(backend) = self.ctx().backends.get_mut(&encoding) {
            let slices = builders.iter().map(|s| s.as_slice()).collect::<Vec<_>>();
            match backend.load(&slices, target.into()) {
                Ok(graph) => {
                    let graph = self.0.table().push(graph)?;
                    Ok(graph)
                }
                Err(error) => {
                    tracing::error!("failed to load graph: {error:?}");
                    Err(Error::RuntimeError)
                }
            }
        } else {
            Err(Error::InvalidEncoding)
        };
        wasmtime::Result::Ok(result)
    }

    fn load_by_name(
        &mut self,
        name: String,
    ) -> wasmtime::Result<Result<Resource<crate::Graph>, Error>> {
        use core::result::Result::*;
        tracing::debug!("load by name {name:?}");
        let registry = &self.ctx().registry;
        let result = if let Some(graph) = registry.get(&name) {
            let graph = graph.clone();
            let graph = self.0.table().push(graph)?;
            Ok(graph)
        } else {
            tracing::error!("failed to find graph with name: {name}");
            Err(Error::NotFound)
        };
        wasmtime::Result::Ok(result)
    }
}

impl<T: WasiNnView> gen::graph::HostGraph for WasiNnImpl<&mut T> {
    fn init_execution_context(
        &mut self,
        graph: Resource<Graph>,
    ) -> wasmtime::Result<Result<Resource<GraphExecutionContext>, Error>> {
        use core::result::Result::*;
        tracing::debug!("initialize execution context");
        let graph = self.0.table().get(&graph)?;
        let result = match graph.init_execution_context() {
            Ok(exec_context) => {
                let exec_context = self.0.table().push(exec_context)?;
                Ok(exec_context)
            }
            Err(error) => {
                tracing::error!("failed to initialize execution context: {error:?}");
                Err(Error::RuntimeError)
            }
        };
        wasmtime::Result::Ok(result)
    }

    fn drop(&mut self, graph: Resource<Graph>) -> wasmtime::Result<()> {
        self.0.table().delete(graph)?;
        Ok(())
    }
}

impl<T: WasiNnView> gen::inference::HostGraphExecutionContext for WasiNnImpl<&mut T> {
    fn set_input(
        &mut self,
        exec_context: Resource<GraphExecutionContext>,
        name: String,
        tensor: Resource<Tensor>,
    ) -> wasmtime::Result<Result<(), Error>> {
        use core::result::Result::*;
        let tensor = self.0.table().get(&tensor)?;
        tracing::debug!("set input {name:?}: {tensor:?}");
        let tensor = tensor.clone(); // TODO: avoid copying the tensor
        let exec_context = self.0.table().get_mut(&exec_context)?;
        let result = if let Err(e) = exec_context.set_input(Id::Name(name), &tensor) {
            tracing::error!("failed to set input: {e:?}");
            Err(Error::InvalidArgument)
        } else {
            Ok(())
        };
        wasmtime::Result::Ok(result)
    }

    fn compute(
        &mut self,
        exec_context: Resource<GraphExecutionContext>,
    ) -> wasmtime::Result<Result<(), Error>> {
        use core::result::Result::*;
        let exec_context = self.0.table().get_mut(&exec_context)?;
        tracing::debug!("compute");
        let result = match exec_context.compute() {
            Ok(()) => Ok(()),
            Err(error) => {
                tracing::error!("failed to compute: {error:?}");
                Err(Error::RuntimeError)
            }
        };
        wasmtime::Result::Ok(result)
    }

    #[doc = r" Extract the outputs after inference."]
    fn get_output(
        &mut self,
        exec_context: Resource<GraphExecutionContext>,
        name: String,
    ) -> wasmtime::Result<Result<Resource<Tensor>, Error>> {
        use core::result::Result::*;
        let exec_context = self.0.table().get_mut(&exec_context)?;
        tracing::debug!("get output {name:?}");
        let result = match exec_context.get_output(Id::Name(name)) {
            Ok(tensor) => {
                let tensor = self.0.table().push(tensor)?;
                Ok(tensor)
            }
            Err(error) => {
                tracing::error!("failed to get output: {error:?}");
                Err(Error::RuntimeError)
            }
        };
        wasmtime::Result::Ok(result)
    }

    fn drop(&mut self, exec_context: Resource<GraphExecutionContext>) -> wasmtime::Result<()> {
        self.0.table().delete(exec_context)?;
        Ok(())
    }
}

impl<T: WasiNnView> gen::tensor::HostTensor for WasiNnImpl<&mut T> {
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
        let tensor = self.0.table().push(tensor)?;
        Ok(tensor)
    }

    fn dimensions(&mut self, tensor: Resource<Tensor>) -> wasmtime::Result<TensorDimensions> {
        let tensor = self.0.table().get(&tensor)?;
        Ok(tensor.dimensions.clone())
    }

    fn ty(&mut self, tensor: Resource<Tensor>) -> wasmtime::Result<TensorType> {
        let tensor = self.0.table().get(&tensor)?;
        Ok(tensor.ty)
    }

    fn data(&mut self, tensor: Resource<Tensor>) -> wasmtime::Result<TensorData> {
        let tensor = self.0.table().get(&tensor)?;
        Ok(tensor.data.clone())
    }

    fn drop(&mut self, tensor: Resource<Tensor>) -> wasmtime::Result<()> {
        self.0.table().delete(tensor)?;
        Ok(())
    }
}

impl<T: WasiNnView> gen::tensor::Host for WasiNnImpl<&mut T> {}
impl<T: WasiNnView> gen::errors::Host for WasiNnImpl<&mut T> {}
impl<T: WasiNnView> gen::inference::Host for WasiNnImpl<&mut T> {}

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
