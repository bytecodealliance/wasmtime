use action::{ActionError, ActionOutcome, RuntimeValue};
use code::Code;
use cranelift_codegen::isa;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, GlobalIndex, MemoryIndex};
use export::Resolver;
use get::get;
use instance::Instance;
use invoke::{invoke, invoke_start_function};
use link::link_module;
use std::str;
use vmcontext::VMGlobal;
use wasmtime_environ::{
    compile_module, Compilation, CompileError, Module, ModuleEnvironment, Tunables,
};

/// A module, an instance of that module, and accompanying compilation artifacts.
///
/// TODO: Rename and reorganize this.
pub struct InstanceWorld {
    module: Module,
    instance: Instance,
}

impl InstanceWorld {
    /// Create a new `InstanceWorld` by compiling the wasm module in `data` and instatiating it.
    pub fn new(
        code: &mut Code,
        isa: &isa::TargetIsa,
        data: &[u8],
        resolver: &mut Resolver,
    ) -> Result<Self, ActionError> {
        let mut module = Module::new();
        // TODO: Allow the tunables to be overridden.
        let tunables = Tunables::default();
        let instance = {
            // TODO: Untie this.
            let ((mut compilation, relocations), lazy_data_initializers) = {
                let (lazy_function_body_inputs, lazy_data_initializers) = {
                    let environ = ModuleEnvironment::new(isa, &mut module, tunables);

                    let translation = environ
                        .translate(&data)
                        .map_err(|error| ActionError::Compile(CompileError::Wasm(error)))?;

                    (
                        translation.lazy.function_body_inputs,
                        translation.lazy.data_initializers,
                    )
                };

                (
                    compile_module(&module, &lazy_function_body_inputs, isa)
                        .map_err(ActionError::Compile)?,
                    lazy_data_initializers,
                )
            };

            let allocated_functions =
                allocate_functions(code, compilation).map_err(ActionError::Resource)?;

            let resolved = link_module(&module, &allocated_functions, relocations, resolver)
                .map_err(ActionError::Link)?;

            let mut instance = Instance::new(
                &module,
                allocated_functions,
                &lazy_data_initializers,
                resolved,
            )
            .map_err(ActionError::Resource)?;

            // The WebAssembly spec specifies that the start function is
            // invoked automatically at instantiation time.
            match invoke_start_function(code, isa, &module, &mut instance)? {
                ActionOutcome::Returned { .. } => {}
                ActionOutcome::Trapped { message } => {
                    // Instantiation fails if the start function traps.
                    return Err(ActionError::Start(message));
                }
            }

            instance
        };

        Ok(Self { module, instance })
    }

    /// Invoke a function in this `InstanceWorld` by name.
    pub fn invoke(
        &mut self,
        code: &mut Code,
        isa: &isa::TargetIsa,
        function_name: &str,
        args: &[RuntimeValue],
    ) -> Result<ActionOutcome, ActionError> {
        invoke(
            code,
            isa,
            &self.module,
            &mut self.instance,
            &function_name,
            args,
        )
    }

    /// Read a global in this `InstanceWorld` by name.
    pub fn get(&mut self, global_name: &str) -> Result<RuntimeValue, ActionError> {
        get(&self.module, &mut self.instance, global_name)
    }

    /// Returns a slice of the contents of allocated linear memory.
    pub fn inspect_memory(&self, memory_index: MemoryIndex, address: usize, len: usize) -> &[u8] {
        self.instance.inspect_memory(memory_index, address, len)
    }

    /// Shows the value of a global variable.
    pub fn inspect_global(&self, global_index: GlobalIndex) -> &VMGlobal {
        self.instance.inspect_global(global_index)
    }
}

fn allocate_functions(
    code: &mut Code,
    compilation: Compilation,
) -> Result<PrimaryMap<DefinedFuncIndex, (*mut u8, usize)>, String> {
    let mut result = PrimaryMap::with_capacity(compilation.functions.len());
    for (_, body) in compilation.functions.into_iter() {
        let slice = code.allocate_copy_of_slice(&body)?;
        result.push((slice.as_mut_ptr(), slice.len()));
    }
    Ok(result)
}
