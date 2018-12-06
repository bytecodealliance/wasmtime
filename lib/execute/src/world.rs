use cranelift_codegen::isa;
use cranelift_wasm::{GlobalIndex, MemoryIndex};
use export::Resolver;
use std::str;
use vmcontext::VMGlobal;
use wasmtime_environ::{Compilation, Module, ModuleEnvironment, Tunables};
use {
    compile_and_link_module, finish_instantiation, get, invoke, Code, Instance, InvokeOutcome,
    Value,
};

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
    pub fn new(
        code: &mut Code,
        isa: &isa::TargetIsa,
        data: &[u8],
        resolver: &mut Resolver,
    ) -> Result<Self, String> {
        let mut module = Module::new();
        // TODO: Allow the tunables to be overridden.
        let tunables = Tunables::default();
        let (instance, compilation) = {
            let translation = {
                let environ = ModuleEnvironment::new(isa, &mut module, tunables);

                environ.translate(&data).map_err(|e| e.to_string())?
            };

            let compilation = compile_and_link_module(isa, &translation, resolver)?;
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

    /// Read a global in this `InstanceWorld` by name.
    pub fn get(&mut self, global_name: &str) -> Result<Value, String> {
        get(&self.module, self.instance.vmctx(), global_name).map_err(|e| e.to_string())
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
