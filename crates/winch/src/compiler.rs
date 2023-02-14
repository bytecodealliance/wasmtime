use anyhow::Result;
use object::write::{Object, SymbolId};
use std::any::Any;
use std::sync::Mutex;
use wasmparser::FuncValidatorAllocations;
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FilePos, FuncIndex, FunctionBodyData, FunctionLoc,
    ModuleTranslation, ModuleTypes, PrimaryMap, Tunables, WasmFunctionInfo,
};
use winch_codegen::TargetIsa;

pub(crate) struct Compiler {
    isa: Box<dyn TargetIsa>,
    allocations: Mutex<Vec<FuncValidatorAllocations>>,
}

impl Compiler {
    pub fn new(isa: Box<dyn TargetIsa>) -> Self {
        Self {
            isa,
            allocations: Mutex::new(Vec::new()),
        }
    }

    fn take_allocations(&self) -> FuncValidatorAllocations {
        self.allocations
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(Default::default)
    }

    fn save_allocations(&self, allocs: FuncValidatorAllocations) {
        self.allocations.lock().unwrap().push(allocs)
    }
}

impl wasmtime_environ::Compiler for Compiler {
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        index: DefinedFuncIndex,
        data: FunctionBodyData<'_>,
        tunables: &Tunables,
        _types: &ModuleTypes,
    ) -> Result<(WasmFunctionInfo, Box<dyn Any + Send>), CompileError> {
        let index = translation.module.func_index(index);
        let sig = translation.get_types().function_at(index.as_u32()).unwrap();
        let FunctionBodyData { body, validator } = data;
        let start_srcloc = FilePos::new(
            body.get_binary_reader()
                .original_position()
                .try_into()
                .unwrap(),
        );
        let mut validator = validator.into_validator(self.take_allocations());
        let buffer = self
            .isa
            .compile_function(&sig, &body, &mut validator)
            .map_err(|e| CompileError::Codegen(format!("{e:?}")));
        self.save_allocations(validator.into_allocations());
        let buffer = buffer?;

        // TODO: this should probably get plumbed into winch
        drop(tunables);

        Ok((
            WasmFunctionInfo {
                start_srcloc,
                stack_maps: Box::new([]),
            },
            Box::new(buffer),
        ))
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
        self.isa.is_branch_protection_enabled()
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

    fn function_alignment(&self) -> u32 {
        self.isa.function_alignment()
    }

    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        self.isa.create_systemv_cie()
    }
}
