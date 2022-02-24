use crate::{Engine, Module};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use wasmtime_environ::component::{ComponentTypes, ModuleUpvarIndex, Translation, Translator};
use wasmtime_environ::PrimaryMap;

/// A compiled WebAssembly Component.
//
// FIXME: need to write more docs here.
#[derive(Clone)]
pub struct Component {
    inner: Arc<ComponentInner>,
}

struct ComponentInner {
    component: wasmtime_environ::component::Component,
    upvars: PrimaryMap<ModuleUpvarIndex, Module>,
    types: Arc<ComponentTypes>,
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
        let upvars = upvars.into_iter().map(|(_, t)| t).collect::<Vec<_>>();
        let upvars = engine
            .run_maybe_parallel(upvars, |module| {
                let (mmap, info) = Module::compile_functions(engine, module, types.module_types())?;
                // FIXME: the `SignatureCollection` here is re-registering the
                // entire list of wasm types within `types` on each invocation.
                // That's ok semantically but is quite slow to do so. This
                // should build up a mapping from `SignatureIndex` to
                // `VMSharedSignatureIndex` once and then reuse that for each
                // module somehow.
                Module::from_parts(engine, mmap, info, types.clone())
            })?
            .into_iter()
            .collect();

        Ok(Component {
            inner: Arc::new(ComponentInner {
                component,
                upvars,
                types,
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
}
