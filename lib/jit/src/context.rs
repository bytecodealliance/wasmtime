use crate::action::{get, inspect_memory, invoke};
use crate::{
    instantiate, ActionError, ActionOutcome, Compiler, InstanceHandle, Namespace, RuntimeValue,
    SetupError,
};
use cranelift_codegen::isa::TargetIsa;
use std::borrow::ToOwned;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::string::{String, ToString};
use std::{fmt, str};
use wasmparser::{validate, OperatorValidatorConfig, ValidatingParserConfig};

/// Indicates an unknown instance was specified.
#[derive(Fail, Debug)]
pub struct UnknownInstance {
    instance_name: String,
}

impl fmt::Display for UnknownInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no instance {} present", self.instance_name)
    }
}

/// Error message used by `WastContext`.
#[derive(Fail, Debug)]
pub enum ContextError {
    /// An unknown instance name was used.
    Instance(UnknownInstance),
    /// An error occured while performing an action.
    Action(ActionError),
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ContextError::Instance(ref error) => error.fmt(f),
            ContextError::Action(ref error) => error.fmt(f),
        }
    }
}

/// A convenient context for compiling and executing WebAssembly instances.
pub struct Context {
    namespace: Namespace,
    compiler: Box<Compiler>,
    global_exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>>,
}

impl Context {
    /// Construct a new instance of `Context`.
    pub fn new(compiler: Box<Compiler>) -> Self {
        Self {
            namespace: Namespace::new(),
            compiler,
            global_exports: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Construct a new instance of `Context` with the given target.
    pub fn with_isa(isa: Box<TargetIsa>) -> Self {
        Self::new(Box::new(Compiler::new(isa)))
    }

    fn validate(&mut self, data: &[u8]) -> Result<(), String> {
        let config = ValidatingParserConfig {
            operator_config: OperatorValidatorConfig {
                enable_threads: false,
                enable_reference_types: false,
                enable_bulk_memory: false,
                enable_simd: false,
            },
            mutable_global_imports: true,
        };

        // TODO: Fix Cranelift to be able to perform validation itself, rather
        // than calling into wasmparser ourselves here.
        if validate(data, Some(config)) {
            Ok(())
        } else {
            // TODO: Work with wasmparser to get better error messages.
            Err("module did not validate".to_owned())
        }
    }

    fn instantiate(&mut self, data: &[u8]) -> Result<InstanceHandle, SetupError> {
        self.validate(&data).map_err(SetupError::Validate)?;

        instantiate(
            &mut *self.compiler,
            &data,
            &mut self.namespace,
            Rc::clone(&self.global_exports),
        )
    }

    /// Return the instance associated with the given name.
    pub fn get_instance(
        &mut self,
        instance_name: &str,
    ) -> Result<&mut InstanceHandle, UnknownInstance> {
        self.namespace
            .get_instance(instance_name)
            .ok_or_else(|| UnknownInstance {
                instance_name: instance_name.to_string(),
            })
    }

    /// Instantiate a module instance and register the instance.
    pub fn instantiate_module(
        &mut self,
        instance_name: Option<String>,
        data: &[u8],
    ) -> Result<InstanceHandle, ActionError> {
        let instance = self.instantiate(data).map_err(ActionError::Setup)?;
        self.optionally_name_instance(instance_name, instance.clone());
        Ok(instance)
    }

    /// If `name` isn't None, register it for the given instance.
    pub fn optionally_name_instance(&mut self, name: Option<String>, instance: InstanceHandle) {
        if let Some(name) = name {
            self.namespace.name_instance(name, instance);
        }
    }

    /// Register a name for the given instance.
    pub fn name_instance(&mut self, name: String, instance: InstanceHandle) {
        self.namespace.name_instance(name, instance);
    }

    /// Register an additional name for an existing registered instance.
    pub fn alias(&mut self, name: &str, as_name: String) -> Result<(), UnknownInstance> {
        let instance = self.get_instance(&name)?.clone();
        self.name_instance(as_name, instance);
        Ok(())
    }

    /// Invoke an exported function from a named instance.
    pub fn invoke_named(
        &mut self,
        instance_name: &str,
        field: &str,
        args: &[RuntimeValue],
    ) -> Result<ActionOutcome, ContextError> {
        let mut instance = self
            .get_instance(&instance_name)
            .map_err(ContextError::Instance)?
            .clone();
        self.invoke(&mut instance, field, args)
            .map_err(ContextError::Action)
    }

    /// Invoke an exported function from an instance.
    pub fn invoke(
        &mut self,
        instance: &mut InstanceHandle,
        field: &str,
        args: &[RuntimeValue],
    ) -> Result<ActionOutcome, ActionError> {
        invoke(&mut *self.compiler, instance, field, &args)
    }

    /// Get the value of an exported global variable from an instance.
    pub fn get_named(
        &mut self,
        instance_name: &str,
        field: &str,
    ) -> Result<ActionOutcome, ContextError> {
        let instance = self
            .get_instance(&instance_name)
            .map_err(ContextError::Instance)?
            .clone();
        self.get(&instance, field).map_err(ContextError::Action)
    }

    /// Get the value of an exported global variable from an instance.
    pub fn get(
        &mut self,
        instance: &InstanceHandle,
        field: &str,
    ) -> Result<ActionOutcome, ActionError> {
        get(instance, field).map(|value| ActionOutcome::Returned {
            values: vec![value],
        })
    }

    /// Get a slice of memory from an instance.
    pub fn inspect_memory<'instance>(
        &self,
        instance: &'instance InstanceHandle,
        field_name: &str,
        start: usize,
        len: usize,
    ) -> Result<&'instance [u8], ActionError> {
        inspect_memory(instance, field_name, start, len)
    }
}
