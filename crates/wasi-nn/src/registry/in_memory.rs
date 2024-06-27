//! Implement a [`GraphRegistry`] with a hash map.

use super::{Graph, GraphRegistry};
use crate::backend::BackendFromDir;
use crate::wit::ExecutionTarget;
use anyhow::{anyhow, bail};
use std::{collections::HashMap, path::Path};

pub struct InMemoryRegistry(HashMap<String, Graph>);
impl InMemoryRegistry {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Load a graph from the files contained in the `path` directory.
    ///
    /// This expects the backend to know how to load graphs (i.e., ML model)
    /// from a directory. The name used in the registry is the directory's last
    /// suffix: if the backend can find the files it expects in `/my/model/foo`,
    /// the registry will contain a new graph named `foo`.
    pub fn load(&mut self, backend: &mut dyn BackendFromDir, path: &Path) -> anyhow::Result<()> {
        if !path.is_dir() {
            bail!(
                "preload directory is not a valid directory: {}",
                path.display()
            );
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy())
            .ok_or(anyhow!("no file name in path"))?;

        let graph = backend.load_from_dir(path, ExecutionTarget::Cpu)?;
        self.0.insert(name.into_owned(), graph);
        Ok(())
    }
}

impl GraphRegistry for InMemoryRegistry {
    fn get(&self, name: &str) -> Option<&Graph> {
        self.0.get(name)
    }
    fn get_mut(&mut self, name: &str) -> Option<&mut Graph> {
        self.0.get_mut(name)
    }
}
