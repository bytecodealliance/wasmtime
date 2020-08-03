use crate::module::{EntityIndex, MemoryPlan, Module, TableElements, TablePlan};
use crate::tunables::Tunables;
use cranelift_codegen::ir;
use cranelift_codegen::ir::{AbiParam, ArgumentPurpose};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{
    self, translate_module, DataIndex, DefinedFuncIndex, ElemIndex, FuncIndex, Global, GlobalIndex,
    Memory, MemoryIndex, ModuleTranslationState, SignatureIndex, Table, TableIndex,
    TargetEnvironment, WasmError, WasmFuncType, WasmResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::sync::Arc;
use wasmparser::Type as WasmType;

/// Object containing the standalone environment information.
pub struct ModuleEnvironment<'data> {
    /// The result to be filled in.
    result: ModuleTranslation<'data>,
    code_index: u32,
}

/// The result of translating via `ModuleEnvironment`. Function bodies are not
/// yet translated, and data initializers have not yet been copied out of the
/// original buffer.
pub struct ModuleTranslation<'data> {
    /// Compilation setting flags.
    pub target_config: TargetFrontendConfig,

    /// Module information.
    pub module: Module,

    /// References to the function bodies.
    pub function_body_inputs: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,

    /// References to the data initializers.
    pub data_initializers: Vec<DataInitializer<'data>>,

    /// Tunable parameters.
    pub tunables: Tunables,

    /// The decoded Wasm types for the module.
    pub module_translation: Option<ModuleTranslationState>,

    /// DWARF debug information, if enabled, parsed from the module.
    pub debuginfo: Option<DebugInfoData<'data>>,
}

/// Contains function data: byte code and its offset in the module.
#[derive(Hash)]
pub struct FunctionBodyData<'a> {
    /// Body byte code.
    pub data: &'a [u8],

    /// Body offset in the module file.
    pub module_offset: usize,
}

#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct DebugInfoData<'a> {
    pub dwarf: Dwarf<'a>,
    pub name_section: NameSection<'a>,
    pub wasm_file: WasmFileInfo,
    debug_loc: gimli::DebugLoc<Reader<'a>>,
    debug_loclists: gimli::DebugLocLists<Reader<'a>>,
    debug_ranges: gimli::DebugRanges<Reader<'a>>,
    debug_rnglists: gimli::DebugRngLists<Reader<'a>>,
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
    pub fn new(target_config: TargetFrontendConfig, tunables: &Tunables) -> Self {
        Self {
            result: ModuleTranslation {
                target_config,
                module: Module::new(),
                function_body_inputs: PrimaryMap::new(),
                data_initializers: Vec::new(),
                tunables: tunables.clone(),
                module_translation: None,
                debuginfo: if tunables.debug_info {
                    Some(DebugInfoData::default())
                } else {
                    None
                },
            },
            code_index: 0,
        }
    }

    fn pointer_type(&self) -> ir::Type {
        self.result.target_config.pointer_type()
    }

    /// Translate a wasm module using this environment. This consumes the
    /// `ModuleEnvironment` and produces a `ModuleTranslation`.
    pub fn translate(mut self, data: &'data [u8]) -> WasmResult<ModuleTranslation<'data>> {
        assert!(self.result.module_translation.is_none());
        let module_translation = translate_module(data, &mut self)?;
        self.result.module_translation = Some(module_translation);
        Ok(self.result)
    }

    fn declare_export(&mut self, export: EntityIndex, name: &str) -> WasmResult<()> {
        self.result
            .module
            .exports
            .insert(String::from(name), export);
        Ok(())
    }

    fn register_dwarf_section(&mut self, name: &str, data: &'data [u8]) {
        let info = match &mut self.result.debuginfo {
            Some(info) => info,
            None => return,
        };
        if !name.starts_with(".debug_") {
            return;
        }
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
}

impl<'data> TargetEnvironment for ModuleEnvironment<'data> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.result.target_config
    }

    fn reference_type(&self, ty: cranelift_wasm::WasmType) -> ir::Type {
        crate::reference_type(ty, self.pointer_type())
    }
}

/// This trait is useful for `translate_module` because it tells how to translate
/// environment-dependent wasm instructions. These functions should not be called by the user.
impl<'data> cranelift_wasm::ModuleEnvironment<'data> for ModuleEnvironment<'data> {
    fn reserve_signatures(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .signatures
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_signature(&mut self, wasm: WasmFuncType, sig: ir::Signature) -> WasmResult<()> {
        let sig = translate_signature(sig, self.pointer_type());
        // TODO: Deduplicate signatures.
        self.result.module.local.signatures.push((wasm, sig));
        Ok(())
    }

    fn reserve_imports(&mut self, num: u32) -> WasmResult<()> {
        Ok(self
            .result
            .module
            .imports
            .reserve_exact(usize::try_from(num).unwrap()))
    }

    fn declare_func_import(
        &mut self,
        sig_index: SignatureIndex,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.local.functions.len(),
            self.result.module.local.num_imported_funcs,
            "Imported functions must be declared first"
        );
        let func_index = self.result.module.local.functions.push(sig_index);
        self.result.module.imports.push((
            module.to_owned(),
            field.to_owned(),
            EntityIndex::Function(func_index),
        ));
        self.result.module.local.num_imported_funcs += 1;
        if let Some(info) = &mut self.result.debuginfo {
            info.wasm_file.imported_func_count += 1;
        }
        Ok(())
    }

    fn declare_table_import(&mut self, table: Table, module: &str, field: &str) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.local.table_plans.len(),
            self.result.module.local.num_imported_tables,
            "Imported tables must be declared first"
        );
        let plan = TablePlan::for_table(table, &self.result.tunables);
        let table_index = self.result.module.local.table_plans.push(plan);
        self.result.module.imports.push((
            module.to_owned(),
            field.to_owned(),
            EntityIndex::Table(table_index),
        ));
        self.result.module.local.num_imported_tables += 1;
        Ok(())
    }

    fn declare_memory_import(
        &mut self,
        memory: Memory,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.local.memory_plans.len(),
            self.result.module.local.num_imported_memories,
            "Imported memories must be declared first"
        );
        if memory.shared {
            return Err(WasmError::Unsupported("shared memories".to_owned()));
        }
        let plan = MemoryPlan::for_memory(memory, &self.result.tunables);
        let memory_index = self.result.module.local.memory_plans.push(plan);
        self.result.module.imports.push((
            module.to_owned(),
            field.to_owned(),
            EntityIndex::Memory(memory_index),
        ));
        self.result.module.local.num_imported_memories += 1;
        Ok(())
    }

    fn declare_global_import(
        &mut self,
        global: Global,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.local.globals.len(),
            self.result.module.local.num_imported_globals,
            "Imported globals must be declared first"
        );
        let global_index = self.result.module.local.globals.push(global);
        self.result.module.imports.push((
            module.to_owned(),
            field.to_owned(),
            EntityIndex::Global(global_index),
        ));
        self.result.module.local.num_imported_globals += 1;
        Ok(())
    }

    fn reserve_func_types(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .functions
            .reserve_exact(usize::try_from(num).unwrap());
        self.result
            .function_body_inputs
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) -> WasmResult<()> {
        self.result.module.local.functions.push(sig_index);
        Ok(())
    }

    fn reserve_tables(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .table_plans
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_table(&mut self, table: Table) -> WasmResult<()> {
        let plan = TablePlan::for_table(table, &self.result.tunables);
        self.result.module.local.table_plans.push(plan);
        Ok(())
    }

    fn reserve_memories(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .memory_plans
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_memory(&mut self, memory: Memory) -> WasmResult<()> {
        if memory.shared {
            return Err(WasmError::Unsupported("shared memories".to_owned()));
        }
        let plan = MemoryPlan::for_memory(memory, &self.result.tunables);
        self.result.module.local.memory_plans.push(plan);
        Ok(())
    }

    fn reserve_globals(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .globals
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    fn declare_global(&mut self, global: Global) -> WasmResult<()> {
        self.result.module.local.globals.push(global);
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
        if let Some(info) = &mut self.result.debuginfo {
            info.wasm_file.code_section_offset = offset;
        }
    }

    fn define_function_body(
        &mut self,
        _module_translation: &ModuleTranslationState,
        body_bytes: &'data [u8],
        body_offset: usize,
    ) -> WasmResult<()> {
        self.result.function_body_inputs.push(FunctionBodyData {
            data: body_bytes,
            module_offset: body_offset,
        });
        if let Some(info) = &mut self.result.debuginfo {
            let func_index = self.code_index + self.result.module.local.num_imported_funcs as u32;
            let func_index = FuncIndex::from_u32(func_index);
            let sig_index = self.result.module.local.functions[func_index];
            let sig = &self.result.module.local.signatures[sig_index];
            let mut locals = Vec::new();
            let body = wasmparser::FunctionBody::new(body_offset, body_bytes);
            for pair in body.get_locals_reader()? {
                locals.push(pair?);
            }
            info.wasm_file.funcs.push(FunctionMetadata {
                locals: locals.into_boxed_slice(),
                params: sig.0.params.iter().cloned().map(|i| i.into()).collect(),
            });
        }
        self.code_index += 1;
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
        if let Some(info) = &mut self.result.debuginfo {
            info.name_section.module_name = Some(name);
        }
    }

    fn declare_func_name(&mut self, func_index: FuncIndex, name: &'data str) {
        self.result
            .module
            .func_names
            .insert(func_index, name.to_string());
        if let Some(info) = &mut self.result.debuginfo {
            info.name_section
                .func_names
                .insert(func_index.as_u32(), name);
        }
    }

    fn declare_local_name(&mut self, func_index: FuncIndex, local: u32, name: &'data str) {
        if let Some(info) = &mut self.result.debuginfo {
            info.name_section
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
