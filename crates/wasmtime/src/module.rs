use crate::Engine;
use crate::{
    signatures::SignatureCollection,
    types::{ExportType, ExternType, ImportType},
};
use anyhow::{bail, Context, Result};
use once_cell::sync::OnceCell;
use std::fs;
use std::mem;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use wasmparser::{Parser, ValidPayload, Validator};
use wasmtime_environ::{
    DefinedFuncIndex, DefinedMemoryIndex, FunctionInfo, ModuleEnvironment, PrimaryMap,
    SignatureIndex, TypeTables,
};
use wasmtime_jit::{CompiledModule, CompiledModuleInfo};
use wasmtime_runtime::{
    CompiledModuleId, MemoryImage, MmapVec, ModuleMemoryImages, VMSharedSignatureIndex,
};

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
    /// Type information of this module.
    types: Arc<TypeTables>,
    /// Registered shared signature for the module.
    signatures: Arc<SignatureCollection>,
    /// A set of initialization images for memories, if any.
    ///
    /// Note that this is behind a `OnceCell` to lazily create this image. On
    /// Linux where `memfd_create` may be used to create the backing memory
    /// image this is a pretty expensive operation, so by deferring it this
    /// improves memory usage for modules that are created but may not ever be
    /// instantiated.
    memory_images: OnceCell<Option<ModuleMemoryImages>>,
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
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
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
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
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
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
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
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn from_binary(engine: &Engine, binary: &[u8]) -> Result<Module> {
        engine
            .check_compatible_with_native_host()
            .context("compilation settings are not compatible with the native host")?;

        cfg_if::cfg_if! {
            if #[cfg(feature = "cache")] {
                let state = (HashedEngineCompileEnv(engine), binary);
                let (mmap, info, types) = wasmtime_cache::ModuleCacheEntry::new(
                    "wasmtime",
                    engine.cache_config(),
                )
                .get_data_raw(
                    &state,

                    // Cache miss, compute the actual artifacts
                    |(engine, wasm)| Module::build_artifacts(engine.0, wasm),

                    // Implementation of how to serialize artifacts
                    |(engine, _wasm), (mmap, _info, types)| {
                        SerializedModule::from_artifacts(
                            engine.0,
                            mmap,
                            types,
                        ).to_bytes(&engine.0.config().module_version).ok()
                    },

                    // Cache hit, deserialize the provided artifacts
                    |(engine, _wasm), serialized_bytes| {
                        SerializedModule::from_bytes(&serialized_bytes, &engine.0.config().module_version)
                            .ok()?
                            .into_parts(engine.0)
                            .ok()
                    },
                )?;
            } else {
                let (mmap, info, types) = Module::build_artifacts(engine, binary)?;
            }
        };

        let module = CompiledModule::from_artifacts(
            mmap,
            info,
            &*engine.config().profiler,
            engine.unique_id_allocator(),
        )?;

        Self::from_parts(engine, module, Arc::new(types))
    }

    /// Converts an input binary-encoded WebAssembly module to compilation
    /// artifacts and type information.
    ///
    /// This is where compilation actually happens of WebAssembly modules and
    /// translation/parsing/validation of the binary input occurs. The actual
    /// result here is a triple of:
    ///
    /// * The index into the second field of the "main module". The "main
    ///   module" in this case is the outermost module described by the `wasm`
    ///   input, and is here for the module linking proposal.
    /// * A list of compilation artifacts for each module found within `wasm`.
    ///   Note that if module linking is disabled then this list will always
    ///   have a size of exactly 1. These pairs are returned by
    ///   `wasmtime_jit::finish_compile`.
    /// * Type information about all the modules returned. All returned modules
    ///   have local type information with indices that refer to these returned
    ///   tables.
    #[cfg(compiler)]
    pub(crate) fn build_artifacts(
        engine: &Engine,
        wasm: &[u8],
    ) -> Result<(MmapVec, Option<CompiledModuleInfo>, TypeTables)> {
        let tunables = &engine.config().tunables;

        // First a `ModuleEnvironment` is created which records type information
        // about the wasm module. This is where the WebAssembly is parsed and
        // validated. Afterwards `types` will have all the type information for
        // this module.
        let (mut translation, types) = ModuleEnvironment::new(tunables, &engine.config().features)
            .translate(wasm)
            .context("failed to parse WebAssembly module")?;

        // Next compile all functions in parallel using rayon. This will perform
        // the actual validation of all the function bodies.
        let functions = mem::take(&mut translation.function_body_inputs);
        let functions = functions.into_iter().collect::<Vec<_>>();
        let funcs = engine
            .run_maybe_parallel(functions, |(index, func)| {
                engine
                    .compiler()
                    .compile_function(&translation, index, func, tunables, &types)
            })?
            .into_iter()
            .collect();

        // Collect all the function results into a final ELF object.
        let mut obj = engine.compiler().object()?;
        let (funcs, trampolines) =
            engine
                .compiler()
                .emit_obj(&translation, &types, funcs, tunables, &mut obj)?;

        // If configured, attempt to use paged memory initialization
        // instead of the default mode of memory initialization
        if engine.config().paged_memory_initialization {
            translation.try_paged_init();
        }

        // If configured attempt to use static memory initialization which
        // can either at runtime be implemented as a single memcpy to
        // initialize memory or otherwise enabling virtual-memory-tricks
        // such as mmap'ing from a file to get copy-on-write.
        if engine.config().memory_init_cow {
            let align = engine.compiler().page_size_align();
            let max_always_allowed = engine.config().memory_guaranteed_dense_image_size;
            translation.try_static_init(align, max_always_allowed);
        }

        // Attempt to convert table initializer segments to
        // FuncTable representation where possible, to enable
        // table lazy init.
        translation.try_func_table_init();

        let (mmap, info) =
            wasmtime_jit::finish_compile(translation, obj, funcs, trampolines, tunables)?;

        Ok((mmap, Some(info), types))
    }

    /// Deserializes an in-memory compiled module previously created with
    /// [`Module::serialize`] or [`Engine::precompile_module`].
    ///
    /// This function will deserialize the binary blobs emitted by
    /// [`Module::serialize`] and [`Engine::precompile_module`] back into an
    /// in-memory [`Module`] that's ready to be instantiated.
    ///
    /// Note that the [`Module::deserialize_file`] method is more optimized than
    /// this function, so if the serialized module is already present in a file
    /// it's recommended to use that method instead.
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
        let module = SerializedModule::from_bytes(bytes.as_ref(), &engine.config().module_version)?;
        module.into_module(engine)
    }

    /// Same as [`deserialize`], except that the contents of `path` are read to
    /// deserialize into a [`Module`].
    ///
    /// This method is provided because it can be faster than [`deserialize`]
    /// since the data doesn't need to be copied around, but rather the module
    /// can be used directly from an mmap'd view of the file provided.
    ///
    /// [`deserialize`]: Module::deserialize
    ///
    /// # Unsafety
    ///
    /// All of the reasons that [`deserialize`] is `unsafe` applies to this
    /// function as well. Arbitrary data loaded from a file may trick Wasmtime
    /// into arbitrary code execution since the contents of the file are not
    /// validated to be a valid precompiled module.
    ///
    /// Additionally though this function is also `unsafe` because the file
    /// referenced must remain unchanged and a valid precompiled module for the
    /// entire lifetime of the [`Module`] returned. Any changes to the file on
    /// disk may change future instantiations of the module to be incorrect.
    /// This is because the file is mapped into memory and lazily loaded pages
    /// reflect the current state of the file, not necessarily the origianl
    /// state of the file.
    pub unsafe fn deserialize_file(engine: &Engine, path: impl AsRef<Path>) -> Result<Module> {
        let module = SerializedModule::from_file(path.as_ref(), &engine.config().module_version)?;
        module.into_module(engine)
    }

    fn from_parts(
        engine: &Engine,
        module: Arc<CompiledModule>,
        types: Arc<TypeTables>,
    ) -> Result<Self> {
        // Validate the module can be used with the current allocator
        engine.allocator().validate(module.module())?;

        let signatures = Arc::new(SignatureCollection::new_for_module(
            engine.signatures(),
            &types,
            module.trampolines().map(|(idx, f, _)| (idx, f)),
        ));

        Ok(Self {
            inner: Arc::new(ModuleInner {
                engine: engine.clone(),
                types,
                signatures,
                memory_images: OnceCell::new(),
                module,
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

        let mut functions = Vec::new();
        for payload in Parser::new(0).parse_all(binary) {
            if let ValidPayload::Func(a, b) = validator.payload(&payload?)? {
                functions.push((a, b));
            }
        }

        engine.run_maybe_parallel(functions, |(mut validator, body)| validator.validate(&body))?;
        Ok(())
    }

    /// Serializes this module to a vector of bytes.
    ///
    /// This function is similar to the [`Engine::precompile_module`] method
    /// where it produces an artifact of Wasmtime which is suitable to later
    /// pass into [`Module::deserialize`]. If a module is never instantiated
    /// then it's recommended to use [`Engine::precompile_module`] instead of
    /// this method, but if a module is both instantiated and serialized then
    /// this method can be useful to get the serialized version without
    /// compiling twice.
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn serialize(&self) -> Result<Vec<u8>> {
        SerializedModule::new(self).to_bytes(&self.inner.engine.config().module_version)
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
    /// assert_eq!(import.name(), "foo");
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
    pub fn get_export(&self, name: &str) -> Option<ExternType> {
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

    /// Returns the `ModuleInner` cast as `ModuleRuntimeInfo` for use
    /// by the runtime.
    pub(crate) fn runtime_info(&self) -> Arc<dyn wasmtime_runtime::ModuleRuntimeInfo> {
        // N.B.: this needs to return a clone because we cannot
        // statically cast the &Arc<ModuleInner> to &Arc<dyn Trait...>.
        self.inner.clone()
    }

    /// Returns the range of bytes in memory where this module's compilation
    /// image resides.
    ///
    /// The compilation image for a module contains executable code, data, debug
    /// information, etc. This is roughly the same as the `Module::serialize`
    /// but not the exact same.
    ///
    /// The range of memory reported here is exposed to allow low-level
    /// manipulation of the memory in platform-specific manners such as using
    /// `mlock` to force the contents to be paged in immediately or keep them
    /// paged in after they're loaded.
    ///
    /// It is not safe to modify the memory in this range, nor is it safe to
    /// modify the protections of memory in this range.
    pub fn image_range(&self) -> Range<usize> {
        self.compiled_module().image_range()
    }
}

fn _assert_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<Module>();
}

/// This is a helper struct used when caching to hash the state of an `Engine`
/// used for module compilation.
///
/// The hash computed for this structure is used to key the global wasmtime
/// cache and dictates whether artifacts are reused. Consequently the contents
/// of this hash dictate when artifacts are or aren't re-used.
#[cfg(all(feature = "cache", compiler))]
struct HashedEngineCompileEnv<'a>(&'a Engine);

#[cfg(all(feature = "cache", compiler))]
impl std::hash::Hash for HashedEngineCompileEnv<'_> {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        // Hash the compiler's state based on its target and configuration.
        let compiler = self.0.compiler();
        compiler.triple().hash(hasher);
        compiler.flags().hash(hasher);
        compiler.isa_flags().hash(hasher);

        // Hash configuration state read for compilation
        let config = self.0.config();
        config.tunables.hash(hasher);
        config.features.hash(hasher);

        // Catch accidental bugs of reusing across crate versions.
        env!("CARGO_PKG_VERSION").hash(hasher);
    }
}

impl wasmtime_runtime::ModuleRuntimeInfo for ModuleInner {
    fn module(&self) -> &Arc<wasmtime_environ::Module> {
        self.module.module()
    }

    fn signature(&self, index: SignatureIndex) -> VMSharedSignatureIndex {
        self.signatures.as_module_map()[index]
    }

    fn image_base(&self) -> usize {
        self.module.code().as_ptr() as usize
    }

    fn function_info(&self, index: DefinedFuncIndex) -> &FunctionInfo {
        self.module.func_info(index)
    }

    fn memory_image(&self, memory: DefinedMemoryIndex) -> Result<Option<&Arc<MemoryImage>>> {
        let images = self
            .memory_images
            .get_or_try_init(|| memory_images(&self.engine, &self.module))?;
        Ok(images
            .as_ref()
            .and_then(|images| images.get_memory_image(memory)))
    }

    fn unique_id(&self) -> Option<CompiledModuleId> {
        Some(self.module.unique_id())
    }

    fn wasm_data(&self) -> &[u8] {
        self.module.wasm_data()
    }

    fn signature_ids(&self) -> &[VMSharedSignatureIndex] {
        self.signatures.as_module_map().values().as_slice()
    }
}

/// A barebones implementation of ModuleRuntimeInfo that is useful for
/// cases where a purpose-built environ::Module is used and a full
/// CompiledModule does not exist (for example, for tests or for the
/// default-callee instance).
pub(crate) struct BareModuleInfo {
    module: Arc<wasmtime_environ::Module>,
    image_base: usize,
    one_signature: Option<(SignatureIndex, VMSharedSignatureIndex)>,
    function_info: PrimaryMap<DefinedFuncIndex, FunctionInfo>,
}

impl BareModuleInfo {
    pub(crate) fn empty(module: Arc<wasmtime_environ::Module>) -> Self {
        BareModuleInfo {
            module,
            image_base: 0,
            one_signature: None,
            function_info: PrimaryMap::default(),
        }
    }

    pub(crate) fn maybe_imported_func(
        module: Arc<wasmtime_environ::Module>,
        one_signature: Option<(SignatureIndex, VMSharedSignatureIndex)>,
    ) -> Self {
        BareModuleInfo {
            module,
            image_base: 0,
            one_signature,
            function_info: PrimaryMap::default(),
        }
    }

    pub(crate) fn one_func(
        module: Arc<wasmtime_environ::Module>,
        image_base: usize,
        info: FunctionInfo,
        signature_id: SignatureIndex,
        signature: VMSharedSignatureIndex,
    ) -> Self {
        let mut function_info = PrimaryMap::with_capacity(1);
        function_info.push(info);
        BareModuleInfo {
            module,
            image_base,
            function_info,
            one_signature: Some((signature_id, signature)),
        }
    }

    pub(crate) fn into_traitobj(self) -> Arc<dyn wasmtime_runtime::ModuleRuntimeInfo> {
        Arc::new(self)
    }
}

impl wasmtime_runtime::ModuleRuntimeInfo for BareModuleInfo {
    fn module(&self) -> &Arc<wasmtime_environ::Module> {
        &self.module
    }

    fn signature(&self, index: SignatureIndex) -> VMSharedSignatureIndex {
        let (signature_id, signature) = self
            .one_signature
            .expect("Signature for one function should be present if queried");
        assert_eq!(index, signature_id);
        signature
    }

    fn image_base(&self) -> usize {
        self.image_base
    }

    fn function_info(&self, index: DefinedFuncIndex) -> &FunctionInfo {
        &self.function_info[index]
    }

    fn memory_image(&self, _memory: DefinedMemoryIndex) -> Result<Option<&Arc<MemoryImage>>> {
        Ok(None)
    }

    fn unique_id(&self) -> Option<CompiledModuleId> {
        None
    }

    fn wasm_data(&self) -> &[u8] {
        &[]
    }

    fn signature_ids(&self) -> &[VMSharedSignatureIndex] {
        match &self.one_signature {
            Some((_, id)) => std::slice::from_ref(id),
            None => &[],
        }
    }
}

/// Helper method to construct a `ModuleMemoryImages` for an associated
/// `CompiledModule`.
fn memory_images(engine: &Engine, module: &CompiledModule) -> Result<Option<ModuleMemoryImages>> {
    // If initialization via copy-on-write is explicitly disabled in
    // configuration then this path is skipped entirely.
    if !engine.config().memory_init_cow {
        return Ok(None);
    }

    // ... otherwise logic is delegated to the `ModuleMemoryImages::new`
    // constructor.
    let mmap = if engine.config().force_memory_init_memfd {
        None
    } else {
        Some(module.mmap())
    };
    ModuleMemoryImages::new(module.module(), module.wasm_data(), mmap)
}
