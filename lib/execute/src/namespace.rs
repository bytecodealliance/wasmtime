//! The core WebAssembly spec does not specify how imports are to be resolved
//! to exports. This file provides one possible way to manage multiple instances
//! and resolve imports to exports among them.

use action::{ActionError, ActionOutcome, RuntimeValue};
use cranelift_codegen::isa;
use cranelift_entity::PrimaryMap;
use instance_plus::InstancePlus;
use jit_code::JITCode;
use resolver::Resolver;
use std::collections::HashMap;
use wasmtime_runtime::Export;

/// An opaque reference to an `InstancePlus`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstancePlusIndex(u32);
entity_impl!(InstancePlusIndex, "instance");

/// A namespace containing instances keyed by name.
///
/// Note that `Namespace` implements the `Resolver` trait, so it can resolve
/// imports using defined exports.
pub struct Namespace {
    /// Mapping from identifiers to indices in `self.instances`.
    names: HashMap<String, InstancePlusIndex>,

    /// The instances, available by index.
    instances: PrimaryMap<InstancePlusIndex, InstancePlus>,
}

impl Namespace {
    /// Construct a new `Namespace`.
    pub fn new() -> Self {
        Self {
            names: HashMap::new(),
            instances: PrimaryMap::new(),
        }
    }

    /// Install a new `InstancePlus` in this `Namespace`, optionally with the
    /// given name, and return its index.
    pub fn instance(
        &mut self,
        instance_name: Option<&str>,
        instance: InstancePlus,
    ) -> InstancePlusIndex {
        let index = self.instances.push(instance);
        if let Some(instance_name) = instance_name {
            self.names.insert(instance_name.into(), index);
        }
        index
    }

    /// Get the instance index registered with the given `instance_name`.
    pub fn get_instance_index(&mut self, instance_name: &str) -> Option<&mut InstancePlusIndex> {
        self.names.get_mut(instance_name)
    }

    /// Register an instance with a given name.
    pub fn register(&mut self, name: String, index: InstancePlusIndex) {
        self.names.insert(name, index);
    }

    /// Invoke an exported function from an instance.
    pub fn invoke(
        &mut self,
        jit_code: &mut JITCode,
        isa: &isa::TargetIsa,
        index: InstancePlusIndex,
        field_name: &str,
        args: &[RuntimeValue],
    ) -> Result<ActionOutcome, ActionError> {
        self.instances[index].invoke(jit_code, isa, &field_name, &args)
    }

    /// Get the value of an exported global from an instance.
    pub fn get(
        &mut self,
        index: InstancePlusIndex,
        field_name: &str,
    ) -> Result<RuntimeValue, ActionError> {
        self.instances[index].get(&field_name)
    }
}

impl Resolver for Namespace {
    fn resolve(&mut self, instance: &str, field: &str) -> Option<Export> {
        if let Some(index) = self.names.get(instance) {
            self.instances[*index].instance.lookup(field)
        } else {
            None
        }
    }
}
