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
use crate::wasm_unsupported;
use crate::{
    DataIndex, ElemIndex, FuncIndex, GlobalIndex, MemoryIndex, TableIndex, Tag, TagIndex,
    TypeIndex, WasmError, WasmResult,
};
use cranelift_entity::packed_option::ReservedValue;
use cranelift_entity::{EntityRef, Unsigned};
use std::boxed::Box;
use std::vec::Vec;
use wasmparser::{
    Data, DataKind, DataSectionReader, Element, ElementItems, ElementKind, ElementSectionReader,
    Export, ExportSectionReader, ExternalKind, FunctionSectionReader, GlobalSectionReader,
    ImportSectionReader, MemorySectionReader, Operator, TableSectionReader, TagSectionReader,
    TagType, TypeRef, TypeSectionReader,
};
use wasmtime_types::ConstExpr;

fn tag(e: TagType) -> Tag {
    match e.kind {
        wasmparser::TagKind::Exception => Tag {
            ty: TypeIndex::from_u32(e.func_type_idx),
        },
    }
}

/// Parses the Type section of the wasm module.
pub fn parse_type_section<'a>(
    types: TypeSectionReader<'a>,
    environ: &mut dyn ModuleEnvironment<'a>,
) -> WasmResult<()> {
    let count = types.count();
    environ.reserve_types(count)?;

    for ty in types.into_iter_err_on_gc_types() {
        let ty = environ.convert_func_type(&ty?);
        environ.declare_type_func(ty)?;
    }
    Ok(())
}

/// Parses the Import section of the wasm module.
pub fn parse_import_section<'data>(
    imports: ImportSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    environ.reserve_imports(imports.count())?;

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
                environ.declare_memory_import(ty.into(), import.module, import.name)?;
            }
            TypeRef::Tag(e) => {
                environ.declare_tag_import(tag(e), import.module, import.name)?;
            }
            TypeRef::Global(ty) => {
                let ty = environ.convert_global_type(&ty);
                environ.declare_global_import(ty, import.module, import.name)?;
            }
            TypeRef::Table(ty) => {
                let ty = environ.convert_table_type(&ty)?;
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
    let num_functions = functions.count();
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
    environ.reserve_tables(tables.count())?;

    for entry in tables {
        let ty = environ.convert_table_type(&entry?.ty)?;
        environ.declare_table(ty)?;
    }

    Ok(())
}

/// Parses the Memory section of the wasm module.
pub fn parse_memory_section(
    memories: MemorySectionReader,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_memories(memories.count())?;

    for entry in memories {
        environ.declare_memory(entry?.into())?;
    }

    Ok(())
}

/// Parses the Tag section of the wasm module.
pub fn parse_tag_section(
    tags: TagSectionReader,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_tags(tags.count())?;

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
    environ.reserve_globals(globals.count())?;

    for entry in globals {
        let wasmparser::Global { ty, init_expr } = entry?;
        let (initializer, _escaped) = ConstExpr::from_wasmparser(init_expr)?;
        let ty = environ.convert_global_type(&ty);
        environ.declare_global(ty, initializer)?;
    }

    Ok(())
}

/// Parses the Export section of the wasm module.
pub fn parse_export_section<'data>(
    exports: ExportSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    environ.reserve_exports(exports.count())?;

    for entry in exports {
        let Export {
            name,
            ref kind,
            index,
        } = entry?;

        // The input has already been validated, so we should be able to
        // assume valid UTF-8 and use `from_utf8_unchecked` if performance
        // becomes a concern here.
        let index = usize::try_from(index)?;
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
    let mut elems = Vec::new();
    match items {
        ElementItems::Functions(funcs) => {
            for func in funcs.clone() {
                elems.push(FuncIndex::from_u32(func?));
            }
        }
        ElementItems::Expressions(_ty, funcs) => {
            for func in funcs.clone() {
                let idx = match func?.get_binary_reader().read_operator()? {
                    Operator::RefNull { .. } => FuncIndex::reserved_value(),
                    Operator::RefFunc { function_index } => FuncIndex::from_u32(function_index),
                    s => {
                        return Err(WasmError::Unsupported(format!(
                            "unsupported init expr in element section: {s:?}"
                        )));
                    }
                };
                elems.push(idx);
            }
        }
    }
    Ok(elems.into_boxed_slice())
}

/// Parses the Element section of the wasm module.
pub fn parse_element_section<'data>(
    elements: ElementSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_table_elements(elements.count())?;

    for (index, entry) in elements.into_iter().enumerate() {
        let Element {
            kind,
            items,
            range: _,
        } = entry?;
        let segments = read_elems(&items)?;
        match kind {
            ElementKind::Active {
                table_index,
                offset_expr,
            } => {
                let mut offset_expr_reader = offset_expr.get_binary_reader();
                let (base, offset) = match offset_expr_reader.read_operator()? {
                    Operator::I32Const { value } => (None, u64::from(value.unsigned())),
                    Operator::I64Const { value } => (None, value.unsigned()),
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
                    TableIndex::from_u32(table_index.unwrap_or(0)),
                    base,
                    offset,
                    segments,
                )?
            }
            ElementKind::Passive => {
                let index = ElemIndex::from_u32(u32::try_from(index)?);
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
    environ.reserve_data_initializers(data.count())?;

    for (index, entry) in data.into_iter().enumerate() {
        let Data {
            kind,
            data,
            range: _,
        } = entry?;
        match kind {
            DataKind::Active {
                memory_index,
                offset_expr,
            } => {
                let mut offset_expr_reader = offset_expr.get_binary_reader();
                let (base, offset) = match offset_expr_reader.read_operator()? {
                    Operator::I32Const { value } => (None, u64::try_from(value)?),
                    Operator::I64Const { value } => (None, u64::try_from(value)?),
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
                let index = DataIndex::from_u32(u32::try_from(index)?);
                environ.declare_passive_data(index, data)?;
            }
        }
    }

    Ok(())
}
