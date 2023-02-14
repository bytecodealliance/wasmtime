use anyhow::Result;
use object::write::{Object, SymbolId};
use std::any::Any;
use wasmtime_cranelift_shared::LinkOptions;
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FuncIndex, FunctionBodyData, FunctionLoc, ModuleTranslation,
    ModuleTypes, PrimaryMap, Tunables, WasmFunctionInfo,
};
use winch_codegen::TargetIsa;

pub(crate) struct Compiler {
    isa: Box<dyn TargetIsa>,
    linkopts: LinkOptions,
}

impl Compiler {
    pub fn new(isa: Box<dyn TargetIsa>, linkopts: LinkOptions) -> Self {
        Self { isa, linkopts }
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
    ) -> Result<(WasmFunctionInfo, Box<dyn Any + Send>), CompileError> {
        todo!()
    }

    fn compile_host_to_wasm_trampoline(
        &self,
        _ty: &wasmtime_environ::WasmFuncType,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        todo!()
    }

    fn append_code(
        &self,
        _obj: &mut Object<'static>,
        _funcs: &[(String, Box<dyn Any + Send>)],
        _tunables: &Tunables,
        _resolve_reloc: &dyn Fn(usize, FuncIndex) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>> {
        drop(&self.linkopts);
        assert!(_funcs.is_empty());
        Ok(Vec::new())
    }

    fn emit_trampoline_obj(
        &self,
        _ty: &wasmtime_environ::WasmFuncType,
        _host_fn: usize,
        _obj: &mut wasmtime_environ::object::write::Object<'static>,
    ) -> Result<(FunctionLoc, FunctionLoc)> {
        todo!()
    }

    fn triple(&self) -> &target_lexicon::Triple {
        self.isa.triple()
    }

    fn flags(&self) -> std::collections::BTreeMap<String, wasmtime_environ::FlagValue> {
        wasmtime_cranelift_shared::clif_flags_to_wasmtime(self.isa.flags().iter())
    }

    fn isa_flags(&self) -> std::collections::BTreeMap<String, wasmtime_environ::FlagValue> {
        wasmtime_cranelift_shared::clif_flags_to_wasmtime(self.isa.isa_flags())
    }

    fn is_branch_protection_enabled(&self) -> bool {
        todo!()
    }

    #[cfg(feature = "component-model")]
    fn component_compiler(&self) -> &dyn wasmtime_environ::component::ComponentCompiler {
        todo!()
    }

    fn append_dwarf(
        &self,
        _obj: &mut Object<'_>,
        _translation: &ModuleTranslation<'_>,
        _funcs: &PrimaryMap<DefinedFuncIndex, (SymbolId, &(dyn Any + Send))>,
    ) -> Result<()> {
        todo!()
    }
}
