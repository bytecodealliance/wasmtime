//! The core WebAssembly spec does not specify how imports are to be resolved
//! to exports. This file provides one possible way to manage multiple instances
//! and resolve imports to exports among them.

use super::HashMap;
use crate::action::{get, inspect_memory, invoke};
use crate::action::{ActionError, ActionOutcome, RuntimeValue};
use crate::compiler::Compiler;
use crate::resolver::Resolver;
use cranelift_entity::PrimaryMap;
use std::string::String;
use wasmtime_runtime::{Export, Instance};

/// An opaque reference to an `Instance` within a `Namespace`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceIndex(u32);
entity_impl!(InstanceIndex, "instance");

/// A namespace containing instances keyed by name.
///
/// Note that `Namespace` implements the `Resolver` trait, so it can resolve
/// imports using defined exports.
pub struct Namespace {
    /// Mapping from identifiers to indices in `self.instances`.
    names: HashMap<String, InstanceIndex>,

    /// The instances, available by index.
    instances: PrimaryMap<InstanceIndex, Instance>,
}

impl Namespace {
    /// Construct a new `Namespace`.
    pub fn new() -> Self {
        Self {
            names: HashMap::new(),
            instances: PrimaryMap::new(),
        }
    }

    /// Install a new `Instance` in this `Namespace`, optionally with the
    /// given name, and return its index.
    pub fn instance(&mut self, instance_name: Option<String>, instance: Instance) -> InstanceIndex {
        let index = self.instances.push(instance);
        if let Some(instance_name) = instance_name {
            self.names.insert(instance_name, index);
        }
        index
    }

    /// Get the instance index registered with the given `instance_name`.
    pub fn get_instance_index(&mut self, instance_name: &str) -> Option<InstanceIndex> {
        self.names.get_mut(instance_name).cloned()
    }

    /// Register an additional name for an existing registered instance.
    pub fn alias_for_indexed(&mut self, existing_index: InstanceIndex, new_name: String) {
        self.names.insert(new_name, existing_index);
    }

    /// Invoke an exported function from an instance.
    pub fn invoke(
        &mut self,
        compiler: &mut Compiler,
        index: InstanceIndex,
        field_name: &str,
        args: &[RuntimeValue],
    ) -> Result<ActionOutcome, ActionError> {
        invoke(compiler, &mut self.instances[index], field_name, args)
    }

    /// Get a slice of memory from an instance.
    pub fn inspect_memory(
        &self,
        index: InstanceIndex,
        field_name: &str,
        start: usize,
        len: usize,
    ) -> Result<&[u8], ActionError> {
        inspect_memory(&self.instances[index], field_name, start, len)
    }

    /// Get the value of an exported global from an instance.
    pub fn get(&self, index: InstanceIndex, field_name: &str) -> Result<RuntimeValue, ActionError> {
        get(&self.instances[index], field_name)
    }
}

impl Resolver for Namespace {
    fn resolve(&mut self, instance: &str, field: &str) -> Option<Export> {
        if let Some(index) = self.names.get(instance) {
            self.instances[*index].lookup(field)
        } else {
            None
        }
    }
}
