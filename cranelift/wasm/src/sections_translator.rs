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
    tabletype_to_type, type_to_type, FuncIndex, Global, GlobalIndex, GlobalInit, Memory,
    MemoryIndex, SignatureIndex, Table, TableElementType, TableIndex,
};
use crate::{wasm_unsupported, HashMap};
use core::convert::TryFrom;
use cranelift_codegen::ir::immediates::V128Imm;
use cranelift_codegen::ir::{self, AbiParam, Signature};
use cranelift_entity::packed_option::ReservedValue;
use cranelift_entity::EntityRef;
use std::vec::Vec;
use wasmparser::{
    self, CodeSectionReader, Data, DataKind, DataSectionReader, Element, ElementItem, ElementKind,
    ElementSectionReader, Export, ExportSectionReader, ExternalKind, FuncType,
    FunctionSectionReader, GlobalSectionReader, GlobalType, ImportSectionEntryType,
    ImportSectionReader, MemorySectionReader, MemoryType, NameSectionReader, Naming, NamingReader,
    Operator, TableSectionReader, Type, TypeSectionReader,
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
        match entry? {
            FuncType {
                form: wasmparser::Type::Func,
                params,
                returns,
            } => {
                let mut sig =
                    Signature::new(ModuleEnvironment::target_config(environ).default_call_conv);
                sig.params.extend(params.iter().map(|ty| {
                    let cret_arg: ir::Type = type_to_type(*ty, environ)
                        .expect("only numeric types are supported in function signatures");
                    AbiParam::new(cret_arg)
                }));
                sig.returns.extend(returns.iter().map(|ty| {
                    let cret_arg: ir::Type = type_to_type(*ty, environ)
                        .expect("only numeric types are supported in function signatures");
                    AbiParam::new(cret_arg)
                }));
                environ.declare_signature(sig)?;
                module_translation_state.wasm_types.push((params, returns));
            }
            ty => {
                return Err(wasm_unsupported!(
                    "unsupported type in type section: {:?}",
                    ty
                ))
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
        let module_name = import.module;
        let field_name = import.field;

        match import.ty {
            ImportSectionEntryType::Function(sig) => {
                environ.declare_func_import(
                    SignatureIndex::from_u32(sig),
                    module_name,
                    field_name,
                )?;
            }
            ImportSectionEntryType::Memory(MemoryType {
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
            ImportSectionEntryType::Global(ref ty) => {
                environ.declare_global_import(
                    Global {
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
        environ.declare_memory(Memory {
            minimum: memory.limits.initial,
            maximum: memory.limits.maximum,
            shared: memory.shared,
        })?;
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
            Operator::RefNull => GlobalInit::RefNullConst,
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

/// Parses the Element section of the wasm module.
pub fn parse_element_section<'data>(
    elements: ElementSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment,
) -> WasmResult<()> {
    environ.reserve_table_elements(elements.get_count())?;

    for entry in elements {
        let Element { kind, items, ty } = entry?;
        if ty != Type::AnyFunc {
            return Err(wasm_unsupported!(
                "unsupported table element type: {:?}",
                ty
            ));
        }
        if let ElementKind::Active {
            table_index,
            init_expr,
        } = kind
        {
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
            let items_reader = items.get_items_reader()?;
            let mut elems = Vec::with_capacity(usize::try_from(items_reader.get_count()).unwrap());
            for item in items_reader {
                let elem = match item? {
                    ElementItem::Null => FuncIndex::reserved_value(),
                    ElementItem::Func(index) => FuncIndex::from_u32(index),
                };
                elems.push(elem);
            }
            environ.declare_table_elements(
                TableIndex::from_u32(table_index),
                base,
                offset,
                elems.into_boxed_slice(),
            )?
        } else {
            return Err(wasm_unsupported!("unsupported passive elements section",));
        }
    }
    Ok(())
}

/// Parses the Code section of the wasm module.
pub fn parse_code_section<'data>(
    code: CodeSectionReader<'data>,
    module_translation_state: &ModuleTranslationState,
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    for body in code {
        let mut reader = body?.get_binary_reader();
        let size = reader.bytes_remaining();
        let offset = reader.original_position();
        environ.define_function_body(module_translation_state, reader.read_bytes(size)?, offset)?;
    }
    Ok(())
}

/// Parses the Data section of the wasm module.
pub fn parse_data_section<'data>(
    data: DataSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    environ.reserve_data_initializers(data.get_count())?;

    for entry in data {
        let Data { kind, data } = entry?;
        if let DataKind::Active {
            memory_index,
            init_expr,
        } = kind
        {
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
        } else {
            return Err(wasm_unsupported!(
                "unsupported passive data section: {:?}",
                kind
            ));
        }
    }

    Ok(())
}

/// Parses the Name section of the wasm module.
pub fn parse_name_section<'data>(
    mut names: NameSectionReader<'data>,
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    while let Ok(subsection) = names.read() {
        match subsection {
            wasmparser::Name::Function(function_subsection) => {
                if let Some(function_names) = function_subsection
                    .get_map()
                    .ok()
                    .and_then(parse_function_name_subsection)
                {
                    for (index, name) in function_names {
                        environ.declare_func_name(index, name)?;
                    }
                }
                return Ok(());
            }
            wasmparser::Name::Local(_) | wasmparser::Name::Module(_) => {}
        };
    }
    Ok(())
}

fn parse_function_name_subsection(
    mut naming_reader: NamingReader<'_>,
) -> Option<HashMap<FuncIndex, &str>> {
    let mut function_names = HashMap::new();
    for _ in 0..naming_reader.get_count() {
        let Naming { index, name } = naming_reader.read().ok()?;
        if index == std::u32::MAX {
            // We reserve `u32::MAX` for our own use in cranelift-entity.
            return None;
        }

        if function_names
            .insert(FuncIndex::from_u32(index), name)
            .is_some()
        {
            // If the function index has been previously seen, then we
            // break out of the loop and early return `None`, because these
            // should be unique.
            return None;
        }
    }
    Some(function_names)
}
