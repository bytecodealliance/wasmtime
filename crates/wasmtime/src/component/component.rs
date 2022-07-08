use crate::signatures::SignatureCollection;
use crate::{Engine, Module};
use anyhow::{bail, Context, Result};
use std::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::ops::Range;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{
    AlwaysTrapInfo, ComponentTypes, GlobalInitializer, LoweredIndex, LoweringInfo,
    RuntimeAlwaysTrapIndex, StaticModuleIndex, Translator,
};
use wasmtime_environ::{PrimaryMap, SignatureIndex, Trampoline, TrapCode};
use wasmtime_jit::CodeMemory;
use wasmtime_runtime::VMFunctionBody;

/// A compiled WebAssembly Component.
//
// FIXME: need to write more docs here.
#[derive(Clone)]
pub struct Component {
    inner: Arc<ComponentInner>,
}

struct ComponentInner {
    /// Type information calculated during translation about this component.
    component: wasmtime_environ::component::Component,

    /// Core wasm modules that the component defined internally, indexed by the
    /// compile-time-assigned `ModuleUpvarIndex`.
    static_modules: PrimaryMap<StaticModuleIndex, Module>,

    /// Registered core wasm signatures of this component, or otherwise the
    /// mapping of the component-local `SignatureIndex` to the engine-local
    /// `VMSharedSignatureIndex`.
    signatures: SignatureCollection,

    /// Type information about this component and all the various types it
    /// defines internally. All type indices for `component` will point into
    /// this field.
    types: Arc<ComponentTypes>,

    /// The in-memory ELF image of the compiled functions for this component.
    trampoline_obj: CodeMemory,

    /// The index ranges within `trampoline_obj`'s mmap memory for the entire
    /// text section.
    text: Range<usize>,

    /// Where lowered function trampolines are located within the `text`
    /// section of `trampoline_obj`.
    ///
    /// These trampolines are the function pointer within the
    /// `VMCallerCheckedAnyfunc` and will delegate indirectly to a host function
    /// pointer when called.
    lowerings: PrimaryMap<LoweredIndex, LoweringInfo>,

    /// Where the "always trap" functions are located within the `text` section
    /// of `trampoline_obj`.
    ///
    /// These functions are "degenerate functions" here solely to implement
    /// functions that are `canon lift`'d then immediately `canon lower`'d.
    always_trap: PrimaryMap<RuntimeAlwaysTrapIndex, AlwaysTrapInfo>,
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

        let mut validator =
            wasmparser::Validator::new_with_features(engine.config().features.clone());
        let mut types = Default::default();
        let (component, modules) = Translator::new(tunables, &mut validator, &mut types)
            .translate(binary)
            .context("failed to parse WebAssembly module")?;
        let types = Arc::new(types.finish());

        let provided_trampolines = modules
            .iter()
            .flat_map(|(_, m)| m.exported_signatures.iter().copied())
            .collect::<HashSet<_>>();

        let (static_modules, trampolines) = engine.join_maybe_parallel(
            // In one (possibly) parallel task all the modules found within this
            // component are compiled. Note that this will further parallelize
            // function compilation internally too.
            || -> Result<_> {
                let upvars = modules.into_iter().map(|(_, t)| t).collect::<Vec<_>>();
                let modules = engine.run_maybe_parallel(upvars, |module| {
                    let (mmap, info) =
                        Module::compile_functions(engine, module, types.module_types())?;
                    // FIXME: the `SignatureCollection` here is re-registering
                    // the entire list of wasm types within `types` on each
                    // invocation.  That's ok semantically but is quite slow to
                    // do so. This should build up a mapping from
                    // `SignatureIndex` to `VMSharedSignatureIndex` once and
                    // then reuse that for each module somehow.
                    Module::from_parts(engine, mmap, info, types.clone())
                })?;

                Ok(modules.into_iter().collect::<PrimaryMap<_, _>>())
            },
            // In another (possibly) parallel task we compile lowering
            // trampolines necessary found in the component.
            || Component::compile_component(engine, &component, &types, &provided_trampolines),
        );
        let static_modules = static_modules?;
        let (lowerings, always_trap, trampolines, trampoline_obj) = trampolines?;
        let mut trampoline_obj = CodeMemory::new(trampoline_obj);
        let code = trampoline_obj.publish()?;
        let text = wasmtime_jit::subslice_range(code.text, code.mmap);

        // This map is used to register all known tramplines in the
        // `SignatureCollection` created below. This is later consulted during
        // `ModuleRegistry::lookup_trampoline` if a trampoline needs to be
        // located for a signature index where the original function pointer
        // is that of the `trampolines` created above.
        //
        // This situation arises when a core wasm module imports a lowered
        // function and then immediately exports it. Wasmtime will lookup an
        // entry trampoline for the exported function which is actually a
        // lowered host function, hence an entry in the `trampolines` variable
        // above, and the type of that function will be stored in this
        // `vmtrampolines` map since the type is guaranteed to have escaped
        // from at least one of the modules we compiled prior.
        let mut vmtrampolines = HashMap::new();
        for (_, module) in static_modules.iter() {
            for (idx, trampoline, _) in module.compiled_module().trampolines() {
                vmtrampolines.insert(idx, trampoline);
            }
        }
        for trampoline in trampolines {
            vmtrampolines.insert(trampoline.signature, unsafe {
                let ptr =
                    code.text[trampoline.start as usize..][..trampoline.length as usize].as_ptr();
                std::mem::transmute::<*const u8, wasmtime_runtime::VMTrampoline>(ptr)
            });
        }

        // FIXME: for the same reason as above where each module is
        // re-registering everything this should only be registered once. This
        // is benign for now but could do with refactorings later on.
        let signatures = SignatureCollection::new_for_module(
            engine.signatures(),
            types.module_types(),
            vmtrampolines.into_iter(),
        );

        // Assert that this `always_trap` list is sorted which is relied on in
        // `register_component` as well as `Component::lookup_trap_code` below.
        assert!(always_trap
            .values()
            .as_slice()
            .windows(2)
            .all(|window| { window[0].start < window[1].start }));

        crate::module::register_component(code.text, &always_trap);
        Ok(Component {
            inner: Arc::new(ComponentInner {
                component,
                static_modules,
                types,
                signatures,
                trampoline_obj,
                text,
                lowerings,
                always_trap,
            }),
        })
    }

    #[cfg(compiler)]
    fn compile_component(
        engine: &Engine,
        component: &wasmtime_environ::component::Component,
        types: &ComponentTypes,
        provided_trampolines: &HashSet<SignatureIndex>,
    ) -> Result<(
        PrimaryMap<LoweredIndex, LoweringInfo>,
        PrimaryMap<RuntimeAlwaysTrapIndex, AlwaysTrapInfo>,
        Vec<Trampoline>,
        wasmtime_runtime::MmapVec,
    )> {
        let results = engine.join_maybe_parallel(
            || compile_lowerings(engine, component, types),
            || -> Result<_> {
                Ok(engine.join_maybe_parallel(
                    || compile_always_trap(engine, component, types),
                    || compile_trampolines(engine, component, types, provided_trampolines),
                ))
            },
        );
        let (lowerings, other) = results;
        let (always_trap, trampolines) = other?;
        let mut obj = engine.compiler().object()?;
        let (lower, traps, trampolines) = engine.compiler().component_compiler().emit_obj(
            lowerings?,
            always_trap?,
            trampolines?,
            &mut obj,
        )?;
        return Ok((
            lower,
            traps,
            trampolines,
            wasmtime_jit::mmap_vec_from_obj(obj)?,
        ));

        fn compile_lowerings(
            engine: &Engine,
            component: &wasmtime_environ::component::Component,
            types: &ComponentTypes,
        ) -> Result<PrimaryMap<LoweredIndex, Box<dyn Any + Send>>> {
            let lowerings = component
                .initializers
                .iter()
                .filter_map(|init| match init {
                    GlobalInitializer::LowerImport(i) => Some(i),
                    _ => None,
                })
                .collect::<Vec<_>>();
            Ok(engine
                .run_maybe_parallel(lowerings, |lowering| {
                    engine
                        .compiler()
                        .component_compiler()
                        .compile_lowered_trampoline(&component, lowering, &types)
                })?
                .into_iter()
                .collect())
        }

        fn compile_always_trap(
            engine: &Engine,
            component: &wasmtime_environ::component::Component,
            types: &ComponentTypes,
        ) -> Result<PrimaryMap<RuntimeAlwaysTrapIndex, Box<dyn Any + Send>>> {
            let always_trap = component
                .initializers
                .iter()
                .filter_map(|init| match init {
                    GlobalInitializer::AlwaysTrap(i) => Some(i),
                    _ => None,
                })
                .collect::<Vec<_>>();
            Ok(engine
                .run_maybe_parallel(always_trap, |info| {
                    engine
                        .compiler()
                        .component_compiler()
                        .compile_always_trap(&types[info.canonical_abi])
                })?
                .into_iter()
                .collect())
        }

        fn compile_trampolines(
            engine: &Engine,
            component: &wasmtime_environ::component::Component,
            types: &ComponentTypes,
            provided_trampolines: &HashSet<SignatureIndex>,
        ) -> Result<Vec<(SignatureIndex, Box<dyn Any + Send>)>> {
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
            let required_trampolines = component
                .initializers
                .iter()
                .filter_map(|init| match init {
                    GlobalInitializer::LowerImport(i) => Some(i.canonical_abi),
                    GlobalInitializer::AlwaysTrap(i) => Some(i.canonical_abi),
                    _ => None,
                })
                .collect::<HashSet<_>>();
            let mut trampolines_to_compile = required_trampolines
                .difference(&provided_trampolines)
                .collect::<Vec<_>>();
            // Ensure a deterministically compiled artifact by sorting this list
            // which was otherwise created with nondeterministically ordered hash
            // tables.
            trampolines_to_compile.sort();
            engine.run_maybe_parallel(trampolines_to_compile.clone(), |i| {
                let ty = &types[*i];
                Ok((*i, engine.compiler().compile_host_to_wasm_trampoline(ty)?))
            })
        }
    }

    pub(crate) fn env_component(&self) -> &wasmtime_environ::component::Component {
        &self.inner.component
    }

    pub(crate) fn static_module(&self, idx: StaticModuleIndex) -> &Module {
        &self.inner.static_modules[idx]
    }

    pub(crate) fn types(&self) -> &Arc<ComponentTypes> {
        &self.inner.types
    }

    pub(crate) fn signatures(&self) -> &SignatureCollection {
        &self.inner.signatures
    }

    pub(crate) fn text(&self) -> &[u8] {
        self.inner.text()
    }

    pub(crate) fn lowering_ptr(&self, index: LoweredIndex) -> NonNull<VMFunctionBody> {
        let info = &self.inner.lowerings[index];
        self.func(info.start, info.length)
    }

    pub(crate) fn always_trap_ptr(&self, index: RuntimeAlwaysTrapIndex) -> NonNull<VMFunctionBody> {
        let info = &self.inner.always_trap[index];
        self.func(info.start, info.length)
    }

    fn func(&self, start: u32, len: u32) -> NonNull<VMFunctionBody> {
        let text = self.text();
        let trampoline = &text[start as usize..][..len as usize];
        NonNull::new(trampoline.as_ptr() as *mut VMFunctionBody).unwrap()
    }

    /// Looks up a trap code for the instruction at `offset` where the offset
    /// specified is relative to the start of this component's text section.
    pub(crate) fn lookup_trap_code(&self, offset: usize) -> Option<TrapCode> {
        let offset = u32::try_from(offset).ok()?;
        // Currently traps only come from "always trap" adapters so that map is
        // the only map that's searched.
        match self
            .inner
            .always_trap
            .values()
            .as_slice()
            .binary_search_by_key(&offset, |info| info.start + info.trap_offset)
        {
            Ok(_) => Some(TrapCode::AlwaysTrapAdapter),
            Err(_) => None,
        }
    }
}

impl ComponentInner {
    fn text(&self) -> &[u8] {
        &self.trampoline_obj.mmap()[self.text.clone()]
    }
}

impl Drop for ComponentInner {
    fn drop(&mut self) {
        crate::module::unregister_component(self.text());
    }
}
