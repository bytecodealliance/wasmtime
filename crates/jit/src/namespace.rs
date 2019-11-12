//! The core WebAssembly spec does not specify how imports are to be resolved
//! to exports. This file provides one possible way to manage multiple instances
//! and resolve imports to exports among them.

use crate::resolver::Resolver;
use std::collections::HashMap;
use wasmtime_runtime::{Export, InstanceHandle};

/// A namespace containing instances keyed by name.
///
/// Note that `Namespace` implements the `Resolver` trait, so it can resolve
/// imports using defined exports.
pub struct Namespace {
    /// Mapping from identifiers to indices in `self.instances`.
    names: HashMap<String, InstanceHandle>,
}

impl Namespace {
    /// Construct a new `Namespace`.
    pub fn new() -> Self {
        Self {
            names: HashMap::new(),
        }
    }

    /// Install a new `InstanceHandle` in this `Namespace`, optionally with the
    /// given name.
    pub fn name_instance(&mut self, name: String, instance: InstanceHandle) {
        self.names.insert(name, instance);
    }

    /// Get the instance registered with the given `instance_name`.
    pub fn get_instance(&mut self, name: &str) -> Option<&mut InstanceHandle> {
        self.names.get_mut(name)
    }
}

impl Resolver for Namespace {
    fn resolve(&mut self, name: &str, field: &str) -> Option<Export> {
        if let Some(instance) = self.names.get_mut(name) {
            instance.lookup(field)
        } else {
            None
        }
    }
}
