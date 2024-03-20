#[cfg(all(feature = "runtime", feature = "component-model"))]
use crate::component::Component;
use crate::Engine;
#[cfg(feature = "runtime")]
use crate::{instantiate::MmapVecWrapper, CodeMemory, Module};
use anyhow::{anyhow, bail, Context, Result};
use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;
use wasmtime_environ::ObjectKind;
#[cfg(feature = "runtime")]
use wasmtime_runtime::MmapVec;

/// Builder-style structure used to create a [`Module`](crate::Module) or
/// pre-compile a module to a serialized list of bytes.
///
/// This structure can be used for more advanced configuration when compiling a
/// WebAssembly module. Most configuration can use simpler constructors such as:
///
/// * [`Module::new`](crate::Module::new)
/// * [`Module::from_file`](crate::Module::from_file)
/// * [`Module::from_binary`](crate::Module::from_binary)
///
/// Note that a [`CodeBuilder`] always involves compiling WebAssembly bytes
/// to machine code. To deserialize a list of bytes use
/// [`Module::deserialize`](crate::Module::deserialize) instead.
///
/// A [`CodeBuilder`] requires a source of WebAssembly bytes to be configured
/// before calling [`compile_module_serialized`] or [`compile_module`]. This can be
/// provided with either the [`wasm`] or [`wasm_file`] method. Note that only
/// a single source of bytes can be provided.
///
/// # WebAssembly Text Format
///
/// This builder supports the WebAssembly Text Format (`*.wat` files).
/// WebAssembly text files are automatically converted to a WebAssembly binary
/// and then the binary is compiled. This requires the `wat` feature of the
/// `wasmtime` crate to be enabled, and the feature is enabled by default.
///
/// If the text format is not desired then the [`CodeBuilder::wat`] method
/// can be used to disable this conversion.
///
/// [`compile_module_serialized`]: CodeBuilder::compile_module_serialized
/// [`compile_module`]: CodeBuilder::compile_module
/// [`wasm`]: CodeBuilder::wasm
/// [`wasm_file`]: CodeBuilder::wasm_file
pub struct CodeBuilder<'a> {
    engine: &'a Engine,
    wasm: Option<Cow<'a, [u8]>>,
    wasm_path: Option<Cow<'a, Path>>,
    wat: bool,
}

impl<'a> CodeBuilder<'a> {
    /// Creates a new builder which will insert modules into the specified
    /// [`Engine`].
    pub fn new(engine: &'a Engine) -> CodeBuilder<'a> {
        CodeBuilder {
            engine,
            wasm: None,
            wasm_path: None,
            wat: cfg!(feature = "wat"),
        }
    }

    /// Configures the WebAssembly binary or text that is being compiled.
    ///
    /// The `wasm_bytes` parameter is either a binary WebAssembly file or a
    /// WebAssembly module in its text format. This will be stored within the
    /// [`CodeBuilder`] for processing later when compilation is finalized.
    ///
    /// The optional `wasm_path` parameter is the path to the `wasm_bytes` on
    /// disk, if any. This may be used for diagnostics and other
    /// debugging-related purposes, but this method will not read the path
    /// specified.
    ///
    /// # Errors
    ///
    /// If wasm bytes have already been configured via a call to this method or
    /// [`CodeBuilder::wasm_file`] then an error will be returned.
    pub fn wasm(&mut self, wasm_bytes: &'a [u8], wasm_path: Option<&'a Path>) -> Result<&mut Self> {
        if self.wasm.is_some() {
            bail!("cannot call `wasm` or `wasm_file` twice");
        }
        self.wasm = Some(wasm_bytes.into());
        self.wasm_path = wasm_path.map(|p| p.into());
        Ok(self)
    }

    /// Configures whether the WebAssembly text format is supported in this
    /// builder.
    ///
    /// This support is enabled by default if the `wat` crate feature is also
    /// enabled.
    ///
    /// # Errors
    ///
    /// If this feature is explicitly enabled here via this method and the
    /// `wat` crate feature is disabled then an error will be returned.
    pub fn wat(&mut self, enable: bool) -> Result<&mut Self> {
        if !cfg!(feature = "wat") && enable {
            bail!("support for `wat` was disabled at compile time");
        }
        self.wat = enable;
        Ok(self)
    }

    /// Reads the `file` specified for the WebAssembly bytes that are going to
    /// be compiled.
    ///
    /// This method will read `file` from the filesystem and interpret it
    /// either as a WebAssembly binary or as a WebAssembly text file. The
    /// contents are inspected to do this, the file extension is not consulted.
    ///
    /// # Errors
    ///
    /// If wasm bytes have already been configured via a call to this method or
    /// [`CodeBuilder::wasm`] then an error will be returned.
    ///
    /// If `file` can't be read or an error happens reading it then that will
    /// also be returned.
    pub fn wasm_file(&mut self, file: &'a Path) -> Result<&mut Self> {
        if self.wasm.is_some() {
            bail!("cannot call `wasm` or `wasm_file` twice");
        }
        let wasm = std::fs::read(file)
            .with_context(|| format!("failed to read input file: {}", file.display()))?;
        self.wasm = Some(wasm.into());
        self.wasm_path = Some(file.into());
        Ok(self)
    }

    fn wasm_binary(&self) -> Result<Cow<'_, [u8]>> {
        let wasm = self
            .wasm
            .as_ref()
            .ok_or_else(|| anyhow!("no wasm bytes have been configured"))?;
        if self.wat {
            #[cfg(feature = "wat")]
            return wat::parse_bytes(wasm).map_err(|mut e| {
                if let Some(path) = &self.wasm_path {
                    e.set_path(path);
                }
                e.into()
            });
        }
        Ok((&wasm[..]).into())
    }

    #[cfg(feature = "runtime")]
    fn compile_cached<T>(
        &self,
        build_artifacts: fn(&Engine, &[u8]) -> Result<(MmapVecWrapper, Option<T>)>,
    ) -> Result<(Arc<CodeMemory>, Option<T>)> {
        let wasm = self.wasm_binary()?;

        self.engine
            .check_compatible_with_native_host()
            .context("compilation settings are not compatible with the native host")?;

        #[cfg(feature = "cache")]
        {
            let state = (
                HashedEngineCompileEnv(self.engine),
                &wasm,
                // Don't hash this as it's just its own "pure" function pointer.
                NotHashed(build_artifacts),
            );
            let (code, info_and_types) =
                wasmtime_cache::ModuleCacheEntry::new("wasmtime", self.engine.cache_config())
                    .get_data_raw(
                        &state,
                        // Cache miss, compute the actual artifacts
                        |(engine, wasm, build_artifacts)| -> Result<_> {
                            let (mmap, info) = (build_artifacts.0)(engine.0, wasm)?;
                            let code = publish_mmap(mmap.0)?;
                            Ok((code, info))
                        },
                        // Implementation of how to serialize artifacts
                        |(_engine, _wasm, _), (code, _info_and_types)| Some(code.mmap().to_vec()),
                        // Cache hit, deserialize the provided artifacts
                        |(engine, _wasm, _), serialized_bytes| {
                            let code = engine
                                .0
                                .load_code_bytes(&serialized_bytes, ObjectKind::Module)
                                .ok()?;
                            Some((code, None))
                        },
                    )?;
            return Ok((code, info_and_types));
        }

        #[cfg(not(feature = "cache"))]
        {
            let (mmap, info_and_types) = build_artifacts(self.engine, &wasm)?;
            let code = publish_mmap(mmap.0)?;
            return Ok((code, info_and_types));
        }

        struct NotHashed<T>(T);

        impl<T> std::hash::Hash for NotHashed<T> {
            fn hash<H: std::hash::Hasher>(&self, _hasher: &mut H) {}
        }
    }

    /// Finishes this compilation and produces a serialized list of bytes.
    ///
    /// This method requires that either [`CodeBuilder::wasm`] or
    /// [`CodeBuilder::wasm_file`] was invoked prior to indicate what is
    /// being compiled.
    ///
    /// This method will block the current thread until compilation has
    /// finished, and when done the serialized artifact will be returned.
    ///
    /// Note that this method will never cache compilations, even if the
    /// `cache` feature is enabled.
    ///
    /// # Errors
    ///
    /// This can fail if the input wasm module was not valid or if another
    /// compilation-related error is encountered.
    pub fn compile_module_serialized(&self) -> Result<Vec<u8>> {
        let wasm = self.wasm_binary()?;
        let (v, _) = super::build_artifacts(self.engine, &wasm)?;
        Ok(v)
    }

    /// Same as [`CodeBuilder::compile_module_serialized`] except that a
    /// [`Module`](crate::Module) is produced instead.
    ///
    /// Note that this method will cache compilations if the `cache` feature is
    /// enabled and turned on in [`Config`](crate::Config).
    #[cfg(feature = "runtime")]
    #[cfg_attr(docsrs, doc(cfg(feature = "runtime")))]
    pub fn compile_module(&self) -> Result<Module> {
        let (code, info_and_types) = self.compile_cached(super::build_artifacts)?;
        Module::from_parts(self.engine, code, info_and_types)
    }

    /// Same as [`CodeBuilder::compile_module_serialized`] except that it
    /// compiles a serialized [`Component`] instead of a module.
    #[cfg(feature = "component-model")]
    #[cfg_attr(docsrs, doc(cfg(feature = "component-model")))]
    pub fn compile_component_serialized(&self) -> Result<Vec<u8>> {
        let bytes = self.wasm_binary()?;
        let (v, _) = super::build_component_artifacts(self.engine, &bytes)?;
        Ok(v)
    }

    /// Same as [`CodeBuilder::compile_module`] except that it compiles a
    /// [`Component`] instead of a module.
    #[cfg(all(feature = "runtime", feature = "component-model"))]
    #[cfg_attr(
        docsrs,
        doc(cfg(all(feature = "runtime", feature = "component-model")))
    )]
    pub fn compile_component(&self) -> Result<Component> {
        let (code, artifacts) = self.compile_cached(super::build_component_artifacts)?;
        Component::from_parts(self.engine, code, artifacts)
    }
}

/// This is a helper struct used when caching to hash the state of an `Engine`
/// used for module compilation.
///
/// The hash computed for this structure is used to key the global wasmtime
/// cache and dictates whether artifacts are reused. Consequently the contents
/// of this hash dictate when artifacts are or aren't re-used.
pub struct HashedEngineCompileEnv<'a>(pub &'a Engine);

impl std::hash::Hash for HashedEngineCompileEnv<'_> {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        // Hash the compiler's state based on its target and configuration.
        let compiler = self.0.compiler();
        compiler.triple().hash(hasher);
        compiler.flags().hash(hasher);
        compiler.isa_flags().hash(hasher);

        // Hash configuration state read for compilation
        let config = self.0.config();
        self.0.tunables().hash(hasher);
        config.features.hash(hasher);
        config.wmemcheck.hash(hasher);

        // Catch accidental bugs of reusing across crate versions.
        config.module_version.hash(hasher);
    }
}

#[cfg(feature = "runtime")]
fn publish_mmap(mmap: MmapVec) -> Result<Arc<CodeMemory>> {
    let mut code = CodeMemory::new(mmap)?;
    code.publish()?;
    Ok(Arc::new(code))
}
