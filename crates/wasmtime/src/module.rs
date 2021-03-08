use crate::types::{ExportType, ExternType, ImportType};
use crate::{Engine, ModuleType};
use anyhow::{bail, Context, Result};
use bincode::Options;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;
use std::sync::Arc;
use wasmparser::Validator;
#[cfg(feature = "cache")]
use wasmtime_cache::ModuleCacheEntry;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::ModuleIndex;
use wasmtime_jit::{CompilationArtifacts, CompiledModule, TypeTables};

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
/// The `Module` is threadsafe and safe to share accross threads.
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
}

/// A small helper struct which defines modules are serialized.
#[derive(serde::Serialize, serde::Deserialize)]
struct ModuleSerialized<'a> {
    /// All compiled artifacts neeeded by this module, where the last entry in
    /// this list is the artifacts for the module itself.
    artifacts: Vec<MyCow<'a, CompilationArtifacts>>,
    /// Closed-over module values that are also needed for this module.
    modules: Vec<ModuleSerialized<'a>>,
    /// The index into the list of type tables that are used for this module's
    /// type tables.
    type_tables: usize,
}

// This is like `std::borrow::Cow` but it doesn't have a `Clone` bound on `T`
enum MyCow<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T> MyCow<'a, T> {
    fn unwrap_owned(self) -> T {
        match self {
            MyCow::Owned(val) => val,
            MyCow::Borrowed(_) => unreachable!(),
        }
    }
}

impl<'a, T: Serialize> Serialize for MyCow<'a, T> {
    fn serialize<S>(&self, dst: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            MyCow::Borrowed(val) => val.serialize(dst),
            MyCow::Owned(val) => val.serialize(dst),
        }
    }
}

impl<'a, 'b, T: Deserialize<'a>> Deserialize<'a> for MyCow<'b, T> {
    fn deserialize<D>(src: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        Ok(MyCow::Owned(T::deserialize(src)?))
    }
}

impl Module {
    /// Creates a new WebAssembly `Module` from the given in-memory `bytes`.
    ///
    /// The `bytes` provided must be in one of two formats:
    ///
    /// * It can be a [binary-encoded][binary] WebAssembly module. This
    ///   is always supported.
    /// * It may also be a [text-encoded][text] instance of the WebAssembly
    ///   text format. This is only supported when the `wat` feature of this
    ///   crate is enabled. If this is supplied then the text format will be
    ///   parsed before validation. Note that the `wat` feature is enabled by
    ///   default.
    ///
    /// The data for the wasm module must be loaded in-memory if it's present
    /// elsewhere, for example on disk. This requires that the entire binary is
    /// loaded into memory all at once, this API does not support streaming
    /// compilation of a module.
    ///
    /// The WebAssembly binary will be decoded and validated. It will also be
    /// compiled according to the configuration of the provided `engine`.
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
    ///   configuration of `enging`
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
        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(bytes.as_ref())?;
        Module::from_binary(engine, bytes.as_ref())
    }

    /// Creates a new WebAssembly `Module` from the given in-memory `binary`
    /// data. The provided `name` will be used in traps/backtrace details.
    ///
    /// See [`Module::new`] for other details.
    pub fn new_with_name(engine: &Engine, bytes: impl AsRef<[u8]>, name: &str) -> Result<Module> {
        let mut module = Module::new(engine, bytes.as_ref())?;
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
        #[cfg(feature = "wat")]
        let wasm = wat::parse_file(file)?;
        #[cfg(not(feature = "wat"))]
        let wasm = std::fs::read(file)?;
        Module::new(engine, &wasm)
    }

    /// Creates a new WebAssembly `Module` from the given in-memory `binary`
    /// data.
    ///
    /// This is similar to [`Module::new`] except that it requires that the
    /// `binary` input is a WebAssembly binary, the text format is not supported
    /// by this function. It's generally recommended to use [`Module::new`],
    /// but if it's required to not support the text format this function can be
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
        const USE_PAGED_MEM_INIT: bool = cfg!(all(feature = "uffd", target_os = "linux"));

        cfg_if::cfg_if! {
            if #[cfg(feature = "cache")] {
                let (main_module, artifacts, types) = ModuleCacheEntry::new(
                    "wasmtime",
                    engine.cache_config(),
                )
                .get_data((engine.compiler(), binary), |(compiler, binary)| {
                    CompilationArtifacts::build(compiler, binary, USE_PAGED_MEM_INIT)
                })?;
            } else {
                let (main_module, artifacts, types) =
                    CompilationArtifacts::build(engine.compiler(), binary, USE_PAGED_MEM_INIT)?;
            }
        };

        let mut modules = CompiledModule::from_artifacts_list(
            artifacts,
            engine.compiler().isa(),
            &*engine.config().profiler,
        )?;
        let module = modules.remove(main_module);

        // Validate the module can be used with the current allocator
        engine
            .config()
            .instance_allocator()
            .validate(module.module())?;

        Ok(Module {
            inner: Arc::new(ModuleInner {
                engine: engine.clone(),
                module,
                types: Arc::new(types),
                artifact_upvars: modules,
                module_upvars: Vec::new(),
            }),
        })
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

    /// Serialize compilation artifacts to the buffer. See also `deseriaize`.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut pushed = HashMap::new();
        let mut tables = Vec::new();
        let module = self.serialized_module(&mut pushed, &mut tables);
        let artifacts = (compiler_fingerprint(self.engine()), tables, module);
        let buffer = bincode_options().serialize(&artifacts)?;
        Ok(buffer)
    }

    fn serialized_module<'a>(
        &'a self,
        type_tables_pushed: &mut HashMap<usize, usize>,
        type_tables: &mut Vec<&'a TypeTables>,
    ) -> ModuleSerialized<'a> {
        // Deduplicate `Arc<TypeTables>` using our two parameters to ensure we
        // serialize type tables as little as possible.
        let ptr = Arc::as_ptr(self.types());
        let type_tables_idx = *type_tables_pushed.entry(ptr as usize).or_insert_with(|| {
            type_tables.push(self.types());
            type_tables.len() - 1
        });
        ModuleSerialized {
            artifacts: self
                .inner
                .artifact_upvars
                .iter()
                .map(|i| MyCow::Borrowed(i.compilation_artifacts()))
                .chain(Some(MyCow::Borrowed(
                    self.compiled_module().compilation_artifacts(),
                )))
                .collect(),
            modules: self
                .inner
                .module_upvars
                .iter()
                .map(|i| i.serialized_module(type_tables_pushed, type_tables))
                .collect(),
            type_tables: type_tables_idx,
        }
    }

    /// Deserializes and creates a module from the compilation artifacts.
    /// The `serialize` saves the compilation artifacts along with the host
    /// fingerprint, which consists of target, compiler flags, and wasmtime
    /// package version.
    ///
    /// The method will fail if fingerprints of current host and serialized
    /// one are different. The method does not verify the serialized artifacts
    /// for modifications or curruptions. All responsibily of signing and its
    /// verification falls on the embedder.
    pub fn deserialize(engine: &Engine, serialized: &[u8]) -> Result<Module> {
        let (fingerprint, types, serialized) = bincode_options()
            .deserialize::<(u64, Vec<TypeTables>, _)>(serialized)
            .context("Deserialize compilation artifacts")?;

        if fingerprint != compiler_fingerprint(engine) {
            bail!("Incompatible compilation artifact");
        }

        let types = types.into_iter().map(Arc::new).collect::<Vec<_>>();
        return mk(engine, &types, serialized);

        fn mk(
            engine: &Engine,
            types: &Vec<Arc<TypeTables>>,
            module: ModuleSerialized<'_>,
        ) -> Result<Module> {
            let mut artifacts = CompiledModule::from_artifacts_list(
                module
                    .artifacts
                    .into_iter()
                    .map(|i| i.unwrap_owned())
                    .collect(),
                engine.compiler().isa(),
                &*engine.config().profiler,
            )?;
            let inner = ModuleInner {
                engine: engine.clone(),
                types: types[module.type_tables].clone(),
                module: artifacts.pop().unwrap(),
                artifact_upvars: artifacts,
                module_upvars: module
                    .modules
                    .into_iter()
                    .map(|m| mk(engine, types, m))
                    .collect::<Result<Vec<_>>>()?,
            };
            Ok(Module {
                inner: Arc::new(inner),
            })
        }
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
    ///   how to create the e`module_upvars` of the submodule being created.
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
                types: self.types().clone(),
                engine: self.engine().clone(),
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
            }),
        }
    }

    pub(crate) fn compiled_module(&self) -> &CompiledModule {
        &self.inner.module
    }

    pub(crate) fn env_module(&self) -> &wasmtime_environ::Module {
        self.compiled_module().module()
    }

    pub(crate) fn types(&self) -> &Arc<TypeTables> {
        &self.inner.types
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

fn bincode_options() -> impl Options {
    // Use a variable-length integer encoding instead of fixed length. The
    // module shown on #2318 gets compressed from ~160MB to ~110MB simply using
    // this, presumably because there's a lot of 8-byte integers which generally
    // have small values. Local testing shows that the deserialization
    // performance, while higher, is in the few-percent range. For huge size
    // savings this seems worthwhile to lose a small percentage of
    // deserialization performance.
    bincode::DefaultOptions::new().with_varint_encoding()
}

fn compiler_fingerprint(engine: &Engine) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    engine.compiler().hash(&mut hasher);
    hasher.finish()
}

fn _assert_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<Module>();
}
