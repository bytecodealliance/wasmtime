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
    tabletype_to_type, type_to_type, DataIndex, ElemIndex, FuncIndex, Global, GlobalIndex,
    GlobalInit, Memory, MemoryIndex, SignatureIndex, Table, TableElementType, TableIndex,
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
    ElementSectionReader, Export, ExportSectionReader, ExternalKind, FunctionSectionReader,
    GlobalSectionReader, GlobalType, ImportSectionEntryType, ImportSectionReader,
    MemorySectionReader, MemoryType, NameSectionReader, Naming, Operator, TableSectionReader,
    TypeDef, TypeSectionReader,
};

/// Parses the Type section of the wasm module.
pub fn parse_type_section(
    types: TypeSectionReader,
    module_translation_state: &mut ModuleTranslationState,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    let count = types.get_count();
    module_translation_state.wasm_types.reserve(count as usize);
    environ.reserve_signatures(count)?;

    for entry in types {
        if let Ok(TypeDef::Func(wasm_func_ty)) = entry {
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
            environ.declare_signature(wasm_func_ty.clone().try_into()?, sig)?;
            module_translation_state
                .wasm_types
                .push((wasm_func_ty.params, wasm_func_ty.returns));
        } else {
            unimplemented!("module linking not implemented yet")
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
        let module_name = import.module;
        let field_name = import.field.unwrap(); // TODO Handle error when module linking is implemented.

        match import.ty {
            ImportSectionEntryType::Function(sig) => {
                environ.declare_func_import(
                    SignatureIndex::from_u32(sig),
                    module_name,
                    field_name,
                )?;
            }
            ImportSectionEntryType::Module(_sig) | ImportSectionEntryType::Instance(_sig) => {
                unimplemented!("module linking not implemented yet")
            }
            ImportSectionEntryType::Memory(MemoryType::M32 {
                limits: ref memlimits,
                shared,
            }) => {
                environ.declare_memory_import(
                    Memory {
                        minimum: memlimits.initial,
                        maximum: memlimits.maximum,
                        shared,
                    },
                    module_name,
                    field_name,
                )?;
            }
            ImportSectionEntryType::Memory(MemoryType::M64 { .. }) => {
                unimplemented!();
            }
            ImportSectionEntryType::Global(ref ty) => {
                environ.declare_global_import(
                    Global {
                        wasm_ty: ty.content_type.try_into()?,
                        ty: type_to_type(ty.content_type, environ).unwrap(),
                        mutability: ty.mutable,
                        initializer: GlobalInit::Import,
                    },
                    module_name,
                    field_name,
                )?;
            }
            ImportSectionEntryType::Table(ref tab) => {
                environ.declare_table_import(
                    Table {
                        wasm_ty: tab.element_type.try_into()?,
                        ty: match tabletype_to_type(tab.element_type, environ)? {
                            Some(t) => TableElementType::Val(t),
                            None => TableElementType::Func,
                        },
                        minimum: tab.limits.initial,
                        maximum: tab.limits.maximum,
                    },
                    module_name,
                    field_name,
                )?;
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
        environ.declare_func_type(SignatureIndex::from_u32(sigindex))?;
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
        let table = entry?;
        environ.declare_table(Table {
            wasm_ty: table.element_type.try_into()?,
            ty: match tabletype_to_type(table.element_type, environ)? {
                Some(t) => TableElementType::Val(t),
                None => TableElementType::Func,
            },
            minimum: table.limits.initial,
            maximum: table.limits.maximum,
        })?;
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
        let memory = entry?;
        match memory {
            MemoryType::M32 { limits, shared } => {
                environ.declare_memory(Memory {
                    minimum: limits.initial,
                    maximum: limits.maximum,
                    shared: shared,
                })?;
            }
            MemoryType::M64 { .. } => unimplemented!(),
        }
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
        let wasmparser::Global {
            ty: GlobalType {
                content_type,
                mutable,
            },
            init_expr,
        } = entry?;
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
        let global = Global {
            wasm_ty: content_type.try_into()?,
            ty: type_to_type(content_type, environ).unwrap(),
            mutability: mutable,
            initializer,
        };
        environ.declare_global(global)?;
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
