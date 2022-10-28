use crate::code::CodeObject;
use crate::signatures::SignatureCollection;
use crate::{Engine, Module};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::mem;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{
    ComponentTypes, GlobalInitializer, LoweredIndex, RuntimeAlwaysTrapIndex,
    RuntimeTranscoderIndex, StaticModuleIndex, Translator,
};
use wasmtime_environ::{EntityRef, FunctionLoc, PrimaryMap, ScopeVec, SignatureIndex};
use wasmtime_jit::{CodeMemory, CompiledModuleInfo};
use wasmtime_runtime::{MmapVec, VMFunctionBody, VMTrampoline};

/// A compiled WebAssembly Component.
//
// FIXME: need to write more docs here.
#[derive(Clone)]
pub struct Component {
    inner: Arc<ComponentInner>,
}

struct ComponentInner {
    /// Core wasm modules that the component defined internally, indexed by the
    /// compile-time-assigned `ModuleUpvarIndex`.
    static_modules: PrimaryMap<StaticModuleIndex, Module>,

    /// Code-related information such as the compiled artifact, type
    /// information, etc.
    ///
    /// Note that the `Arc` here is used to share this allocation with internal
    /// modules.
    code: Arc<CodeObject>,

    /// Metadata produced during compilation.
    info: CompiledComponentInfo,
}

#[derive(Serialize, Deserialize)]
struct CompiledComponentInfo {
    /// Type information calculated during translation about this component.
    component: wasmtime_environ::component::Component,

    /// Where lowered function trampolines are located within the `text`
    /// section of `code_memory`.
    ///
    /// These trampolines are the function pointer within the
    /// `VMCallerCheckedAnyfunc` and will delegate indirectly to a host function
    /// pointer when called.
    lowerings: PrimaryMap<LoweredIndex, FunctionLoc>,

    /// Where the "always trap" functions are located within the `text` section
    /// of `code_memory`.
    ///
    /// These functions are "degenerate functions" here solely to implement
    /// functions that are `canon lift`'d then immediately `canon lower`'d. The
    /// `u32` value here is the offset of the trap instruction from the start fo
    /// the function.
    always_trap: PrimaryMap<RuntimeAlwaysTrapIndex, FunctionLoc>,

    /// Where all the cranelift-generated transcode functions are located in the
    /// compiled image of this component.
    transcoders: PrimaryMap<RuntimeTranscoderIndex, FunctionLoc>,

    /// Extra trampolines other than those contained in static modules
    /// necessary for this component.
    trampolines: Vec<(SignatureIndex, FunctionLoc)>,
}

impl Component {
    /// Compiles a new WebAssembly component from the in-memory wasm image
    /// provided.
    //
    // FIXME: need to write more docs here.
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn new(engine: &Engine, bytes: impl AsRef<[u8]>) -> Result<Component> {
        let bytes = bytes.as_ref();
        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(bytes)?;
        Component::from_binary(engine, &bytes)
    }

    /// Compiles a new WebAssembly component from a wasm file on disk pointed to
    /// by `file`.
    //
    // FIXME: need to write more docs here.
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn from_file(engine: &Engine, file: impl AsRef<Path>) -> Result<Component> {
        match Self::new(
            engine,
            &fs::read(&file).with_context(|| "failed to read input file")?,
        ) {
            Ok(m) => Ok(m),
            Err(e) => {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "wat")] {
                        let mut e = e.downcast::<wat::Error>()?;
                        e.set_path(file);
                        bail!(e)
                    } else {
                        Err(e)
                    }
                }
            }
        }
    }

    /// Compiles a new WebAssembly component from the in-memory wasm image
    /// provided.
    //
    // FIXME: need to write more docs here.
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn from_binary(engine: &Engine, binary: &[u8]) -> Result<Component> {
        engine
            .check_compatible_with_native_host()
            .context("compilation settings are not compatible with the native host")?;

        let tunables = &engine.config().tunables;
        let compiler = engine.compiler();

        let scope = ScopeVec::new();
        let mut validator =
            wasmparser::Validator::new_with_features(engine.config().features.clone());
        let mut types = Default::default();
        let (component, mut modules) =
            Translator::new(tunables, &mut validator, &mut types, &scope)
                .translate(binary)
                .context("failed to parse WebAssembly module")?;
        let types = Arc::new(types.finish());

        // Compile all core wasm modules, in parallel, which will internally
        // compile all their functions in parallel as well.
        let module_funcs = engine.run_maybe_parallel(modules.values_mut().collect(), |module| {
            Module::compile_functions(engine, module, types.module_types())
        })?;

        // Compile all host-to-wasm trampolines where the required set of
        // trampolines is unioned from all core wasm modules plus what the
        // component itself needs.
        let module_trampolines = modules
            .iter()
            .flat_map(|(_, m)| m.exported_signatures.iter().copied())
            .collect::<BTreeSet<_>>();
        let trampolines = module_trampolines
            .iter()
            .copied()
            .chain(
                // All lowered functions will require a trampoline to be available in
                // case they're used when entering wasm. For example a lowered function
                // could be immediately lifted in which case we'll need a trampoline to
                // call that lowered function.
                //
                // Most of the time trampolines can come from the core wasm modules
                // since lifted functions come from core wasm. For these esoteric cases
                // though we may have to compile trampolines specifically into the
                // component object as well in case core wasm doesn't provide the
                // necessary trampoline.
                component.initializers.iter().filter_map(|init| match init {
                    GlobalInitializer::LowerImport(i) => Some(i.canonical_abi),
                    GlobalInitializer::AlwaysTrap(i) => Some(i.canonical_abi),
                    _ => None,
                }),
            )
            .collect::<BTreeSet<_>>();
        let compiled_trampolines = engine
            .run_maybe_parallel(trampolines.iter().cloned().collect(), |i| {
                compiler.compile_host_to_wasm_trampoline(&types[i])
            })?;

        // Compile all transcoders required which adapt from a
        // core-wasm-specific ABI (e.g. 32 or 64-bit) into the host transcoder
        // ABI through an indirect libcall.
        let transcoders = component
            .initializers
            .iter()
            .filter_map(|init| match init {
                GlobalInitializer::Transcoder(i) => Some(i),
                _ => None,
            })
            .collect();
        let transcoders = engine.run_maybe_parallel(transcoders, |info| {
            compiler
                .component_compiler()
                .compile_transcoder(&component, info, &types)
        })?;

        // Compile all "always trap" functions which are small typed shims that
        // exits to solely trap immediately for components.
        let always_trap = component
            .initializers
            .iter()
            .filter_map(|init| match init {
                GlobalInitializer::AlwaysTrap(i) => Some(i),
                _ => None,
            })
            .collect();
        let always_trap = engine.run_maybe_parallel(always_trap, |info| {
            compiler
                .component_compiler()
                .compile_always_trap(&types[info.canonical_abi])
        })?;

        // Compile all "lowerings" which are adapters that go from core wasm
        // into the host which will process the canonical ABI.
        let lowerings = component
            .initializers
            .iter()
            .filter_map(|init| match init {
                GlobalInitializer::LowerImport(i) => Some(i),
                _ => None,
            })
            .collect();
        let lowerings = engine.run_maybe_parallel(lowerings, |lowering| {
            compiler
                .component_compiler()
                .compile_lowered_trampoline(&component, lowering, &types)
        })?;

        // Collect the results of all of the function-based compilations above
        // into one large list of functions to get appended into the text
        // section of the final module.
        let mut funcs = Vec::new();
        let mut module_func_start_index = Vec::new();
        let mut func_index_to_module_index = Vec::new();
        let mut func_infos = Vec::new();
        for (i, list) in module_funcs.into_iter().enumerate() {
            module_func_start_index.push(func_index_to_module_index.len());
            let mut infos = Vec::new();
            for (j, (info, func)) in list.into_iter().enumerate() {
                func_index_to_module_index.push(i);
                let name = format!("_wasm{i}_function{j}");
                funcs.push((name, func));
                infos.push(info);
            }
            func_infos.push(infos);
        }
        for (sig, func) in trampolines.iter().zip(compiled_trampolines) {
            let name = format!("_wasm_trampoline{}", sig.as_u32());
            funcs.push((name, func));
        }
        let ntranscoders = transcoders.len();
        for (i, func) in transcoders.into_iter().enumerate() {
            let name = format!("_wasm_component_transcoder{i}");
            funcs.push((name, func));
        }
        let nalways_trap = always_trap.len();
        for (i, func) in always_trap.into_iter().enumerate() {
            let name = format!("_wasm_component_always_trap{i}");
            funcs.push((name, func));
        }
        let nlowerings = lowerings.len();
        for (i, func) in lowerings.into_iter().enumerate() {
            let name = format!("_wasm_component_lowering{i}");
            funcs.push((name, func));
        }

        let mut object = compiler.object()?;
        let locs = compiler.append_code(&mut object, &funcs, tunables, &|i, idx| {
            // Map from the `i`th function which is requesting the relocation to
            // the index in `modules` that the function belongs to. Using that
            // metadata we can resolve `idx: FuncIndex` to a `DefinedFuncIndex`
            // to the index of that module's function that's being called.
            //
            // Note that this will panic if `i` is a function beyond the initial
            // set of core wasm module functions. That's intentional, however,
            // since trampolines and otherwise should not have relocations to
            // resolve.
            let module_index = func_index_to_module_index[i];
            let defined_index = modules[StaticModuleIndex::new(module_index)]
                .module
                .defined_func_index(idx)
                .unwrap();
            // Additionally use the module index to determine where that
            // module's list of functions started at to factor in as an offset
            // as well.
            let offset = module_func_start_index[module_index];
            defined_index.index() + offset
        })?;
        engine.append_bti(&mut object);

        // Disassemble the result of the appending to the text section, where
        // each function is in the module, into respective maps.
        let mut locs = locs.into_iter().map(|(_sym, loc)| loc);
        let funcs = func_infos
            .into_iter()
            .map(|infos| {
                infos
                    .into_iter()
                    .zip(&mut locs)
                    .collect::<PrimaryMap<_, _>>()
            })
            .collect::<Vec<_>>();
        let signature_to_trampoline = trampolines
            .iter()
            .cloned()
            .zip(&mut locs)
            .collect::<HashMap<_, _>>();
        let transcoders = locs
            .by_ref()
            .take(ntranscoders)
            .collect::<PrimaryMap<RuntimeTranscoderIndex, _>>();
        let always_trap = locs
            .by_ref()
            .take(nalways_trap)
            .collect::<PrimaryMap<RuntimeAlwaysTrapIndex, _>>();
        let lowerings = locs
            .by_ref()
            .take(nlowerings)
            .collect::<PrimaryMap<LoweredIndex, _>>();
        assert!(locs.next().is_none());

        // Convert all `ModuleTranslation` instances into `CompiledModuleInfo`
        // through an `ObjectBuilder` here. This is then used to create the
        // final `mmap` which is the final compilation artifact.
        let mut builder = wasmtime_jit::ObjectBuilder::new(object, tunables);
        let mut static_modules = PrimaryMap::new();
        for ((_, module), funcs) in modules.into_iter().zip(funcs) {
            // Build the list of trampolines for this module from its set of
            // exported signatures, which is the list of expected trampolines,
            // from the set of trampolines that were compiled for everything
            // within this component.
            let trampolines = module
                .exported_signatures
                .iter()
                .map(|sig| (*sig, signature_to_trampoline[sig]))
                .collect();
            let info = builder.append(module, funcs, trampolines)?;
            static_modules.push(info);
        }
        let mmap = builder.finish()?;

        // Now that all of the AOT artifacts are prepared delegate to the
        // implementation of assembling AOT artifacts into a final `Component`.
        let info = CompiledComponentInfo {
            always_trap,
            component,
            lowerings,
            trampolines: trampolines
                .difference(&module_trampolines)
                .map(|i| (*i, signature_to_trampoline[i]))
                .collect(),
            transcoders,
        };
        Component::from_parts(engine, mmap, info, types, static_modules)
    }

    fn from_parts(
        engine: &Engine,
        mmap: MmapVec,
        info: CompiledComponentInfo,
        types: Arc<ComponentTypes>,
        static_modules: PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
    ) -> Result<Component> {
        let mut code_memory = CodeMemory::new(mmap)?;
        code_memory.publish()?;
        let code_memory = Arc::new(code_memory);

        let signatures = SignatureCollection::new_for_module(
            engine.signatures(),
            types.module_types(),
            static_modules
                .iter()
                .flat_map(|(_, m)| m.trampolines.iter().copied())
                .chain(info.trampolines.iter().copied())
                .map(|(sig, loc)| {
                    let trampoline = code_memory.text()[loc.start as usize..].as_ptr();
                    (sig, unsafe {
                        mem::transmute::<*const u8, VMTrampoline>(trampoline)
                    })
                }),
        );
        let code = Arc::new(CodeObject::new(code_memory, signatures, types.into()));
        let static_modules = static_modules
            .into_iter()
            .map(|(_, info)| Module::from_parts_raw(engine, code.clone(), info))
            .collect::<Result<_>>()?;
        Ok(Component {
            inner: Arc::new(ComponentInner {
                static_modules,
                code,
                info,
            }),
        })
    }

    pub(crate) fn env_component(&self) -> &wasmtime_environ::component::Component {
        &self.inner.info.component
    }

    pub(crate) fn static_module(&self, idx: StaticModuleIndex) -> &Module {
        &self.inner.static_modules[idx]
    }

    pub(crate) fn types(&self) -> &Arc<ComponentTypes> {
        match self.inner.code.types() {
            crate::code::Types::Component(types) => types,
            // The only creator of a `Component` is itself which uses the other
            // variant, so this shouldn't be possible.
            crate::code::Types::Module(_) => unreachable!(),
        }
    }

    pub(crate) fn signatures(&self) -> &SignatureCollection {
        self.inner.code.signatures()
    }

    pub(crate) fn text(&self) -> &[u8] {
        self.inner.code.code_memory().text()
    }

    pub(crate) fn lowering_ptr(&self, index: LoweredIndex) -> NonNull<VMFunctionBody> {
        let info = &self.inner.info.lowerings[index];
        self.func(info)
    }

    pub(crate) fn always_trap_ptr(&self, index: RuntimeAlwaysTrapIndex) -> NonNull<VMFunctionBody> {
        let loc = &self.inner.info.always_trap[index];
        self.func(loc)
    }

    pub(crate) fn transcoder_ptr(&self, index: RuntimeTranscoderIndex) -> NonNull<VMFunctionBody> {
        let info = &self.inner.info.transcoders[index];
        self.func(info)
    }

    fn func(&self, loc: &FunctionLoc) -> NonNull<VMFunctionBody> {
        let text = self.text();
        let trampoline = &text[loc.start as usize..][..loc.length as usize];
        NonNull::new(trampoline.as_ptr() as *mut VMFunctionBody).unwrap()
    }

    pub(crate) fn code_object(&self) -> &Arc<CodeObject> {
        &self.inner.code
    }
}
