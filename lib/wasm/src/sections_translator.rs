//! Helper functions to gather information for each of the non-function sections of a
//! WebAssembly module.
//!
//! The code of theses helper function is straightforward since it is only about reading metadata
//! about linear memories, tables, globals, etc. and storing them for later use.
//!
//! The special case of the initialize expressions for table elements offsets or global variables
//! is handled, according to the semantics of WebAssembly, to only specific expressions that are
//! interpreted on the fly.
use translation_utils::{type_to_type, TableIndex, FunctionIndex, GlobalIndex, SignatureIndex,
                        MemoryIndex, Global, GlobalInit, Table, TableElementType, Memory};
use cretonne::ir::{Signature, ArgumentType, CallConv};
use cretonne;
use wasmparser::{Parser, ParserState, FuncType, ImportSectionEntryType, ExternalKind, WasmDecoder,
                 MemoryType, Operator};
use wasmparser;
use std::str::from_utf8;
use runtime::ModuleEnvironment;

pub enum SectionParsingError {
    WrongSectionContent(String),
}

/// Reads the Type Section of the wasm module and returns the corresponding function signatures.
pub fn parse_function_signatures(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> Result<(), SectionParsingError> {
    loop {
        match *parser.read() {
            ParserState::EndSection => break,
            ParserState::TypeSectionEntry(FuncType {
                                              form: wasmparser::Type::Func,
                                              ref params,
                                              ref returns,
                                          }) => {
                let mut sig = Signature::new(CallConv::Native);
                sig.argument_types.extend(params.iter().map(|ty| {
                    let cret_arg: cretonne::ir::Type = type_to_type(ty).expect(
                        "only numeric types are supported in function signatures",
                    );
                    ArgumentType::new(cret_arg)
                }));
                sig.return_types.extend(returns.iter().map(|ty| {
                    let cret_arg: cretonne::ir::Type = type_to_type(ty).expect(
                        "only numeric types are supported in function signatures",
                    );
                    ArgumentType::new(cret_arg)
                }));
                environ.declare_signature(&sig);
            }
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        }
    }
    Ok(())
}

/// Retrieves the imports from the imports section of the binary.
pub fn parse_import_section<'data>(
    parser: &mut Parser<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> Result<(), SectionParsingError> {
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
                ty: ImportSectionEntryType::Memory(MemoryType { limits: ref memlimits }), ..
            } => {
                environ.declare_memory(Memory {
                    pages_count: memlimits.initial as usize,
                    maximum: memlimits.maximum.map(|x| x as usize),
                });
            }
            ParserState::ImportSectionEntry {
                ty: ImportSectionEntryType::Global(ref ty), ..
            } => {
                environ.declare_global(Global {
                    ty: type_to_type(&ty.content_type).unwrap(),
                    mutability: ty.mutable,
                    initializer: GlobalInit::Import(),
                });
            }
            ParserState::ImportSectionEntry {
                ty: ImportSectionEntryType::Table(ref tab), ..
            } => {
                environ.declare_table(Table {
                    ty: match type_to_type(&tab.element_type) {
                        Ok(t) => TableElementType::Val(t),
                        Err(()) => TableElementType::Func(),
                    },
                    size: tab.limits.initial as usize,
                    maximum: tab.limits.maximum.map(|x| x as usize),
                })
            }
            ParserState::EndSection => break,
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
    }
    Ok(())
}

/// Retrieves the correspondances between functions and signatures from the function section
pub fn parse_function_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> Result<(), SectionParsingError> {
    loop {
        match *parser.read() {
            ParserState::FunctionSectionEntry(sigindex) => {
                environ.declare_func_type(sigindex as SignatureIndex);
            }
            ParserState::EndSection => break,
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
    }
    Ok(())
}

/// Retrieves the names of the functions from the export section
pub fn parse_export_section<'data>(
    parser: &mut Parser<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> Result<(), SectionParsingError> {
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
                let func_index = index as FunctionIndex;
                match *kind {
                    ExternalKind::Function => environ.declare_func_export(func_index, name),
                    ExternalKind::Table => environ.declare_table_export(func_index, name),
                    ExternalKind::Memory => environ.declare_memory_export(func_index, name),
                    ExternalKind::Global => environ.declare_global_export(func_index, name),
                }
            }
            ParserState::EndSection => break,
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
    }
    Ok(())
}

/// Retrieves the start function index from the start section
pub fn parse_start_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> Result<(), SectionParsingError> {
    loop {
        match *parser.read() {
            ParserState::StartSectionEntry(index) => {
                environ.declare_start_func(index as FunctionIndex);
            }
            ParserState::EndSection => break,
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
    }
    Ok(())
}

/// Retrieves the size and maximum fields of memories from the memory section
pub fn parse_memory_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> Result<(), SectionParsingError> {
    loop {
        match *parser.read() {
            ParserState::MemorySectionEntry(ref ty) => {
                environ.declare_memory(Memory {
                    pages_count: ty.limits.initial as usize,
                    maximum: ty.limits.maximum.map(|x| x as usize),
                });
            }
            ParserState::EndSection => break,
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
    }
    Ok(())
}

/// Retrieves the size and maximum fields of memories from the memory section
pub fn parse_global_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> Result<(), SectionParsingError> {
    loop {
        let (content_type, mutability) = match *parser.read() {
            ParserState::BeginGlobalSectionEntry(ref ty) => (ty.content_type, ty.mutable),
            ParserState::EndSection => break,
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        match *parser.read() {
            ParserState::BeginInitExpressionBody => (),
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
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
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        match *parser.read() {
            ParserState::EndInitExpressionBody => (),
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        }
        let global = Global {
            ty: type_to_type(&content_type).unwrap(),
            mutability: mutability,
            initializer: initializer,
        };
        environ.declare_global(global);
        match *parser.read() {
            ParserState::EndGlobalSectionEntry => (),
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        }
    }
    Ok(())
}

pub fn parse_data_section<'data>(
    parser: &mut Parser<'data>,
    environ: &mut ModuleEnvironment<'data>,
) -> Result<(), SectionParsingError> {
    loop {
        let memory_index = match *parser.read() {
            ParserState::BeginDataSectionEntry(memory_index) => memory_index,
            ParserState::EndSection => break,
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        match *parser.read() {
            ParserState::BeginInitExpressionBody => (),
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
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
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        match *parser.read() {
            ParserState::EndInitExpressionBody => (),
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        match *parser.read() {
            ParserState::BeginDataSectionEntryBody(_) => (),
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        let mut running_offset = offset;
        loop {
            let data = match *parser.read() {
                ParserState::DataSectionEntryBodyChunk(data) => data,
                ParserState::EndDataSectionEntryBody => break,
                ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
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
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
    }
    Ok(())
}

/// Retrieves the tables from the table section
pub fn parse_table_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> Result<(), SectionParsingError> {
    loop {
        match *parser.read() {
            ParserState::TableSectionEntry(ref table) => {
                environ.declare_table(Table {
                    ty: match type_to_type(&table.element_type) {
                        Ok(t) => TableElementType::Val(t),
                        Err(()) => TableElementType::Func(),
                    },
                    size: table.limits.initial as usize,
                    maximum: table.limits.maximum.map(|x| x as usize),
                })
            }
            ParserState::EndSection => break,
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
    }
    Ok(())
}

/// Retrieves the tables from the table section
pub fn parse_elements_section(
    parser: &mut Parser,
    environ: &mut ModuleEnvironment,
) -> Result<(), SectionParsingError> {
    loop {
        let table_index = match *parser.read() {
            ParserState::BeginElementSectionEntry(table_index) => table_index as TableIndex,
            ParserState::EndSection => break,
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        match *parser.read() {
            ParserState::BeginInitExpressionBody => (),
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
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
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        match *parser.read() {
            ParserState::EndInitExpressionBody => (),
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        match *parser.read() {
            ParserState::ElementSectionEntryBody(ref elements) => {
                let elems: Vec<FunctionIndex> =
                    elements.iter().map(|&x| x as FunctionIndex).collect();
                environ.declare_table_elements(table_index, base, offset, elems)
            }
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
        match *parser.read() {
            ParserState::EndElementSectionEntry => (),
            ref s => return Err(SectionParsingError::WrongSectionContent(format!("{:?}", s))),
        };
    }
    Ok(())
}
