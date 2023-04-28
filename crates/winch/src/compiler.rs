use anyhow::Result;
use cranelift_codegen::{Final, MachBufferFinalized};
use object::write::{Object, SymbolId};
use std::any::Any;
use std::sync::Mutex;
use wasmparser::FuncValidatorAllocations;
use wasmtime_cranelift_shared::obj::ModuleTextBuilder;
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FilePos, FuncIndex, FunctionBodyData, FunctionLoc,
    ModuleTranslation, ModuleTypes, PrimaryMap, Tunables, WasmFunctionInfo,
};
use winch_codegen::TargetIsa;
use winch_environ::FuncEnv;

pub(crate) struct Compiler {
    isa: Box<dyn TargetIsa>,
    allocations: Mutex<Vec<FuncValidatorAllocations>>,
}

struct CompiledFunction(MachBufferFinalized<Final>);

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
        _tunables: &Tunables,
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
        let env = FuncEnv::new(&translation.module, translation.get_types(), &self.isa);
        let buffer = self
            .isa
            .compile_function(&sig, &body, &env, &mut validator)
            .map_err(|e| CompileError::Codegen(format!("{e:?}")));
        self.save_allocations(validator.into_allocations());
        let buffer = buffer?;

        Ok((
            WasmFunctionInfo {
                start_srcloc,
                stack_maps: Box::new([]),
            },
            Box::new(CompiledFunction(buffer)),
        ))
    }

    fn compile_array_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypes,
        index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let _ = (translation, types, index);
        todo!()
    }

    fn compile_native_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypes,
        index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let _ = (translation, types, index);
        todo!()
    }

    fn compile_wasm_to_native_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        wasm_func_ty: &wasmtime_environ::WasmFuncType,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let _ = (translation, wasm_func_ty);
        todo!()
    }

    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send>)],
        _tunables: &Tunables,
        resolve_reloc: &dyn Fn(usize, FuncIndex) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>> {
        let mut builder =
            ModuleTextBuilder::new(obj, self, self.isa.text_section_builder(funcs.len()));

        let mut ret = Vec::with_capacity(funcs.len());
        for (i, (sym, func)) in funcs.iter().enumerate() {
            let func = &func.downcast_ref::<CompiledFunction>().unwrap().0;

            // TODO: Implement copying over this data into the
            // `ModuleTextBuilder` type. Note that this should probably be
            // deduplicated with the cranelift implementation in the long run.
            assert!(func.relocs().is_empty());
            assert!(func.traps().is_empty());
            assert!(func.stack_maps().is_empty());

            let (sym, range) = builder.append_func(
                &sym,
                func.data(),
                self.isa.function_alignment(),
                None,
                &[],
                |idx| resolve_reloc(i, idx),
            );

            let info = FunctionLoc {
                start: u32::try_from(range.start).unwrap(),
                length: u32::try_from(range.end - range.start).unwrap(),
            };
            ret.push((sym, info));
        }
        builder.finish();
        Ok(ret)
    }

    fn emit_trampolines_for_array_call_host_func(
        &self,
        ty: &wasmtime_environ::WasmFuncType,
        // Actually `host_fn: VMArrayCallFunction` but that type is not
        // available in `wasmtime-environ`.
        host_fn: usize,
        obj: &mut Object<'static>,
    ) -> Result<(FunctionLoc, FunctionLoc)> {
        drop((ty, host_fn, obj));
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

    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        self.isa.create_systemv_cie()
    }
}
