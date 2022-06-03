use crate::signatures::SignatureCollection;
use crate::{Engine, Module};
use anyhow::{bail, Context, Result};
use std::fs;
use std::ops::Range;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{
    ComponentTypes, Initializer, LoweredIndex, ModuleUpvarIndex, TrampolineInfo, Translation,
    Translator,
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
    upvars: PrimaryMap<ModuleUpvarIndex, Module>,

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
    trampolines: PrimaryMap<LoweredIndex, TrampolineInfo>,
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
        let translation = Translator::new(tunables, &mut validator, &mut types)
            .translate(binary)
            .context("failed to parse WebAssembly module")?;
        let types = Arc::new(types.finish());

        let Translation {
            component, upvars, ..
        } = translation;
        let (upvars, trampolines) = engine.join_maybe_parallel(
            // In one (possibly) parallel task all the modules found within this
            // component are compiled. Note that this will further parallelize
            // function compilation internally too.
            || -> Result<_> {
                let upvars = upvars.into_iter().map(|(_, t)| t).collect::<Vec<_>>();
                let modules = engine.run_maybe_parallel(upvars, |module| {
                    let (mmap, info) =
                        Module::compile_functions(engine, module, types.module_types())?;
                    // FIXME: the `SignatureCollection` here is re-registering the
                    // entire list of wasm types within `types` on each invocation.
                    // That's ok semantically but is quite slow to do so. This
                    // should build up a mapping from `SignatureIndex` to
                    // `VMSharedSignatureIndex` once and then reuse that for each
                    // module somehow.
                    Module::from_parts(engine, mmap, info, types.clone())
                })?;

                Ok(modules.into_iter().collect::<PrimaryMap<_, _>>())
            },
            // In another (possibly) parallel task we compile lowering
            // trampolines necessary found in the component.
            || -> Result<_> {
                let lowerings = component
                    .initializers
                    .iter()
                    .filter_map(|init| match init {
                        Initializer::LowerImport(i) => Some(i),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let compiler = engine.compiler().component_compiler();
                let trampolines = engine
                    .run_maybe_parallel(lowerings, |lowering| {
                        compiler.compile_lowered_trampoline(&component, lowering, &types)
                    })?
                    .into_iter()
                    .collect();
                let mut obj = engine.compiler().object()?;
                let trampolines = compiler.emit_obj(trampolines, &mut obj)?;
                Ok((trampolines, wasmtime_jit::mmap_vec_from_obj(obj)?))
            },
        );
        let upvars = upvars?;
        let (trampolines, trampoline_obj) = trampolines?;
        let mut trampoline_obj = CodeMemory::new(trampoline_obj);
        let code = trampoline_obj.publish()?;
        let text = wasmtime_jit::subslice_range(code.text, code.mmap);

        // FIXME: for the same reason as above where each module is
        // re-registering everything this should only be registered once. This
        // is benign for now but could do with refactorings later on.
        let signatures = SignatureCollection::new_for_module(
            engine.signatures(),
            types.module_types(),
            [].into_iter(),
        );

        Ok(Component {
            inner: Arc::new(ComponentInner {
                component,
                upvars,
                types,
                trampolines,
                trampoline_obj,
                text,
                signatures,
            }),
        })
    }

    pub(crate) fn env_component(&self) -> &wasmtime_environ::component::Component {
        &self.inner.component
    }

    pub(crate) fn upvar(&self, idx: ModuleUpvarIndex) -> &Module {
        &self.inner.upvars[idx]
    }

    pub(crate) fn types(&self) -> &Arc<ComponentTypes> {
        &self.inner.types
    }

    pub(crate) fn signatures(&self) -> &SignatureCollection {
        &self.inner.signatures
    }

    pub(crate) fn trampoline_ptr(&self, index: LoweredIndex) -> NonNull<VMFunctionBody> {
        let info = &self.inner.trampolines[index];
        let text = &self.inner.trampoline_obj.mmap()[self.inner.text.clone()];
        let trampoline = &text[info.start as usize..][..info.length as usize];
        NonNull::new(trampoline.as_ptr() as *mut VMFunctionBody).unwrap()
    }
}
