//! Helper functions to gather information for each of the non-function sections of a
//! WebAssembly module.
//!
//! The code of these helper functions is straightforward since they only read metadata
//! about linear memories, tables, globals, etc. and store them for later use.
//!
//! The special case of the initialize expressions for table elements offsets or global variables
//! is handled, according to the semantics of WebAssembly, to only specific expressions that are
//! interpreted on the fly.
use crate::environ::{ModuleEnvironment, WasmError, WasmResult};
use crate::state::ModuleTranslationState;
use crate::translation_utils::{
    tabletype_to_type, type_to_type, DataIndex, ElemIndex, EntityType, Event, EventIndex,
    FuncIndex, Global, GlobalIndex, GlobalInit, Memory, MemoryIndex, Table, TableElementType,
    TableIndex, TypeIndex,
};
use crate::wasm_unsupported;
use core::convert::TryFrom;
use core::convert::TryInto;
use cranelift_codegen::ir::immediates::V128Imm;
use cranelift_codegen::ir::{self, AbiParam, Signature};
use cranelift_entity::packed_option::ReservedValue;
use cranelift_entity::EntityRef;
use std::boxed::Box;
use std::vec::Vec;
use wasmparser::{
    self, Data, DataKind, DataSectionReader, Element, ElementItem, ElementItems, ElementKind,
    ElementSectionReader, EventSectionReader, EventType, Export, ExportSectionReader, ExternalKind,
    FunctionSectionReader, GlobalSectionReader, GlobalType, ImportSectionEntryType,
    ImportSectionReader, MemorySectionReader, MemoryType, NameSectionReader, Naming, Operator,
    TableSectionReader, TableType, TypeDef, TypeSectionReader,
};

fn entity_type(
    ty: ImportSectionEntryType,
    environ: &mut dyn ModuleEnvironment<'_>,
) -> WasmResult<EntityType> {
    Ok(match ty {
        ImportSectionEntryType::Function(sig) => EntityType::Function(TypeIndex::from_u32(sig)),
        ImportSectionEntryType::Module(sig) => EntityType::Module(TypeIndex::from_u32(sig)),
        ImportSectionEntryType::Instance(sig) => EntityType::Instance(TypeIndex::from_u32(sig)),
        ImportSectionEntryType::Memory(ty) => EntityType::Memory(memory(ty)),
        ImportSectionEntryType::Event(evt) => EntityType::Event(event(evt)),
        ImportSectionEntryType::Global(ty) => {
            EntityType::Global(global(ty, environ, GlobalInit::Import)?)
        }
        ImportSectionEntryType::Table(ty) => EntityType::Table(table(ty, environ)?),
    })
}

fn memory(ty: MemoryType) -> Memory {
    match ty {
        MemoryType::M32 { limits, shared } => Memory {
            minimum: limits.initial,
            maximum: limits.maximum,
            shared: shared,
        },
        // FIXME(#2361)
        MemoryType::M64 { .. } => unimplemented!(),
    }
}

fn event(e: EventType) -> Event {
    Event {
        ty: TypeIndex::from_u32(e.type_index),
    }
}

fn table(ty: TableType, environ: &mut dyn ModuleEnvironment<'_>) -> WasmResult<Table> {
    Ok(Table {
        wasm_ty: ty.element_type.try_into()?,
        ty: match tabletype_to_type(ty.element_type, environ)? {
            Some(t) => TableElementType::Val(t),
            None => TableElementType::Func,
        },
        minimum: ty.limits.initial,
        maximum: ty.limits.maximum,
    })
}

fn global(
    ty: GlobalType,
    environ: &mut dyn ModuleEnvironment<'_>,
    initializer: GlobalInit,
) -> WasmResult<Global> {
    Ok(Global {
        wasm_ty: ty.content_type.try_into()?,
        ty: type_to_type(ty.content_type, environ).unwrap(),
        mutability: ty.mutable,
        initializer,
    })
}

/// Parses the Type section of the wasm module.
pub fn parse_type_section<'a>(
    types: TypeSectionReader<'a>,
    module_translation_state: &mut ModuleTranslationState,
    environ: &mut dyn ModuleEnvironment<'a>,
) -> WasmResult<()> {
    let count = types.get_count();
    module_translation_state.wasm_types.reserve(count as usize);
    environ.reserve_types(count)?;

    for entry in types {
        match entry? {
            TypeDef::Func(wasm_func_ty) => {
                let mut sig =
                    Signature::new(ModuleEnvironment::target_config(environ).default_call_conv);
                sig.params.extend(wasm_func_ty.params.iter().map(|ty| {
                    let cret_arg: ir::Type = type_to_type(*ty, environ)
                        .expect("only numeric types are supported in function signatures");
                    AbiParam::new(cret_arg)
                }));
                sig.returns.extend(wasm_func_ty.returns.iter().map(|ty| {
                    let cret_arg: ir::Type = type_to_type(*ty, environ)
                        .expect("only numeric types are supported in function signatures");
                    AbiParam::new(cret_arg)
                }));
                environ.declare_type_func(wasm_func_ty.clone().try_into()?, sig)?;
                module_translation_state
                    .wasm_types
                    .push((wasm_func_ty.params, wasm_func_ty.returns));
            }
            TypeDef::Module(t) => {
                let imports = t
                    .imports
                    .iter()
                    .map(|i| Ok((i.module, i.field, entity_type(i.ty, environ)?)))
                    .collect::<WasmResult<Vec<_>>>()?;
                let exports = t
                    .exports
                    .iter()
                    .map(|e| Ok((e.name, entity_type(e.ty, environ)?)))
                    .collect::<WasmResult<Vec<_>>>()?;
                environ.declare_type_module(&imports, &exports)?;
            }
            TypeDef::Instance(t) => {
                let exports = t
                    .exports
                    .iter()
                    .map(|e| Ok((e.name, entity_type(e.ty, environ)?)))
                    .collect::<WasmResult<Vec<_>>>()?;
                environ.declare_type_instance(&exports)?;
            }
        }
    }
    Ok(())
}

/// Parses the Import section of the wasm module.
pub fn parse_import_section<'data>(
    imports: ImportSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    environ.reserve_imports(imports.get_count())?;

    for entry in imports {
        let import = entry?;
        match entity_type(import.ty, environ)? {
            EntityType::Function(idx) => {
                environ.declare_func_import(idx, import.module, import.field)?;
            }
            EntityType::Module(idx) => {
                environ.declare_module_import(idx, import.module, import.field)?;
            }
            EntityType::Instance(idx) => {
                environ.declare_instance_import(idx, import.module, import.field)?;
            }
            EntityType::Memory(ty) => {
                environ.declare_memory_import(ty, import.module, import.field)?;
            }
            EntityType::Event(e) => environ.declare_event_import(e, import.module, import.field)?,
            EntityType::Global(ty) => {
                environ.declare_global_import(ty, import.module, import.field)?;
            }
            EntityType::Table(ty) => {
                environ.declare_table_import(ty, import.module, import.field)?;
            }
        }
    }

    environ.finish_imports()?;
    Ok(())
}

/// Parses the Function section of the wasm module.
pub fn parse_function_section(
    functions: FunctionSectionReader,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    let num_functions = functions.get_count();
    if num_functions == std::u32::MAX {
        // We reserve `u32::MAX` for our own use in cranelift-entity.
        return Err(WasmError::ImplLimitExceeded);
    }

    environ.reserve_func_types(num_functions)?;

    for entry in functions {
        let sigindex = entry?;
        environ.declare_func_type(TypeIndex::from_u32(sigindex))?;
    }

    Ok(())
}

/// Parses the Table section of the wasm module.
pub fn parse_table_section(
    tables: TableSectionReader,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_tables(tables.get_count())?;

    for entry in tables {
        let ty = table(entry?, environ)?;
        environ.declare_table(ty)?;
    }

    Ok(())
}

/// Parses the Memory section of the wasm module.
pub fn parse_memory_section(
    memories: MemorySectionReader,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_memories(memories.get_count())?;

    for entry in memories {
        let memory = memory(entry?);
        environ.declare_memory(memory)?;
    }

    Ok(())
}

/// Parses the Event section of the wasm module.
pub fn parse_event_section(
    events: EventSectionReader,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_events(events.get_count())?;

    for entry in events {
        let event = event(entry?);
        environ.declare_event(event)?;
    }

    Ok(())
}

/// Parses the Global section of the wasm module.
pub fn parse_global_section(
    globals: GlobalSectionReader,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_globals(globals.get_count())?;

    for entry in globals {
        let wasmparser::Global { ty, init_expr } = entry?;
        let mut init_expr_reader = init_expr.get_binary_reader();
        let initializer = match init_expr_reader.read_operator()? {
            Operator::I32Const { value } => GlobalInit::I32Const(value),
            Operator::I64Const { value } => GlobalInit::I64Const(value),
            Operator::F32Const { value } => GlobalInit::F32Const(value.bits()),
            Operator::F64Const { value } => GlobalInit::F64Const(value.bits()),
            Operator::V128Const { value } => {
                GlobalInit::V128Const(V128Imm::from(value.bytes().to_vec().as_slice()))
            }
            Operator::RefNull { ty: _ } => GlobalInit::RefNullConst,
            Operator::RefFunc { function_index } => {
                GlobalInit::RefFunc(FuncIndex::from_u32(function_index))
            }
            Operator::GlobalGet { global_index } => {
                GlobalInit::GetGlobal(GlobalIndex::from_u32(global_index))
            }
            ref s => {
                return Err(wasm_unsupported!(
                    "unsupported init expr in global section: {:?}",
                    s
                ));
            }
        };
        let ty = global(ty, environ, initializer)?;
        environ.declare_global(ty)?;
    }

    Ok(())
}

/// Parses the Export section of the wasm module.
pub fn parse_export_section<'data>(
    exports: ExportSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    environ.reserve_exports(exports.get_count())?;

    for entry in exports {
        let Export {
            field,
            ref kind,
            index,
        } = entry?;

        // The input has already been validated, so we should be able to
        // assume valid UTF-8 and use `from_utf8_unchecked` if performance
        // becomes a concern here.
        let index = index as usize;
        match *kind {
            ExternalKind::Function => environ.declare_func_export(FuncIndex::new(index), field)?,
            ExternalKind::Table => environ.declare_table_export(TableIndex::new(index), field)?,
            ExternalKind::Memory => {
                environ.declare_memory_export(MemoryIndex::new(index), field)?
            }
            ExternalKind::Event => environ.declare_event_export(EventIndex::new(index), field)?,
            ExternalKind::Global => {
                environ.declare_global_export(GlobalIndex::new(index), field)?
            }
            ExternalKind::Type | ExternalKind::Module | ExternalKind::Instance => {
                unimplemented!("module linking not implemented yet")
            }
        }
    }

    environ.finish_exports()?;
    Ok(())
}

/// Parses the Start section of the wasm module.
pub fn parse_start_section(index: u32, environ: &mut dyn ModuleEnvironment) -> WasmResult<()> {
    environ.declare_start_func(FuncIndex::from_u32(index))?;
    Ok(())
}

fn read_elems(items: &ElementItems) -> WasmResult<Box<[FuncIndex]>> {
    let items_reader = items.get_items_reader()?;
    let mut elems = Vec::with_capacity(usize::try_from(items_reader.get_count()).unwrap());
    for item in items_reader {
        let elem = match item? {
            ElementItem::Null(_ty) => FuncIndex::reserved_value(),
            ElementItem::Func(index) => FuncIndex::from_u32(index),
        };
        elems.push(elem);
    }
    Ok(elems.into_boxed_slice())
}

/// Parses the Element section of the wasm module.
pub fn parse_element_section<'data>(
    elements: ElementSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_table_elements(elements.get_count())?;

    for (index, entry) in elements.into_iter().enumerate() {
        let Element { kind, items, ty: _ } = entry?;
        let segments = read_elems(&items)?;
        match kind {
            ElementKind::Active {
                table_index,
                init_expr,
            } => {
                let mut init_expr_reader = init_expr.get_binary_reader();
                let (base, offset) = match init_expr_reader.read_operator()? {
                    Operator::I32Const { value } => (None, value as u32 as usize),
                    Operator::GlobalGet { global_index } => {
                        (Some(GlobalIndex::from_u32(global_index)), 0)
                    }
                    ref s => {
                        return Err(wasm_unsupported!(
                            "unsupported init expr in element section: {:?}",
                            s
                        ));
                    }
                };
                environ.declare_table_elements(
                    TableIndex::from_u32(table_index),
                    base,
                    offset,
                    segments,
                )?
            }
            ElementKind::Passive => {
                let index = ElemIndex::from_u32(index as u32);
                environ.declare_passive_element(index, segments)?;
            }
            ElementKind::Declared => {
                // Nothing to do here.
            }
        }
    }
    Ok(())
}

/// Parses the Data section of the wasm module.
pub fn parse_data_section<'data>(
    data: DataSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    environ.reserve_data_initializers(data.get_count())?;

    for (index, entry) in data.into_iter().enumerate() {
        let Data { kind, data } = entry?;
        match kind {
            DataKind::Active {
                memory_index,
                init_expr,
            } => {
                let mut init_expr_reader = init_expr.get_binary_reader();
                let (base, offset) = match init_expr_reader.read_operator()? {
                    Operator::I32Const { value } => (None, value as u32 as usize),
                    Operator::GlobalGet { global_index } => {
                        (Some(GlobalIndex::from_u32(global_index)), 0)
                    }
                    ref s => {
                        return Err(wasm_unsupported!(
                            "unsupported init expr in data section: {:?}",
                            s
                        ))
                    }
                };
                environ.declare_data_initialization(
                    MemoryIndex::from_u32(memory_index),
                    base,
                    offset,
                    data,
                )?;
            }
            DataKind::Passive => {
                let index = DataIndex::from_u32(index as u32);
                environ.declare_passive_data(index, data)?;
            }
        }
    }

    Ok(())
}

/// Parses the Name section of the wasm module.
pub fn parse_name_section<'data>(
    names: NameSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    for subsection in names {
        match subsection? {
            wasmparser::Name::Function(f) => {
                let mut names = f.get_map()?;
                for _ in 0..names.get_count() {
                    let Naming { index, name } = names.read()?;
                    // We reserve `u32::MAX` for our own use in cranelift-entity.
                    if index != u32::max_value() {
                        environ.declare_func_name(FuncIndex::from_u32(index), name);
                    }
                }
            }
            wasmparser::Name::Module(module) => {
                let name = module.get_name()?;
                environ.declare_module_name(name);
            }
            wasmparser::Name::Local(l) => {
                let mut reader = l.get_function_local_reader()?;
                for _ in 0..reader.get_count() {
                    let f = reader.read()?;
                    if f.func_index == u32::max_value() {
                        continue;
                    }
                    let mut map = f.get_map()?;
                    for _ in 0..map.get_count() {
                        let Naming { index, name } = map.read()?;
                        environ.declare_local_name(FuncIndex::from_u32(f.func_index), index, name)
                    }
                }
            }
        }
    }
    Ok(())
}
