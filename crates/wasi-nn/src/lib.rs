pub mod backend;
mod registry;
pub mod wit;
pub mod witx;

use anyhow::anyhow;
use core::fmt;
pub use registry::{GraphRegistry, InMemoryRegistry};
use std::path::Path;
use std::sync::Arc;

/// Construct an in-memory registry from the available backends and a list of
/// `(<backend name>, <graph directory>)`. This assumes graphs can be loaded
/// from a local directory, which is a safe assumption currently for the current
/// model types.
pub fn preload(preload_graphs: &[(String, String)]) -> anyhow::Result<(Vec<Backend>, Registry)> {
    let mut backends = backend::list();
    let mut registry = InMemoryRegistry::new();
    for (kind, path) in preload_graphs {
        let kind_ = kind.parse()?;
        let backend = backends
            .iter_mut()
            .find(|b| b.encoding() == kind_)
            .ok_or(anyhow!("unsupported backend: {}", kind))?
            .as_dir_loadable()
            .ok_or(anyhow!("{} does not support directory loading", kind))?;
        registry.load(backend, Path::new(path))?;
    }
    Ok((backends, Registry::from(registry)))
}

/// A machine learning backend.
pub struct Backend(Box<dyn backend::BackendInner>);
impl std::ops::Deref for Backend {
    type Target = dyn backend::BackendInner;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Backend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T: backend::BackendInner + 'static> From<T> for Backend {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

/// A backend-defined graph (i.e., ML model).
#[derive(Clone)]
pub struct Graph(Arc<dyn backend::BackendGraph>);
impl From<Box<dyn backend::BackendGraph>> for Graph {
    fn from(value: Box<dyn backend::BackendGraph>) -> Self {
        Self(value.into())
    }
}
impl std::ops::Deref for Graph {
    type Target = dyn backend::BackendGraph;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

/// A host-side tensor.
///
/// Eventually, this may be defined in each backend as they gain the ability to
/// hold tensors on various devices (TODO:
/// https://github.com/WebAssembly/wasi-nn/pull/70).
#[derive(Clone, PartialEq)]
pub struct Tensor {
    dimensions: Vec<u32>,
    ty: wit::TensorType,
    data: Vec<u8>,
}
impl fmt::Debug for Tensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tensor")
            .field("dimensions", &self.dimensions)
            .field("ty", &self.ty)
            .field("data (bytes)", &self.data.len())
            .finish()
    }
}

/// A backend-defined execution context.
pub struct ExecutionContext(Box<dyn backend::BackendExecutionContext>);
impl From<Box<dyn backend::BackendExecutionContext>> for ExecutionContext {
    fn from(value: Box<dyn backend::BackendExecutionContext>) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for ExecutionContext {
    type Target = dyn backend::BackendExecutionContext;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for ExecutionContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

/// A container for graphs.
pub struct Registry(Box<dyn GraphRegistry>);
impl std::ops::Deref for Registry {
    type Target = dyn GraphRegistry;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Registry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T> From<T> for Registry
where
    T: GraphRegistry + 'static,
{
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}
