use anyhow::Result;
use cranelift_codegen::isa::unwind::UnwindInfoKind;
use object::write::{Object, SymbolId};
use std::any::Any;
use std::mem;
use std::sync::Mutex;
use wasmparser::FuncValidatorAllocations;
use wasmtime_cranelift::CompiledFunction;
use wasmtime_environ::{
    BuiltinFunctionIndex, CompileError, CompiledFunctionBody, DefinedFuncIndex, FunctionBodyData,
    FunctionLoc, ModuleTranslation, ModuleTypesBuilder, PrimaryMap, RelocationTarget,
    StaticModuleIndex, Tunables, VMOffsets,
};
use winch_codegen::{BuiltinFunctions, CallingConvention, TargetIsa};

/// Function compilation context.
/// This struct holds information that can be shared globally across
/// all function compilations.
struct CompilationContext {
    /// Validator allocations.
    allocations: FuncValidatorAllocations,
    /// Builtin functions available to JIT code.
    builtins: BuiltinFunctions,
}

pub(crate) struct Compiler {
    isa: Box<dyn TargetIsa>,
    trampolines: Box<dyn wasmtime_environ::Compiler>,
    contexts: Mutex<Vec<CompilationContext>>,
    tunables: Tunables,
}

impl Compiler {
    pub fn new(
        isa: Box<dyn TargetIsa>,
        trampolines: Box<dyn wasmtime_environ::Compiler>,
        tunables: Tunables,
    ) -> Self {
        Self {
            isa,
            trampolines,
            contexts: Mutex::new(Vec::new()),
            tunables,
        }
    }

    /// Get a compilation context or create a new one if none available.
    fn get_context(&self, translation: &ModuleTranslation) -> CompilationContext {
        self.contexts.lock().unwrap().pop().unwrap_or_else(|| {
            let pointer_size = self.isa.pointer_bytes();
            let vmoffsets = VMOffsets::new(pointer_size, &translation.module);
            CompilationContext {
                allocations: Default::default(),
                builtins: BuiltinFunctions::new(
                    &vmoffsets,
                    self.isa.wasmtime_call_conv(),
                    CallingConvention::Default,
                ),
            }
        })
    }

    /// Save a compilation context.
    fn save_context(&self, mut context: CompilationContext, allocs: FuncValidatorAllocations) {
        context.allocations = allocs;
        self.contexts.lock().unwrap().push(context);
    }

    /// Emit unwind info into the [`CompiledFunction`].
    fn emit_unwind_info(
        &self,
        compiled_function: &mut CompiledFunction,
    ) -> Result<(), CompileError> {
        let kind = match self.isa.triple().operating_system {
            target_lexicon::OperatingSystem::Windows => UnwindInfoKind::Windows,
            _ => UnwindInfoKind::SystemV,
        };

        if let Some(info) = self
            .isa
            .emit_unwind_info(&compiled_function.buffer, kind)
            .map_err(|e| CompileError::Codegen(format!("{e:?}")))?
        {
            compiled_function.set_unwind_info(info);
        }

        Ok(())
    }
}

impl wasmtime_environ::Compiler for Compiler {
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        index: DefinedFuncIndex,
        data: FunctionBodyData<'_>,
        types: &ModuleTypesBuilder,
        _symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        let index = translation.module.func_index(index);
        let sig = translation.module.functions[index]
            .signature
            .unwrap_module_type_index();
        let ty = types[sig].unwrap_func();
        let FunctionBodyData {
            body, validator, ..
        } = data;
        let mut context = self.get_context(translation);
        let mut validator = validator.into_validator(mem::take(&mut context.allocations));
        let func = self
            .isa
            .compile_function(
                ty,
                &body,
                translation,
                types,
                &mut context.builtins,
                &mut validator,
                &self.tunables,
            )
            .map_err(|e| CompileError::Codegen(format!("{e:?}")));
        self.save_context(context, validator.into_allocations());
        let mut func = func?;

        let reader = body.get_binary_reader();
        func.set_address_map(
            reader.original_position() as u32,
            reader.bytes_remaining() as u32,
            self.tunables.generate_address_map,
        );

        if self.isa.flags().unwind_info() {
            self.emit_unwind_info(&mut func)?;
        }

        Ok(CompiledFunctionBody {
            code: Box::new(func),
            // TODO: Winch doesn't support GC objects and stack maps and all that yet.
            needs_gc_heap: false,
        })
    }

    fn compile_array_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        index: DefinedFuncIndex,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        self.trampolines
            .compile_array_to_wasm_trampoline(translation, types, index, symbol)
    }

    fn compile_wasm_to_array_trampoline(
        &self,
        wasm_func_ty: &wasmtime_environ::WasmFuncType,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        self.trampolines
            .compile_wasm_to_array_trampoline(wasm_func_ty, symbol)
    }

    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send>)],
        resolve_reloc: &dyn Fn(usize, wasmtime_environ::RelocationTarget) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>> {
        self.trampolines.append_code(obj, funcs, resolve_reloc)
    }

    fn triple(&self) -> &target_lexicon::Triple {
        self.isa.triple()
    }

    fn flags(&self) -> Vec<(&'static str, wasmtime_environ::FlagValue<'static>)> {
        wasmtime_cranelift::clif_flags_to_wasmtime(self.isa.flags().iter())
    }

    fn isa_flags(&self) -> Vec<(&'static str, wasmtime_environ::FlagValue<'static>)> {
        wasmtime_cranelift::clif_flags_to_wasmtime(self.isa.isa_flags())
    }

    fn is_branch_protection_enabled(&self) -> bool {
        self.isa.is_branch_protection_enabled()
    }

    #[cfg(feature = "component-model")]
    fn component_compiler(&self) -> &dyn wasmtime_environ::component::ComponentCompiler {
        self.trampolines.component_compiler()
    }

    fn append_dwarf<'a>(
        &self,
        _obj: &mut Object<'_>,
        _translations: &'a PrimaryMap<StaticModuleIndex, ModuleTranslation<'a>>,
        _get_func: &'a dyn Fn(
            StaticModuleIndex,
            DefinedFuncIndex,
        ) -> (SymbolId, &'a (dyn Any + Send)),
        _dwarf_package_bytes: Option<&'a [u8]>,
        _tunables: &'a Tunables,
    ) -> Result<()> {
        todo!()
    }

    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        self.isa.create_systemv_cie()
    }

    fn compile_wasm_to_builtin(
        &self,
        index: BuiltinFunctionIndex,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        self.trampolines.compile_wasm_to_builtin(index, symbol)
    }

    fn compiled_function_relocation_targets<'a>(
        &'a self,
        func: &'a dyn Any,
    ) -> Box<dyn Iterator<Item = RelocationTarget> + 'a> {
        self.trampolines.compiled_function_relocation_targets(func)
    }
}
