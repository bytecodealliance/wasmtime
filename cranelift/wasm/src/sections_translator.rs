//! Helper functions to gather information for each of the non-function sections of a
//! WebAssembly module.
//!
//! The code of these helper functions is straightforward since they only read metadata
//! about linear memories, tables, globals, etc. and store them for later use.
//!
//! The special case of the initialize expressions for table elements offsets or global variables
//! is handled, according to the semantics of WebAssembly, to only specific expressions that are
//! interpreted on the fly.
use crate::environ::ModuleEnvironment;
use crate::state::ModuleTranslationState;
use crate::wasm_unsupported;
use crate::{
    DataIndex, ElemIndex, FuncIndex, Global, GlobalIndex, GlobalInit, Memory, MemoryIndex, Table,
    TableIndex, Tag, TagIndex, TypeIndex, WasmError, WasmResult,
};
use core::convert::TryFrom;
use core::convert::TryInto;
use cranelift_entity::packed_option::ReservedValue;
use cranelift_entity::EntityRef;
use std::boxed::Box;
use std::vec::Vec;
use wasmparser::{
    self, Data, DataKind, DataSectionReader, Element, ElementItem, ElementItems, ElementKind,
    ElementSectionReader, Export, ExportSectionReader, ExternalKind, FunctionSectionReader,
    GlobalSectionReader, GlobalType, ImportSectionReader, MemorySectionReader, MemoryType,
    NameSectionReader, Naming, Operator, TableSectionReader, TableType, TagSectionReader, TagType,
    TypeDef, TypeRef, TypeSectionReader,
};

fn memory(ty: MemoryType) -> Memory {
    Memory {
        minimum: ty.initial,
        maximum: ty.maximum,
        shared: ty.shared,
        memory64: ty.memory64,
    }
}

fn tag(e: TagType) -> Tag {
    match e.kind {
        wasmparser::TagKind::Exception => Tag {
            ty: TypeIndex::from_u32(e.func_type_idx),
        },
    }
}

fn table(ty: TableType) -> WasmResult<Table> {
    Ok(Table {
        wasm_ty: ty.element_type.try_into()?,
        minimum: ty.initial,
        maximum: ty.maximum,
    })
}

fn global(ty: GlobalType, initializer: GlobalInit) -> WasmResult<Global> {
    Ok(Global {
        wasm_ty: ty.content_type.try_into()?,
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
                environ.declare_type_func(wasm_func_ty.clone().try_into()?)?;
                module_translation_state
                    .wasm_types
                    .push((wasm_func_ty.params, wasm_func_ty.returns));
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
        match import.ty {
            TypeRef::Func(sig) => {
                environ.declare_func_import(
                    TypeIndex::from_u32(sig),
                    import.module,
                    import.name,
                )?;
            }
            TypeRef::Memory(ty) => {
                environ.declare_memory_import(memory(ty), import.module, import.name)?;
            }
            TypeRef::Tag(e) => {
                environ.declare_tag_import(tag(e), import.module, import.name)?;
            }
            TypeRef::Global(ty) => {
                let ty = global(ty, GlobalInit::Import)?;
                environ.declare_global_import(ty, import.module, import.name)?;
            }
            TypeRef::Table(ty) => {
                let ty = table(ty)?;
                environ.declare_table_import(ty, import.module, import.name)?;
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
        let ty = table(entry?)?;
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

/// Parses the Tag section of the wasm module.
pub fn parse_tag_section(
    tags: TagSectionReader,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_tags(tags.get_count())?;

    for entry in tags {
        let tag = tag(entry?);
        environ.declare_tag(tag)?;
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
                GlobalInit::V128Const(u128::from_le_bytes(*value.bytes()))
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
        let ty = global(ty, initializer)?;
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
            name,
            ref kind,
            index,
        } = entry?;

        // The input has already been validated, so we should be able to
        // assume valid UTF-8 and use `from_utf8_unchecked` if performance
        // becomes a concern here.
        let index = index as usize;
        match *kind {
            ExternalKind::Func => environ.declare_func_export(FuncIndex::new(index), name)?,
            ExternalKind::Table => environ.declare_table_export(TableIndex::new(index), name)?,
            ExternalKind::Memory => environ.declare_memory_export(MemoryIndex::new(index), name)?,
            ExternalKind::Tag => environ.declare_tag_export(TagIndex::new(index), name)?,
            ExternalKind::Global => environ.declare_global_export(GlobalIndex::new(index), name)?,
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
            ElementItem::Expr(init) => match init.get_binary_reader().read_operator()? {
                Operator::RefNull { .. } => FuncIndex::reserved_value(),
                Operator::RefFunc { function_index } => FuncIndex::from_u32(function_index),
                s => {
                    return Err(WasmError::Unsupported(format!(
                        "unsupported init expr in element section: {:?}",
                        s
                    )));
                }
            },
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
        let Element {
            kind,
            items,
            ty: _,
            range: _,
        } = entry?;
        let segments = read_elems(&items)?;
        match kind {
            ElementKind::Active {
                table_index,
                init_expr,
            } => {
                let mut init_expr_reader = init_expr.get_binary_reader();
                let (base, offset) = match init_expr_reader.read_operator()? {
                    Operator::I32Const { value } => (None, value as u32),
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
                environ.declare_elements(segments)?;
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
        let Data {
            kind,
            data,
            range: _,
        } = entry?;
        match kind {
            DataKind::Active {
                memory_index,
                init_expr,
            } => {
                let mut init_expr_reader = init_expr.get_binary_reader();
                let (base, offset) = match init_expr_reader.read_operator()? {
                    Operator::I32Const { value } => (None, value as u64),
                    Operator::I64Const { value } => (None, value as u64),
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
                let mut reader = l.get_indirect_map()?;
                for _ in 0..reader.get_indirect_count() {
                    let f = reader.read()?;
                    if f.indirect_index == u32::max_value() {
                        continue;
                    }
                    let mut map = f.get_map()?;
                    for _ in 0..map.get_count() {
                        let Naming { index, name } = map.read()?;
                        environ.declare_local_name(
                            FuncIndex::from_u32(f.indirect_index),
                            index,
                            name,
                        )
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
