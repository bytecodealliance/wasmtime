//! The core WebAssembly spec does not specify how imports are to be resolved
//! to exports. This file provides one possible way to manage multiple instances
//! and resolve imports to exports among them.

use action::{get, inspect_memory, invoke};
use action::{ActionError, ActionOutcome, RuntimeValue};
use compiler::Compiler;
use cranelift_entity::PrimaryMap;
use resolver::Resolver;
use std::boxed::Box;
use std::collections::HashMap;
use std::string::String;
use wasmtime_runtime::{Export, Instance};

/// An opaque reference to an `Instance`.
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
    instances: PrimaryMap<InstanceIndex, Box<Instance>>,
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
    pub fn instance(
        &mut self,
        instance_name: Option<&str>,
        instance: Box<Instance>,
    ) -> InstanceIndex {
        let index = self.instances.push(instance);
        if let Some(instance_name) = instance_name {
            self.names.insert(instance_name.into(), index);
        }
        index
    }

    /// Get the instance index registered with the given `instance_name`.
    pub fn get_instance_index(&mut self, instance_name: &str) -> Option<&mut InstanceIndex> {
        self.names.get_mut(instance_name)
    }

    /// Register an instance with a given name.
    pub fn register(&mut self, name: String, index: InstanceIndex) {
        self.names.insert(name, index);
    }

    /// Invoke an exported function from an instance.
    pub fn invoke(
        &mut self,
        compiler: &mut Compiler,
        index: InstanceIndex,
        field_name: &str,
        args: &[RuntimeValue],
    ) -> Result<ActionOutcome, ActionError> {
        invoke(compiler, &mut self.instances[index], &field_name, &args)
    }

    /// Get a slice of memory from an instance.
    pub fn inspect_memory(
        &self,
        index: InstanceIndex,
        field_name: &str,
        start: usize,
        len: usize,
    ) -> Result<&[u8], ActionError> {
        inspect_memory(&self.instances[index], &field_name, start, len)
    }

    /// Get the value of an exported global from an instance.
    pub fn get(&self, index: InstanceIndex, field_name: &str) -> Result<RuntimeValue, ActionError> {
        get(&self.instances[index], &field_name)
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
