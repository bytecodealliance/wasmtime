use crate::signatures::SignatureCollection;
use crate::{Engine, Module};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::ops::Range;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{
    ComponentTypes, GlobalInitializer, LoweredIndex, LoweringInfo, StaticModuleIndex, Translator,
};
use wasmtime_environ::PrimaryMap;
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

    /// The in-memory ELF image of the compiled trampolines for this component.
    ///
    /// This is currently only used for wasm-to-host trampolines when
    /// `canon.lower` is encountered.
    trampoline_obj: CodeMemory,

    /// The index ranges within `trampoline_obj`'s mmap memory for the entire
    /// text section.
    text: Range<usize>,

    /// Where trampolines are located within the `text` section of
    /// `trampoline_obj`.
    trampolines: PrimaryMap<LoweredIndex, LoweringInfo>,
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
        let lowerings = component
            .initializers
            .iter()
            .filter_map(|init| match init {
                GlobalInitializer::LowerImport(i) => Some(i),
                _ => None,
            })
            .collect::<Vec<_>>();
        let required_trampolines = lowerings
            .iter()
            .map(|l| l.canonical_abi)
            .collect::<HashSet<_>>();
        let provided_trampolines = modules
            .iter()
            .flat_map(|(_, m)| m.exported_signatures.iter().copied())
            .collect::<HashSet<_>>();
        let mut trampolines_to_compile = required_trampolines
            .difference(&provided_trampolines)
            .collect::<Vec<_>>();
        // Ensure a deterministically compiled artifact by sorting this list
        // which was otherwise created with nondeterministically ordered hash
        // tables.
        trampolines_to_compile.sort();

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
            || -> Result<_> {
                let compiler = engine.compiler();
                let (lowered_trampolines, core_trampolines) = engine.join_maybe_parallel(
                    // Compile all the lowered trampolines here which implement
                    // `canon lower` and are used to exit wasm into the host.
                    || -> Result<_> {
                        Ok(engine
                            .run_maybe_parallel(lowerings, |lowering| {
                                compiler
                                    .component_compiler()
                                    .compile_lowered_trampoline(&component, lowering, &types)
                            })?
                            .into_iter()
                            .collect())
                    },
                    // Compile all entry host-to-wasm trampolines here that
                    // aren't otherwise provided by core wasm modules.
                    || -> Result<_> {
                        engine.run_maybe_parallel(trampolines_to_compile.clone(), |i| {
                            let ty = &types[*i];
                            Ok((*i, compiler.compile_host_to_wasm_trampoline(ty)?))
                        })
                    },
                );
                let mut obj = engine.compiler().object()?;
                let trampolines = compiler.component_compiler().emit_obj(
                    lowered_trampolines?,
                    core_trampolines?,
                    &mut obj,
                )?;
                Ok((trampolines, wasmtime_jit::mmap_vec_from_obj(obj)?))
            },
        );
        let static_modules = static_modules?;
        let ((lowering_trampolines, core_trampolines), trampoline_obj) = trampolines?;
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
        for (signature, trampoline) in trampolines_to_compile.iter().zip(core_trampolines) {
            vmtrampolines.insert(**signature, unsafe {
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

        Ok(Component {
            inner: Arc::new(ComponentInner {
                component,
                static_modules,
                types,
                signatures,
                trampoline_obj,
                text,
                trampolines: lowering_trampolines,
            }),
        })
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
        &self.inner.trampoline_obj.mmap()[self.inner.text.clone()]
    }

    pub(crate) fn trampoline_ptr(&self, index: LoweredIndex) -> NonNull<VMFunctionBody> {
        let info = &self.inner.trampolines[index];
        let text = self.text();
        let trampoline = &text[info.start as usize..][..info.length as usize];
        NonNull::new(trampoline.as_ptr() as *mut VMFunctionBody).unwrap()
    }
}
