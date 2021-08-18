use crate::{
    signatures::SignatureCollection,
    types::{ExportType, ExternType, ImportType},
};
use crate::{Engine, ModuleType};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use wasmparser::Validator;
#[cfg(feature = "cache")]
use wasmtime_cache::ModuleCacheEntry;
use wasmtime_environ::{ModuleIndex, PrimaryMap};
use wasmtime_jit::{CompilationArtifacts, CompiledModule, TypeTables};

mod registry;
mod serialization;

pub use registry::{FrameInfo, FrameSymbol, GlobalModuleRegistry, ModuleRegistry};
pub use serialization::SerializedModule;

/// A compiled WebAssembly module, ready to be instantiated.
///
/// A `Module` is a compiled in-memory representation of an input WebAssembly
/// binary. A `Module` is then used to create an [`Instance`](crate::Instance)
/// through an instantiation process. You cannot call functions or fetch
/// globals, for example, on a `Module` because it's purely a code
/// representation. Instead you'll need to create an
/// [`Instance`](crate::Instance) to interact with the wasm module.
///
/// Creating a `Module` currently involves compiling code, meaning that it can
/// be an expensive operation. All `Module` instances are compiled according to
/// the configuration in [`Config`], but typically they're JIT-compiled. If
/// you'd like to instantiate a module multiple times you can do so with
/// compiling the original wasm module only once with a single [`Module`]
/// instance.
///
/// The `Module` is thread-safe and safe to share across threads.
///
/// ## Modules and `Clone`
///
/// Using `clone` on a `Module` is a cheap operation. It will not create an
/// entirely new module, but rather just a new reference to the existing module.
/// In other words it's a shallow copy, not a deep copy.
///
/// ## Examples
///
/// There are a number of ways you can create a `Module`, for example pulling
/// the bytes from a number of locations. One example is loading a module from
/// the filesystem:
///
/// ```no_run
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let engine = Engine::default();
/// let module = Module::from_file(&engine, "path/to/foo.wasm")?;
/// # Ok(())
/// # }
/// ```
///
/// You can also load the wasm text format if more convenient too:
///
/// ```no_run
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let engine = Engine::default();
/// // Now we're using the WebAssembly text extension: `.wat`!
/// let module = Module::from_file(&engine, "path/to/foo.wat")?;
/// # Ok(())
/// # }
/// ```
///
/// And if you've already got the bytes in-memory you can use the
/// [`Module::new`] constructor:
///
/// ```no_run
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let engine = Engine::default();
/// # let wasm_bytes: Vec<u8> = Vec::new();
/// let module = Module::new(&engine, &wasm_bytes)?;
///
/// // It also works with the text format!
/// let module = Module::new(&engine, "(module (func))")?;
/// # Ok(())
/// # }
/// ```
///
/// [`Config`]: crate::Config
#[derive(Clone)]
pub struct Module {
    inner: Arc<ModuleInner>,
}

struct ModuleInner {
    engine: Engine,
    /// The compiled artifacts for this module that will be instantiated and
    /// executed.
    module: Arc<CompiledModule>,
    /// Closed-over compilation artifacts used to create submodules when this
    /// module is instantiated.
    artifact_upvars: Vec<Arc<CompiledModule>>,
    /// Closed-over module values which are used when this module is
    /// instantiated.
    module_upvars: Vec<Module>,
    /// Type information of this module and all `artifact_upvars` compiled
    /// modules.
    types: Arc<TypeTables>,
    /// Registered shared signature for the module.
    signatures: Arc<SignatureCollection>,
}

impl Module {
    /// Creates a new WebAssembly `Module` from the given in-memory `bytes`.
    ///
    /// The `bytes` provided must be in one of the following formats:
    ///
    /// * A [binary-encoded][binary] WebAssembly module. This is always supported.
    /// * A [text-encoded][text] instance of the WebAssembly text format.
    ///   This is only supported when the `wat` feature of this crate is enabled.
    ///   If this is supplied then the text format will be parsed before validation.
    ///   Note that the `wat` feature is enabled by default.
    ///
    /// The data for the wasm module must be loaded in-memory if it's present
    /// elsewhere, for example on disk. This requires that the entire binary is
    /// loaded into memory all at once, this API does not support streaming
    /// compilation of a module.
    ///
    /// If the module has not been already been compiled, the WebAssembly binary will
    /// be decoded and validated. It will also be compiled according to the
    /// configuration of the provided `engine`.
    ///
    /// # Errors
    ///
    /// This function may fail and return an error. Errors may include
    /// situations such as:
    ///
    /// * The binary provided could not be decoded because it's not a valid
    ///   WebAssembly binary
    /// * The WebAssembly binary may not validate (e.g. contains type errors)
    /// * Implementation-specific limits were exceeded with a valid binary (for
    ///   example too many locals)
    /// * The wasm binary may use features that are not enabled in the
    ///   configuration of `engine`
    /// * If the `wat` feature is enabled and the input is text, then it may be
    ///   rejected if it fails to parse.
    ///
    /// The error returned should contain full information about why module
    /// creation failed if one is returned.
    ///
    /// [binary]: https://webassembly.github.io/spec/core/binary/index.html
    /// [text]: https://webassembly.github.io/spec/core/text/index.html
    ///
    /// # Examples
    ///
    /// The `new` function can be invoked with a in-memory array of bytes:
    ///
    /// ```no_run
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// # let wasm_bytes: Vec<u8> = Vec::new();
    /// let module = Module::new(&engine, &wasm_bytes)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Or you can also pass in a string to be parsed as the wasm text
    /// format:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let module = Module::new(&engine, "(module (func))")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(engine: &Engine, bytes: impl AsRef<[u8]>) -> Result<Module> {
        let bytes = bytes.as_ref();
        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(bytes)?;
        Self::from_binary(engine, &bytes)
    }

    /// Creates a new WebAssembly `Module` from the given in-memory `binary`
    /// data. The provided `name` will be used in traps/backtrace details.
    ///
    /// See [`Module::new`] for other details.
    pub fn new_with_name(engine: &Engine, bytes: impl AsRef<[u8]>, name: &str) -> Result<Module> {
        let mut module = Self::new(engine, bytes.as_ref())?;
        Arc::get_mut(&mut Arc::get_mut(&mut module.inner).unwrap().module)
            .unwrap()
            .module_mut()
            .expect("mutable module")
            .name = Some(name.to_string());
        Ok(module)
    }

    /// Creates a new WebAssembly `Module` from the contents of the given
    /// `file` on disk.
    ///
    /// This is a convenience function that will read the `file` provided and
    /// pass the bytes to the [`Module::new`] function. For more information
    /// see [`Module::new`]
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let module = Module::from_file(&engine, "./path/to/foo.wasm")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// The `.wat` text format is also supported:
    ///
    /// ```no_run
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let module = Module::from_file(&engine, "./path/to/foo.wat")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file(engine: &Engine, file: impl AsRef<Path>) -> Result<Module> {
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

    /// Creates a new WebAssembly `Module` from the given in-memory `binary`
    /// data.
    ///
    /// This is similar to [`Module::new`] except that it requires that the
    /// `binary` input is a WebAssembly binary, the text format is not supported
    /// by this function. It's generally recommended to use [`Module::new`], but
    /// if it's required to not support the text format this function can be
    /// used instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let wasm = b"\0asm\x01\0\0\0";
    /// let module = Module::from_binary(&engine, wasm)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Note that the text format is **not** accepted by this function:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// assert!(Module::from_binary(&engine, b"(module)").is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_binary(engine: &Engine, binary: &[u8]) -> Result<Module> {
        // Check to see that the config's target matches the host
        let target = engine.compiler().compiler().triple();
        if *target != target_lexicon::Triple::host() {
            bail!(
                "target '{}' specified in the configuration does not match the host",
                target
            );
        }

        // FIXME: we may want to validate that the ISA flags in the config match those that
        // would be inferred for the host, otherwise the JIT might produce unrunnable code
        // for the features the host's CPU actually has.

        const USE_PAGED_MEM_INIT: bool = cfg!(all(feature = "uffd", target_os = "linux"));

        cfg_if::cfg_if! {
            if #[cfg(feature = "cache")] {
                let (main_module, artifacts, types) = ModuleCacheEntry::new(
                    "wasmtime",
                    engine.cache_config(),
                )
                .get_data((engine.compiler(), binary), |(compiler, binary)| {
                    CompilationArtifacts::build(
                        compiler,
                        binary,
                        USE_PAGED_MEM_INIT,
                    )
                })?;
            } else {
                let (main_module, artifacts, types) =
                    CompilationArtifacts::build(
                        engine.compiler(),
                        binary,
                        USE_PAGED_MEM_INIT,
                    )?;
            }
        };

        let modules = CompiledModule::from_artifacts_list(
            artifacts,
            &*engine.config().profiler,
            engine.compiler(),
        )?;

        Self::from_parts(engine, modules, main_module, Arc::new(types), &[])
    }

    /// Deserializes an in-memory compiled module previously created with
    /// [`Module::serialize`] or [`Engine::precompile_module`].
    ///
    /// This function will deserialize the binary blobs emitted by
    /// [`Module::serialize`] and [`Engine::precompile_module`] back into an
    /// in-memory [`Module`] that's ready to be instantiated.
    ///
    /// # Unsafety
    ///
    /// This function is marked as `unsafe` because if fed invalid input or used
    /// improperly this could lead to memory safety vulnerabilities. This method
    /// should not, for example, be exposed to arbitrary user input.
    ///
    /// The structure of the binary blob read here is only lightly validated
    /// internally in `wasmtime`. This is intended to be an efficient
    /// "rehydration" for a [`Module`] which has very few runtime checks beyond
    /// deserialization. Arbitrary input could, for example, replace valid
    /// compiled code with any other valid compiled code, meaning that this can
    /// trivially be used to execute arbitrary code otherwise.
    ///
    /// For these reasons this function is `unsafe`. This function is only
    /// designed to receive the previous input from [`Module::serialize`] and
    /// [`Engine::precompile_module`]. If the exact output of those functions
    /// (unmodified) is passed to this function then calls to this function can
    /// be considered safe. It is the caller's responsibility to provide the
    /// guarantee that only previously-serialized bytes are being passed in
    /// here.
    ///
    /// Note that this function is designed to be safe receiving output from
    /// *any* compiled version of `wasmtime` itself. This means that it is safe
    /// to feed output from older versions of Wasmtime into this function, in
    /// addition to newer versions of wasmtime (from the future!). These inputs
    /// will deterministically and safely produce an `Err`. This function only
    /// successfully accepts inputs from the same version of `wasmtime`, but the
    /// safety guarantee only applies to externally-defined blobs of bytes, not
    /// those defined by any version of wasmtime. (this means that if you cache
    /// blobs across versions of wasmtime you can be safely guaranteed that
    /// future versions of wasmtime will reject old cache entries).
    pub unsafe fn deserialize(engine: &Engine, bytes: impl AsRef<[u8]>) -> Result<Module> {
        let module = SerializedModule::from_bytes(
            bytes.as_ref(),
            engine.config().deserialize_check_wasmtime_version,
        )?;
        module.into_module(engine)
    }

    fn from_parts(
        engine: &Engine,
        mut modules: Vec<Arc<CompiledModule>>,
        main_module: usize,
        types: Arc<TypeTables>,
        module_upvars: &[serialization::SerializedModuleUpvar],
    ) -> Result<Self> {
        // Validate the module can be used with the current allocator
        engine.allocator().validate(modules[main_module].module())?;

        let signatures = Arc::new(SignatureCollection::new_for_module(
            engine.signatures(),
            &types.wasm_signatures,
            modules.iter().flat_map(|m| m.trampolines().iter().cloned()),
        ));

        let module = modules.remove(main_module);

        let module_upvars = module_upvars
            .iter()
            .map(|m| {
                mk(
                    engine,
                    &modules,
                    &types,
                    m.index,
                    &m.artifact_upvars,
                    &m.module_upvars,
                    &signatures,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        return Ok(Self {
            inner: Arc::new(ModuleInner {
                engine: engine.clone(),
                types,
                module,
                artifact_upvars: modules,
                module_upvars,
                signatures,
            }),
        });

        fn mk(
            engine: &Engine,
            artifacts: &[Arc<CompiledModule>],
            types: &Arc<TypeTables>,
            module_index: usize,
            artifact_upvars: &[usize],
            module_upvars: &[serialization::SerializedModuleUpvar],
            signatures: &Arc<SignatureCollection>,
        ) -> Result<Module> {
            Ok(Module {
                inner: Arc::new(ModuleInner {
                    engine: engine.clone(),
                    types: types.clone(),
                    module: artifacts[module_index].clone(),
                    artifact_upvars: artifact_upvars
                        .iter()
                        .map(|i| artifacts[*i].clone())
                        .collect(),
                    module_upvars: module_upvars
                        .into_iter()
                        .map(|m| {
                            mk(
                                engine,
                                artifacts,
                                types,
                                m.index,
                                &m.artifact_upvars,
                                &m.module_upvars,
                                signatures,
                            )
                        })
                        .collect::<Result<Vec<_>>>()?,
                    signatures: signatures.clone(),
                }),
            })
        }
    }

    /// Validates `binary` input data as a WebAssembly binary given the
    /// configuration in `engine`.
    ///
    /// This function will perform a speedy validation of the `binary` input
    /// WebAssembly module (which is in [binary form][binary], the text format
    /// is not accepted by this function) and return either `Ok` or `Err`
    /// depending on the results of validation. The `engine` argument indicates
    /// configuration for WebAssembly features, for example, which are used to
    /// indicate what should be valid and what shouldn't be.
    ///
    /// Validation automatically happens as part of [`Module::new`].
    ///
    /// # Errors
    ///
    /// If validation fails for any reason (type check error, usage of a feature
    /// that wasn't enabled, etc) then an error with a description of the
    /// validation issue will be returned.
    ///
    /// [binary]: https://webassembly.github.io/spec/core/binary/index.html
    pub fn validate(engine: &Engine, binary: &[u8]) -> Result<()> {
        let mut validator = Validator::new();
        validator.wasm_features(engine.config().features);
        validator.validate_all(binary)?;
        Ok(())
    }

    /// Returns the type signature of this module.
    pub fn ty(&self) -> ModuleType {
        let mut sig = ModuleType::new();
        let env_module = self.compiled_module().module();
        let types = self.types();
        for (module, field, ty) in env_module.imports() {
            sig.add_named_import(module, field, ExternType::from_wasmtime(types, &ty));
        }
        for (name, index) in env_module.exports.iter() {
            sig.add_named_export(
                name,
                ExternType::from_wasmtime(types, &env_module.type_of(*index)),
            );
        }
        sig
    }

    /// Serialize the module to a vector of bytes.
    ///
    /// Use `Module::new` or `Module::from_binary` to create the module
    /// from the bytes.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        SerializedModule::new(self).to_bytes()
    }

    /// Creates a submodule `Module` value from the specified parameters.
    ///
    /// This is used for creating submodules as part of module instantiation.
    ///
    /// * `artifact_index` - the index in `artifact_upvars` that we're creating
    ///   a module for
    /// * `artifact_upvars` - the mapping of indices of what artifact upvars are
    ///   needed for the submodule. The length of this array is the length of
    ///   the upvars array in the submodule to be created, and each element of
    ///   this array is an index into this module's upvar array.
    /// * `module_upvars` - similar to `artifact_upvars` this is a mapping of
    ///   how to create the `module_upvars` of the submodule being created.
    ///   Each entry in this array is either an index into this module's own
    ///   module upvars array or it's an index into `modules`, the list of
    ///   modules so far for the instance where this submodule is being
    ///   created.
    /// * `modules` - array indexed by `module_upvars`.
    ///
    /// Note that the real meat of this happens in `ModuleEnvironment`
    /// translation inside of `wasmtime_environ`. This just does the easy thing
    /// of handling all the indices, over there is where the indices are
    /// actually calculated and such.
    pub(crate) fn create_submodule(
        &self,
        artifact_index: usize,
        artifact_upvars: &[usize],
        module_upvars: &[wasmtime_environ::ModuleUpvar],
        modules: &PrimaryMap<ModuleIndex, Module>,
    ) -> Module {
        Module {
            inner: Arc::new(ModuleInner {
                types: self.inner.types.clone(),
                engine: self.inner.engine.clone(),
                module: self.inner.artifact_upvars[artifact_index].clone(),
                artifact_upvars: artifact_upvars
                    .iter()
                    .map(|i| self.inner.artifact_upvars[*i].clone())
                    .collect(),
                module_upvars: module_upvars
                    .iter()
                    .map(|i| match *i {
                        wasmtime_environ::ModuleUpvar::Inherit(i) => {
                            self.inner.module_upvars[i].clone()
                        }
                        wasmtime_environ::ModuleUpvar::Local(i) => modules[i].clone(),
                    })
                    .collect(),
                signatures: self.inner.signatures.clone(),
            }),
        }
    }

    pub(crate) fn compiled_module(&self) -> &Arc<CompiledModule> {
        &self.inner.module
    }

    pub(crate) fn env_module(&self) -> &wasmtime_environ::Module {
        self.compiled_module().module()
    }

    pub(crate) fn types(&self) -> &Arc<TypeTables> {
        &self.inner.types
    }

    pub(crate) fn signatures(&self) -> &Arc<SignatureCollection> {
        &self.inner.signatures
    }

    /// Looks up the module upvar value at the `index` specified.
    ///
    /// Note that this panics if `index` is out of bounds since this should
    /// only be called for valid indices as part of instantiation.
    pub(crate) fn module_upvar(&self, index: usize) -> &Module {
        &self.inner.module_upvars[index]
    }

    /// Returns identifier/name that this [`Module`] has. This name
    /// is used in traps/backtrace details.
    ///
    /// Note that most LLVM/clang/Rust-produced modules do not have a name
    /// associated with them, but other wasm tooling can be used to inject or
    /// add a name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let module = Module::new(&engine, "(module $foo)")?;
    /// assert_eq!(module.name(), Some("foo"));
    ///
    /// let module = Module::new(&engine, "(module)")?;
    /// assert_eq!(module.name(), None);
    ///
    /// let module = Module::new_with_name(&engine, "(module)", "bar")?;
    /// assert_eq!(module.name(), Some("bar"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn name(&self) -> Option<&str> {
        self.compiled_module().module().name.as_deref()
    }

    /// Returns the list of imports that this [`Module`] has and must be
    /// satisfied.
    ///
    /// This function returns the list of imports that the wasm module has, but
    /// only the types of each import. The type of each import is used to
    /// typecheck the [`Instance::new`](crate::Instance::new) method's `imports`
    /// argument. The arguments to that function must match up 1-to-1 with the
    /// entries in the array returned here.
    ///
    /// The imports returned reflect the order of the imports in the wasm module
    /// itself, and note that no form of deduplication happens.
    ///
    /// # Examples
    ///
    /// Modules with no imports return an empty list here:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let module = Module::new(&engine, "(module)")?;
    /// assert_eq!(module.imports().len(), 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// and modules with imports will have a non-empty list:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let wat = r#"
    ///     (module
    ///         (import "host" "foo" (func))
    ///     )
    /// "#;
    /// let module = Module::new(&engine, wat)?;
    /// assert_eq!(module.imports().len(), 1);
    /// let import = module.imports().next().unwrap();
    /// assert_eq!(import.module(), "host");
    /// assert_eq!(import.name(), Some("foo"));
    /// match import.ty() {
    ///     ExternType::Func(_) => { /* ... */ }
    ///     _ => panic!("unexpected import type!"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn imports<'module>(
        &'module self,
    ) -> impl ExactSizeIterator<Item = ImportType<'module>> + 'module {
        let module = self.compiled_module().module();
        let types = self.types();
        module
            .imports()
            .map(move |(module, field, ty)| ImportType::new(module, field, ty, types))
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Returns the list of exports that this [`Module`] has and will be
    /// available after instantiation.
    ///
    /// This function will return the type of each item that will be returned
    /// from [`Instance::exports`](crate::Instance::exports). Each entry in this
    /// list corresponds 1-to-1 with that list, and the entries here will
    /// indicate the name of the export along with the type of the export.
    ///
    /// # Examples
    ///
    /// Modules might not have any exports:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let module = Module::new(&engine, "(module)")?;
    /// assert!(module.exports().next().is_none());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// When the exports are not empty, you can inspect each export:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let wat = r#"
    ///     (module
    ///         (func (export "foo"))
    ///         (memory (export "memory") 1)
    ///     )
    /// "#;
    /// let module = Module::new(&engine, wat)?;
    /// assert_eq!(module.exports().len(), 2);
    ///
    /// let mut exports = module.exports();
    /// let foo = exports.next().unwrap();
    /// assert_eq!(foo.name(), "foo");
    /// match foo.ty() {
    ///     ExternType::Func(_) => { /* ... */ }
    ///     _ => panic!("unexpected export type!"),
    /// }
    ///
    /// let memory = exports.next().unwrap();
    /// assert_eq!(memory.name(), "memory");
    /// match memory.ty() {
    ///     ExternType::Memory(_) => { /* ... */ }
    ///     _ => panic!("unexpected export type!"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn exports<'module>(
        &'module self,
    ) -> impl ExactSizeIterator<Item = ExportType<'module>> + 'module {
        let module = self.compiled_module().module();
        let types = self.types();
        module.exports.iter().map(move |(name, entity_index)| {
            ExportType::new(name, module.type_of(*entity_index), types)
        })
    }

    /// Looks up an export in this [`Module`] by name.
    ///
    /// This function will return the type of an export with the given name.
    ///
    /// # Examples
    ///
    /// There may be no export with that name:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let module = Module::new(&engine, "(module)")?;
    /// assert!(module.get_export("foo").is_none());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// When there is an export with that name, it is returned:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let engine = Engine::default();
    /// let wat = r#"
    ///     (module
    ///         (func (export "foo"))
    ///         (memory (export "memory") 1)
    ///     )
    /// "#;
    /// let module = Module::new(&engine, wat)?;
    /// let foo = module.get_export("foo");
    /// assert!(foo.is_some());
    ///
    /// let foo = foo.unwrap();
    /// match foo {
    ///     ExternType::Func(_) => { /* ... */ }
    ///     _ => panic!("unexpected export type!"),
    /// }
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_export<'module>(&'module self, name: &'module str) -> Option<ExternType> {
        let module = self.compiled_module().module();
        let entity_index = module.exports.get(name)?;
        Some(ExternType::from_wasmtime(
            self.types(),
            &module.type_of(*entity_index),
        ))
    }

    /// Returns the [`Engine`] that this [`Module`] was compiled by.
    pub fn engine(&self) -> &Engine {
        &self.inner.engine
    }
}

fn _assert_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<Module>();
}
