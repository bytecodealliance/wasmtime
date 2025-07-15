//! Wasm compilation orchestration.
//!
//! It works roughly like this:
//!
//! * We walk over the Wasm module/component and make a list of all the things
//!   we need to compile. This is a `CompileInputs`.
//!
//! * The `CompileInputs::compile` method compiles each of these in parallel,
//!   producing a `UnlinkedCompileOutputs`. This is an unlinked set of compiled
//!   functions, bucketed by type of function.
//!
//! * The `UnlinkedCompileOutputs::pre_link` method re-arranges the compiled
//!   functions into a flat list. This is the order we will place them within
//!   the ELF file, so we must also keep track of all the functions' indices
//!   within this list, because we will need them for resolving
//!   relocations. These indices are kept track of in the resulting
//!   `FunctionIndices`.
//!
//! * The `FunctionIndices::link_and_append_code` method appends the functions
//!   to the given ELF file and resolves relocations. It produces an `Artifacts`
//!   which contains the data needed at runtime to find and call Wasm
//!   functions. It is up to the caller to serialize the relevant parts of the
//!   `Artifacts` into the ELF file.

use crate::Engine;
use crate::hash_map::HashMap;
use crate::hash_set::HashSet;
use crate::prelude::*;
use std::{
    any::Any,
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, btree_map},
    mem,
};

use wasmtime_environ::CompiledFunctionBody;
#[cfg(feature = "component-model")]
use wasmtime_environ::component::Translator;
use wasmtime_environ::{
    BuiltinFunctionIndex, CompiledFunctionInfo, CompiledModuleInfo, Compiler, DefinedFuncIndex,
    FilePos, FinishedObject, FunctionBodyData, ModuleEnvironment, ModuleInternedTypeIndex,
    ModuleTranslation, ModuleTypes, ModuleTypesBuilder, ObjectKind, PrimaryMap, RelocationTarget,
    StaticModuleIndex,
};

mod scc;

mod code_builder;
pub use self::code_builder::{CodeBuilder, CodeHint, HashedEngineCompileEnv};

#[cfg(feature = "runtime")]
mod runtime;

/// Converts an input binary-encoded WebAssembly module to compilation
/// artifacts and type information.
///
/// This is where compilation actually happens of WebAssembly modules and
/// translation/parsing/validation of the binary input occurs. The binary
/// artifact represented in the `MmapVec` returned here is an in-memory ELF
/// file in an owned area of virtual linear memory where permissions (such
/// as the executable bit) can be applied.
///
/// Additionally compilation returns an `Option` here which is always
/// `Some`, notably compiled metadata about the module in addition to the
/// type information found within.
pub(crate) fn build_artifacts<T: FinishedObject>(
    engine: &Engine,
    wasm: &[u8],
    dwarf_package: Option<&[u8]>,
    obj_state: &T::State,
) -> Result<(T, Option<(CompiledModuleInfo, ModuleTypes)>)> {
    let tunables = engine.tunables();

    // First a `ModuleEnvironment` is created which records type information
    // about the wasm module. This is where the WebAssembly is parsed and
    // validated. Afterwards `types` will have all the type information for
    // this module.
    let mut parser = wasmparser::Parser::new(0);
    let mut validator = wasmparser::Validator::new_with_features(engine.features());
    parser.set_features(*validator.features());
    let mut types = ModuleTypesBuilder::new(&validator);
    let mut translation = ModuleEnvironment::new(tunables, &mut validator, &mut types)
        .translate(parser, wasm)
        .context("failed to parse WebAssembly module")?;
    let functions = mem::take(&mut translation.function_body_inputs);

    let compile_inputs = CompileInputs::for_module(&types, &translation, functions);
    let unlinked_compile_outputs = compile_inputs.compile(engine)?;
    let PreLinkOutput {
        needs_gc_heap,
        compiled_funcs,
        indices,
    } = unlinked_compile_outputs.pre_link();
    translation.module.needs_gc_heap |= needs_gc_heap;

    // Emplace all compiled functions into the object file with any other
    // sections associated with code as well.
    let mut object = engine.compiler().object(ObjectKind::Module)?;
    // Insert `Engine` and type-level information into the compiled
    // artifact so if this module is deserialized later it contains all
    // information necessary.
    //
    // Note that `append_compiler_info` and `append_types` here in theory
    // can both be skipped if this module will never get serialized.
    // They're only used during deserialization and not during runtime for
    // the module itself. Currently there's no need for that, however, so
    // it's left as an exercise for later.
    engine.append_compiler_info(&mut object);
    engine.append_bti(&mut object);

    let (mut object, compilation_artifacts) = indices.link_and_append_code(
        &types,
        object,
        engine,
        compiled_funcs,
        std::iter::once(translation).collect(),
        dwarf_package,
    )?;

    let info = compilation_artifacts.unwrap_as_module_info();
    let types = types.finish();
    object.serialize_info(&(&info, &types));
    let result = T::finish_object(object, obj_state)?;

    Ok((result, Some((info, types))))
}

/// Performs the compilation phase for a component, translating and
/// validating the provided wasm binary to machine code.
///
/// This method will compile all nested core wasm binaries in addition to
/// any necessary extra functions required for operation with components.
/// The output artifact here is the serialized object file contained within
/// an owned mmap along with metadata about the compilation itself.
#[cfg(feature = "component-model")]
pub(crate) fn build_component_artifacts<T: FinishedObject>(
    engine: &Engine,
    binary: &[u8],
    _dwarf_package: Option<&[u8]>,
    obj_state: &T::State,
) -> Result<(T, Option<wasmtime_environ::component::ComponentArtifacts>)> {
    use wasmtime_environ::ScopeVec;
    use wasmtime_environ::component::{
        CompiledComponentInfo, ComponentArtifacts, ComponentTypesBuilder,
    };

    let tunables = engine.tunables();
    let compiler = engine.compiler();

    let scope = ScopeVec::new();
    let mut validator = wasmparser::Validator::new_with_features(engine.features());
    let mut types = ComponentTypesBuilder::new(&validator);
    let (component, mut module_translations) =
        Translator::new(tunables, &mut validator, &mut types, &scope)
            .translate(binary)
            .context("failed to parse WebAssembly module")?;

    let compile_inputs = CompileInputs::for_component(
        engine,
        &types,
        &component,
        module_translations.iter_mut().map(|(i, translation)| {
            let functions = mem::take(&mut translation.function_body_inputs);
            (i, &*translation, functions)
        }),
    );
    let unlinked_compile_outputs = compile_inputs.compile(&engine)?;

    let PreLinkOutput {
        needs_gc_heap,
        compiled_funcs,
        indices,
    } = unlinked_compile_outputs.pre_link();
    for (_, t) in &mut module_translations {
        t.module.needs_gc_heap |= needs_gc_heap
    }

    let mut object = compiler.object(ObjectKind::Component)?;
    engine.append_compiler_info(&mut object);
    engine.append_bti(&mut object);

    let (mut object, compilation_artifacts) = indices.link_and_append_code(
        types.module_types_builder(),
        object,
        engine,
        compiled_funcs,
        module_translations,
        None, // TODO: Support dwarf packages for components.
    )?;
    let (types, ty) = types.finish(&component.component);

    let info = CompiledComponentInfo {
        component: component.component,
        trampolines: compilation_artifacts.trampolines,
        resource_drop_wasm_to_array_trampoline: compilation_artifacts
            .resource_drop_wasm_to_array_trampoline,
    };
    let artifacts = ComponentArtifacts {
        info,
        ty,
        types,
        static_modules: compilation_artifacts.modules,
    };
    object.serialize_info(&artifacts);

    let result = T::finish_object(object, obj_state)?;
    Ok((result, Some(artifacts)))
}

type CompileInput<'a> = Box<dyn FnOnce(&dyn Compiler) -> Result<CompileOutput> + Send + 'a>;

/// A sortable, comparable key for a compilation output.
///
/// Two `u32`s to align with `cranelift_codegen::ir::UserExternalName`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CompileKey {
    // The namespace field is bitpacked like:
    //
    //     [ kind:i3 module:i29 ]
    namespace: u32,

    index: u32,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CompileKind {
    WasmFunction = CompileKey::new_kind(0),
    ArrayToWasmTrampoline = CompileKey::new_kind(1),
    WasmToArrayTrampoline = CompileKey::new_kind(2),
    WasmToBuiltinTrampoline = CompileKey::new_kind(3),

    #[cfg(feature = "component-model")]
    Trampoline = CompileKey::new_kind(4),
    #[cfg(feature = "component-model")]
    ResourceDropWasmToArrayTrampoline = CompileKey::new_kind(5),
}

impl From<CompileKind> for u32 {
    fn from(kind: CompileKind) -> Self {
        kind as u32
    }
}

impl core::fmt::Debug for CompileKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CompileKey")
            .field("kind", &self.kind())
            .field("module", &self.module())
            .field("index", &self.index)
            .finish()
    }
}

impl CompileKey {
    const KIND_BITS: u32 = 3;
    const KIND_OFFSET: u32 = 32 - Self::KIND_BITS;
    const KIND_MASK: u32 = ((1 << Self::KIND_BITS) - 1) << Self::KIND_OFFSET;

    fn kind(&self) -> CompileKind {
        let k = self.namespace & Self::KIND_MASK;
        if k == u32::from(CompileKind::WasmFunction) {
            return CompileKind::WasmFunction;
        }
        if k == u32::from(CompileKind::ArrayToWasmTrampoline) {
            return CompileKind::ArrayToWasmTrampoline;
        }
        if k == u32::from(CompileKind::WasmToArrayTrampoline) {
            return CompileKind::WasmToArrayTrampoline;
        }
        if k == u32::from(CompileKind::WasmToBuiltinTrampoline) {
            return CompileKind::WasmToBuiltinTrampoline;
        }

        #[cfg(feature = "component-model")]
        {
            if k == u32::from(CompileKind::Trampoline) {
                return CompileKind::Trampoline;
            }
            if k == u32::from(CompileKind::ResourceDropWasmToArrayTrampoline) {
                return CompileKind::ResourceDropWasmToArrayTrampoline;
            }
        }

        unreachable!()
    }

    fn module(&self) -> StaticModuleIndex {
        StaticModuleIndex::from_u32(self.namespace & !Self::KIND_MASK)
    }

    const fn new_kind(kind: u32) -> u32 {
        assert!(kind < (1 << Self::KIND_BITS));
        kind << Self::KIND_OFFSET
    }

    // NB: more kinds in the other `impl` block.

    fn wasm_function(module: StaticModuleIndex, index: DefinedFuncIndex) -> Self {
        debug_assert_eq!(module.as_u32() & Self::KIND_MASK, 0);
        Self {
            namespace: u32::from(CompileKind::WasmFunction) | module.as_u32(),
            index: index.as_u32(),
        }
    }

    fn array_to_wasm_trampoline(module: StaticModuleIndex, index: DefinedFuncIndex) -> Self {
        debug_assert_eq!(module.as_u32() & Self::KIND_MASK, 0);
        Self {
            namespace: u32::from(CompileKind::ArrayToWasmTrampoline) | module.as_u32(),
            index: index.as_u32(),
        }
    }

    fn wasm_to_array_trampoline(index: ModuleInternedTypeIndex) -> Self {
        Self {
            namespace: CompileKind::WasmToArrayTrampoline.into(),
            index: index.as_u32(),
        }
    }

    fn wasm_to_builtin_trampoline(index: BuiltinFunctionIndex) -> Self {
        Self {
            namespace: CompileKind::WasmToBuiltinTrampoline.into(),
            index: index.index(),
        }
    }
}

#[cfg(feature = "component-model")]
impl CompileKey {
    fn trampoline(index: wasmtime_environ::component::TrampolineIndex) -> Self {
        Self {
            namespace: CompileKind::Trampoline.into(),
            index: index.as_u32(),
        }
    }

    fn resource_drop_wasm_to_array_trampoline() -> Self {
        Self {
            namespace: CompileKind::ResourceDropWasmToArrayTrampoline.into(),
            index: 0,
        }
    }
}

#[derive(Clone, Copy)]
enum CompiledFunction<T> {
    Function(T),
    #[cfg(feature = "component-model")]
    AllCallFunc(wasmtime_environ::component::AllCallFunc<T>),
}

impl<T> CompiledFunction<T> {
    fn unwrap_function(self) -> T {
        match self {
            Self::Function(f) => f,
            #[cfg(feature = "component-model")]
            Self::AllCallFunc(_) => panic!("CompiledFunction::unwrap_function"),
        }
    }

    #[cfg(feature = "component-model")]
    fn unwrap_all_call_func(self) -> wasmtime_environ::component::AllCallFunc<T> {
        match self {
            Self::AllCallFunc(f) => f,
            Self::Function(_) => panic!("CompiledFunction::unwrap_all_call_func"),
        }
    }
}

#[cfg(feature = "component-model")]
impl<T> From<wasmtime_environ::component::AllCallFunc<T>> for CompiledFunction<T> {
    fn from(f: wasmtime_environ::component::AllCallFunc<T>) -> Self {
        Self::AllCallFunc(f)
    }
}

struct CompileOutput {
    key: CompileKey,
    symbol: String,
    function: CompiledFunction<CompiledFunctionBody>,
    start_srcloc: FilePos,
}

/// The collection of things we need to compile for a Wasm module or component.
#[derive(Default)]
struct CompileInputs<'a> {
    inputs: Vec<CompileInput<'a>>,
}

impl<'a> CompileInputs<'a> {
    fn push_input(&mut self, f: impl FnOnce(&dyn Compiler) -> Result<CompileOutput> + Send + 'a) {
        self.inputs.push(Box::new(f));
    }

    /// Create the `CompileInputs` for a core Wasm module.
    fn for_module(
        types: &'a ModuleTypesBuilder,
        translation: &'a ModuleTranslation<'a>,
        functions: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'a>>,
    ) -> Self {
        let mut ret = CompileInputs { inputs: vec![] };

        let module_index = StaticModuleIndex::from_u32(0);
        ret.collect_inputs_in_translations(types, [(module_index, translation, functions)]);

        ret
    }

    /// Create a `CompileInputs` for a component.
    #[cfg(feature = "component-model")]
    fn for_component(
        engine: &'a Engine,
        types: &'a wasmtime_environ::component::ComponentTypesBuilder,
        component: &'a wasmtime_environ::component::ComponentTranslation,
        module_translations: impl IntoIterator<
            Item = (
                StaticModuleIndex,
                &'a ModuleTranslation<'a>,
                PrimaryMap<DefinedFuncIndex, FunctionBodyData<'a>>,
            ),
        >,
    ) -> Self {
        let mut ret = CompileInputs { inputs: vec![] };

        ret.collect_inputs_in_translations(types.module_types_builder(), module_translations);
        let tunables = engine.tunables();

        for (idx, trampoline) in component.trampolines.iter() {
            ret.push_input(move |compiler| {
                let symbol = trampoline.symbol_name();
                Ok(CompileOutput {
                    key: CompileKey::trampoline(idx),
                    function: compiler
                        .component_compiler()
                        .compile_trampoline(component, types, idx, tunables, &symbol)
                        .with_context(|| format!("failed to compile {symbol}"))?
                        .into(),
                    symbol,
                    start_srcloc: FilePos::default(),
                })
            });
        }

        // If there are any resources defined within this component, the
        // signature for `resource.drop` is mentioned somewhere, and the
        // wasm-to-native trampoline for `resource.drop` hasn't been created yet
        // then insert that here. This is possibly required by destruction of
        // resources from the embedder and otherwise won't be explicitly
        // requested through initializers above or such.
        if component.component.num_resources > 0 {
            if let Some(sig) = types.find_resource_drop_signature() {
                ret.push_input(move |compiler| {
                    let symbol = "resource_drop_trampoline".to_string();
                    let function = compiler
                        .compile_wasm_to_array_trampoline(types[sig].unwrap_func(), &symbol)
                        .with_context(|| format!("failed to compile `{symbol}`"))?;
                    Ok(CompileOutput {
                        key: CompileKey::resource_drop_wasm_to_array_trampoline(),
                        function: CompiledFunction::Function(function),
                        symbol,
                        start_srcloc: FilePos::default(),
                    })
                });
            }
        }

        ret
    }

    fn clean_symbol(name: &str) -> Cow<'_, str> {
        /// Maximum length of symbols generated in objects.
        const MAX_SYMBOL_LEN: usize = 96;

        // Just to be on the safe side, filter out characters that could
        // pose issues to tools such as "perf" or "objdump".  To avoid
        // having to update a list of allowed characters for each different
        // language that compiles to Wasm, allows only graphic ASCII
        // characters; replace runs of everything else with a "?".
        let bad_char = |c: char| !c.is_ascii_graphic();
        if name.chars().any(bad_char) {
            let mut last_char_seen = '\u{0000}';
            Cow::Owned(
                name.chars()
                    .map(|c| if bad_char(c) { '?' } else { c })
                    .filter(|c| {
                        let skip = last_char_seen == '?' && *c == '?';
                        last_char_seen = *c;
                        !skip
                    })
                    .take(MAX_SYMBOL_LEN)
                    .collect::<String>(),
            )
        } else if name.len() <= MAX_SYMBOL_LEN {
            Cow::Borrowed(&name[..])
        } else {
            Cow::Borrowed(&name[..MAX_SYMBOL_LEN])
        }
    }

    fn collect_inputs_in_translations(
        &mut self,
        types: &'a ModuleTypesBuilder,
        translations: impl IntoIterator<
            Item = (
                StaticModuleIndex,
                &'a ModuleTranslation<'a>,
                PrimaryMap<DefinedFuncIndex, FunctionBodyData<'a>>,
            ),
        >,
    ) {
        for (module, translation, functions) in translations {
            for (def_func_index, func_body) in functions {
                self.push_input(move |compiler| {
                    let func_index = translation.module.func_index(def_func_index);
                    let symbol = match translation
                        .debuginfo
                        .name_section
                        .func_names
                        .get(&func_index)
                    {
                        Some(name) => format!(
                            "wasm[{}]::function[{}]::{}",
                            module.as_u32(),
                            func_index.as_u32(),
                            Self::clean_symbol(&name)
                        ),
                        None => format!(
                            "wasm[{}]::function[{}]",
                            module.as_u32(),
                            func_index.as_u32()
                        ),
                    };
                    let data = func_body.body.get_binary_reader();
                    let offset = data.original_position();
                    let start_srcloc = FilePos::new(u32::try_from(offset).unwrap());
                    let function = compiler
                        .compile_function(translation, def_func_index, func_body, types, &symbol)
                        .with_context(|| format!("failed to compile: {symbol}"))?;

                    Ok(CompileOutput {
                        key: CompileKey::wasm_function(module, def_func_index),
                        symbol,
                        function: CompiledFunction::Function(function),
                        start_srcloc,
                    })
                });

                let func_index = translation.module.func_index(def_func_index);
                if translation.module.functions[func_index].is_escaping() {
                    self.push_input(move |compiler| {
                        let func_index = translation.module.func_index(def_func_index);
                        let symbol = format!(
                            "wasm[{}]::array_to_wasm_trampoline[{}]",
                            module.as_u32(),
                            func_index.as_u32()
                        );
                        let trampoline = compiler
                            .compile_array_to_wasm_trampoline(
                                translation,
                                types,
                                def_func_index,
                                &symbol,
                            )
                            .with_context(|| format!("failed to compile: {symbol}"))?;
                        Ok(CompileOutput {
                            key: CompileKey::array_to_wasm_trampoline(module, def_func_index),
                            symbol,
                            function: CompiledFunction::Function(trampoline),
                            start_srcloc: FilePos::default(),
                        })
                    });
                }
            }
        }

        let mut trampoline_types_seen = HashSet::new();
        for (_func_type_index, trampoline_type_index) in types.trampoline_types() {
            let is_new = trampoline_types_seen.insert(trampoline_type_index);
            if !is_new {
                continue;
            }
            let trampoline_func_ty = types[trampoline_type_index].unwrap_func();
            self.push_input(move |compiler| {
                let symbol = format!(
                    "signatures[{}]::wasm_to_array_trampoline",
                    trampoline_type_index.as_u32()
                );
                let trampoline = compiler
                    .compile_wasm_to_array_trampoline(trampoline_func_ty, &symbol)
                    .with_context(|| format!("failed to compile: {symbol}"))?;
                Ok(CompileOutput {
                    key: CompileKey::wasm_to_array_trampoline(trampoline_type_index),
                    function: CompiledFunction::Function(trampoline),
                    symbol,
                    start_srcloc: FilePos::default(),
                })
            });
        }
    }

    /// Compile these `CompileInput`s (maybe in parallel) and return the
    /// resulting `UnlinkedCompileOutput`s.
    fn compile(self, engine: &Engine) -> Result<UnlinkedCompileOutputs> {
        let compiler = engine.compiler();

        if self.inputs.len() > 0 && cfg!(miri) {
            bail!(
                "\
You are attempting to compile a WebAssembly module or component that contains
functions in Miri. Running Cranelift through Miri is known to take quite a long
time and isn't what we want in CI at least. If this is a mistake then you should
ignore this test in Miri with:

    #[cfg_attr(miri, ignore)]

If this is not a mistake then try to edit the `pulley_provenance_test` test
which runs Cranelift outside of Miri. If you still feel this is a mistake then
please open an issue or a topic on Zulip to talk about how best to accomodate
the use case.
"
            );
        }

        // Compile each individual input in parallel.
        let mut raw_outputs = engine.run_maybe_parallel(self.inputs, |f| f(compiler))?;

        // Now that all functions have been compiled see if any
        // wasmtime-builtin functions are necessary. If so those need to be
        // collected and then those trampolines additionally need to be
        // compiled.
        compile_required_builtins(engine, &mut raw_outputs)?;

        // Bucket the outputs by kind.
        let mut outputs: BTreeMap<CompileKind, Vec<CompileOutput>> = BTreeMap::new();
        for output in raw_outputs {
            outputs.entry(output.key.kind()).or_default().push(output);
        }

        Ok(UnlinkedCompileOutputs { outputs })
    }
}

fn compile_required_builtins(engine: &Engine, raw_outputs: &mut Vec<CompileOutput>) -> Result<()> {
    let compiler = engine.compiler();
    let mut builtins = HashSet::new();
    let mut new_inputs: Vec<CompileInput<'_>> = Vec::new();

    let compile_builtin = |builtin: BuiltinFunctionIndex| {
        Box::new(move |compiler: &dyn Compiler| {
            let symbol = format!("wasmtime_builtin_{}", builtin.name());
            let trampoline = compiler
                .compile_wasm_to_builtin(builtin, &symbol)
                .with_context(|| format!("failed to compile `{symbol}`"))?;
            Ok(CompileOutput {
                key: CompileKey::wasm_to_builtin_trampoline(builtin),
                function: CompiledFunction::Function(trampoline),
                symbol,
                start_srcloc: FilePos::default(),
            })
        })
    };

    for output in raw_outputs.iter() {
        let f = match &output.function {
            CompiledFunction::Function(f) => f,
            #[cfg(feature = "component-model")]
            CompiledFunction::AllCallFunc(_) => continue,
        };
        for reloc in compiler.compiled_function_relocation_targets(&*f.code) {
            match reloc {
                RelocationTarget::Builtin(i) => {
                    if builtins.insert(i) {
                        new_inputs.push(compile_builtin(i));
                    }
                }
                _ => {}
            }
        }
    }
    raw_outputs.extend(engine.run_maybe_parallel(new_inputs, |c| c(compiler))?);
    Ok(())
}

#[derive(Default)]
struct UnlinkedCompileOutputs {
    // A map from kind to `CompileOutput`.
    outputs: BTreeMap<CompileKind, Vec<CompileOutput>>,
}

impl UnlinkedCompileOutputs {
    /// Flatten all our functions into a single list and remember each of their
    /// indices within it.
    fn pre_link(self) -> PreLinkOutput {
        // The order the functions end up within `compiled_funcs` is the order
        // that they will be laid out in the ELF file, so try and group hot and
        // cold functions together as best we can. However, because we bucket by
        // kind, we shouldn't have any issues with, e.g., cold trampolines
        // appearing in between hot Wasm functions.
        let mut compiled_funcs = vec![];
        let mut indices = FunctionIndices::default();
        let mut needs_gc_heap = false;

        for output in self.outputs.into_iter().flat_map(|(_kind, outs)| outs) {
            let index = match output.function {
                CompiledFunction::Function(f) => {
                    needs_gc_heap |= f.needs_gc_heap;
                    let index = compiled_funcs.len();
                    compiled_funcs.push((output.symbol, f.code));
                    CompiledFunction::Function(index)
                }
                #[cfg(feature = "component-model")]
                CompiledFunction::AllCallFunc(wasmtime_environ::component::AllCallFunc {
                    wasm_call,
                    array_call,
                }) => {
                    needs_gc_heap |= array_call.needs_gc_heap;
                    let array_call_idx = compiled_funcs.len();
                    compiled_funcs.push((format!("{}_array_call", output.symbol), array_call.code));

                    needs_gc_heap |= wasm_call.needs_gc_heap;
                    let wasm_call_idx = compiled_funcs.len();
                    compiled_funcs.push((format!("{}_wasm_call", output.symbol), wasm_call.code));

                    CompiledFunction::AllCallFunc(wasmtime_environ::component::AllCallFunc {
                        array_call: array_call_idx,
                        wasm_call: wasm_call_idx,
                    })
                }
            };

            if output.key.kind() == CompileKind::WasmFunction
                || output.key.kind() == CompileKind::ArrayToWasmTrampoline
            {
                indices
                    .compiled_func_index_to_module
                    .insert(index.unwrap_function(), output.key.module());
                indices
                    .start_srclocs
                    .insert(output.key, output.start_srcloc);
            }

            indices
                .indices
                .entry(output.key.kind())
                .or_default()
                .insert(output.key, index);
        }

        PreLinkOutput {
            needs_gc_heap,
            compiled_funcs,
            indices,
        }
    }
}

/// Our pre-link functions that have been flattened into a single list.
struct PreLinkOutput {
    /// Whether or not any of these functions require a GC heap
    needs_gc_heap: bool,
    /// The flattened list of (symbol name, compiled function) pairs, as they
    /// will be laid out in the object file.
    compiled_funcs: Vec<(String, Box<dyn Any + Send>)>,
    /// The `FunctionIndices` mapping our function keys to indices in that flat
    /// list.
    indices: FunctionIndices,
}

#[derive(Default)]
struct FunctionIndices {
    // A reverse map from an index in `compiled_funcs` to the
    // `StaticModuleIndex` for that function.
    compiled_func_index_to_module: HashMap<usize, StaticModuleIndex>,

    // A map of wasm functions and where they're located in the original file.
    start_srclocs: HashMap<CompileKey, FilePos>,

    // The index of each compiled function, bucketed by compile key kind.
    indices: BTreeMap<CompileKind, BTreeMap<CompileKey, CompiledFunction<usize>>>,
}

impl FunctionIndices {
    /// Link the compiled functions together, resolving relocations, and append
    /// them to the given ELF file.
    fn link_and_append_code<'a>(
        mut self,
        types: &ModuleTypesBuilder,
        mut obj: object::write::Object<'static>,
        engine: &'a Engine,
        compiled_funcs: Vec<(String, Box<dyn Any + Send>)>,
        translations: PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
        dwarf_package_bytes: Option<&[u8]>,
    ) -> Result<(wasmtime_environ::ObjectBuilder<'a>, Artifacts)> {
        // Append all the functions to the ELF file.
        //
        // The result is a vector parallel to `compiled_funcs` where
        // `symbol_ids_and_locs[i]` is the symbol ID and function location of
        // `compiled_funcs[i]`.
        let compiler = engine.compiler();
        let tunables = engine.tunables();
        let symbol_ids_and_locs = compiler.append_code(
            &mut obj,
            &compiled_funcs,
            &|caller_index: usize, callee: RelocationTarget| match callee {
                RelocationTarget::Wasm(callee_index) => {
                    let caller_module = self
                        .compiled_func_index_to_module
                        .get(&caller_index)
                        .copied()
                        .expect("should only reloc inside wasm function callers");
                    let key = if let Some(def_func_index) = translations[caller_module]
                        .module
                        .defined_func_index(callee_index)
                    {
                        CompileKey::wasm_function(caller_module, def_func_index)
                    } else {
                        let (def_module, def_func_index) = translations[caller_module]
                            .known_imported_functions[callee_index]
                            .expect(
                                "a direct call to an imported function must have a \
                                 statically-known import",
                            );
                        CompileKey::wasm_function(def_module, def_func_index)
                    };
                    self.indices[&CompileKind::WasmFunction][&key].unwrap_function()
                }
                RelocationTarget::Builtin(builtin) => self.indices
                    [&CompileKind::WasmToBuiltinTrampoline]
                    [&CompileKey::wasm_to_builtin_trampoline(builtin)]
                    .unwrap_function(),
                RelocationTarget::PulleyHostcall(_) => {
                    unreachable!("relocation is resolved at runtime, not compile time");
                }
            },
        )?;

        // If requested, generate and add DWARF information.
        if tunables.generate_native_debuginfo {
            compiler.append_dwarf(
                &mut obj,
                &translations,
                &|module, func| {
                    let bucket = &self.indices[&CompileKind::WasmFunction];
                    let i = bucket[&CompileKey::wasm_function(module, func)].unwrap_function();
                    (symbol_ids_and_locs[i].0, &*compiled_funcs[i].1)
                },
                dwarf_package_bytes,
                tunables,
            )?;
        }

        let mut obj = wasmtime_environ::ObjectBuilder::new(obj, tunables);
        let mut artifacts = Artifacts::default();

        // Remove this as it's not needed by anything below and we'll debug
        // assert `self.indices` is empty, so this is acknowledgement that this
        // is a pure runtime implementation detail and not needed in any
        // metadata generated below.
        self.indices.remove(&CompileKind::WasmToBuiltinTrampoline);

        // Finally, build our binary artifacts that map things like `FuncIndex`
        // to a function location and all of that using the indices we saved
        // earlier and the function locations we just received after appending
        // the code.

        let mut wasm_functions = self
            .indices
            .remove(&CompileKind::WasmFunction)
            .unwrap_or_default()
            .into_iter()
            .peekable();

        fn wasm_functions_for_module(
            wasm_functions: &mut std::iter::Peekable<
                btree_map::IntoIter<CompileKey, CompiledFunction<usize>>,
            >,
            module: StaticModuleIndex,
        ) -> impl Iterator<Item = (CompileKey, CompiledFunction<usize>)> + '_ {
            std::iter::from_fn(move || {
                let (key, _) = wasm_functions.peek()?;
                if key.module() == module {
                    wasm_functions.next()
                } else {
                    None
                }
            })
        }

        let mut array_to_wasm_trampolines = self
            .indices
            .remove(&CompileKind::ArrayToWasmTrampoline)
            .unwrap_or_default();

        // NB: unlike the above maps this is not emptied out during iteration
        // since each module may reach into different portions of this map.
        let wasm_to_array_trampolines = self
            .indices
            .remove(&CompileKind::WasmToArrayTrampoline)
            .unwrap_or_default();

        artifacts.modules = translations
            .into_iter()
            .map(|(module, mut translation)| {
                // If configured attempt to use static memory initialization which
                // can either at runtime be implemented as a single memcpy to
                // initialize memory or otherwise enabling virtual-memory-tricks
                // such as mmap'ing from a file to get copy-on-write.
                if engine.tunables().memory_init_cow {
                    let align = compiler.page_size_align();
                    let max_always_allowed = engine.config().memory_guaranteed_dense_image_size;
                    translation.try_static_init(align, max_always_allowed);
                }

                // Attempt to convert table initializer segments to
                // FuncTable representation where possible, to enable
                // table lazy init.
                if engine.tunables().table_lazy_init {
                    translation.try_func_table_init();
                }

                let funcs: PrimaryMap<DefinedFuncIndex, CompiledFunctionInfo> =
                    wasm_functions_for_module(&mut wasm_functions, module)
                        .map(|(key, wasm_func_index)| {
                            let wasm_func_index = wasm_func_index.unwrap_function();
                            let wasm_func_loc = symbol_ids_and_locs[wasm_func_index].1;
                            let start_srcloc = self.start_srclocs.remove(&key).unwrap();

                            let array_to_wasm_trampoline = array_to_wasm_trampolines
                                .remove(&CompileKey::array_to_wasm_trampoline(
                                    key.module(),
                                    DefinedFuncIndex::from_u32(key.index),
                                ))
                                .map(|x| symbol_ids_and_locs[x.unwrap_function()].1);

                            CompiledFunctionInfo {
                                start_srcloc,
                                wasm_func_loc,
                                array_to_wasm_trampoline,
                            }
                        })
                        .collect();

                let unique_and_sorted_trampoline_sigs = translation
                    .module
                    .types
                    .iter()
                    .map(|(_, ty)| ty.unwrap_module_type_index())
                    .filter(|idx| types[*idx].is_func())
                    .map(|idx| types.trampoline_type(idx))
                    .collect::<BTreeSet<_>>();
                let wasm_to_array_trampolines = unique_and_sorted_trampoline_sigs
                    .iter()
                    .map(|idx| {
                        let trampoline = types.trampoline_type(*idx);
                        let key = CompileKey::wasm_to_array_trampoline(trampoline);
                        let compiled = wasm_to_array_trampolines[&key];
                        (*idx, symbol_ids_and_locs[compiled.unwrap_function()].1)
                    })
                    .collect();

                obj.append(translation, funcs, wasm_to_array_trampolines)
            })
            .collect::<Result<PrimaryMap<_, _>>>()?;

        #[cfg(feature = "component-model")]
        {
            artifacts.trampolines = self
                .indices
                .remove(&CompileKind::Trampoline)
                .unwrap_or_default()
                .into_iter()
                .map(|(_id, x)| x.unwrap_all_call_func().map(|i| symbol_ids_and_locs[i].1))
                .collect();
            let map = self
                .indices
                .remove(&CompileKind::ResourceDropWasmToArrayTrampoline)
                .unwrap_or_default();
            assert!(map.len() <= 1);
            artifacts.resource_drop_wasm_to_array_trampoline = map
                .into_iter()
                .next()
                .map(|(_id, x)| symbol_ids_and_locs[x.unwrap_function()].1);
        }

        debug_assert!(
            self.indices.is_empty(),
            "Should have processed all compile outputs"
        );

        Ok((obj, artifacts))
    }
}

/// The artifacts necessary for finding and calling Wasm functions at runtime,
/// to be serialized into an ELF file.
#[derive(Default)]
struct Artifacts {
    modules: PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
    #[cfg(feature = "component-model")]
    trampolines: PrimaryMap<
        wasmtime_environ::component::TrampolineIndex,
        wasmtime_environ::component::AllCallFunc<wasmtime_environ::FunctionLoc>,
    >,
    #[cfg(feature = "component-model")]
    resource_drop_wasm_to_array_trampoline: Option<wasmtime_environ::FunctionLoc>,
}

impl Artifacts {
    /// Assuming this compilation was for a single core Wasm module, get the
    /// resulting `CompiledModuleInfo`.
    fn unwrap_as_module_info(self) -> CompiledModuleInfo {
        assert_eq!(self.modules.len(), 1);
        #[cfg(feature = "component-model")]
        assert!(self.trampolines.is_empty());
        self.modules.into_iter().next().unwrap().1
    }
}
