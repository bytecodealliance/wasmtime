use crate::module::{
    Initializer, InstanceSignature, MemoryPlan, Module, ModuleSignature, ModuleType, TableElements,
    TablePlan, TypeTables,
};
use crate::tunables::Tunables;
use cranelift_codegen::ir;
use cranelift_codegen::ir::{AbiParam, ArgumentPurpose};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{
    self, translate_module, Alias, DataIndex, DefinedFuncIndex, ElemIndex, EntityIndex, EntityType,
    FuncIndex, Global, GlobalIndex, InstanceIndex, InstanceTypeIndex, Memory, MemoryIndex,
    ModuleIndex, ModuleTypeIndex, SignatureIndex, Table, TableIndex, TargetEnvironment, TypeIndex,
    WasmError, WasmFuncType, WasmResult,
};
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};
use std::convert::TryFrom;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use wasmparser::Type as WasmType;
use wasmparser::{FuncValidator, FunctionBody, ValidatorResources, WasmFeatures};

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

    // Various bits and pieces of configuration
    features: WasmFeatures,
    target_config: TargetFrontendConfig,
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

    /// References to the data initializers.
    pub data_initializers: Vec<DataInitializer<'data>>,

    /// DWARF debug information, if enabled, parsed from the module.
    pub debuginfo: DebugInfoData<'data>,

    /// Set if debuginfo was found but it was not parsed due to `Tunables`
    /// configuration.
    pub has_unparsed_debuginfo: bool,

    /// When we're parsing the code section this will be incremented so we know
    /// which function is currently being defined.
    code_index: u32,

    implicit_instances: HashMap<&'data str, InstanceIndex>,
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
    pub fn new(
        target_config: TargetFrontendConfig,
        tunables: &Tunables,
        features: &WasmFeatures,
    ) -> Self {
        Self {
            result: ModuleTranslation::default(),
            results: Vec::with_capacity(1),
            in_progress: Vec::new(),
            modules_to_be: 1,
            types: Default::default(),
            target_config,
            tunables: tunables.clone(),
            features: *features,
            first_module: true,
        }
    }

    fn pointer_type(&self) -> ir::Type {
        self.target_config.pointer_type()
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
        translate_module(data, &mut self)?;
        assert!(self.results.len() > 0);
        Ok((self.results.len() - 1, self.results, self.types))
    }

    fn declare_export(&mut self, export: EntityIndex, name: &str) -> WasmResult<()> {
        self.result
            .module
            .exports
            .insert(String::from(name), export);
        Ok(())
    }

    fn register_dwarf_section(&mut self, name: &str, data: &'data [u8]) {
        if !self.tunables.generate_native_debuginfo && !self.tunables.parse_wasm_debuginfo {
            self.result.has_unparsed_debuginfo = true;
            return;
        }

        if !name.starts_with(".debug_") {
            return;
        }
        let info = &mut self.result.debuginfo;
        let dwarf = &mut info.dwarf;
        let endian = gimli::LittleEndian;
        let slice = gimli::EndianSlice::new(data, endian);

        match name {
            ".debug_str" => dwarf.debug_str = gimli::DebugStr::new(data, endian),
            ".debug_abbrev" => dwarf.debug_abbrev = gimli::DebugAbbrev::new(data, endian),
            ".debug_info" => dwarf.debug_info = gimli::DebugInfo::new(data, endian),
            ".debug_line" => dwarf.debug_line = gimli::DebugLine::new(data, endian),
            ".debug_addr" => dwarf.debug_addr = gimli::DebugAddr::from(slice),
            ".debug_line_str" => dwarf.debug_line_str = gimli::DebugLineStr::from(slice),
            ".debug_str_sup" => dwarf.debug_str_sup = gimli::DebugStr::from(slice),
            ".debug_ranges" => info.debug_ranges = gimli::DebugRanges::new(data, endian),
            ".debug_rnglists" => info.debug_rnglists = gimli::DebugRngLists::new(data, endian),
            ".debug_loc" => info.debug_loc = gimli::DebugLoc::from(slice),
            ".debug_loclists" => info.debug_loclists = gimli::DebugLocLists::from(slice),
            ".debug_str_offsets" => dwarf.debug_str_offsets = gimli::DebugStrOffsets::from(slice),
            ".debug_types" => dwarf.debug_types = gimli::DebugTypes::from(slice),
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
            EntityType::Event(_) => unimplemented!(),
        }
    }

    fn gen_type_of_module(&mut self, module: usize) -> ModuleTypeIndex {
        let module = &self.results[module].module;
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
}

impl<'data> TargetEnvironment for ModuleEnvironment<'data> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.target_config
    }

    fn reference_type(&self, ty: cranelift_wasm::WasmType) -> ir::Type {
        crate::reference_type(ty, self.pointer_type())
    }
}

/// This trait is useful for `translate_module` because it tells how to translate
/// environment-dependent wasm instructions. These functions should not be called by the user.
impl<'data> cranelift_wasm::ModuleEnvironment<'data> for ModuleEnvironment<'data> {
    fn reserve_types(&mut self, num: u32) -> WasmResult<()> {
        let num = usize::try_from(num).unwrap();
        self.result.module.types.reserve(num);
        self.types.native_signatures.reserve(num);
        self.types.wasm_signatures.reserve(num);
        Ok(())
    }

    fn declare_type_func(&mut self, wasm: WasmFuncType, sig: ir::Signature) -> WasmResult<()> {
        let sig = translate_signature(sig, self.pointer_type());

        // FIXME(#2469): Signatures should be deduplicated in these two tables
        // since `SignatureIndex` is already a index space separate from the
        // module's index space. Note that this may get more urgent with
        // module-linking modules where types are more likely to get repeated
        // (across modules).
        let sig_index = self.types.native_signatures.push(sig);
        let sig_index2 = self.types.wasm_signatures.push(wasm);
        debug_assert_eq!(sig_index, sig_index2);
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

    fn reserve_imports(&mut self, num: u32) -> WasmResult<()> {
        Ok(self
            .result
            .module
            .initializers
            .reserve(usize::try_from(num).unwrap()))
    }

    fn declare_func_import(
        &mut self,
        index: TypeIndex,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.functions.len(),
            self.result.module.num_imported_funcs,
            "Imported functions must be declared first"
        );
        let sig_index = self.result.module.types[index].unwrap_function();
        self.declare_import(module, field, EntityType::Function(sig_index));
        self.result.module.num_imported_funcs += 1;
        self.result.debuginfo.wasm_file.imported_func_count += 1;
        Ok(())
    }

    fn declare_table_import(
        &mut self,
        table: Table,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.table_plans.len(),
            self.result.module.num_imported_tables,
            "Imported tables must be declared first"
        );
        self.declare_import(module, field, EntityType::Table(table));
        self.result.module.num_imported_tables += 1;
        Ok(())
    }

    fn declare_memory_import(
        &mut self,
        memory: Memory,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.memory_plans.len(),
            self.result.module.num_imported_memories,
            "Imported memories must be declared first"
        );
        if memory.shared {
            return Err(WasmError::Unsupported("shared memories".to_owned()));
        }
        self.declare_import(module, field, EntityType::Memory(memory));
        self.result.module.num_imported_memories += 1;
        Ok(())
    }

    fn declare_global_import(
        &mut self,
        global: Global,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.globals.len(),
            self.result.module.num_imported_globals,
            "Imported globals must be declared first"
        );
        self.declare_import(module, field, EntityType::Global(global));
        self.result.module.num_imported_globals += 1;
        Ok(())
    }

    fn declare_module_import(
        &mut self,
        ty_index: TypeIndex,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        let signature = self.type_to_module_type(ty_index)?;
        self.declare_import(module, field, EntityType::Module(signature));
        Ok(())
    }

    fn declare_instance_import(
        &mut self,
        ty_index: TypeIndex,
        module: &'data str,
        field: Option<&'data str>,
    ) -> WasmResult<()> {
        let signature = self.type_to_instance_type(ty_index)?;
        self.declare_import(module, field, EntityType::Instance(signature));
        Ok(())
    }

    fn reserve_func_types(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .functions
            .reserve_exact(usize::try_from(num).unwrap());
        self.result
            .function_body_inputs
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_func_type(&mut self, index: TypeIndex) -> WasmResult<()> {
        let sig_index = self.result.module.types[index].unwrap_function();
        self.result.module.functions.push(sig_index);
        Ok(())
    }

    fn reserve_tables(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .table_plans
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_table(&mut self, table: Table) -> WasmResult<()> {
        let plan = TablePlan::for_table(table, &self.tunables);
        self.result.module.table_plans.push(plan);
        Ok(())
    }

    fn reserve_memories(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .memory_plans
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_memory(&mut self, memory: Memory) -> WasmResult<()> {
        if memory.shared {
            return Err(WasmError::Unsupported("shared memories".to_owned()));
        }
        let plan = MemoryPlan::for_memory(memory, &self.tunables);
        self.result.module.memory_plans.push(plan);
        Ok(())
    }

    fn reserve_globals(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .globals
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_global(&mut self, global: Global) -> WasmResult<()> {
        self.result.module.globals.push(global);
        Ok(())
    }

    fn reserve_exports(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .exports
            .reserve(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_func_export(&mut self, func_index: FuncIndex, name: &str) -> WasmResult<()> {
        self.declare_export(EntityIndex::Function(func_index), name)
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &str) -> WasmResult<()> {
        self.declare_export(EntityIndex::Table(table_index), name)
    }

    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &str) -> WasmResult<()> {
        self.declare_export(EntityIndex::Memory(memory_index), name)
    }

    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &str) -> WasmResult<()> {
        self.declare_export(EntityIndex::Global(global_index), name)
    }

    fn declare_module_export(&mut self, index: ModuleIndex, name: &str) -> WasmResult<()> {
        self.declare_export(EntityIndex::Module(index), name)
    }

    fn declare_instance_export(&mut self, index: InstanceIndex, name: &str) -> WasmResult<()> {
        self.declare_export(EntityIndex::Instance(index), name)
    }

    fn declare_start_func(&mut self, func_index: FuncIndex) -> WasmResult<()> {
        debug_assert!(self.result.module.start_func.is_none());
        self.result.module.start_func = Some(func_index);
        Ok(())
    }

    fn reserve_table_elements(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .table_elements
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        elements: Box<[FuncIndex]>,
    ) -> WasmResult<()> {
        self.result.module.table_elements.push(TableElements {
            table_index,
            base,
            offset,
            elements,
        });
        Ok(())
    }

    fn declare_passive_element(
        &mut self,
        elem_index: ElemIndex,
        segments: Box<[FuncIndex]>,
    ) -> WasmResult<()> {
        let old = self
            .result
            .module
            .passive_elements
            .insert(elem_index, segments);
        debug_assert!(
            old.is_none(),
            "should never get duplicate element indices, that would be a bug in `cranelift_wasm`'s \
             translation"
        );
        Ok(())
    }

    fn reserve_function_bodies(&mut self, _count: u32, offset: u64) {
        self.result.debuginfo.wasm_file.code_section_offset = offset;
    }

    fn define_function_body(
        &mut self,
        validator: FuncValidator<ValidatorResources>,
        body: FunctionBody<'data>,
    ) -> WasmResult<()> {
        if self.tunables.generate_native_debuginfo {
            let func_index = self.result.code_index + self.result.module.num_imported_funcs as u32;
            let func_index = FuncIndex::from_u32(func_index);
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
                    params: sig.params.iter().cloned().map(|i| i.into()).collect(),
                });
        }
        self.result
            .function_body_inputs
            .push(FunctionBodyData { validator, body });
        self.result.code_index += 1;
        Ok(())
    }

    fn reserve_data_initializers(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .data_initializers
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        data: &'data [u8],
    ) -> WasmResult<()> {
        self.result.data_initializers.push(DataInitializer {
            location: DataInitializerLocation {
                memory_index,
                base,
                offset,
            },
            data,
        });
        Ok(())
    }

    fn reserve_passive_data(&mut self, count: u32) -> WasmResult<()> {
        self.result.module.passive_data.reserve(count as usize);
        Ok(())
    }

    fn declare_passive_data(&mut self, data_index: DataIndex, data: &'data [u8]) -> WasmResult<()> {
        let old = self
            .result
            .module
            .passive_data
            .insert(data_index, Arc::from(data));
        debug_assert!(
            old.is_none(),
            "a module can't have duplicate indices, this would be a cranelift-wasm bug"
        );
        Ok(())
    }

    fn declare_module_name(&mut self, name: &'data str) {
        self.result.module.name = Some(name.to_string());
        if self.tunables.generate_native_debuginfo {
            self.result.debuginfo.name_section.module_name = Some(name);
        }
    }

    fn declare_func_name(&mut self, func_index: FuncIndex, name: &'data str) {
        self.result
            .module
            .func_names
            .insert(func_index, name.to_string());
        if self.tunables.generate_native_debuginfo {
            self.result
                .debuginfo
                .name_section
                .func_names
                .insert(func_index.as_u32(), name);
        }
    }

    fn declare_local_name(&mut self, func_index: FuncIndex, local: u32, name: &'data str) {
        if self.tunables.generate_native_debuginfo {
            self.result
                .debuginfo
                .name_section
                .locals_names
                .entry(func_index.as_u32())
                .or_insert(HashMap::new())
                .insert(local, name);
        }
    }

    fn custom_section(&mut self, name: &'data str, data: &'data [u8]) -> WasmResult<()> {
        self.register_dwarf_section(name, data);

        match name {
            "webidl-bindings" | "wasm-interface-types" => Err(WasmError::Unsupported(
                "\
Support for interface types has temporarily been removed from `wasmtime`.

For more information about this temoprary you can read on the issue online:

    https://github.com/bytecodealliance/wasmtime/issues/1271

and for re-adding support for interface types you can see this issue:

    https://github.com/bytecodealliance/wasmtime/issues/677
"
                .to_owned(),
            )),

            // skip other sections
            _ => Ok(()),
        }
    }

    fn wasm_features(&self) -> WasmFeatures {
        self.features
    }

    fn reserve_modules(&mut self, amount: u32) {
        // Go ahead and reserve space in the final `results` array for `amount`
        // more modules.
        self.modules_to_be += amount as usize;
        self.results.reserve(self.modules_to_be);

        // Then also reserve space in our own local module's metadata fields
        // we'll be adding to.
        self.result.module.modules.reserve(amount as usize);
        self.result.module.initializers.reserve(amount as usize);
    }

    fn module_start(&mut self) {
        // If this is the first time this method is called, nothing to do.
        if self.first_module {
            self.first_module = false;
            return;
        }
        // Reset our internal state for a new module by saving the current
        // module in `results`.
        let in_progress = mem::replace(&mut self.result, ModuleTranslation::default());
        self.in_progress.push(in_progress);
        self.modules_to_be -= 1;
    }

    fn module_end(&mut self) {
        let (record_initializer, done) = match self.in_progress.pop() {
            Some(m) => (true, mem::replace(&mut self.result, m)),
            None => (false, mem::take(&mut self.result)),
        };
        self.results.push(done);
        if record_initializer {
            let index = self.results.len() - 1;
            self.result
                .module
                .initializers
                .push(Initializer::DefineModule(index));
            let sig = self.gen_type_of_module(index);
            self.result.module.modules.push(sig);
        }
    }

    fn reserve_instances(&mut self, amt: u32) {
        self.result.module.instances.reserve(amt as usize);
        self.result.module.initializers.reserve(amt as usize);
    }

    fn declare_instance(
        &mut self,
        module: ModuleIndex,
        args: Vec<(&'data str, EntityIndex)>,
    ) -> WasmResult<()> {
        let args = args.into_iter().map(|(s, i)| (s.to_string(), i)).collect();
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
        Ok(())
    }

    fn declare_alias(&mut self, alias: Alias) -> WasmResult<()> {
        match alias {
            // Types are easy, we statically know everything so we're just
            // copying some pointers from our parent module to our own module.
            //
            // Note that we don't add an initializer for this alias because
            // we statically know where all types point to.
            Alias::OuterType {
                relative_depth,
                index,
            } => {
                let module_idx = self.in_progress.len() - 1 - (relative_depth as usize);
                let ty = self.in_progress[module_idx].module.types[index];
                self.result.module.types.push(ty);
            }

            // FIXME(WebAssembly/module-linking#28) unsure how to implement this
            // at this time, if we can alias imported modules it's a lot harder,
            // otherwise we'll need to figure out how to translate `index` to a
            // `usize` for a defined module (creating Initializer::DefineModule)
            Alias::OuterModule {
                relative_depth,
                index,
            } => {
                drop((relative_depth, index));
                unimplemented!()
            }

            // This case is slightly more involved, we'll be recording all the
            // type information for each kind of entity, and then we also need
            // to record an initialization step to get the export from the
            // instance.
            Alias::InstanceExport { instance, export } => {
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
                    EntityType::Event(_) => unimplemented!(),
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

        Ok(())
    }
}

/// Add environment-specific function parameters.
pub fn translate_signature(mut sig: ir::Signature, pointer_type: ir::Type) -> ir::Signature {
    // Prepend the vmctx argument.
    sig.params.insert(
        0,
        AbiParam::special(pointer_type, ArgumentPurpose::VMContext),
    );
    // Prepend the caller vmctx argument.
    sig.params.insert(1, AbiParam::new(pointer_type));
    sig
}

/// A memory index and offset within that memory where a data initialization
/// should is to be performed.
#[derive(Clone, Serialize, Deserialize)]
pub struct DataInitializerLocation {
    /// The index of the memory to initialize.
    pub memory_index: MemoryIndex,

    /// Optionally a globalvar base to initialize at.
    pub base: Option<GlobalIndex>,

    /// A constant offset to initialize at.
    pub offset: usize,
}

/// A data initializer for linear memory.
pub struct DataInitializer<'data> {
    /// The location where the initialization is to be performed.
    pub location: DataInitializerLocation,

    /// The initialization data.
    pub data: &'data [u8],
}
