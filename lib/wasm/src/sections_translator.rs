//! Helper functions to gather information for each of the non-function sections of a
//! WebAssembly module.
//!
//! The code of theses helper function is straightforward since it is only about reading metadata
//! about linear memories, tables, globals, etc. and storing them for later use.
//!
//! The special case of the initialize expressions for table elements offsets or global variables
//! is handled, according to the semantics of WebAssembly, to only specific expressions that are
//! interpreted on the fly.
use cranelift_codegen::ir::{self, AbiParam, Signature};
use cranelift_entity::EntityRef;
use environ::{ModuleEnvironment, WasmResult};
use std::str::from_utf8;
use std::vec::Vec;
use translation_utils::{
    type_to_type, FuncIndex, Global, GlobalIndex, GlobalInit, Memory, MemoryIndex, SignatureIndex,
    Table, TableElementType, TableIndex,
};
use wasmparser::{
    self, CodeSectionReader, Data, DataSectionReader, Element, ElementSectionReader, Export,
    ExportSectionReader, ExternalKind, FuncType, FunctionSectionReader, GlobalSectionReader,
    GlobalType, Import, ImportSectionEntryType, ImportSectionReader, MemorySectionReader,
    MemoryType, Operator, TableSectionReader, TypeSectionReader,
};

/// Parses the Type section of the wasm module.
pub fn parse_type_section(
    types: TypeSectionReader,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    for entry in types {
        match entry? {
            FuncType {
                form: wasmparser::Type::Func,
                ref params,
                ref returns,
            } => {
                let mut sig = Signature::new(environ.target_config().default_call_conv);
                sig.params.extend(params.iter().map(|ty| {
                    let cret_arg: ir::Type = type_to_type(*ty)
                        .expect("only numeric types are supported in function signatures");
                    AbiParam::new(cret_arg)
                }));
                sig.returns.extend(returns.iter().map(|ty| {
                    let cret_arg: ir::Type = type_to_type(*ty)
                        .expect("only numeric types are supported in function signatures");
                    AbiParam::new(cret_arg)
                }));
                environ.declare_signature(&sig);
            }
            ref s => panic!("unsupported type: {:?}", s),
        }
    }
    Ok(())
}

/// Parses the Import section of the wasm module.
pub fn parse_import_section<'data>(
    imports: ImportSectionReader<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> WasmResult<()> {
    for entry in imports {
        match entry? {
            Import {
                module,
                field,
                ty: ImportSectionEntryType::Function(sig),
            } => {
                // The input has already been validated, so we should be able to
                // assume valid UTF-8 and use `from_utf8_unchecked` if performance
                // becomes a concern here.
                let module_name = from_utf8(module).unwrap();
                let field_name = from_utf8(field).unwrap();
                environ.declare_func_import(
                    SignatureIndex::new(sig as usize),
                    module_name,
                    field_name,
                );
            }
            Import {
                ty:
                    ImportSectionEntryType::Memory(MemoryType {
                        limits: ref memlimits,
                        shared,
                    }),
                ..
            } => {
                environ.declare_memory(Memory {
                    pages_count: memlimits.initial as usize,
                    maximum: memlimits.maximum.map(|x| x as usize),
                    shared,
                });
            }
            Import {
                ty: ImportSectionEntryType::Global(ref ty),
                ..
            } => {
                environ.declare_global(Global {
                    ty: type_to_type(ty.content_type).unwrap(),
                    mutability: ty.mutable,
                    initializer: GlobalInit::Import(),
                });
            }
            Import {
                ty: ImportSectionEntryType::Table(ref tab),
                ..
            } => environ.declare_table(Table {
                ty: match type_to_type(tab.element_type) {
                    Ok(t) => TableElementType::Val(t),
                    Err(()) => TableElementType::Func(),
                },
                size: tab.limits.initial as usize,
                maximum: tab.limits.maximum.map(|x| x as usize),
            }),
        }
    }
    Ok(())
}

/// Parses the Function section of the wasm module.
pub fn parse_function_section(
    functions: FunctionSectionReader,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    for entry in functions {
        let sigindex = entry?;
        environ.declare_func_type(SignatureIndex::new(sigindex as usize));
    }
    Ok(())
}

/// Parses the Table section of the wasm module.
pub fn parse_table_section(
    tables: TableSectionReader,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    for entry in tables {
        let table = entry?;
        environ.declare_table(Table {
            ty: match type_to_type(table.element_type) {
                Ok(t) => TableElementType::Val(t),
                Err(()) => TableElementType::Func(),
            },
            size: table.limits.initial as usize,
            maximum: table.limits.maximum.map(|x| x as usize),
        });
    }
    Ok(())
}

/// Parses the Memory section of the wasm module.
pub fn parse_memory_section(
    memories: MemorySectionReader,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    for entry in memories {
        let memory = entry?;
        environ.declare_memory(Memory {
            pages_count: memory.limits.initial as usize,
            maximum: memory.limits.maximum.map(|x| x as usize),
            shared: memory.shared,
        });
    }
    Ok(())
}

/// Parses the Global section of the wasm module.
pub fn parse_global_section(
    globals: GlobalSectionReader,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
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
            Operator::GetGlobal { global_index } => {
                GlobalInit::GlobalRef(GlobalIndex::new(global_index as usize))
            }
            ref s => panic!("unsupported init expr in global section: {:?}", s),
        };
        let global = Global {
            ty: type_to_type(content_type).unwrap(),
            mutability: mutable,
            initializer,
        };
        environ.declare_global(global);
    }
    Ok(())
}

/// Parses the Export section of the wasm module.
pub fn parse_export_section<'data>(
    exports: ExportSectionReader<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> WasmResult<()> {
    for entry in exports {
        let Export {
            field,
            ref kind,
            index,
        } = entry?;

        // The input has already been validated, so we should be able to
        // assume valid UTF-8 and use `from_utf8_unchecked` if performance
        // becomes a concern here.
        let name = from_utf8(field).unwrap();
        let index = index as usize;
        match *kind {
            ExternalKind::Function => environ.declare_func_export(FuncIndex::new(index), name),
            ExternalKind::Table => environ.declare_table_export(TableIndex::new(index), name),
            ExternalKind::Memory => environ.declare_memory_export(MemoryIndex::new(index), name),
            ExternalKind::Global => environ.declare_global_export(GlobalIndex::new(index), name),
        }
    }
    Ok(())
}

/// Parses the Start section of the wasm module.
pub fn parse_start_section(index: u32, environ: &mut ModuleEnvironment) -> WasmResult<()> {
    environ.declare_start_func(FuncIndex::new(index as usize));
    Ok(())
}

/// Parses the Element section of the wasm module.
pub fn parse_element_section<'data>(
    elements: ElementSectionReader<'data>,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    for entry in elements {
        let Element {
            table_index,
            init_expr,
            items,
        } = entry?;
        let mut init_expr_reader = init_expr.get_binary_reader();
        let (base, offset) = match init_expr_reader.read_operator()? {
            Operator::I32Const { value } => (None, value as u32 as usize),
            Operator::GetGlobal { global_index } => match environ
                .get_global(GlobalIndex::new(global_index as usize))
                .initializer
            {
                GlobalInit::I32Const(value) => (None, value as u32 as usize),
                GlobalInit::Import() => (Some(GlobalIndex::new(global_index as usize)), 0),
                _ => panic!("should not happen"),
            },
            ref s => panic!("unsupported init expr in element section: {:?}", s),
        };
        let items_reader = items.get_items_reader()?;
        let mut elems = Vec::new();
        for item in items_reader {
            let x = item?;
            elems.push(FuncIndex::new(x as usize));
        }
        environ.declare_table_elements(TableIndex::new(table_index as usize), base, offset, elems)
    }
    Ok(())
}

/// Parses the Code section of the wasm module.
pub fn parse_code_section<'data>(
    code: CodeSectionReader<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> WasmResult<()> {
    for body in code {
        let mut reader = body?.get_binary_reader();
        let size = reader.bytes_remaining();
        environ.define_function_body(reader.read_bytes(size)?)?;
    }
    Ok(())
}

/// Parses the Data section of the wasm module.
pub fn parse_data_section<'data>(
    data: DataSectionReader<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> WasmResult<()> {
    for entry in data {
        let Data {
            memory_index,
            init_expr,
            data,
        } = entry?;
        let mut init_expr_reader = init_expr.get_binary_reader();
        let (base, offset) = match init_expr_reader.read_operator()? {
            Operator::I32Const { value } => (None, value as u32 as usize),
            Operator::GetGlobal { global_index } => match environ
                .get_global(GlobalIndex::new(global_index as usize))
                .initializer
            {
                GlobalInit::I32Const(value) => (None, value as u32 as usize),
                GlobalInit::Import() => (Some(GlobalIndex::new(global_index as usize)), 0),
                _ => panic!("should not happen"),
            },
            ref s => panic!("unsupported init expr in data section: {:?}", s),
        };
        environ.declare_data_initialization(
            MemoryIndex::new(memory_index as usize),
            base,
            offset,
            data,
        );
    }
    Ok(())
}
