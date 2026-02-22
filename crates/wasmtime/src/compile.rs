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
use std::{any::Any, borrow::Cow, collections::BTreeMap, mem, ops::Range};
use wasmtime_environ::{
    Abi, CompiledFunctionBody, CompiledFunctionsTable, CompiledFunctionsTableBuilder,
    CompiledModuleInfo, Compiler, DefinedFuncIndex, FilePos, FinishedObject, FuncKey,
    FunctionBodyData, InliningCompiler, IntraModuleInlining, ModuleEnvironment, ModuleTranslation,
    ModuleTypes, ModuleTypesBuilder, ObjectKind, PrimaryMap, StaticModuleIndex, Tunables,
    graphs::{EntityGraph, Graph as _},
};
#[cfg(feature = "component-model")]
use wasmtime_environ::{WasmChecksum, component::Translator};

mod stratify;

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
pub(crate) fn build_module_artifacts<T: FinishedObject>(
    engine: &Engine,
    wasm: &[u8],
    dwarf_package: Option<&[u8]>,
    obj_state: &T::State,
) -> Result<(
    T,
    Option<(CompiledModuleInfo, CompiledFunctionsTable, ModuleTypes)>,
)> {
    let compiler = engine.try_compiler()?;
    let tunables = engine.tunables();

    // First a `ModuleEnvironment` is created which records type information
    // about the wasm module. This is where the WebAssembly is parsed and
    // validated. Afterwards `types` will have all the type information for
    // this module.
    let mut parser = wasmparser::Parser::new(0);
    let mut validator = wasmparser::Validator::new_with_features(engine.features());
    parser.set_features(*validator.features());
    let mut types = ModuleTypesBuilder::new(&validator);
    let mut translation = ModuleEnvironment::new(
        tunables,
        &mut validator,
        &mut types,
        StaticModuleIndex::from_u32(0),
    )
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
    let mut object = compiler.object(ObjectKind::Module)?;
    // Insert `Engine` and type-level information into the compiled
    // artifact so if this module is deserialized later it contains all
    // information necessary.
    //
    // Note that `append_compiler_info` and `append_types` here in theory
    // can both be skipped if this module will never get serialized.
    // They're only used during deserialization and not during runtime for
    // the module itself. Currently there's no need for that, however, so
    // it's left as an exercise for later.
    engine.append_compiler_info(&mut object)?;
    engine.append_bti(&mut object);

    let (mut object, compilation_artifacts) = indices.link_and_append_code(
        object,
        engine,
        compiled_funcs,
        std::iter::once(translation).collect(),
        dwarf_package,
    )?;

    let (info, index) = compilation_artifacts.unwrap_as_module_info();
    let types = types.finish();
    object.serialize_info(&(&info, &index, &types));
    let result = T::finish_object(object, obj_state)?;

    Ok((result, Some((info, index, types))))
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
    unsafe_intrinsics_import: Option<&str>,
    obj_state: &T::State,
) -> Result<(T, Option<wasmtime_environ::component::ComponentArtifacts>)> {
    use wasmtime_environ::ScopeVec;
    use wasmtime_environ::component::{
        CompiledComponentInfo, ComponentArtifacts, ComponentTypesBuilder,
    };

    let compiler = engine.try_compiler()?;
    let tunables = engine.tunables();

    let scope = ScopeVec::new();
    let mut validator = wasmparser::Validator::new_with_features(engine.features());
    let mut types = ComponentTypesBuilder::new(&validator);
    let mut translator = Translator::new(tunables, &mut validator, &mut types, &scope);
    if let Some(name) = unsafe_intrinsics_import {
        translator.expose_unsafe_intrinsics(name);
    }
    let (component, mut module_translations) = translator
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
    engine.append_compiler_info(&mut object)?;
    engine.append_bti(&mut object);

    let (mut object, compilation_artifacts) = indices.link_and_append_code(
        object,
        engine,
        compiled_funcs,
        module_translations,
        None, // TODO: Support dwarf packages for components.
    )?;
    let (types, ty) = types.finish(&component.component);

    let info = CompiledComponentInfo {
        component: component.component,
    };
    let artifacts = ComponentArtifacts {
        info,
        table: compilation_artifacts.table,
        ty,
        types,
        static_modules: compilation_artifacts.modules,
        checksum: WasmChecksum::from_binary(binary, tunables.recording),
    };
    object.serialize_info(&artifacts);

    let result = T::finish_object(object, obj_state)?;
    Ok((result, Some(artifacts)))
}

type CompileInput<'a> = Box<dyn FnOnce(&dyn Compiler) -> Result<CompileOutput<'a>> + Send + 'a>;

struct CompileOutput<'a> {
    key: FuncKey,
    symbol: String,
    function: CompiledFunctionBody,
    start_srcloc: FilePos,

    // Only present when `self.key` is a `FuncKey::DefinedWasmFunction(..)`.
    translation: Option<&'a ModuleTranslation<'a>>,

    // Only present when `self.key` is a `FuncKey::DefinedWasmFunction(..)`.
    func_body: Option<wasmparser::FunctionBody<'a>>,
}

/// Inputs to our inlining heuristics.
struct InlineHeuristicParams<'a> {
    tunables: &'a Tunables,
    caller_size: u32,
    caller_key: FuncKey,
    caller_needs_gc_heap: bool,
    callee_size: u32,
    callee_key: FuncKey,
    callee_needs_gc_heap: bool,
}

/// The collection of things we need to compile for a Wasm module or component.
#[derive(Default)]
struct CompileInputs<'a> {
    inputs: Vec<CompileInput<'a>>,
}

impl<'a> CompileInputs<'a> {
    fn push_input(
        &mut self,
        f: impl FnOnce(&dyn Compiler) -> Result<CompileOutput<'a>> + Send + 'a,
    ) {
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
        use wasmtime_environ::Abi;
        use wasmtime_environ::component::UnsafeIntrinsic;

        let mut ret = CompileInputs { inputs: vec![] };

        ret.collect_inputs_in_translations(types.module_types_builder(), module_translations);
        let tunables = engine.tunables();

        for i in component
            .component
            .unsafe_intrinsics
            .iter()
            .enumerate()
            .filter_map(|(i, ty)| if ty.is_some() { Some(i) } else { None })
        {
            let i = u32::try_from(i).unwrap();
            let intrinsic = UnsafeIntrinsic::from_u32(i);
            for abi in [Abi::Wasm, Abi::Array] {
                ret.push_input(move |compiler| {
                    let symbol = format!(
                        "unsafe-intrinsics-{}-{}",
                        match abi {
                            Abi::Wasm => "wasm-call",
                            Abi::Array => "array-call",
                            Abi::Patchable => "patchable-call",
                        },
                        intrinsic.name(),
                    );
                    Ok(CompileOutput {
                        key: FuncKey::UnsafeIntrinsic(abi, intrinsic),
                        function: compiler
                            .component_compiler()
                            .compile_intrinsic(tunables, component, types, intrinsic, abi, &symbol)
                            .with_context(|| format!("failed to compile `{symbol}`"))?,
                        symbol,
                        start_srcloc: FilePos::default(),
                        translation: None,
                        func_body: None,
                    })
                });
            }
        }

        for (idx, trampoline) in component.trampolines.iter() {
            for abi in [Abi::Wasm, Abi::Array] {
                ret.push_input(move |compiler| {
                    let key = FuncKey::ComponentTrampoline(abi, idx);
                    let symbol = format!(
                        "component-trampolines[{}]-{}-{}",
                        idx.as_u32(),
                        match abi {
                            Abi::Wasm => "wasm-call",
                            Abi::Array => "array-call",
                            Abi::Patchable => "patchable-call",
                        },
                        trampoline.symbol_name(),
                    );
                    let function = compiler
                        .component_compiler()
                        .compile_trampoline(component, types, key, abi, tunables, &symbol)
                        .with_context(|| format!("failed to compile {symbol}"))?;
                    Ok(CompileOutput {
                        key,
                        function,
                        symbol,
                        start_srcloc: FilePos::default(),
                        translation: None,
                        func_body: None,
                    })
                });
            }
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
                    let key = FuncKey::ResourceDropTrampoline;
                    let symbol = "resource_drop_trampoline".to_string();
                    let function = compiler
                        .compile_wasm_to_array_trampoline(types[sig].unwrap_func(), key, &symbol)
                        .with_context(|| format!("failed to compile `{symbol}`"))?;
                    Ok(CompileOutput {
                        key,
                        function,
                        symbol,
                        start_srcloc: FilePos::default(),
                        translation: None,
                        func_body: None,
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
            for (def_func_index, func_body_data) in functions {
                self.push_input(move |compiler| {
                    let key = FuncKey::DefinedWasmFunction(module, def_func_index);
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
                    let func_body = func_body_data.body.clone();
                    let data = func_body.get_binary_reader();
                    let offset = data.original_position();
                    let start_srcloc = FilePos::new(u32::try_from(offset).unwrap());
                    let function = compiler
                        .compile_function(translation, key, func_body_data, types, &symbol)
                        .with_context(|| format!("failed to compile: {symbol}"))?;

                    Ok(CompileOutput {
                        key,
                        symbol,
                        function,
                        start_srcloc,
                        translation: Some(translation),
                        func_body: Some(func_body),
                    })
                });

                let func_index = translation.module.func_index(def_func_index);
                if translation.module.functions[func_index].is_escaping() {
                    self.push_input(move |compiler| {
                        let key = FuncKey::ArrayToWasmTrampoline(module, def_func_index);
                        let func_index = translation.module.func_index(def_func_index);
                        let symbol = format!(
                            "wasm[{}]::array_to_wasm_trampoline[{}]",
                            module.as_u32(),
                            func_index.as_u32()
                        );
                        let function = compiler
                            .compile_array_to_wasm_trampoline(translation, types, key, &symbol)
                            .with_context(|| format!("failed to compile: {symbol}"))?;
                        Ok(CompileOutput {
                            key,
                            symbol,
                            function,
                            start_srcloc: FilePos::default(),
                            translation: None,
                            func_body: None,
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
                let key = FuncKey::WasmToArrayTrampoline(trampoline_type_index);
                let symbol = format!(
                    "signatures[{}]::wasm_to_array_trampoline",
                    trampoline_type_index.as_u32()
                );
                let function = compiler
                    .compile_wasm_to_array_trampoline(trampoline_func_ty, key, &symbol)
                    .with_context(|| format!("failed to compile: {symbol}"))?;
                Ok(CompileOutput {
                    key,
                    function,
                    symbol,
                    start_srcloc: FilePos::default(),
                    translation: None,
                    func_body: None,
                })
            });
        }
    }

    /// Compile these `CompileInput`s (maybe in parallel) and return the
    /// resulting `UnlinkedCompileOutput`s.
    fn compile(self, engine: &Engine) -> Result<UnlinkedCompileOutputs<'a>> {
        let compiler = engine.try_compiler()?;

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
please open an issue or a topic on Zulip to talk about how best to accommodate
the use case.
"
            );
        }

        let mut raw_outputs = if let Some(inlining_compiler) = compiler.inlining_compiler() {
            if engine.tunables().inlining {
                self.compile_with_inlining(engine, compiler, inlining_compiler)?
            } else {
                // Inlining compiler but inlining is disabled: compile each
                // input and immediately finish its output in parallel, skipping
                // call graph computation and all that.
                engine.run_maybe_parallel::<_, _, Error, _>(self.inputs, |f| {
                    let mut compiled = f(compiler)?;
                    inlining_compiler.finish_compiling(
                        &mut compiled.function,
                        compiled.func_body.take(),
                        &compiled.symbol,
                    )?;
                    Ok(compiled)
                })?
            }
        } else {
            // No inlining: just compile each individual input in parallel.
            engine.run_maybe_parallel(self.inputs, |f| f(compiler))?
        };

        if cfg!(debug_assertions) {
            let mut symbols: Vec<_> = raw_outputs.iter().map(|i| &i.symbol).collect();
            symbols.sort();
            for w in symbols.windows(2) {
                assert_ne!(
                    w[0], w[1],
                    "should never have duplicate symbols, but found two functions with the symbol `{}`",
                    w[0]
                );
            }
        }

        // Now that all functions have been compiled see if any
        // wasmtime-builtin functions are necessary. If so those need to be
        // collected and then those trampolines additionally need to be
        // compiled.
        compile_required_builtins(engine, &mut raw_outputs)?;

        // Bucket the outputs by kind.
        let mut outputs: BTreeMap<FuncKey, CompileOutput> = BTreeMap::new();
        for output in raw_outputs {
            outputs.insert(output.key, output);
        }

        Ok(UnlinkedCompileOutputs { outputs })
    }

    fn compile_with_inlining(
        self,
        engine: &Engine,
        compiler: &dyn Compiler,
        inlining_compiler: &dyn InliningCompiler,
    ) -> Result<Vec<CompileOutput<'a>>, Error> {
        // Our list of unlinked outputs.
        let mut outputs = PrimaryMap::<OutputIndex, Option<CompileOutput<'_>>>::from(
            engine.run_maybe_parallel(self.inputs, |f| f(compiler).map(Some))?,
        );

        // A map from a `FuncKey` to its index in our unlinked outputs.
        //
        // We will generally just be working with `OutputIndex`es, but
        // occasionally we must translate from keys back to our index space, for
        // example when we know that one module's function import is always
        // satisfied with a particular `FuncKey::DefinedWasmFunction`. This map
        // enables that translation.
        let key_to_output: HashMap<FuncKey, OutputIndex> = inlining_functions(&outputs)
            .map(|output_index| {
                let output = outputs[output_index].as_ref().unwrap();
                (output.key, output_index)
            })
            .collect();

        // Construct the call graph for inlining.
        //
        // We only inline Wasm functions, not trampolines, because we rely on
        // trampolines being in their own stack frame when we save the entry and
        // exit SP, FP, and PC for backtraces in trampolines.
        let call_graph = EntityGraph::<OutputIndex>::new(inlining_functions(&outputs), {
            let mut func_keys = IndexSet::default();
            let outputs = &outputs;
            let key_to_output = &key_to_output;
            move |output_index, calls| {
                let output = outputs[output_index].as_ref().unwrap();
                debug_assert!(is_inlining_function(output.key));

                // Get this function's call graph edges as `FuncKey`s.
                func_keys.clear();
                inlining_compiler.calls(&output.function, &mut func_keys)?;

                // Translate each of those to keys to output indices, which is
                // what we actually need.
                debug_assert!(calls.is_empty());
                calls.extend(
                    func_keys
                        .iter()
                        .copied()
                        .filter_map(|key| key_to_output.get(&key).copied()),
                );

                log::trace!(
                    "call graph edges for {output_index:?} = {:?}: {calls:?}",
                    output.key
                );

                crate::error::Ok(())
            }
        })?;

        // Stratify the call graph into a sequence of layers. We process each
        // layer in order, but process functions within a layer in parallel
        // (because they either do not call each other or are part of a
        // mutual-recursion cycle; either way we won't inline members of the
        // same layer into each other).
        let strata = stratify::Strata::<OutputIndex>::new(&call_graph.filter_nodes(|f| {
            let key = outputs[*f].as_ref().unwrap().key;
            is_inlining_function(key)
        }));
        let mut layer_outputs = vec![];
        for layer in strata.layers() {
            // Temporarily take this layer's outputs out of our unlinked outputs
            // list so that we can mutate these outputs (by inlining callee
            // functions into them) while also accessing shared borrows of the
            // unlinked outputs list (finding the callee functions we will
            // inline).
            debug_assert!(layer_outputs.is_empty());
            layer_outputs.extend(layer.iter().map(|f| outputs[*f].take().unwrap()));

            // Process this layer's members in parallel.
            engine.run_maybe_parallel_mut(
                &mut layer_outputs,
                |output: &mut CompileOutput<'_>| {
                    log::trace!("processing inlining for {:?}", output.key);
                    debug_assert!(is_inlining_function(output.key));

                    let caller_key = output.key;
                    let caller_needs_gc_heap =
                        output.translation.is_some_and(|t| t.module.needs_gc_heap);
                    let caller = &mut output.function;

                    let mut caller_size = inlining_compiler.size(caller);

                    inlining_compiler.inline(caller, &mut |callee_key: FuncKey| {
                        log::trace!("  --> considering call to {callee_key:?}");
                        let callee_output_index: OutputIndex = key_to_output[&callee_key];

                        // NB: If the callee is not inside `outputs`, then it is
                        // in the same `Strata` layer as the caller (and
                        // therefore is in the same strongly-connected component
                        // as the caller, and they mutually recursive). In this
                        // case, we do not do any inlining; communicate this
                        // command via `?`-propagation.
                        let callee_output = outputs[callee_output_index].as_ref()?;

                        debug_assert_eq!(callee_output.key, callee_key);

                        let callee = &callee_output.function;
                        let callee_size = inlining_compiler.size(callee);

                        let callee_needs_gc_heap = callee_output
                            .translation
                            .is_some_and(|t| t.module.needs_gc_heap);

                        if Self::should_inline(InlineHeuristicParams {
                            tunables: engine.tunables(),
                            caller_size,
                            caller_key,
                            caller_needs_gc_heap,
                            callee_size,
                            callee_key,
                            callee_needs_gc_heap,
                        }) {
                            caller_size = caller_size.saturating_add(callee_size);
                            Some(callee)
                        } else {
                            None
                        }
                    })
                },
            )?;

            for (f, func) in layer.iter().zip(layer_outputs.drain(..)) {
                debug_assert!(outputs[*f].is_none());
                outputs[*f] = Some(func);
            }
        }

        // Fan out in parallel again and finish compiling each function.
        engine.run_maybe_parallel(outputs.into(), |output| {
            let mut output = output.unwrap();
            inlining_compiler.finish_compiling(
                &mut output.function,
                output.func_body.take(),
                &output.symbol,
            )?;
            Ok(output)
        })
    }

    /// Implementation of our inlining heuristics.
    ///
    /// TODO: We should improve our heuristics:
    ///
    /// * One potentially promising hint that we don't currently make use of is
    ///   how many times a function appears as the callee in call sites. For
    ///   example, a function that appears in only a single call site, and does
    ///   not otherwise escape, is often beneficial to inline regardless of its
    ///   size (assuming we can then GC away the non-inlined version of the
    ///   function, which we do not currently attempt to do).
    ///
    /// * Another potentially promising hint would be whether any of the call
    ///   site's actual arguments are constants.
    ///
    /// * A general improvement would be removing the decision-tree style of
    ///   control flow below and replacing it with (1) a pure estimated-benefit
    ///   formula and (2) a benefit threshold. Whenever the estimated benefit
    ///   reaches the threshold, we would inline the call. Both the formula and
    ///   the threshold would be parameterized by tunables. This would
    ///   effectively allow reprioritizing the relative importance of different
    ///   hint sources, rather than being stuck with the sequence hard-coded in
    ///   the decision tree below.
    fn should_inline(
        InlineHeuristicParams {
            tunables,
            caller_size,
            caller_key,
            caller_needs_gc_heap,
            callee_size,
            callee_key,
            callee_needs_gc_heap,
        }: InlineHeuristicParams,
    ) -> bool {
        log::trace!(
            "considering inlining:\n\
             \tcaller = {caller_key:?}\n\
             \t\tsize = {caller_size}\n\
             \t\tneeds_gc_heap = {caller_needs_gc_heap}\n\
             \tcallee = {callee_key:?}\n\
             \t\tsize = {callee_size}\n\
             \t\tneeds_gc_heap = {callee_needs_gc_heap}"
        );

        debug_assert!(
            tunables.inlining,
            "shouldn't even call this method if we aren't configured for inlining"
        );
        debug_assert_ne!(caller_key, callee_key, "we never inline recursion");

        // Put a limit on how large we can make a function via inlining to cap
        // code bloat.
        let sum_size = caller_size.saturating_add(callee_size);
        if sum_size > tunables.inlining_sum_size_threshold {
            log::trace!(
                "  --> not inlining: the sum of the caller's and callee's sizes is greater than \
                 the inlining-sum-size threshold: {callee_size} + {caller_size} > {}",
                tunables.inlining_sum_size_threshold
            );
            return false;
        }

        // Skip inlining into array-abi functions which are entry
        // trampolines into wasm. ABI-wise it's required that these have a
        // single `try_call` into the module and it doesn't work if multiple
        // get inlined or if the `try_call` goes away. Prevent all inlining
        // to guarantee the structure of entry trampolines.
        if caller_key.abi() == Abi::Array {
            log::trace!("  --> not inlining: not inlining into array-abi caller");
            return false;
        }

        // Consider whether this is an intra-module call.
        //
        // Inlining within a single core module has most often already been done
        // by the toolchain that produced the module, e.g. LLVM, and any extant
        // function calls to small callees were presumably annotated with the
        // equivalent of `#[inline(never)]` or `#[cold]` but we don't have that
        // information anymore.
        match (caller_key, callee_key) {
            (
                FuncKey::DefinedWasmFunction(caller_module, _),
                FuncKey::DefinedWasmFunction(callee_module, _),
            ) => {
                if caller_module == callee_module {
                    match tunables.inlining_intra_module {
                        IntraModuleInlining::Yes => {}

                        IntraModuleInlining::WhenUsingGc
                            if caller_needs_gc_heap || callee_needs_gc_heap => {}

                        IntraModuleInlining::WhenUsingGc => {
                            log::trace!(
                                "  --> not inlining: intra-module call that does not use GC"
                            );
                            return false;
                        }

                        IntraModuleInlining::No => {
                            log::trace!("  --> not inlining: intra-module call");
                            return false;
                        }
                    }
                }
            }

            _ => {}
        }

        // Small callees are often worth inlining regardless of the size of the
        // caller.
        if callee_size <= tunables.inlining_small_callee_size {
            log::trace!(
                "  --> inlining: callee's size is less than the small-callee size: \
                 {callee_size} <= {}",
                tunables.inlining_small_callee_size
            );
            return true;
        }

        log::trace!("  --> inlining: did not find a reason we should not");
        true
    }
}

/// The index of a function (of any kind: Wasm function, trampoline, or
/// etc...) in our list of unlinked outputs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct OutputIndex(u32);
wasmtime_environ::entity_impl!(OutputIndex);

/// Whether a function (as described by the given `FuncKey`) can
/// participate in inlining or not (either as a candidate for being
/// inlined into a caller or having a callee inlined into a callsite
/// within itself).
fn is_inlining_function(key: FuncKey) -> bool {
    match key {
        // Wasm functions can both be inlined into other functions and
        // have other functions inlined into them.
        FuncKey::DefinedWasmFunction(..) => true,

        // Intrinsics can be inlined into other functions.
        #[cfg(feature = "component-model")]
        FuncKey::UnsafeIntrinsic(..) => true,

        // Trampolines cannot participate in inlining since our
        // unwinding and exceptions infrastructure relies on them being
        // in their own call frames.
        FuncKey::ArrayToWasmTrampoline(..)
        | FuncKey::WasmToArrayTrampoline(..)
        | FuncKey::WasmToBuiltinTrampoline(..)
        | FuncKey::PatchableToBuiltinTrampoline(..) => false,
        #[cfg(feature = "component-model")]
        FuncKey::ComponentTrampoline(..) | FuncKey::ResourceDropTrampoline => false,

        FuncKey::PulleyHostCall(_) => {
            unreachable!("we don't compile artifacts for Pulley host calls")
        }
    }
}

/// Get just the output indices of the functions that can participate in
/// inlining from our unlinked outputs.
fn inlining_functions<'a>(
    outputs: &'a PrimaryMap<OutputIndex, Option<CompileOutput<'_>>>,
) -> impl Iterator<Item = OutputIndex> + 'a {
    outputs.iter().filter_map(|(index, output)| {
        if is_inlining_function(output.as_ref().unwrap().key) {
            Some(index)
        } else {
            None
        }
    })
}

fn compile_required_builtins(engine: &Engine, raw_outputs: &mut Vec<CompileOutput>) -> Result<()> {
    let compiler = engine.try_compiler()?;
    let mut builtins = HashSet::new();
    let mut new_inputs: Vec<CompileInput<'_>> = Vec::new();

    let compile_builtin = |key: FuncKey| {
        Box::new(move |compiler: &dyn Compiler| {
            let symbol = match key {
                FuncKey::WasmToBuiltinTrampoline(builtin) => {
                    format!("wasmtime_builtin_{}", builtin.name())
                }
                FuncKey::PatchableToBuiltinTrampoline(builtin) => {
                    format!("wasmtime_patchable_builtin_{}", builtin.name())
                }
                _ => unreachable!(),
            };
            let mut function = compiler
                .compile_wasm_to_builtin(key, &symbol)
                .with_context(|| format!("failed to compile `{symbol}`"))?;
            if let Some(compiler) = compiler.inlining_compiler() {
                compiler.finish_compiling(&mut function, None, &symbol)?;
            }
            Ok(CompileOutput {
                key,
                function,
                symbol,
                start_srcloc: FilePos::default(),
                translation: None,
                func_body: None,
            })
        })
    };

    for output in raw_outputs.iter() {
        for reloc in compiler.compiled_function_relocation_targets(&*output.function.code) {
            match reloc {
                FuncKey::WasmToBuiltinTrampoline(builtin)
                | FuncKey::PatchableToBuiltinTrampoline(builtin) => {
                    if builtins.insert(builtin) {
                        new_inputs.push(compile_builtin(reloc));
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
struct UnlinkedCompileOutputs<'a> {
    // A map from kind to `CompileOutput`.
    outputs: BTreeMap<FuncKey, CompileOutput<'a>>,
}

impl UnlinkedCompileOutputs<'_> {
    /// Flatten all our functions into a single list and remember each of their
    /// indices within it.
    fn pre_link(self) -> PreLinkOutput {
        // We must ensure that `compiled_funcs` contains the function bodies
        // sorted by their `FuncKey`, as `CompiledFunctionsTable` relies on that
        // property.
        //
        // Furthermore, note that, because the order functions end up in
        // `compiled_funcs` is the order they will ultimately be laid out inside
        // the object file, we will group all trampolines together, all defined
        // Wasm functions from the same module together, and etc... This is a
        // nice property, because it means that (a) cold functions, like builtin
        // trampolines, are not interspersed between hot Wasm functions, and (b)
        // Wasm functions that are likely to call each other (i.e. are in the
        // same module together) are grouped together.
        let mut compiled_funcs = vec![];

        let mut indices = FunctionIndices::default();
        let mut needs_gc_heap = false;

        // NB: Iteration over this `BTreeMap` ensures that we uphold
        // `compiled_func`'s sorted property.
        for output in self.outputs.into_values() {
            needs_gc_heap |= output.function.needs_gc_heap;

            let index = compiled_funcs.len();
            compiled_funcs.push((output.symbol, output.key, output.function.code));

            if output.start_srcloc != FilePos::none() {
                indices
                    .start_srclocs
                    .insert(output.key, output.start_srcloc);
            }

            indices.indices.insert(output.key, index);
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
    /// The flattened list of (symbol name, FuncKey, compiled
    /// function) triples, as they will be laid out in the object
    /// file.
    compiled_funcs: Vec<(String, FuncKey, Box<dyn Any + Send + Sync>)>,
    /// The `FunctionIndices` mapping our function keys to indices in that flat
    /// list.
    indices: FunctionIndices,
}

#[derive(Default)]
struct FunctionIndices {
    // A map of wasm functions and where they're located in the original file.
    start_srclocs: HashMap<FuncKey, FilePos>,

    // The index of each compiled function in `compiled_funcs`.
    indices: BTreeMap<FuncKey, usize>,
}

impl FunctionIndices {
    /// Link the compiled functions together, resolving relocations, and append
    /// them to the given ELF file.
    fn link_and_append_code<'a>(
        self,
        mut obj: object::write::Object<'static>,
        engine: &'a Engine,
        compiled_funcs: Vec<(String, FuncKey, Box<dyn Any + Send + Sync>)>,
        translations: PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
        dwarf_package_bytes: Option<&[u8]>,
    ) -> Result<(wasmtime_environ::ObjectBuilder<'a>, Artifacts)> {
        // Append all the functions to the ELF file.
        //
        // The result is a vector parallel to `compiled_funcs` where
        // `symbol_ids_and_locs[i]` is the symbol ID and function location of
        // `compiled_funcs[i]`.
        let compiler = engine.try_compiler()?;
        let tunables = engine.tunables();
        let symbol_ids_and_locs = compiler.append_code(
            &mut obj,
            &compiled_funcs,
            &|_caller_index: usize, callee: FuncKey| {
                self.indices.get(&callee).copied().unwrap_or_else(|| {
                    panic!("cannot resolve relocation! no index for callee {callee:?}")
                })
            },
        )?;

        // If requested, generate and add DWARF information.
        if tunables.debug_native {
            compiler.append_dwarf(
                &mut obj,
                &translations,
                &|module, func| {
                    let i = self.indices[&FuncKey::DefinedWasmFunction(module, func)];
                    let (symbol, _) = symbol_ids_and_locs[i];
                    let (_, _, compiled_func) = &compiled_funcs[i];
                    (symbol, &**compiled_func)
                },
                dwarf_package_bytes,
                tunables,
            )?;
        }

        let mut table_builder = CompiledFunctionsTableBuilder::new();
        for (key, compiled_func_index) in &self.indices {
            let (_, func_loc) = symbol_ids_and_locs[*compiled_func_index];
            let src_loc = self
                .start_srclocs
                .get(key)
                .copied()
                .unwrap_or_else(FilePos::none);
            table_builder.push_func(*key, func_loc, src_loc);
        }

        let mut obj = wasmtime_environ::ObjectBuilder::new(obj, tunables);
        let modules = translations
            .into_iter()
            .map(|(_, mut translation)| {
                // If configured attempt to use static memory initialization
                // which can either at runtime be implemented as a single memcpy
                // to initialize memory or otherwise enabling
                // virtual-memory-tricks such as mmap'ing from a file to get
                // copy-on-write.
                if engine.tunables().memory_init_cow {
                    let align = compiler.page_size_align();
                    let max_always_allowed = engine.config().memory_guaranteed_dense_image_size;
                    translation.try_static_init(align, max_always_allowed);
                }

                // Attempt to convert table initializer segments to FuncTable
                // representation where possible, to enable table lazy init.
                if engine.tunables().table_lazy_init {
                    translation.try_func_table_init();
                }

                obj.append(translation)
            })
            .collect::<Result<PrimaryMap<_, _>>>()?;

        let artifacts = Artifacts {
            modules,
            table: table_builder.finish(),
        };

        Ok((obj, artifacts))
    }
}

/// The artifacts necessary for finding and calling Wasm functions at runtime,
/// to be serialized into an ELF file.
struct Artifacts {
    modules: PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
    table: CompiledFunctionsTable,
}

impl Artifacts {
    /// Assuming this compilation was for a single core Wasm module, get the
    /// resulting `CompiledModuleInfo`.
    fn unwrap_as_module_info(self) -> (CompiledModuleInfo, CompiledFunctionsTable) {
        assert_eq!(self.modules.len(), 1);
        let info = self.modules.into_iter().next().unwrap().1;
        let table = self.table;
        (info, table)
    }
}

/// Extend `dest` with `items` and return the range of indices in `dest` where
/// they ended up.
fn extend_with_range<T>(dest: &mut Vec<T>, items: impl IntoIterator<Item = T>) -> Range<u32> {
    let start = dest.len();
    let start = u32::try_from(start).unwrap();

    dest.extend(items);

    let end = dest.len();
    let end = u32::try_from(end).unwrap();

    start..end
}
