use crate::action::{get, inspect_memory, invoke};
use crate::{
    instantiate, ActionError, ActionOutcome, CompilationStrategy, CompiledModule, Compiler,
    InstanceHandle, Namespace, RuntimeValue, SetupError,
};
use thiserror::Error;
use wasmparser::{validate, OperatorValidatorConfig, ValidatingParserConfig};
use wasmtime_environ::isa::TargetIsa;

/// Indicates an unknown instance was specified.
#[derive(Error, Debug)]
#[error("no instance {instance_name} present")]
pub struct UnknownInstance {
    instance_name: String,
}

/// Error message used by `WastContext`.
#[derive(Error, Debug)]
pub enum ContextError {
    /// An unknown instance name was used.
    #[error("An error occured due to an unknown instance being specified")]
    Instance(#[from] UnknownInstance),
    /// An error occured while performing an action.
    #[error("An error occurred while performing an action")]
    Action(#[from] ActionError),
}

/// The collection of features configurable during compilation
#[derive(Clone, Default)]
pub struct Features {
    /// marks whether the proposed thread feature is enabled or disabled
    pub threads: bool,
    /// marks whether the proposed reference type feature is enabled or disabled
    pub reference_types: bool,
    /// marks whether the proposed SIMD feature is enabled or disabled
    pub simd: bool,
    /// marks whether the proposed bulk memory feature is enabled or disabled
    pub bulk_memory: bool,
    /// marks whether the proposed multi-value feature is enabled or disabled
    pub multi_value: bool,
}

impl Into<ValidatingParserConfig> for Features {
    fn into(self) -> ValidatingParserConfig {
        ValidatingParserConfig {
            operator_config: OperatorValidatorConfig {
                enable_threads: self.threads,
                enable_reference_types: self.reference_types,
                enable_bulk_memory: self.bulk_memory,
                enable_simd: self.simd,
                enable_multi_value: self.multi_value,
            },
        }
    }
}

/// A convenient context for compiling and executing WebAssembly instances.
pub struct Context {
    namespace: Namespace,
    compiler: Box<Compiler>,
    debug_info: bool,
    features: Features,
}

impl Context {
    /// Construct a new instance of `Context`.
    pub fn new(compiler: Box<Compiler>) -> Self {
        Self {
            namespace: Namespace::new(),
            compiler,
            debug_info: false,
            features: Default::default(),
        }
    }

    /// Get debug_info settings.
    pub fn debug_info(&self) -> bool {
        self.debug_info
    }

    /// Set debug_info settings.
    pub fn set_debug_info(&mut self, value: bool) {
        self.debug_info = value;
    }

    /// Construct a new instance of `Context` with the given target.
    pub fn with_isa(isa: Box<dyn TargetIsa>, strategy: CompilationStrategy) -> Self {
        Self::new(Box::new(Compiler::new(isa, strategy)))
    }

    /// Retrieve the context features
    pub fn features(&self) -> &Features {
        &self.features
    }

    /// Construct a new instance with the given features from the current `Context`
    pub fn with_features(self, features: Features) -> Self {
        Self { features, ..self }
    }

    fn validate(&mut self, data: &[u8]) -> Result<(), String> {
        // TODO: Fix Cranelift to be able to perform validation itself, rather
        // than calling into wasmparser ourselves here.
        validate(data, Some(self.features.clone().into()))
            .map_err(|e| format!("module did not validate: {}", e.to_string()))
    }

    fn instantiate(&mut self, data: &[u8]) -> Result<InstanceHandle, SetupError> {
        self.validate(&data).map_err(SetupError::Validate)?;
        let debug_info = self.debug_info();

        instantiate(
            &mut *self.compiler,
            &data,
            None,
            &mut self.namespace,
            debug_info,
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

    /// Compile a module.
    pub fn compile_module(&mut self, data: &[u8]) -> Result<CompiledModule, SetupError> {
        self.validate(&data).map_err(SetupError::Validate)?;
        let debug_info = self.debug_info();

        CompiledModule::new(&mut *self.compiler, data, None, debug_info)
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
