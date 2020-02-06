use crate::frame_info::{GlobalFrameInfoRegistration, FRAME_INFO};
use crate::runtime::Store;
use crate::types::{
    ExportType, ExternType, FuncType, GlobalType, ImportType, Limits, MemoryType, Mutability,
    TableType, ValType,
};
use anyhow::{bail, Error, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};
use wasmparser::{
    validate, CustomSectionKind, ExternalKind, ImportSectionEntryType, ModuleReader, Name,
    OperatorValidatorConfig, SectionCode, ValidatingParserConfig,
};
use wasmtime_jit::CompiledModule;

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
/// ## Modules and `Clone`
///
/// Using `clone` on a `Module` is a cheap operation. It will not create an
/// entirely new module, but rather just a new reference to the existing module.
/// In other words it's a shallow copy, not a deep copy.
#[derive(Clone)]
pub struct Module {
    inner: Arc<ModuleInner>,
}

struct ModuleInner {
    store: Store,
    imports: Box<[ImportType]>,
    exports: Box<[ExportType]>,
    compiled: CompiledModule,
    frame_info_registration: Mutex<Option<Option<GlobalFrameInfoRegistration>>>,
    names: Arc<Names>,
}

pub struct Names {
    pub module: Arc<wasmtime_environ::Module>,
    pub module_name: Option<String>,
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
        Arc::get_mut(&mut inner.names).unwrap().module_name = Some(name.to_string());
        Ok(module)
    }

    /// Creates a new WebAssembly `Module` from the contents of the given
    /// `file` on disk.
    ///
    /// This is a convenience function that will read the `file` provided and
    /// pass the bytes to the [`Module::new`] function. For more information
    /// see [`Module::new`]
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
    /// requirement for [`Module::new_unchecked`] to be safe.
    ///
    /// # Errors
    ///
    /// If validation fails for any reason (type check error, usage of a feature
    /// that wasn't enabled, etc) then an error with a description of the
    /// validation issue will be returned.
    ///
    /// [binary]: https://webassembly.github.io/spec/core/binary/index.html
    pub fn validate(store: &Store, binary: &[u8]) -> Result<()> {
        let features = store.engine().config().features.clone();
        let config = ValidatingParserConfig {
            operator_config: OperatorValidatorConfig {
                enable_threads: features.threads,
                enable_reference_types: features.reference_types,
                enable_bulk_memory: features.bulk_memory,
                enable_simd: features.simd,
                enable_multi_value: features.multi_value,
            },
        };
        validate(binary, Some(config)).map_err(Error::new)
    }

    unsafe fn compile(store: &Store, binary: &[u8]) -> Result<Self> {
        let compiled = CompiledModule::new(
            &mut store.compiler_mut(),
            binary,
            store.engine().config().debug_info,
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
                names,
                compiled,
                frame_info_registration: Mutex::new(None),
            }),
        })
    }

    pub(crate) fn compiled_module(&self) -> &CompiledModule {
        &self.inner.compiled
    }

    /// Returns identifier/name that this [`Module`] has. This name
    /// is used in traps/backtrace details.
    pub fn name(&self) -> Option<&str> {
        self.inner.names.module_name.as_deref()
    }

    /// Returns the list of imports that this [`Module`] has and must be
    /// satisfied.
    pub fn imports(&self) -> &[ImportType] {
        &self.inner.imports
    }

    /// Returns the list of exports that this [`Module`] has and will be
    /// available after instantiation.
    pub fn exports(&self) -> &[ExportType] {
        &self.inner.exports
    }

    /// Returns the [`Store`] that this [`Module`] was compiled into.
    pub fn store(&self) -> &Store {
        &self.inner.store
    }

    fn read_imports_and_exports(&mut self, binary: &[u8]) -> Result<()> {
        let inner = Arc::get_mut(&mut self.inner).unwrap();
        let mut reader = ModuleReader::new(binary)?;
        let mut imports = Vec::new();
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
                        let r#type = match entry.ty {
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
                        imports.push(ImportType::new(entry.module, entry.field, r#type));
                    }
                }
                SectionCode::Export => {
                    let section = section.get_export_section_reader()?;
                    exports.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        let entry = entry?;
                        let r#type = match entry.kind {
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
                        exports.push(ExportType::new(entry.field, r#type));
                    }
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

        inner.imports = imports.into();
        inner.exports = exports.into();
        Ok(())
    }

    /// Register this module's stack frame information into the global scope.
    ///
    /// This is required to ensure that any traps can be properly symbolicated.
    pub(crate) fn register_frame_info(&self) {
        let mut info = self.inner.frame_info_registration.lock().unwrap();
        if info.is_some() {
            return;
        }
        *info = Some(FRAME_INFO.register(&self.inner.names, &self.inner.compiled));
    }
}
