use cranelift_codegen::isa;
use std::str;
use wasmtime_environ::{Compilation, Module, ModuleEnvironment, Tunables};
use {compile_and_link_module, finish_instantiation, invoke, Code, Instance, InvokeOutcome, Value};

/// A module, an instance of that module, and accompanying compilation artifacts.
///
/// TODO: Rename and reorganize this.
pub struct InstanceWorld {
    module: Module,
    instance: Instance,
    compilation: Compilation,
}

impl InstanceWorld {
    /// Create a new `InstanceWorld` by compiling the wasm module in `data` and instatiating it.
    pub fn new(code: &mut Code, isa: &isa::TargetIsa, data: &[u8]) -> Result<Self, String> {
        let mut module = Module::new();
        let tunables = Tunables::default();
        let (instance, compilation) = {
            let translation = {
                let environ = ModuleEnvironment::new(isa, &mut module, tunables);

                environ.translate(&data).map_err(|e| e.to_string())?
            };

            let imports_resolver = |_env: &str, _function: &str| None;

            let compilation = compile_and_link_module(isa, &translation, &imports_resolver)?;
            let mut instance = Instance::new(
                translation.module,
                &compilation,
                &translation.lazy.data_initializers,
            )?;

            finish_instantiation(code, isa, &translation.module, &compilation, &mut instance)?;

            (instance, compilation)
        };

        Ok(Self {
            module,
            instance,
            compilation,
        })
    }

    /// Invoke a function in this `InstanceWorld` by name.
    pub fn invoke(
        &mut self,
        code: &mut Code,
        isa: &isa::TargetIsa,
        function_name: &str,
        args: &[Value],
    ) -> Result<InvokeOutcome, String> {
        invoke(
            code,
            isa,
            &self.module,
            &self.compilation,
            self.instance.vmctx(),
            &function_name,
            args,
        )
        .map_err(|e| e.to_string())
    }
}
