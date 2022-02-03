use crate::module::{
    Initializer, InstanceSignature, MemoryInitialization, MemoryInitializer, MemoryPlan, Module,
    ModuleSignature, ModuleType, ModuleUpvar, TableInitializer, TablePlan, TypeTables,
};
use crate::{
    DataIndex, DefinedFuncIndex, ElemIndex, EntityIndex, EntityType, FuncIndex, Global,
    GlobalIndex, GlobalInit, InstanceIndex, InstanceTypeIndex, MemoryIndex, ModuleIndex,
    ModuleTypeIndex, PrimaryMap, SignatureIndex, TableIndex, Tunables, TypeIndex, WasmError,
    WasmFuncType, WasmResult,
};
use cranelift_entity::packed_option::ReservedValue;
use std::borrow::Cow;
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use wasmparser::Type as WasmType;
use wasmparser::{
    Alias, DataKind, ElementItem, ElementKind, ExternalKind, FuncValidator, FunctionBody,
    ImportSectionEntryType, NameSectionReader, Naming, Operator, Parser, Payload, TypeDef,
    Validator, ValidatorResources, WasmFeatures,
};

/// Object containing the standalone environment information.
pub struct ModuleEnvironment<'data> {
    /// The current module being translated
    result: ModuleTranslation<'data>,

    /// Modules which have finished translation. This only really applies for
    /// the module linking proposal.
    results: Vec<ModuleTranslation<'data>>,

    /// Modules which are in-progress being translated, or otherwise also known
    /// as the outer modules of the current module being processed.
    in_progress: Vec<ModuleTranslation<'data>>,

    /// How many modules that have not yet made their way into `results` which
    /// are coming at some point.
    modules_to_be: usize,

    /// Intern'd types for this entire translation, shared by all modules.
    types: TypeTables,

    interned_func_types: HashMap<WasmFuncType, SignatureIndex>,

    // Various bits and pieces of configuration
    features: WasmFeatures,
    tunables: Tunables,
    first_module: bool,
}

/// The result of translating via `ModuleEnvironment`. Function bodies are not
/// yet translated, and data initializers have not yet been copied out of the
/// original buffer.
#[derive(Default)]
pub struct ModuleTranslation<'data> {
    /// Module information.
    pub module: Module,

    /// References to the function bodies.
    pub function_body_inputs: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,

    /// The set of defined functions within this module which are "possibly
    /// exported" which means that the host can possibly call them. This
    /// includes functions such as:
    ///
    /// * Exported functions
    /// * Functions in element segments
    /// * Functions via `ref.func` instructions
    ///
    /// This set is used to determine the set of type signatures that need
    /// trampolines for the host to call into.
    pub escaped_funcs: HashSet<DefinedFuncIndex>,

    /// A list of type signatures which are considered exported from this
    /// module, or those that can possibly be called. This list is sorted, and
    /// trampolines for each of these signatures are required.
    pub exported_signatures: Vec<SignatureIndex>,

    /// DWARF debug information, if enabled, parsed from the module.
    pub debuginfo: DebugInfoData<'data>,

    /// Set if debuginfo was found but it was not parsed due to `Tunables`
    /// configuration.
    pub has_unparsed_debuginfo: bool,

    /// List of data segments found in this module which should be concatenated
    /// together for the final compiled artifact.
    ///
    /// These data segments, when concatenated, are indexed by the
    /// `MemoryInitializer` type.
    pub data: Vec<Cow<'data, [u8]>>,

    /// Total size of all data pushed onto `data` so far.
    total_data: u32,

    /// List of passive element segments found in this module which will get
    /// concatenated for the final artifact.
    pub passive_data: Vec<&'data [u8]>,

    /// Total size of all passive data pushed into `passive_data` so far.
    total_passive_data: u32,

    /// When we're parsing the code section this will be incremented so we know
    /// which function is currently being defined.
    code_index: u32,

    implicit_instances: HashMap<&'data str, InstanceIndex>,

    /// The artifacts which are needed from the parent module when this module
    /// is created. This is used to insert into `Initializer::CreateModule` when
    /// this module is defined in the parent.
    creation_artifacts: Vec<usize>,

    /// Same as `creation_artifacts`, but for modules instead of artifacts.
    creation_modules: Vec<ModuleUpvar>,
}

/// Contains function data: byte code and its offset in the module.
pub struct FunctionBodyData<'a> {
    /// The body of the function, containing code and locals.
    pub body: FunctionBody<'a>,
    /// Validator for the function body
    pub validator: FuncValidator<ValidatorResources>,
}

#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct DebugInfoData<'a> {
    pub dwarf: Dwarf<'a>,
    pub name_section: NameSection<'a>,
    pub wasm_file: WasmFileInfo,
    debug_loc: gimli::DebugLoc<Reader<'a>>,
    debug_loclists: gimli::DebugLocLists<Reader<'a>>,
    pub debug_ranges: gimli::DebugRanges<Reader<'a>>,
    pub debug_rnglists: gimli::DebugRngLists<Reader<'a>>,
}

#[allow(missing_docs)]
pub type Dwarf<'input> = gimli::Dwarf<Reader<'input>>;

type Reader<'input> = gimli::EndianSlice<'input, gimli::LittleEndian>;

#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct NameSection<'a> {
    pub module_name: Option<&'a str>,
    pub func_names: HashMap<u32, &'a str>,
    pub locals_names: HashMap<u32, HashMap<u32, &'a str>>,
}

#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct WasmFileInfo {
    pub path: Option<PathBuf>,
    pub code_section_offset: u64,
    pub imported_func_count: u32,
    pub funcs: Vec<FunctionMetadata>,
}

#[derive(Debug)]
#[allow(missing_docs)]
pub struct FunctionMetadata {
    pub params: Box<[WasmType]>,
    pub locals: Box<[(u32, WasmType)]>,
}

impl<'data> ModuleEnvironment<'data> {
    /// Allocates the environment data structures.
    pub fn new(tunables: &Tunables, features: &WasmFeatures) -> Self {
        Self {
            result: ModuleTranslation::default(),
            results: Vec::with_capacity(1),
            in_progress: Vec::new(),
            modules_to_be: 1,
            types: Default::default(),
            tunables: tunables.clone(),
            features: *features,
            first_module: true,
            interned_func_types: Default::default(),
        }
    }

    /// Translate a wasm module using this environment.
    ///
    /// This consumes the `ModuleEnvironment` and produces a list of
    /// `ModuleTranslation`s as well as a `TypeTables`. The list of module
    /// translations corresponds to all wasm modules found in the input `data`.
    /// Note that for MVP modules this will always be a list with one element,
    /// but with the module linking proposal this may have many elements.
    ///
    /// For the module linking proposal the top-level module is returned as the
    /// first return value.
    ///
    /// The `TypeTables` structure returned contains intern'd versions of types
    /// referenced from each module translation. This primarily serves as the
    /// source of truth for module-linking use cases where modules can refer to
    /// other module's types. All `SignatureIndex`, `ModuleTypeIndex`, and
    /// `InstanceTypeIndex` values are resolved through the returned tables.
    pub fn translate(
        mut self,
        data: &'data [u8],
    ) -> WasmResult<(usize, Vec<ModuleTranslation<'data>>, TypeTables)> {
        let mut validator = Validator::new();
        validator.wasm_features(self.features);

        for payload in Parser::new(0).parse_all(data) {
            self.translate_payload(&mut validator, payload?)?;
        }

        assert!(self.results.len() > 0);
        Ok((self.results.len() - 1, self.results, self.types))
    }

    fn translate_payload(
        &mut self,
        validator: &mut Validator,
        payload: Payload<'data>,
    ) -> WasmResult<()> {
        match payload {
            Payload::Version { num, range } => {
                validator.version(num, &range)?;

                // If this is the first time this method is called, nothing to
                // do.
                if self.first_module {
                    self.first_module = false;
                } else {
                    // Reset our internal state for a new module by saving the
                    // current module in `results`.
                    let in_progress = mem::replace(&mut self.result, ModuleTranslation::default());
                    self.in_progress.push(in_progress);
                    self.modules_to_be -= 1;
                }
            }

            Payload::End => {
                validator.end()?;

                // With the `escaped_funcs` set of functions finished
                // we can calculate the set of signatures that are exported as
                // the set of exported functions' signatures.
                self.result.exported_signatures = self
                    .result
                    .module
                    .functions
                    .iter()
                    .filter_map(|(i, sig)| match self.result.module.defined_func_index(i) {
                        Some(i) if !self.result.escaped_funcs.contains(&i) => None,
                        _ => Some(*sig),
                    })
                    .collect();
                self.result.exported_signatures.sort_unstable();
                self.result.exported_signatures.dedup();

                self.result.creation_artifacts.shrink_to_fit();
                self.result.creation_modules.shrink_to_fit();

                let (record_initializer, mut done) = match self.in_progress.pop() {
                    Some(m) => (true, mem::replace(&mut self.result, m)),
                    None => (false, mem::take(&mut self.result)),
                };

                if record_initializer {
                    // Record the type of the module we just finished in our own
                    // module's list of modules.
                    let sig = self.gen_type_of_module(&done.module);
                    self.result.module.modules.push(sig);

                    // The root module will store the artifacts for this
                    // finished module at `artifact_index`. This then needs to
                    // be inherited by all later modules coming down to our
                    // now-current `self.result`...
                    let mut artifact_index = self.results.len();
                    for result in self.in_progress.iter_mut().chain(Some(&mut self.result)) {
                        result.creation_artifacts.push(artifact_index);
                        artifact_index = result.creation_artifacts.len() - 1;
                    }
                    // ... and then `self.result` needs to create a new module
                    // with whatever was record to save off as its own
                    // artifacts/modules.
                    self.result
                        .module
                        .initializers
                        .push(Initializer::CreateModule {
                            artifact_index,
                            artifacts: mem::take(&mut done.creation_artifacts),
                            modules: mem::take(&mut done.creation_modules),
                        });
                }

                // And the final step is to insert the module into the list of
                // finished modules to get returned at the end.
                self.results.push(done);
            }

            Payload::TypeSection(types) => {
                validator.type_section(&types)?;
                let num = usize::try_from(types.get_count()).unwrap();
                self.result.module.types.reserve(num);
                self.types.wasm_signatures.reserve(num);

                for ty in types {
                    match ty? {
                        TypeDef::Func(wasm_func_ty) => {
                            self.declare_type_func(wasm_func_ty.try_into()?)?;
                        }
                        TypeDef::Module(t) => {
                            let imports = t
                                .imports
                                .iter()
                                .map(|i| Ok((i.module, i.field, self.entity_type(i.ty)?)))
                                .collect::<WasmResult<Vec<_>>>()?;
                            let exports = t
                                .exports
                                .iter()
                                .map(|e| Ok((e.name, self.entity_type(e.ty)?)))
                                .collect::<WasmResult<Vec<_>>>()?;
                            self.declare_type_module(&imports, &exports)?;
                        }
                        TypeDef::Instance(t) => {
                            let exports = t
                                .exports
                                .iter()
                                .map(|e| Ok((e.name, self.entity_type(e.ty)?)))
                                .collect::<WasmResult<Vec<_>>>()?;
                            self.declare_type_instance(&exports)?;
                        }
                    }
                }
            }

            Payload::ImportSection(imports) => {
                validator.import_section(&imports)?;

                let cnt = usize::try_from(imports.get_count()).unwrap();
                self.result.module.initializers.reserve(cnt);

                for entry in imports {
                    let import = entry?;
                    let ty = match import.ty {
                        ImportSectionEntryType::Function(index) => {
                            let index = TypeIndex::from_u32(index);
                            let sig_index = self.result.module.types[index].unwrap_function();
                            self.result.module.num_imported_funcs += 1;
                            self.result.debuginfo.wasm_file.imported_func_count += 1;
                            EntityType::Function(sig_index)
                        }
                        ImportSectionEntryType::Module(index) => {
                            let index = TypeIndex::from_u32(index);
                            let signature = self.type_to_module_type(index)?;
                            EntityType::Module(signature)
                        }
                        ImportSectionEntryType::Instance(index) => {
                            let index = TypeIndex::from_u32(index);
                            let signature = self.type_to_instance_type(index)?;
                            EntityType::Instance(signature)
                        }
                        ImportSectionEntryType::Memory(ty) => {
                            if ty.shared {
                                return Err(WasmError::Unsupported("shared memories".to_owned()));
                            }
                            self.result.module.num_imported_memories += 1;
                            EntityType::Memory(ty.into())
                        }
                        ImportSectionEntryType::Global(ty) => {
                            self.result.module.num_imported_globals += 1;
                            EntityType::Global(Global::new(ty, GlobalInit::Import)?)
                        }
                        ImportSectionEntryType::Table(ty) => {
                            self.result.module.num_imported_tables += 1;
                            EntityType::Table(ty.try_into()?)
                        }

                        // doesn't get past validation
                        ImportSectionEntryType::Tag(_) => unreachable!(),
                    };
                    self.declare_import(import.module, import.field, ty);
                }
            }

            Payload::FunctionSection(functions) => {
                validator.function_section(&functions)?;

                let cnt = usize::try_from(functions.get_count()).unwrap();
                self.result.module.functions.reserve_exact(cnt);

                for entry in functions {
                    let sigindex = entry?;
                    let ty = TypeIndex::from_u32(sigindex);
                    let sig_index = self.result.module.types[ty].unwrap_function();
                    self.result.module.functions.push(sig_index);
                }
            }

            Payload::TableSection(tables) => {
                validator.table_section(&tables)?;
                let cnt = usize::try_from(tables.get_count()).unwrap();
                self.result.module.table_plans.reserve_exact(cnt);

                for entry in tables {
                    let table = entry?.try_into()?;
                    let plan = TablePlan::for_table(table, &self.tunables);
                    self.result.module.table_plans.push(plan);
                }
            }

            Payload::MemorySection(memories) => {
                validator.memory_section(&memories)?;

                let cnt = usize::try_from(memories.get_count()).unwrap();
                self.result.module.memory_plans.reserve_exact(cnt);

                for entry in memories {
                    let memory = entry?;
                    if memory.shared {
                        return Err(WasmError::Unsupported("shared memories".to_owned()));
                    }
                    let plan = MemoryPlan::for_memory(memory.into(), &self.tunables);
                    self.result.module.memory_plans.push(plan);
                }
            }

            Payload::TagSection(tags) => {
                validator.tag_section(&tags)?;

                // This feature isn't enabled at this time, so we should
                // never get here.
                unreachable!();
            }

            Payload::GlobalSection(globals) => {
                validator.global_section(&globals)?;

                let cnt = usize::try_from(globals.get_count()).unwrap();
                self.result.module.globals.reserve_exact(cnt);

                for entry in globals {
                    let wasmparser::Global { ty, init_expr } = entry?;
                    let mut init_expr_reader = init_expr.get_binary_reader();
                    let initializer = match init_expr_reader.read_operator()? {
                        Operator::I32Const { value } => GlobalInit::I32Const(value),
                        Operator::I64Const { value } => GlobalInit::I64Const(value),
                        Operator::F32Const { value } => GlobalInit::F32Const(value.bits()),
                        Operator::F64Const { value } => GlobalInit::F64Const(value.bits()),
                        Operator::V128Const { value } => {
                            GlobalInit::V128Const(u128::from_le_bytes(*value.bytes()))
                        }
                        Operator::RefNull { ty: _ } => GlobalInit::RefNullConst,
                        Operator::RefFunc { function_index } => {
                            let index = FuncIndex::from_u32(function_index);
                            self.flag_func_escaped(index);
                            GlobalInit::RefFunc(index)
                        }
                        Operator::GlobalGet { global_index } => {
                            GlobalInit::GetGlobal(GlobalIndex::from_u32(global_index))
                        }
                        s => {
                            return Err(WasmError::Unsupported(format!(
                                "unsupported init expr in global section: {:?}",
                                s
                            )));
                        }
                    };
                    let ty = Global::new(ty, initializer)?;
                    self.result.module.globals.push(ty);
                }
            }

            Payload::ExportSection(exports) => {
                validator.export_section(&exports)?;

                let cnt = usize::try_from(exports.get_count()).unwrap();
                self.result.module.exports.reserve(cnt);

                for entry in exports {
                    let wasmparser::Export { field, kind, index } = entry?;
                    let entity = match kind {
                        ExternalKind::Function => {
                            let index = FuncIndex::from_u32(index);
                            self.flag_func_escaped(index);
                            EntityIndex::Function(index)
                        }
                        ExternalKind::Table => EntityIndex::Table(TableIndex::from_u32(index)),
                        ExternalKind::Memory => EntityIndex::Memory(MemoryIndex::from_u32(index)),
                        ExternalKind::Global => EntityIndex::Global(GlobalIndex::from_u32(index)),
                        ExternalKind::Module => EntityIndex::Module(ModuleIndex::from_u32(index)),
                        ExternalKind::Instance => {
                            EntityIndex::Instance(InstanceIndex::from_u32(index))
                        }

                        // this never gets past validation
                        ExternalKind::Tag | ExternalKind::Type => unreachable!(),
                    };
                    self.result
                        .module
                        .exports
                        .insert(String::from(field), entity);
                }
            }

            Payload::StartSection { func, range } => {
                validator.start_section(func, &range)?;

                let func_index = FuncIndex::from_u32(func);
                self.flag_func_escaped(func_index);
                debug_assert!(self.result.module.start_func.is_none());
                self.result.module.start_func = Some(func_index);
            }

            Payload::ElementSection(elements) => {
                validator.element_section(&elements)?;

                let cnt = usize::try_from(elements.get_count()).unwrap();
                self.result.module.table_initializers.reserve_exact(cnt);

                for (index, entry) in elements.into_iter().enumerate() {
                    let wasmparser::Element {
                        kind,
                        items,
                        ty: _,
                        range: _,
                    } = entry?;

                    // Build up a list of `FuncIndex` corresponding to all the
                    // entries listed in this segment. Note that it's not
                    // possible to create anything other than a `ref.null
                    // extern` for externref segments, so those just get
                    // translate to the reserved value of `FuncIndex`.
                    let items_reader = items.get_items_reader()?;
                    let mut elements =
                        Vec::with_capacity(usize::try_from(items_reader.get_count()).unwrap());
                    for item in items_reader {
                        let func = match item? {
                            ElementItem::Func(f) => Some(f),
                            ElementItem::Expr(init) => {
                                match init.get_binary_reader().read_operator()? {
                                    Operator::RefNull { .. } => None,
                                    Operator::RefFunc { function_index } => Some(function_index),
                                    s => {
                                        return Err(WasmError::Unsupported(format!(
                                            "unsupported init expr in element section: {:?}",
                                            s
                                        )));
                                    }
                                }
                            }
                        };
                        elements.push(match func {
                            Some(f) => {
                                let f = FuncIndex::from_u32(f);
                                self.flag_func_escaped(f);
                                f
                            }
                            None => FuncIndex::reserved_value(),
                        });
                    }

                    match kind {
                        ElementKind::Active {
                            table_index,
                            init_expr,
                        } => {
                            let table_index = TableIndex::from_u32(table_index);
                            let mut init_expr_reader = init_expr.get_binary_reader();
                            let (base, offset) = match init_expr_reader.read_operator()? {
                                Operator::I32Const { value } => (None, value as u32),
                                Operator::GlobalGet { global_index } => {
                                    (Some(GlobalIndex::from_u32(global_index)), 0)
                                }
                                ref s => {
                                    return Err(WasmError::Unsupported(format!(
                                        "unsupported init expr in element section: {:?}",
                                        s
                                    )));
                                }
                            };
                            self.result
                                .module
                                .table_initializers
                                .push(TableInitializer {
                                    table_index,
                                    base,
                                    offset,
                                    elements: elements.into(),
                                });
                        }

                        ElementKind::Passive => {
                            let elem_index = ElemIndex::from_u32(index as u32);
                            let index = self.result.module.passive_elements.len();
                            self.result.module.passive_elements.push(elements.into());
                            self.result
                                .module
                                .passive_elements_map
                                .insert(elem_index, index);
                        }

                        ElementKind::Declared => {}
                    }
                }
            }

            Payload::CodeSectionStart { count, range, .. } => {
                validator.code_section_start(count, &range)?;
                let cnt = usize::try_from(count).unwrap();
                self.result.function_body_inputs.reserve_exact(cnt);
                self.result.debuginfo.wasm_file.code_section_offset = range.start as u64;
            }

            Payload::CodeSectionEntry(mut body) => {
                let validator = validator.code_section_entry()?;
                let func_index =
                    self.result.code_index + self.result.module.num_imported_funcs as u32;
                let func_index = FuncIndex::from_u32(func_index);

                if self.tunables.generate_native_debuginfo {
                    let sig_index = self.result.module.functions[func_index];
                    let sig = &self.types.wasm_signatures[sig_index];
                    let mut locals = Vec::new();
                    for pair in body.get_locals_reader()? {
                        locals.push(pair?);
                    }
                    self.result
                        .debuginfo
                        .wasm_file
                        .funcs
                        .push(FunctionMetadata {
                            locals: locals.into_boxed_slice(),
                            params: sig.params().iter().cloned().map(|i| i.into()).collect(),
                        });
                }
                body.allow_memarg64(self.features.memory64);
                self.result
                    .function_body_inputs
                    .push(FunctionBodyData { validator, body });
                self.result.code_index += 1;
            }

            Payload::DataSection(data) => {
                validator.data_section(&data)?;

                let initializers = match &mut self.result.module.memory_initialization {
                    MemoryInitialization::Segmented(i) => i,
                    _ => unreachable!(),
                };

                let cnt = usize::try_from(data.get_count()).unwrap();
                initializers.reserve_exact(cnt);
                self.result.data.reserve_exact(cnt);

                for (index, entry) in data.into_iter().enumerate() {
                    let wasmparser::Data {
                        kind,
                        data,
                        range: _,
                    } = entry?;
                    let mk_range = |total: &mut u32| -> Result<_, WasmError> {
                        let range = u32::try_from(data.len())
                            .ok()
                            .and_then(|size| {
                                let start = *total;
                                let end = start.checked_add(size)?;
                                Some(start..end)
                            })
                            .ok_or_else(|| {
                                WasmError::Unsupported(format!(
                                    "more than 4 gigabytes of data in wasm module",
                                ))
                            })?;
                        *total += range.end - range.start;
                        Ok(range)
                    };
                    match kind {
                        DataKind::Active {
                            memory_index,
                            init_expr,
                        } => {
                            let range = mk_range(&mut self.result.total_data)?;
                            let memory_index = MemoryIndex::from_u32(memory_index);
                            let mut init_expr_reader = init_expr.get_binary_reader();
                            let (base, offset) = match init_expr_reader.read_operator()? {
                                Operator::I32Const { value } => (None, value as u64),
                                Operator::I64Const { value } => (None, value as u64),
                                Operator::GlobalGet { global_index } => {
                                    (Some(GlobalIndex::from_u32(global_index)), 0)
                                }
                                s => {
                                    return Err(WasmError::Unsupported(format!(
                                        "unsupported init expr in data section: {:?}",
                                        s
                                    )));
                                }
                            };

                            initializers.push(MemoryInitializer {
                                memory_index,
                                base,
                                offset,
                                data: range,
                            });
                            self.result.data.push(data.into());
                        }
                        DataKind::Passive => {
                            let data_index = DataIndex::from_u32(index as u32);
                            let range = mk_range(&mut self.result.total_passive_data)?;
                            self.result.passive_data.push(data);
                            self.result
                                .module
                                .passive_data_map
                                .insert(data_index, range);
                        }
                    }
                }
            }

            Payload::DataCountSection { count, range } => {
                validator.data_count_section(count, &range)?;

                // Note: the count passed in here is the *total* segment count
                // There is no way to reserve for just the passive segments as
                // they are discovered when iterating the data section entries
                // Given that the total segment count might be much larger than
                // the passive count, do not reserve anything here.
            }

            Payload::InstanceSection(s) => {
                validator.instance_section(&s)?;

                let cnt = usize::try_from(s.get_count()).unwrap();
                self.result.module.instances.reserve(cnt);
                self.result.module.initializers.reserve(cnt);

                for instance in s {
                    let instance = instance?;
                    let module = ModuleIndex::from_u32(instance.module());
                    let args = instance
                        .args()?
                        .into_iter()
                        .map(|arg| {
                            let arg = arg?;
                            let index = match arg.kind {
                                ExternalKind::Function => {
                                    EntityIndex::Function(FuncIndex::from_u32(arg.index))
                                }
                                ExternalKind::Table => {
                                    EntityIndex::Table(TableIndex::from_u32(arg.index))
                                }
                                ExternalKind::Memory => {
                                    EntityIndex::Memory(MemoryIndex::from_u32(arg.index))
                                }
                                ExternalKind::Global => {
                                    EntityIndex::Global(GlobalIndex::from_u32(arg.index))
                                }
                                ExternalKind::Module => {
                                    EntityIndex::Module(ModuleIndex::from_u32(arg.index))
                                }
                                ExternalKind::Instance => {
                                    EntityIndex::Instance(InstanceIndex::from_u32(arg.index))
                                }

                                // this won't pass validation
                                ExternalKind::Tag | ExternalKind::Type => unreachable!(),
                            };
                            Ok((arg.name.to_string(), index))
                        })
                        .collect::<WasmResult<_>>()?;

                    // Record the type of this instance with the type signature of the
                    // module we're instantiating and then also add an initializer which
                    // records that we'll be adding to the instance index space here.
                    let module_ty = self.result.module.modules[module];
                    let instance_ty = self.types.module_signatures[module_ty].exports;
                    self.result.module.instances.push(instance_ty);
                    self.result
                        .module
                        .initializers
                        .push(Initializer::Instantiate { module, args });
                }
            }
            Payload::AliasSection(s) => {
                validator.alias_section(&s)?;

                for alias in s {
                    match alias? {
                        // Types are easy, we statically know everything so
                        // we're just copying some pointers from our parent
                        // module to our own module.
                        //
                        // Note that we don't add an initializer for this alias
                        // because we statically know where all types point to.
                        Alias::OuterType {
                            relative_depth,
                            index,
                        } => {
                            let index = TypeIndex::from_u32(index);
                            let module_idx = self.in_progress.len() - 1 - (relative_depth as usize);
                            let ty = self.in_progress[module_idx].module.types[index];
                            self.result.module.types.push(ty);
                        }

                        // Modules are a bit trickier since we need to record
                        // how to track the state from the original module down
                        // to our own.
                        Alias::OuterModule {
                            relative_depth,
                            index,
                        } => {
                            let index = ModuleIndex::from_u32(index);

                            // First we can copy the type from the parent module
                            // into our own module to record what type our
                            // module definition will have.
                            let module_idx = self.in_progress.len() - 1 - (relative_depth as usize);
                            let module_ty = self.in_progress[module_idx].module.modules[index];
                            self.result.module.modules.push(module_ty);

                            // Next we'll be injecting a module value that is
                            // closed over, and that will be used to define the
                            // module into the index space. Record an
                            // initializer about where our module is sourced
                            // from (which will be stored within each module
                            // value itself).
                            let module_index = self.result.creation_modules.len();
                            self.result
                                .module
                                .initializers
                                .push(Initializer::DefineModule(module_index));

                            // And finally we need to record a breadcrumb trail
                            // of how to get the module value into
                            // `module_index`. The module just after our
                            // destination module will use a `ModuleIndex` to
                            // fetch the module value, and everything else
                            // inbetween will inherit that module's closed-over
                            // value.
                            let mut upvar = ModuleUpvar::Local(index);
                            for outer in self.in_progress[module_idx + 1..].iter_mut() {
                                let upvar = mem::replace(
                                    &mut upvar,
                                    ModuleUpvar::Inherit(outer.creation_modules.len()),
                                );
                                outer.creation_modules.push(upvar);
                            }
                            self.result.creation_modules.push(upvar);
                        }

                        // This case is slightly more involved, we'll be
                        // recording all the type information for each kind of
                        // entity, and then we also need to record an
                        // initialization step to get the export from the
                        // instance.
                        Alias::InstanceExport {
                            instance,
                            export,
                            kind: _,
                        } => {
                            let instance = InstanceIndex::from_u32(instance);
                            let ty = self.result.module.instances[instance];
                            match &self.types.instance_signatures[ty].exports[export] {
                                EntityType::Global(g) => {
                                    self.result.module.globals.push(g.clone());
                                    self.result.module.num_imported_globals += 1;
                                }
                                EntityType::Memory(mem) => {
                                    let plan = MemoryPlan::for_memory(*mem, &self.tunables);
                                    self.result.module.memory_plans.push(plan);
                                    self.result.module.num_imported_memories += 1;
                                }
                                EntityType::Table(t) => {
                                    let plan = TablePlan::for_table(*t, &self.tunables);
                                    self.result.module.table_plans.push(plan);
                                    self.result.module.num_imported_tables += 1;
                                }
                                EntityType::Function(sig) => {
                                    self.result.module.functions.push(*sig);
                                    self.result.module.num_imported_funcs += 1;
                                    self.result.debuginfo.wasm_file.imported_func_count += 1;
                                }
                                EntityType::Instance(sig) => {
                                    self.result.module.instances.push(*sig);
                                }
                                EntityType::Module(sig) => {
                                    self.result.module.modules.push(*sig);
                                }
                                EntityType::Tag(_) => unimplemented!(),
                            }
                            self.result
                                .module
                                .initializers
                                .push(Initializer::AliasInstanceExport {
                                    instance,
                                    export: export.to_string(),
                                })
                        }
                    }
                }
            }

            Payload::ModuleSectionStart {
                count,
                range,
                size: _,
            } => {
                validator.module_section_start(count, &range)?;

                // Go ahead and reserve space in the final `results` array for `amount`
                // more modules.
                self.modules_to_be += count as usize;
                self.results.reserve(self.modules_to_be);

                // Then also reserve space in our own local module's metadata fields
                // we'll be adding to.
                self.result.module.modules.reserve(count as usize);
                self.result.module.initializers.reserve(count as usize);
            }

            Payload::ModuleSectionEntry { .. } => {
                validator.module_section_entry();
                // note that nothing else happens here since we rely on the next
                // `Version` payload to recurse in the parsed modules.
            }

            Payload::CustomSection {
                name: "name",
                data,
                data_offset,
                range: _,
            } => {
                let result = NameSectionReader::new(data, data_offset)
                    .map_err(|e| e.into())
                    .and_then(|s| self.name_section(s));
                if let Err(e) = result {
                    log::warn!("failed to parse name section {:?}", e);
                }
            }

            Payload::CustomSection {
                name: "webidl-bindings",
                ..
            }
            | Payload::CustomSection {
                name: "wasm-interface-types",
                ..
            } => {
                return Err(WasmError::Unsupported(
                    "\
Support for interface types has temporarily been removed from `wasmtime`.

For more information about this temoprary you can read on the issue online:

    https://github.com/bytecodealliance/wasmtime/issues/1271

and for re-adding support for interface types you can see this issue:

    https://github.com/bytecodealliance/wasmtime/issues/677
"
                    .to_string(),
                ))
            }

            Payload::CustomSection { name, data, .. } => {
                self.register_dwarf_section(name, data);
            }

            Payload::UnknownSection { id, range, .. } => {
                validator.unknown_section(id, &range)?;
                unreachable!();
            }
        }
        Ok(())
    }

    fn register_dwarf_section(&mut self, name: &str, data: &'data [u8]) {
        if !name.starts_with(".debug_") {
            return;
        }
        if !self.tunables.generate_native_debuginfo && !self.tunables.parse_wasm_debuginfo {
            self.result.has_unparsed_debuginfo = true;
            return;
        }
        let info = &mut self.result.debuginfo;
        let dwarf = &mut info.dwarf;
        let endian = gimli::LittleEndian;
        let slice = gimli::EndianSlice::new(data, endian);

        match name {
            // `gimli::Dwarf` fields.
            ".debug_abbrev" => dwarf.debug_abbrev = gimli::DebugAbbrev::new(data, endian),
            ".debug_addr" => dwarf.debug_addr = gimli::DebugAddr::from(slice),
            ".debug_info" => dwarf.debug_info = gimli::DebugInfo::new(data, endian),
            ".debug_line" => dwarf.debug_line = gimli::DebugLine::new(data, endian),
            ".debug_line_str" => dwarf.debug_line_str = gimli::DebugLineStr::from(slice),
            ".debug_str" => dwarf.debug_str = gimli::DebugStr::new(data, endian),
            ".debug_str_offsets" => dwarf.debug_str_offsets = gimli::DebugStrOffsets::from(slice),
            ".debug_str_sup" => {
                let mut dwarf_sup: Dwarf<'data> = Default::default();
                dwarf_sup.debug_str = gimli::DebugStr::from(slice);
                dwarf.sup = Some(Arc::new(dwarf_sup));
            }
            ".debug_types" => dwarf.debug_types = gimli::DebugTypes::from(slice),

            // Additional fields.
            ".debug_loc" => info.debug_loc = gimli::DebugLoc::from(slice),
            ".debug_loclists" => info.debug_loclists = gimli::DebugLocLists::from(slice),
            ".debug_ranges" => info.debug_ranges = gimli::DebugRanges::new(data, endian),
            ".debug_rnglists" => info.debug_rnglists = gimli::DebugRngLists::new(data, endian),

            // We don't use these at the moment.
            ".debug_aranges" | ".debug_pubnames" | ".debug_pubtypes" => return,

            other => {
                log::warn!("unknown debug section `{}`", other);
                return;
            }
        }

        dwarf.ranges = gimli::RangeLists::new(info.debug_ranges, info.debug_rnglists);
        dwarf.locations = gimli::LocationLists::new(info.debug_loc, info.debug_loclists);
    }

    /// Declares a new import with the `module` and `field` names, importing the
    /// `ty` specified.
    ///
    /// Note that this method is somewhat tricky due to the implementation of
    /// the module linking proposal. In the module linking proposal two-level
    /// imports are recast as single-level imports of instances. That recasting
    /// happens here by recording an import of an instance for the first time
    /// we see a two-level import.
    ///
    /// When the module linking proposal is disabled, however, disregard this
    /// logic and instead work directly with two-level imports since no
    /// instances are defined.
    fn declare_import(&mut self, module: &'data str, field: Option<&'data str>, ty: EntityType) {
        if !self.features.module_linking {
            assert!(field.is_some());
            let index = self.push_type(ty);
            self.result.module.initializers.push(Initializer::Import {
                name: module.to_owned(),
                field: field.map(|s| s.to_string()),
                index,
            });
            return;
        }

        match field {
            Some(field) => {
                // If this is a two-level import then this is actually an
                // implicit import of an instance, where each two-level import
                // is an alias directive from the original instance. The first
                // thing we do here is lookup our implicit instance, creating a
                // blank one if it wasn't already created.
                let instance = match self.result.implicit_instances.entry(module) {
                    Entry::Occupied(e) => *e.get(),
                    Entry::Vacant(v) => {
                        let ty = self
                            .types
                            .instance_signatures
                            .push(InstanceSignature::default());
                        let idx = self.result.module.instances.push(ty);
                        self.result.module.initializers.push(Initializer::Import {
                            name: module.to_owned(),
                            field: None,
                            index: EntityIndex::Instance(idx),
                        });
                        *v.insert(idx)
                    }
                };

                // Update the implicit instance's type signature with this new
                // field and its type.
                self.types.instance_signatures[self.result.module.instances[instance]]
                    .exports
                    .insert(field.to_string(), ty.clone());

                // Record our implicit alias annotation which corresponds to
                // this import that we're processing.
                self.result
                    .module
                    .initializers
                    .push(Initializer::AliasInstanceExport {
                        instance,
                        export: field.to_string(),
                    });

                // And then record the type information for the item that we're
                // processing.
                self.push_type(ty);
            }
            None => {
                // Without a field then this is a single-level import (a feature
                // of module linking) which means we're simply importing that
                // name with the specified type. Record the type information and
                // then the name that we're importing.
                let index = self.push_type(ty);
                self.result.module.initializers.push(Initializer::Import {
                    name: module.to_owned(),
                    field: None,
                    index,
                });
            }
        }
    }

    fn entity_type(&self, ty: ImportSectionEntryType) -> WasmResult<EntityType> {
        Ok(match ty {
            ImportSectionEntryType::Function(sig) => {
                EntityType::Function(self.type_to_signature(TypeIndex::from_u32(sig))?)
            }
            ImportSectionEntryType::Module(sig) => {
                EntityType::Module(self.type_to_module_type(TypeIndex::from_u32(sig))?)
            }
            ImportSectionEntryType::Instance(sig) => {
                EntityType::Instance(self.type_to_instance_type(TypeIndex::from_u32(sig))?)
            }
            ImportSectionEntryType::Memory(ty) => EntityType::Memory(ty.into()),
            ImportSectionEntryType::Tag(t) => EntityType::Tag(t.into()),
            ImportSectionEntryType::Global(ty) => {
                EntityType::Global(Global::new(ty, GlobalInit::Import)?)
            }
            ImportSectionEntryType::Table(ty) => EntityType::Table(ty.try_into()?),
        })
    }

    fn push_type(&mut self, ty: EntityType) -> EntityIndex {
        match ty {
            EntityType::Function(ty) => {
                EntityIndex::Function(self.result.module.functions.push(ty))
            }
            EntityType::Table(ty) => {
                let plan = TablePlan::for_table(ty, &self.tunables);
                EntityIndex::Table(self.result.module.table_plans.push(plan))
            }
            EntityType::Memory(ty) => {
                let plan = MemoryPlan::for_memory(ty, &self.tunables);
                EntityIndex::Memory(self.result.module.memory_plans.push(plan))
            }
            EntityType::Global(ty) => EntityIndex::Global(self.result.module.globals.push(ty)),
            EntityType::Instance(ty) => {
                EntityIndex::Instance(self.result.module.instances.push(ty))
            }
            EntityType::Module(ty) => EntityIndex::Module(self.result.module.modules.push(ty)),
            EntityType::Tag(_) => unimplemented!(),
        }
    }

    fn gen_type_of_module(&mut self, module: &Module) -> ModuleTypeIndex {
        let imports = module
            .imports()
            .map(|(s, field, ty)| {
                assert!(field.is_none());
                (s.to_string(), ty)
            })
            .collect();
        let exports = module
            .exports
            .iter()
            .map(|(name, idx)| (name.clone(), module.type_of(*idx)))
            .collect();

        // FIXME(#2469): this instance/module signature insertion should likely
        // be deduplicated.
        let exports = self
            .types
            .instance_signatures
            .push(InstanceSignature { exports });
        self.types
            .module_signatures
            .push(ModuleSignature { imports, exports })
    }

    fn flag_func_escaped(&mut self, func: FuncIndex) {
        if let Some(idx) = self.result.module.defined_func_index(func) {
            self.result.escaped_funcs.insert(idx);
        }
    }

    fn declare_type_func(&mut self, wasm: WasmFuncType) -> WasmResult<()> {
        // Deduplicate wasm function signatures through `interned_func_types`,
        // which also deduplicates across wasm modules with module linking.
        let sig_index = match self.interned_func_types.get(&wasm) {
            Some(idx) => *idx,
            None => {
                let sig_index = self.types.wasm_signatures.push(wasm.clone());
                self.interned_func_types.insert(wasm, sig_index);
                sig_index
            }
        };
        self.result
            .module
            .types
            .push(ModuleType::Function(sig_index));
        Ok(())
    }

    fn declare_type_module(
        &mut self,
        declared_imports: &[(&'data str, Option<&'data str>, EntityType)],
        exports: &[(&'data str, EntityType)],
    ) -> WasmResult<()> {
        let mut imports = indexmap::IndexMap::new();
        let mut instance_types = HashMap::new();
        for (module, field, ty) in declared_imports {
            match field {
                Some(field) => {
                    let idx = *instance_types
                        .entry(module)
                        .or_insert_with(|| self.types.instance_signatures.push(Default::default()));
                    self.types.instance_signatures[idx]
                        .exports
                        .insert(field.to_string(), ty.clone());
                    if !imports.contains_key(*module) {
                        imports.insert(module.to_string(), EntityType::Instance(idx));
                    }
                }
                None => {
                    imports.insert(module.to_string(), ty.clone());
                }
            }
        }
        let exports = exports
            .iter()
            .map(|e| (e.0.to_string(), e.1.clone()))
            .collect();

        // FIXME(#2469): Like signatures above we should probably deduplicate
        // the listings of module types since with module linking it's possible
        // you'll need to write down the module type in multiple locations.
        let exports = self
            .types
            .instance_signatures
            .push(InstanceSignature { exports });
        let idx = self
            .types
            .module_signatures
            .push(ModuleSignature { imports, exports });
        self.result.module.types.push(ModuleType::Module(idx));
        Ok(())
    }

    fn declare_type_instance(&mut self, exports: &[(&'data str, EntityType)]) -> WasmResult<()> {
        let exports = exports
            .iter()
            .map(|e| (e.0.to_string(), e.1.clone()))
            .collect();

        // FIXME(#2469): Like signatures above we should probably deduplicate
        // the listings of instance types since with module linking it's
        // possible you'll need to write down the module type in multiple
        // locations.
        let idx = self
            .types
            .instance_signatures
            .push(InstanceSignature { exports });
        self.result.module.types.push(ModuleType::Instance(idx));
        Ok(())
    }

    fn type_to_signature(&self, index: TypeIndex) -> WasmResult<SignatureIndex> {
        match self.result.module.types[index] {
            ModuleType::Function(sig) => Ok(sig),
            _ => unreachable!(),
        }
    }

    fn type_to_module_type(&self, index: TypeIndex) -> WasmResult<ModuleTypeIndex> {
        match self.result.module.types[index] {
            ModuleType::Module(sig) => Ok(sig),
            _ => unreachable!(),
        }
    }

    fn type_to_instance_type(&self, index: TypeIndex) -> WasmResult<InstanceTypeIndex> {
        match self.result.module.types[index] {
            ModuleType::Instance(sig) => Ok(sig),
            _ => unreachable!(),
        }
    }

    /// Parses the Name section of the wasm module.
    fn name_section(&mut self, names: NameSectionReader<'data>) -> WasmResult<()> {
        for subsection in names {
            match subsection? {
                wasmparser::Name::Function(f) => {
                    let mut names = f.get_map()?;
                    for _ in 0..names.get_count() {
                        let Naming { index, name } = names.read()?;
                        // Skip this naming if it's naming a function that
                        // doesn't actually exist.
                        if (index as usize) >= self.result.module.functions.len() {
                            continue;
                        }
                        let index = FuncIndex::from_u32(index);
                        self.result
                            .module
                            .func_names
                            .insert(index, name.to_string());
                        if self.tunables.generate_native_debuginfo {
                            self.result
                                .debuginfo
                                .name_section
                                .func_names
                                .insert(index.as_u32(), name);
                        }
                    }
                }
                wasmparser::Name::Module(module) => {
                    let name = module.get_name()?;
                    self.result.module.name = Some(name.to_string());
                    if self.tunables.generate_native_debuginfo {
                        self.result.debuginfo.name_section.module_name = Some(name);
                    }
                }
                wasmparser::Name::Local(l) => {
                    if !self.tunables.generate_native_debuginfo {
                        continue;
                    }
                    let mut reader = l.get_indirect_map()?;
                    for _ in 0..reader.get_indirect_count() {
                        let f = reader.read()?;
                        // Skip this naming if it's naming a function that
                        // doesn't actually exist.
                        if (f.indirect_index as usize) >= self.result.module.functions.len() {
                            continue;
                        }
                        let mut map = f.get_map()?;
                        for _ in 0..map.get_count() {
                            let Naming { index, name } = map.read()?;

                            self.result
                                .debuginfo
                                .name_section
                                .locals_names
                                .entry(f.indirect_index)
                                .or_insert(HashMap::new())
                                .insert(index, name);
                        }
                    }
                }
                wasmparser::Name::Label(_)
                | wasmparser::Name::Type(_)
                | wasmparser::Name::Table(_)
                | wasmparser::Name::Global(_)
                | wasmparser::Name::Memory(_)
                | wasmparser::Name::Element(_)
                | wasmparser::Name::Data(_)
                | wasmparser::Name::Unknown { .. } => {}
            }
        }
        Ok(())
    }
}
