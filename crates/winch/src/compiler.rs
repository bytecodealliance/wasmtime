use anyhow::Result;
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FunctionBodyData, ModuleTranslation, ModuleTypes, Tunables,
};
use winch_codegen::isa::TargetIsa;

pub(crate) struct Compiler {
    isa: Box<dyn TargetIsa>,
}

impl Compiler {
    pub fn new(isa: Box<dyn TargetIsa>) -> Self {
        Self { isa }
    }
}

impl wasmtime_environ::Compiler for Compiler {
    fn compile_function(
        &self,
        _translation: &ModuleTranslation<'_>,
        _index: DefinedFuncIndex,
        _data: FunctionBodyData<'_>,
        _tunables: &Tunables,
        _types: &ModuleTypes,
    ) -> Result<Box<dyn std::any::Any + Send>, CompileError> {
        todo!()
    }

    fn compile_host_to_wasm_trampoline(
        &self,
        _ty: &wasmtime_environ::WasmFuncType,
    ) -> Result<Box<dyn std::any::Any + Send>, CompileError> {
        todo!()
    }

    fn emit_obj(
        &self,
        _module: &ModuleTranslation,
        _funcs: wasmtime_environ::PrimaryMap<DefinedFuncIndex, Box<dyn std::any::Any + Send>>,
        _trampolines: Vec<Box<dyn std::any::Any + Send>>,
        _tunables: &Tunables,
        _obj: &mut wasmtime_environ::object::write::Object<'static>,
    ) -> Result<(
        wasmtime_environ::PrimaryMap<DefinedFuncIndex, wasmtime_environ::FunctionInfo>,
        Vec<wasmtime_environ::Trampoline>,
    )> {
        todo!()
    }

    fn emit_trampoline_obj(
        &self,
        _ty: &wasmtime_environ::WasmFuncType,
        _host_fn: usize,
        _obj: &mut wasmtime_environ::object::write::Object<'static>,
    ) -> Result<(wasmtime_environ::Trampoline, wasmtime_environ::Trampoline)> {
        todo!()
    }

    fn triple(&self) -> &target_lexicon::Triple {
        self.isa.triple()
    }

    fn page_size_align(&self) -> u64 {
        todo!()
    }

    fn flags(&self) -> std::collections::BTreeMap<String, wasmtime_environ::FlagValue> {
        todo!()
    }

    fn isa_flags(&self) -> std::collections::BTreeMap<String, wasmtime_environ::FlagValue> {
        todo!()
    }

    fn is_branch_protection_enabled(&self) -> bool {
        todo!()
    }

    #[cfg(feature = "component-model")]
    fn component_compiler(&self) -> &dyn wasmtime_environ::component::ComponentCompiler {
        todo!()
    }
}
