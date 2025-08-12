use anyhow::Result;
use cranelift_codegen::isa::unwind::UnwindInfoKind;
use object::write::{Object, SymbolId};
use std::any::Any;
use std::mem;
use std::sync::Mutex;
use wasmparser::FuncValidatorAllocations;
use wasmtime_cranelift::CompiledFunction;
use wasmtime_environ::{
    CompileError, CompiledFunctionBody, DefinedFuncIndex, FuncKey, FunctionBodyData, FunctionLoc,
    ModuleTranslation, ModuleTypesBuilder, PrimaryMap, StaticModuleIndex, Tunables, VMOffsets,
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
    trampolines: NoInlineCompiler,
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
            trampolines: NoInlineCompiler(trampolines),
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

fn box_dyn_any_compiled_function(f: CompiledFunction) -> Box<dyn Any + Send + Sync> {
    let b = box_dyn_any(f);
    debug_assert!(b.is::<CompiledFunction>());
    b
}

fn box_dyn_any(x: impl Any + Send + Sync) -> Box<dyn Any + Send + Sync> {
    log::trace!(
        "making Box<dyn Any + Send + Sync> of {}",
        std::any::type_name_of_val(&x)
    );
    let b = Box::new(x);
    let r: &(dyn Any + Sync + Send) = &*b;
    log::trace!("  --> {r:#p}");
    b
}

impl wasmtime_environ::Compiler for Compiler {
    fn inlining_compiler(&self) -> Option<&dyn wasmtime_environ::InliningCompiler> {
        None
    }

    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        key: FuncKey,
        data: FunctionBodyData<'_>,
        types: &ModuleTypesBuilder,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        log::trace!("compiling function: {key:?} = {symbol:?}");

        let (module_index, def_func_index) = key.unwrap_defined_wasm_function();
        debug_assert_eq!(module_index, translation.module_index);

        let index = translation.module.func_index(def_func_index);
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
            code: box_dyn_any_compiled_function(func),
            // TODO: Winch doesn't support GC objects and stack maps and all that yet.
            needs_gc_heap: false,
        })
    }

    fn compile_array_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        key: FuncKey,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        self.trampolines
            .compile_array_to_wasm_trampoline(translation, types, key, symbol)
    }

    fn compile_wasm_to_array_trampoline(
        &self,
        wasm_func_ty: &wasmtime_environ::WasmFuncType,
        key: FuncKey,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        self.trampolines
            .compile_wasm_to_array_trampoline(wasm_func_ty, key, symbol)
    }

    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send + Sync>)],
        resolve_reloc: &dyn Fn(usize, wasmtime_environ::FuncKey) -> usize,
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
        ) -> (SymbolId, &'a (dyn Any + Send + Sync)),
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
        key: FuncKey,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        self.trampolines.compile_wasm_to_builtin(key, symbol)
    }

    fn compiled_function_relocation_targets<'a>(
        &'a self,
        func: &'a dyn Any,
    ) -> Box<dyn Iterator<Item = FuncKey> + 'a> {
        self.trampolines.compiled_function_relocation_targets(func)
    }
}

/// A wrapper around another `Compiler` implementation that may or may not be an
/// inlining compiler and turns it into a non-inlining compiler.
struct NoInlineCompiler(Box<dyn wasmtime_environ::Compiler>);

impl wasmtime_environ::Compiler for NoInlineCompiler {
    fn inlining_compiler(&self) -> Option<&dyn wasmtime_environ::InliningCompiler> {
        None
    }

    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        key: FuncKey,
        data: FunctionBodyData<'_>,
        types: &ModuleTypesBuilder,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        let input = data.body.clone();
        let mut body = self
            .0
            .compile_function(translation, key, data, types, symbol)?;
        if let Some(c) = self.0.inlining_compiler() {
            c.finish_compiling(&mut body, Some(input), symbol)
                .map_err(|e| CompileError::Codegen(e.to_string()))?;
        }
        Ok(body)
    }

    fn compile_array_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        key: FuncKey,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        let mut body = self
            .0
            .compile_array_to_wasm_trampoline(translation, types, key, symbol)?;
        if let Some(c) = self.0.inlining_compiler() {
            c.finish_compiling(&mut body, None, symbol)
                .map_err(|e| CompileError::Codegen(e.to_string()))?;
        }
        Ok(body)
    }

    fn compile_wasm_to_array_trampoline(
        &self,
        wasm_func_ty: &wasmtime_environ::WasmFuncType,
        key: FuncKey,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        let mut body = self
            .0
            .compile_wasm_to_array_trampoline(wasm_func_ty, key, symbol)?;
        if let Some(c) = self.0.inlining_compiler() {
            c.finish_compiling(&mut body, None, symbol)
                .map_err(|e| CompileError::Codegen(e.to_string()))?;
        }
        Ok(body)
    }

    fn compile_wasm_to_builtin(
        &self,
        key: FuncKey,
        symbol: &str,
    ) -> Result<CompiledFunctionBody, CompileError> {
        let mut body = self.0.compile_wasm_to_builtin(key, symbol)?;
        if let Some(c) = self.0.inlining_compiler() {
            c.finish_compiling(&mut body, None, symbol)
                .map_err(|e| CompileError::Codegen(e.to_string()))?;
        }
        Ok(body)
    }

    fn compiled_function_relocation_targets<'a>(
        &'a self,
        func: &'a dyn Any,
    ) -> Box<dyn Iterator<Item = FuncKey> + 'a> {
        self.0.compiled_function_relocation_targets(func)
    }

    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send + Sync>)],
        resolve_reloc: &dyn Fn(usize, FuncKey) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>> {
        self.0.append_code(obj, funcs, resolve_reloc)
    }

    fn triple(&self) -> &target_lexicon::Triple {
        self.0.triple()
    }

    fn flags(&self) -> Vec<(&'static str, wasmtime_environ::FlagValue<'static>)> {
        self.0.flags()
    }

    fn isa_flags(&self) -> Vec<(&'static str, wasmtime_environ::FlagValue<'static>)> {
        self.0.isa_flags()
    }

    fn is_branch_protection_enabled(&self) -> bool {
        self.0.is_branch_protection_enabled()
    }

    #[cfg(feature = "component-model")]
    fn component_compiler(&self) -> &dyn wasmtime_environ::component::ComponentCompiler {
        self
    }

    fn append_dwarf<'a>(
        &self,
        obj: &mut Object<'_>,
        translations: &'a PrimaryMap<StaticModuleIndex, ModuleTranslation<'a>>,
        get_func: &'a dyn Fn(
            StaticModuleIndex,
            DefinedFuncIndex,
        ) -> (SymbolId, &'a (dyn Any + Send + Sync)),
        dwarf_package_bytes: Option<&'a [u8]>,
        tunables: &'a Tunables,
    ) -> Result<()> {
        self.0
            .append_dwarf(obj, translations, get_func, dwarf_package_bytes, tunables)
    }
}

#[cfg(feature = "component-model")]
impl wasmtime_environ::component::ComponentCompiler for NoInlineCompiler {
    fn compile_trampoline(
        &self,
        component: &wasmtime_environ::component::ComponentTranslation,
        types: &wasmtime_environ::component::ComponentTypesBuilder,
        key: FuncKey,
        tunables: &Tunables,
        symbol: &str,
    ) -> Result<wasmtime_environ::component::AllCallFunc<CompiledFunctionBody>> {
        let mut body = self
            .0
            .component_compiler()
            .compile_trampoline(component, types, key, tunables, symbol)?;
        if let Some(c) = self.0.inlining_compiler() {
            c.finish_compiling(&mut body.array_call, None, symbol)
                .map_err(|e| CompileError::Codegen(e.to_string()))?;
            c.finish_compiling(&mut body.wasm_call, None, symbol)
                .map_err(|e| CompileError::Codegen(e.to_string()))?;
        }
        Ok(body)
    }
}
