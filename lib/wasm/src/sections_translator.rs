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
use environ::{ModuleEnvironment, WasmError, WasmResult};
use std::str::from_utf8;
use std::vec::Vec;
use translation_utils::{
    type_to_type, FuncIndex, Global, GlobalIndex, GlobalInit, Memory, MemoryIndex, SignatureIndex,
    Table, TableElementType, TableIndex,
};
use wasmparser;
use wasmparser::{
    ExternalKind, FuncType, ImportSectionEntryType, MemoryType, Operator, Parser, ParserState,
    WasmDecoder,
};

/// Reads the Type Section of the wasm module and returns the corresponding function signatures.
pub fn parse_function_signatures(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    loop {
        match *parser.read() {
            ParserState::EndSection => break,
            ParserState::TypeSectionEntry(FuncType {
                form: wasmparser::Type::Func,
                ref params,
                ref returns,
            }) => {
                let mut sig = Signature::new(environ.flags().call_conv());
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
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        }
    }
    Ok(())
}

/// Retrieves the imports from the imports section of the binary.
pub fn parse_import_section<'data>(
    parser: &mut Parser<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> WasmResult<()> {
    loop {
        match *parser.read() {
            ParserState::ImportSectionEntry {
                ty: ImportSectionEntryType::Function(sig),
                module,
                field,
            } => {
                // The input has already been validated, so we should be able to
                // assume valid UTF-8 and use `from_utf8_unchecked` if performance
                // becomes a concern here.
                let module_name = from_utf8(module).unwrap();
                let field_name = from_utf8(field).unwrap();
                environ.declare_func_import(sig as SignatureIndex, module_name, field_name);
            }
            ParserState::ImportSectionEntry {
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
            ParserState::ImportSectionEntry {
                ty: ImportSectionEntryType::Global(ref ty),
                ..
            } => {
                environ.declare_global(Global {
                    ty: type_to_type(ty.content_type).unwrap(),
                    mutability: ty.mutable,
                    initializer: GlobalInit::Import(),
                });
            }
            ParserState::ImportSectionEntry {
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
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
    }
    Ok(())
}

/// Retrieves the correspondences between functions and signatures from the function section
pub fn parse_function_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    loop {
        match *parser.read() {
            ParserState::FunctionSectionEntry(sigindex) => {
                environ.declare_func_type(sigindex as SignatureIndex);
            }
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
    }
    Ok(())
}

/// Retrieves the names of the functions from the export section
pub fn parse_export_section<'data>(
    parser: &mut Parser<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> WasmResult<()> {
    loop {
        match *parser.read() {
            ParserState::ExportSectionEntry {
                field,
                ref kind,
                index,
            } => {
                // The input has already been validated, so we should be able to
                // assume valid UTF-8 and use `from_utf8_unchecked` if performance
                // becomes a concern here.
                let name = from_utf8(field).unwrap();
                let func_index = FuncIndex::new(index as usize);
                match *kind {
                    ExternalKind::Function => environ.declare_func_export(func_index, name),
                    ExternalKind::Table => environ.declare_table_export(func_index.index(), name),
                    ExternalKind::Memory => environ.declare_memory_export(func_index.index(), name),
                    ExternalKind::Global => environ.declare_global_export(func_index.index(), name),
                }
            }
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
    }
    Ok(())
}

/// Retrieves the start function index from the start section
pub fn parse_start_section(parser: &mut Parser, environ: &mut ModuleEnvironment) -> WasmResult<()> {
    loop {
        match *parser.read() {
            ParserState::StartSectionEntry(index) => {
                environ.declare_start_func(FuncIndex::new(index as usize));
            }
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
    }
    Ok(())
}

/// Retrieves the size and maximum fields of memories from the memory section
pub fn parse_memory_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    loop {
        match *parser.read() {
            ParserState::MemorySectionEntry(ref ty) => {
                environ.declare_memory(Memory {
                    pages_count: ty.limits.initial as usize,
                    maximum: ty.limits.maximum.map(|x| x as usize),
                    shared: ty.shared,
                });
            }
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
    }
    Ok(())
}

/// Retrieves the size and maximum fields of memories from the memory section
pub fn parse_global_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    loop {
        let (content_type, mutability) = match *parser.read() {
            ParserState::BeginGlobalSectionEntry(ref ty) => (ty.content_type, ty.mutable),
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        match *parser.read() {
            ParserState::BeginInitExpressionBody => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        }
        let initializer = match *parser.read() {
            ParserState::InitExpressionOperator(Operator::I32Const { value }) => {
                GlobalInit::I32Const(value)
            }
            ParserState::InitExpressionOperator(Operator::I64Const { value }) => {
                GlobalInit::I64Const(value)
            }
            ParserState::InitExpressionOperator(Operator::F32Const { value }) => {
                GlobalInit::F32Const(value.bits())
            }
            ParserState::InitExpressionOperator(Operator::F64Const { value }) => {
                GlobalInit::F64Const(value.bits())
            }
            ParserState::InitExpressionOperator(Operator::GetGlobal { global_index }) => {
                GlobalInit::GlobalRef(global_index as GlobalIndex)
            }
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        match *parser.read() {
            ParserState::EndInitExpressionBody => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        }
        let global = Global {
            ty: type_to_type(content_type).unwrap(),
            mutability,
            initializer,
        };
        environ.declare_global(global);
        match *parser.read() {
            ParserState::EndGlobalSectionEntry => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        }
    }
    Ok(())
}

pub fn parse_data_section<'data>(
    parser: &mut Parser<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> WasmResult<()> {
    loop {
        let memory_index = match *parser.read() {
            ParserState::BeginDataSectionEntry(memory_index) => memory_index,
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        match *parser.read() {
            ParserState::BeginInitExpressionBody => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        let (base, offset) = match *parser.read() {
            ParserState::InitExpressionOperator(Operator::I32Const { value }) => {
                (None, value as u32 as usize)
            }
            ParserState::InitExpressionOperator(Operator::GetGlobal { global_index }) => {
                match environ.get_global(global_index as GlobalIndex).initializer {
                    GlobalInit::I32Const(value) => (None, value as u32 as usize),
                    GlobalInit::Import() => (Some(global_index as GlobalIndex), 0),
                    _ => panic!("should not happen"),
                }
            }
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        match *parser.read() {
            ParserState::EndInitExpressionBody => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        match *parser.read() {
            ParserState::BeginDataSectionEntryBody(_) => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        let mut running_offset = offset;
        loop {
            let data = match *parser.read() {
                ParserState::DataSectionEntryBodyChunk(data) => data,
                ParserState::EndDataSectionEntryBody => break,
                ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
                ref s => panic!("unexpected section content: {:?}", s),
            };
            environ.declare_data_initialization(
                memory_index as MemoryIndex,
                base,
                running_offset,
                data,
            );
            running_offset += data.len();
        }
        match *parser.read() {
            ParserState::EndDataSectionEntry => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
    }
    Ok(())
}

/// Retrieves the tables from the table section
pub fn parse_table_section(parser: &mut Parser, environ: &mut ModuleEnvironment) -> WasmResult<()> {
    loop {
        match *parser.read() {
            ParserState::TableSectionEntry(ref table) => environ.declare_table(Table {
                ty: match type_to_type(table.element_type) {
                    Ok(t) => TableElementType::Val(t),
                    Err(()) => TableElementType::Func(),
                },
                size: table.limits.initial as usize,
                maximum: table.limits.maximum.map(|x| x as usize),
            }),
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
    }
    Ok(())
}

/// Retrieves the tables from the table section
pub fn parse_elements_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> WasmResult<()> {
    loop {
        let table_index = match *parser.read() {
            ParserState::BeginElementSectionEntry(table_index) => table_index as TableIndex,
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        match *parser.read() {
            ParserState::BeginInitExpressionBody => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        let (base, offset) = match *parser.read() {
            ParserState::InitExpressionOperator(Operator::I32Const { value }) => {
                (None, value as u32 as usize)
            }
            ParserState::InitExpressionOperator(Operator::GetGlobal { global_index }) => {
                match environ.get_global(global_index as GlobalIndex).initializer {
                    GlobalInit::I32Const(value) => (None, value as u32 as usize),
                    GlobalInit::Import() => (Some(global_index as GlobalIndex), 0),
                    _ => panic!("should not happen"),
                }
            }
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        match *parser.read() {
            ParserState::EndInitExpressionBody => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        match *parser.read() {
            ParserState::ElementSectionEntryBody(ref elements) => {
                let elems: Vec<FuncIndex> = elements
                    .iter()
                    .map(|&x| FuncIndex::new(x as usize))
                    .collect();
                environ.declare_table_elements(table_index, base, offset, elems)
            }
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
        match *parser.read() {
            ParserState::EndElementSectionEntry => (),
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("unexpected section content: {:?}", s),
        };
    }
    Ok(())
}

/// Parses every function body in the code section and defines the corresponding function.
pub fn parse_code_section<'data>(
    parser: &mut Parser<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> WasmResult<()> {
    loop {
        match *parser.read() {
            ParserState::BeginFunctionBody { .. } => {}
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("wrong content in code section: {:?}", s),
        }
        let mut reader = parser.create_binary_reader();
        let size = reader.bytes_remaining();
        environ.define_function_body(
            reader
                .read_bytes(size)
                .map_err(WasmError::from_binary_reader_error)?,
        )?;
    }
    Ok(())
}
