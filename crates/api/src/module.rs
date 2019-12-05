use crate::callable::{WasmtimeFn, WrappedCallable};
use crate::frame_info::{GlobalFrameInfoRegistration, FRAME_INFO};
use crate::types::{
    ExportType, ExternType, FuncType, GlobalType, ImportType, Limits, MemoryType, Mutability,
    TableType, ValType,
};
use crate::{Callable, Func, Store, Trap, Val};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::convert::TryInto;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wasmparser::{
    validate, CustomSectionKind, ExternalKind, ImportSectionEntryType, ModuleReader, Name,
    SectionCode,
};
use wasmtime_environ::wasm::FuncIndex;
use wasmtime_jit::CompiledModule;
use wasmtime_runtime::InstanceHandle;

fn into_memory_type(mt: wasmparser::MemoryType) -> Result<MemoryType> {
    if mt.shared {
        bail!("shared memories are not supported yet");
    }
    Ok(MemoryType::new(Limits::new(
        mt.limits.initial,
        mt.limits.maximum,
    )))
}

fn into_global_type(gt: wasmparser::GlobalType) -> GlobalType {
    let mutability = if gt.mutable {
        Mutability::Var
    } else {
        Mutability::Const
    };
    GlobalType::new(into_valtype(&gt.content_type), mutability)
}

// `into_valtype` is used for `map` which requires `&T`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn into_valtype(ty: &wasmparser::Type) -> ValType {
    use wasmparser::Type::*;
    match ty {
        I32 => ValType::I32,
        I64 => ValType::I64,
        F32 => ValType::F32,
        F64 => ValType::F64,
        V128 => ValType::V128,
        AnyFunc => ValType::FuncRef,
        AnyRef => ValType::AnyRef,
        _ => unimplemented!("types in into_valtype"),
    }
}

fn into_func_type(mt: wasmparser::FuncType) -> FuncType {
    assert_eq!(mt.form, wasmparser::Type::Func);
    let params = mt.params.iter().map(into_valtype).collect::<Vec<_>>();
    let returns = mt.returns.iter().map(into_valtype).collect::<Vec<_>>();
    FuncType::new(params.into_boxed_slice(), returns.into_boxed_slice())
}

fn into_table_type(tt: wasmparser::TableType) -> TableType {
    assert!(
        tt.element_type == wasmparser::Type::AnyFunc || tt.element_type == wasmparser::Type::AnyRef
    );
    let ty = into_valtype(&tt.element_type);
    let limits = Limits::new(tt.limits.initial, tt.limits.maximum);
    TableType::new(ty, limits)
}

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
/// let store = Store::default();
/// let module = Module::from_file(&store, "path/to/foo.wasm")?;
/// # Ok(())
/// # }
/// ```
///
/// You can also load the wasm text format if more convenient too:
///
/// ```no_run
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let store = Store::default();
/// // Now we're using the WebAssembly text extension: `.wat`!
/// let module = Module::from_file(&store, "path/to/foo.wat")?;
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
/// let store = Store::default();
/// # let wasm_bytes: Vec<u8> = Vec::new();
/// let module = Module::new(&store, &wasm_bytes)?;
///
/// // It also works with the text format!
/// let module = Module::new(&store, "(module (func))")?;
/// # Ok(())
/// # }
/// ```
///
/// [`Config`]: crate::Config
#[derive(Clone)]
pub struct Module {
    pub(crate) inner: Arc<ModuleInner>,
}

pub(crate) struct ModuleInner {
    store: Store,

    /// List of imports that are expected to be provided to `Instance::new`.
    imports: Box<[ImportType]>,

    /// List of exports that will be available from `Instance::exports`.
    exports: Box<[ExportType]>,

    compiled: CompiledModule,
    frame_info_registration: Mutex<Option<Option<GlobalFrameInfoRegistration>>>,
    names: Arc<Names>,

    /// Adapter functions in this module defined in the wasm interface types
    /// section.
    adapters: Box<[(FuncType, Adapter)]>,

    /// Map from index of import in the core module to where that import is
    /// going to be satisfied.
    core_import_sources: Box<[ImportSource]>,

    /// Map of export name to what is being exported,
    pub(crate) export_map: HashMap<String, Export>,
}

pub struct Names {
    pub module: Arc<wasmtime_environ::Module>,
    pub module_name: Option<String>,
}

enum Adapter {
    /// This adapter is an imported function, and imports the nth function in
    /// the user-provided imports array
    Import(usize),
    /// This adapter is locally defined and has a list of instructions.
    Local(Vec<wit_parser::Instruction>),
}

pub(crate) enum Export {
    /// This export is from the core wasm module's exports with the same name.
    Core,
    /// This export is the nth adapter function.
    Adapter(usize),
}

pub(crate) enum ImportSource {
    /// This import is going to be satisfied by the nth entry in the imports
    /// provided by the user.
    UserProvided(usize),
    /// This import is going to be provided by the nth adapter function.
    Adapter(usize),
}

enum ImportKind {
    /// An import in the core wasm module, optionally implemented with an
    /// adapter function.
    Core {
        implemented_with_adapter: Option<usize>,
    },
    /// An import from the interface types section.
    Adapter,
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
    /// compiled according to the configuration of the provided `store` and
    /// cached in this type.
    ///
    /// The provided `store` is a global cache for compiled resources as well as
    /// configuration for what wasm features are enabled. It's recommended to
    /// share a `store` among modules if possible.
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
    ///   configuration of `store`
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
    /// # let store = Store::default();
    /// # let wasm_bytes: Vec<u8> = Vec::new();
    /// let module = Module::new(&store, &wasm_bytes)?;
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
    /// # let store = Store::default();
    /// let module = Module::new(&store, "(module (func))")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(store: &Store, bytes: impl AsRef<[u8]>) -> Result<Module> {
        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(bytes.as_ref())?;
        Module::from_binary(store, bytes.as_ref())
    }

    /// Creates a new WebAssembly `Module` from the given in-memory `binary`
    /// data. The provided `name` will be used in traps/backtrace details.
    ///
    /// See [`Module::new`] for other details.
    pub fn new_with_name(store: &Store, bytes: impl AsRef<[u8]>, name: &str) -> Result<Module> {
        let mut module = Module::new(store, bytes.as_ref())?;
        let inner = Arc::get_mut(&mut module.inner).unwrap();
        Arc::get_mut(inner.compiled.module_mut()).unwrap().name = Some(name.to_string());
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
    /// let store = Store::default();
    /// let module = Module::from_file(&store, "./path/to/foo.wasm")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// The `.wat` text format is also supported:
    ///
    /// ```no_run
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let module = Module::from_file(&store, "./path/to/foo.wat")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file(store: &Store, file: impl AsRef<Path>) -> Result<Module> {
        #[cfg(feature = "wat")]
        let wasm = wat::parse_file(file)?;
        #[cfg(not(feature = "wat"))]
        let wasm = std::fs::read(file)?;
        Module::new(store, &wasm)
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
    /// # let store = Store::default();
    /// let wasm = b"\0asm\x01\0\0\0";
    /// let module = Module::from_binary(&store, wasm)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Note that the text format is **not** accepted by this function:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// assert!(Module::from_binary(&store, b"(module)").is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_binary(store: &Store, binary: &[u8]) -> Result<Module> {
        Module::validate(store, binary)?;
        // Note that the call to `from_binary_unchecked` here should be ok
        // because we previously validated the binary, meaning we're guaranteed
        // to pass a valid binary for `store`.
        unsafe { Module::from_binary_unchecked(store, binary) }
    }

    /// Creates a new WebAssembly `Module` from the given in-memory `binary`
    /// data, skipping validation and asserting that `binary` is a valid
    /// WebAssembly module.
    ///
    /// This function is the same as [`Module::new`] except that it skips the
    /// call to [`Module::validate`] and it does not support the text format of
    /// WebAssembly. The WebAssembly binary is not validated for
    /// correctness and it is simply assumed as valid.
    ///
    /// For more information about creation of a module and the `store` argument
    /// see the documentation of [`Module::new`].
    ///
    /// # Unsafety
    ///
    /// This function is `unsafe` due to the unchecked assumption that the input
    /// `binary` is valid. If the `binary` is not actually a valid wasm binary it
    /// may cause invalid machine code to get generated, cause panics, etc.
    ///
    /// It is only safe to call this method if [`Module::validate`] succeeds on
    /// the same arguments passed to this function.
    ///
    /// # Errors
    ///
    /// This function may fail for many of the same reasons as [`Module::new`].
    /// While this assumes that the binary is valid it still needs to actually
    /// be somewhat valid for decoding purposes, and the basics of decoding can
    /// still fail.
    pub unsafe fn from_binary_unchecked(store: &Store, binary: &[u8]) -> Result<Module> {
        let mut ret = Module::compile(store, binary)?;
        ret.read_imports_and_exports(binary)?;
        Ok(ret)
    }

    /// Validates `binary` input data as a WebAssembly binary given the
    /// configuration in `store`.
    ///
    /// This function will perform a speedy validation of the `binary` input
    /// WebAssembly module (which is in [binary form][binary], the text format
    /// is not accepted by this function) and return either `Ok` or `Err`
    /// depending on the results of validation. The `store` argument indicates
    /// configuration for WebAssembly features, for example, which are used to
    /// indicate what should be valid and what shouldn't be.
    ///
    /// Validation automatically happens as part of [`Module::new`], but is a
    /// requirement for [`Module::from_binary_unchecked`] to be safe.
    ///
    /// # Errors
    ///
    /// If validation fails for any reason (type check error, usage of a feature
    /// that wasn't enabled, etc) then an error with a description of the
    /// validation issue will be returned.
    ///
    /// [binary]: https://webassembly.github.io/spec/core/binary/index.html
    pub fn validate(store: &Store, binary: &[u8]) -> Result<()> {
        let config = store.engine().config();
        validate(binary, Some(config.validating_config.clone()))?;
        if config.interface_types {
            wit_validator::validate(binary)?;
        }
        Ok(())
    }

    unsafe fn compile(store: &Store, binary: &[u8]) -> Result<Self> {
        let compiled = CompiledModule::new(
            &mut store.compiler_mut(),
            binary,
            store.engine().config().debug_info,
            store.engine().config().profiler.as_ref(),
        )?;

        let names = Arc::new(Names {
            module_name: None,
            module: compiled.module().clone(),
        });
        Ok(Module {
            inner: Arc::new(ModuleInner {
                store: store.clone(),
                imports: Box::new([]),
                exports: Box::new([]),
                compiled,
                names,
                frame_info_registration: Mutex::new(None),
                adapters: Default::default(),
                core_import_sources: Default::default(),
                export_map: Default::default(),
            }),
        })
    }

    pub(crate) fn compiled_module(&self) -> &CompiledModule {
        &self.inner.compiled
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
    /// # let store = Store::default();
    /// let module = Module::new(&store, "(module $foo)")?;
    /// assert_eq!(module.name(), Some("foo"));
    ///
    /// let module = Module::new(&store, "(module)")?;
    /// assert_eq!(module.name(), None);
    ///
    /// let module = Module::new_with_name(&store, "(module)", "bar")?;
    /// assert_eq!(module.name(), Some("bar"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn name(&self) -> Option<&str> {
        self.inner.compiled.module().name.as_deref()
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
    /// # let store = Store::default();
    /// let module = Module::new(&store, "(module)")?;
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
    /// # let store = Store::default();
    /// let wat = r#"
    ///     (module
    ///         (import "host" "foo" (func))
    ///     )
    /// "#;
    /// let module = Module::new(&store, wat)?;
    /// assert_eq!(module.imports().len(), 1);
    /// let import = &module.imports()[0];
    /// assert_eq!(import.module(), "host");
    /// assert_eq!(import.name(), "foo");
    /// match import.ty() {
    ///     ExternType::Func(_) => { /* ... */ }
    ///     _ => panic!("unexpected import type!"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn imports(&self) -> &[ImportType] {
        &self.inner.imports
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
    /// # let store = Store::default();
    /// let module = Module::new(&store, "(module)")?;
    /// assert!(module.exports().is_empty());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// When the exports are not empty, you can inspect each export:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let wat = r#"
    ///     (module
    ///         (func (export "foo"))
    ///         (memory (export "memory") 1)
    ///     )
    /// "#;
    /// let module = Module::new(&store, wat)?;
    /// assert_eq!(module.exports().len(), 2);
    ///
    /// let foo = &module.exports()[0];
    /// assert_eq!(foo.name(), "foo");
    /// match foo.ty() {
    ///     ExternType::Func(_) => { /* ... */ }
    ///     _ => panic!("unexpected export type!"),
    /// }
    ///
    /// let memory = &module.exports()[1];
    /// assert_eq!(memory.name(), "memory");
    /// match memory.ty() {
    ///     ExternType::Memory(_) => { /* ... */ }
    ///     _ => panic!("unexpected export type!"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn exports(&self) -> &[ExportType] {
        &self.inner.exports
    }

    /// Returns the [`Store`] that this [`Module`] was compiled into.
    pub fn store(&self) -> &Store {
        &self.inner.store
    }

    /// Register this module's stack frame information into the global scope.
    ///
    /// This is required to ensure that any traps can be properly symbolicated.
    pub(crate) fn register_frame_info(&self) {
        let mut info = self.inner.frame_info_registration.lock().unwrap();
        if info.is_some() {
            return;
        }
        *info = Some(FRAME_INFO.register(&self.inner.compiled));
    }

    fn read_imports_and_exports(&mut self, binary: &[u8]) -> Result<()> {
        let mut inner = Arc::get_mut(&mut self.inner).unwrap();
        let mut reader = ModuleReader::new(binary)?;
        let mut imports = Vec::<(ImportType, ImportKind)>::new();
        let mut exports = Vec::new();
        let mut memories = Vec::new();
        let mut tables = Vec::new();
        let mut func_sig = Vec::new();
        let mut sigs = Vec::new();
        let mut globals = Vec::new();
        while !reader.eof() {
            let section = reader.read()?;
            match section.code {
                SectionCode::Memory => {
                    let section = section.get_memory_section_reader()?;
                    memories.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        memories.push(into_memory_type(entry?)?);
                    }
                }
                SectionCode::Type => {
                    let section = section.get_type_section_reader()?;
                    sigs.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        sigs.push(into_func_type(entry?));
                    }
                }
                SectionCode::Function => {
                    let section = section.get_function_section_reader()?;
                    func_sig.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        func_sig.push(entry?);
                    }
                }
                SectionCode::Global => {
                    let section = section.get_global_section_reader()?;
                    globals.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        globals.push(into_global_type(entry?.ty));
                    }
                }
                SectionCode::Table => {
                    let section = section.get_table_section_reader()?;
                    tables.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        tables.push(into_table_type(entry?))
                    }
                }
                SectionCode::Import => {
                    let section = section.get_import_section_reader()?;
                    imports.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        let entry = entry?;
                        let ty = match entry.ty {
                            ImportSectionEntryType::Function(index) => {
                                func_sig.push(index);
                                let sig = &sigs[index as usize];
                                ExternType::Func(sig.clone())
                            }
                            ImportSectionEntryType::Table(tt) => {
                                let table = into_table_type(tt);
                                tables.push(table.clone());
                                ExternType::Table(table)
                            }
                            ImportSectionEntryType::Memory(mt) => {
                                let memory = into_memory_type(mt)?;
                                memories.push(memory.clone());
                                ExternType::Memory(memory)
                            }
                            ImportSectionEntryType::Global(gt) => {
                                let global = into_global_type(gt);
                                globals.push(global.clone());
                                ExternType::Global(global)
                            }
                        };
                        imports.push((
                            ImportType::new(entry.module, entry.field, ty),
                            ImportKind::Core {
                                implemented_with_adapter: None,
                            },
                        ));
                    }
                }
                SectionCode::Export => {
                    let section = section.get_export_section_reader()?;
                    exports.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        let entry = entry?;
                        let ty = match entry.kind {
                            ExternalKind::Function => {
                                let sig_index = func_sig[entry.index as usize] as usize;
                                let sig = &sigs[sig_index];
                                ExternType::Func(sig.clone())
                            }
                            ExternalKind::Table => {
                                ExternType::Table(tables[entry.index as usize].clone())
                            }
                            ExternalKind::Memory => {
                                ExternType::Memory(memories[entry.index as usize].clone())
                            }
                            ExternalKind::Global => {
                                ExternType::Global(globals[entry.index as usize].clone())
                            }
                        };
                        exports.push(ExportType::new(entry.field, ty));
                        inner
                            .export_map
                            .insert(entry.field.to_string(), Export::Core);
                    }
                }
                SectionCode::Custom {
                    name: wit_schema_version::SECTION_NAME,
                    ..
                } => {
                    let range = section.range();
                    let bytes = &binary[range.start..range.end];
                    self.parse_wasm_interface_types_section(
                        range.start,
                        bytes,
                        &mut imports,
                        &mut exports,
                    )?;
                    inner = Arc::get_mut(&mut self.inner).unwrap();
                }
                SectionCode::Custom {
                    kind: CustomSectionKind::Name,
                    ..
                } => {
                    // Read name section. Per spec, ignore invalid custom section.
                    if let Ok(mut reader) = section.get_name_section_reader() {
                        while let Ok(entry) = reader.read() {
                            if let Name::Module(name) = entry {
                                if let Ok(name) = name.get_name() {
                                    Arc::get_mut(&mut inner.names).unwrap().module_name =
                                        Some(name.to_string());
                                }
                                break;
                            }
                        }
                    }
                }
                _ => {
                    // skip other sections
                }
            }
        }

        inner.exports = exports.into();

        // Given our list of imports, as well as any adapters which implement
        // those imports, build up the necessary metadata in our module.
        let mut import_list = Vec::new();
        let mut core_import_sources = Vec::new();
        for (import, kind) in imports {
            match kind {
                // If our core import has been implemented with an adapter, then
                // we record that it was implemented, and we don't add this to
                // the list of imports we expect the user to provide.
                ImportKind::Core {
                    implemented_with_adapter: Some(idx),
                } => core_import_sources.push(ImportSource::Adapter(idx)),

                // If the core import wasn't implemented with an adapter, then
                // it's expected to be user-provided.
                ImportKind::Core {
                    implemented_with_adapter: None,
                } => {
                    core_import_sources.push(ImportSource::UserProvided(import_list.len()));
                    import_list.push(import);
                }

                // .. and finally imports in the interface types section are
                // always expected to be imported.
                ImportKind::Adapter => import_list.push(import),
            }
        }
        inner.imports = import_list.into();
        inner.core_import_sources = core_import_sources.into();

        Ok(())
    }

    fn parse_wasm_interface_types_section(
        &mut self,
        offset: usize,
        section: &[u8],
        imports: &mut Vec<(ImportType, ImportKind)>,
        exports: &mut Vec<ExportType>,
    ) -> Result<()> {
        // If interface types aren't enabled then we treat this as an unknown
        // custom section, which like all others means we just skip over it.
        if !self.store().engine().config().interface_types {
            return Ok(());
        }

        let inner = Arc::get_mut(&mut self.inner).unwrap();
        let mut parser = wit_parser::Parser::new(offset, section)?;
        let mut types = Vec::new();
        let mut adapters = Vec::new();

        // With the presence of a wasm interface types section the list of
        // exports for a module are the interface types exports, not the core
        // module exports.
        exports.truncate(0);
        inner.export_map.drain();

        while !parser.is_empty() {
            match parser.section()? {
                wit_parser::Section::Type(list) => {
                    for ty in list {
                        let ty = ty?;
                        let params = ty.params.iter().map(cvt_ty).collect();
                        let results = ty.params.iter().map(cvt_ty).collect();
                        let ty = FuncType::new(params, results);
                        types.push(ty);
                    }
                }
                wit_parser::Section::Import(list) => {
                    for import in list {
                        let import = import?;
                        let ty = &types[import.ty as usize];
                        imports.push((
                            ImportType::new(
                                import.module,
                                import.name,
                                ExternType::Func(ty.clone()),
                            ),
                            ImportKind::Adapter,
                        ));
                        let idx = adapters.len();
                        adapters.push((ty.clone(), Adapter::Import(idx)));
                    }
                }
                wit_parser::Section::Func(list) => {
                    for func in list {
                        let func = func?;
                        let ty = types[func.ty as usize].clone();
                        let instrs = func.instrs().collect::<Result<Vec<_>, _>>()?;
                        adapters.push((ty, Adapter::Local(instrs)));
                    }
                }
                wit_parser::Section::Export(list) => {
                    for export in list {
                        let export = export?;
                        let ty = adapters[export.func as usize].0.clone();
                        exports.push(ExportType::new(export.name, ExternType::Func(ty)));
                        inner.export_map.insert(
                            export.name.to_string(),
                            Export::Adapter(export.func as usize),
                        );
                    }
                }
                wit_parser::Section::Implement(list) => {
                    let mut func_idx_to_import_idx = HashMap::new();
                    for (i, (import, _)) in imports.iter().enumerate() {
                        let func_idx = func_idx_to_import_idx.len() as u32;
                        if let ExternType::Func(_) = import.ty() {
                            func_idx_to_import_idx.insert(func_idx, i);
                        }
                    }

                    for implement in list {
                        let implement = implement?;
                        let import_idx = func_idx_to_import_idx[&implement.core_func];
                        match &mut imports[import_idx].1 {
                            ImportKind::Core {
                                implemented_with_adapter,
                            } => {
                                assert!(implemented_with_adapter.is_none());
                                *implemented_with_adapter = Some(implement.adapter_func as usize);
                            }
                            ImportKind::Adapter => panic!("invalid implement section"),
                        }
                    }
                }
            }
        }

        inner.adapters = adapters.into();

        return Ok(());

        fn cvt_ty(ty: &wit_parser::ValType) -> ValType {
            match ty {
                wit_parser::ValType::S8 => ValType::S8,
                wit_parser::ValType::S16 => ValType::S16,
                wit_parser::ValType::S32 => ValType::S32,
                wit_parser::ValType::S64 => ValType::S64,
                wit_parser::ValType::U8 => ValType::U8,
                wit_parser::ValType::U16 => ValType::U16,
                wit_parser::ValType::U32 => ValType::U32,
                wit_parser::ValType::U64 => ValType::U64,
                wit_parser::ValType::I32 => ValType::I32,
                wit_parser::ValType::I64 => ValType::I64,
                wit_parser::ValType::F32 => ValType::F32,
                wit_parser::ValType::F64 => ValType::F64,
                wit_parser::ValType::String => ValType::String,
                wit_parser::ValType::Anyref => ValType::AnyRef,
            }
        }
    }

    pub(crate) fn adapter(module: &Self, instance: InstanceHandle, idx: usize) -> Func {
        let ty = module.inner.adapters[idx].0.clone();
        let callable = Rc::new(CallAdapter {
            module: module.clone(),
            idx,
            instance,
        });
        Func::new(&module.inner.store, ty, callable)
    }
}

struct CallAdapter {
    module: Module,
    instance: InstanceHandle,
    idx: usize,
}

impl Callable for CallAdapter {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
        let (ty, adapter) = &self.module.inner.adapters[self.idx];
        let ty_params = ty.params();
        if params.len() != ty_params.len() {
            return Err(Trap::new(format!(
                "expected {} parameters, got {}",
                ty_params.len(),
                params.len()
            )));
        }
        if results.len() != ty.results().len() {
            return Err(Trap::new(format!(
                "expected {} results, got {}",
                ty.results().len(),
                params.len()
            )));
        }

        for ((i, param), expected) in params.iter().enumerate().zip(ty_params) {
            if param.ty() != *expected {
                return Err(Trap::new(format!(
                    "expected {:?} for parameter {}, got {:?}",
                    param.ty(),
                    i,
                    expected
                )));
            }
        }

        let mut stack = Vec::new();

        match adapter {
            Adapter::Local(instrs) => {
                for instr in instrs {
                    self.execute(&mut stack, params, instr)?;
                }
            }
            Adapter::Import(_) => panic!("unimplemented import"),
        }

        // should be true because of validation
        assert_eq!(stack.len(), results.len());
        for (item, slot) in stack.into_iter().zip(results) {
            *slot = item;
        }
        Ok(())
    }
}

impl CallAdapter {
    fn execute(
        &self,
        stack: &mut Vec<Val>,
        args: &[Val],
        instr: &wit_parser::Instruction,
    ) -> Result<(), Trap> {
        use wit_parser::Instruction::*;

        fn pop(stack: &mut Vec<Val>, ty: ValType) -> Val {
            let ret = stack.pop().unwrap();
            assert_eq!(ret.ty(), ty);
            return ret;
        }

        match instr {
            ArgGet(arg) => stack.push(args[*arg as usize].clone()),

            // Create a `FuncIndex` from the functin that we're calling and
            // then run through our other infrastructure to call into this
            // function.
            CallCore(f) => {
                let idx = FuncIndex::from_u32(*f);
                let sigidx = self.instance.module().local.functions[idx];
                let sig = &self.instance.module().local.signatures[sigidx];
                let export = wasmtime_environ::Export::Function(idx);
                if let wasmtime_runtime::Export::Function(export) = self.instance.clone().lookup_by_declaration(&export) {
                // let export = unimplemented!();
                let trampoline = self.instance.trampoline(export.signature).expect("failed to retrieve trampoline from module");
                let mut ret = vec![Val::I32(0); sig.returns.len()];
                // offset 1 here for the vmctx parameter
                let params_start = stack.len() + 1 - sig.params.len();
                WasmtimeFn::new(&self.module.inner.store, self.instance.clone(), export, trampoline)
                    .call(&stack[params_start..], &mut ret)?;
                stack.truncate(params_start);
                stack.extend(ret);
                }
            }

            I32ToS8 => {
                let val = pop(stack, ValType::I32).unwrap_i32();
                stack.push(Val::S8(val as i8));
            }
            I32ToS8X => {
                let val = pop(stack, ValType::I32).unwrap_i32();
                match val.try_into() {
                    Ok(v) => stack.push(Val::S8(v)),
                    Err(_) => return Err(Trap::new("integer overflow")),
                }
            }
            I32ToU8 => {
                let val = pop(stack, ValType::I32).unwrap_i32();
                stack.push(Val::U8(val as u8));
            }
            i => return Err(Trap::new(format!("unimplemented instruction {:?}", i))),
        }

        Ok(())
    }
}
