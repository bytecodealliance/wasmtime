//! Translation skeletton that traverses the whole WebAssembly module and call helper functions
//! to deal with each part of it.
use wasmparser::{ParserState, SectionCode, ParserInput, Parser, WasmDecoder, BinaryReaderError};
use sections_translator::{SectionParsingError, parse_function_signatures, parse_import_section,
                          parse_function_section, parse_export_section, parse_memory_section,
                          parse_global_section, parse_table_section, parse_elements_section,
                          parse_data_section};
use translation_utils::{type_to_type, Import, SignatureIndex, FunctionIndex, invert_hashmaps};
use cretonne::ir::{Function, Type, FuncRef, SigRef};
use code_translator::translate_function_body;
use cton_frontend::ILBuilder;
use std::collections::HashMap;
use runtime::WasmRuntime;

/// Output of the [`translate_module`](fn.translate_module.html) function.
pub struct TranslationResult {
    /// The translated functions.
    pub functions: Vec<FunctionTranslation>,
    /// When present, the index of the function defined as `start` of the module.
    pub start_index: Option<FunctionIndex>,
}

/// A function in a WebAssembly module can be either imported, or defined inside it.
#[derive(Clone)]
pub enum FunctionTranslation {
    /// A function defined inside the WebAssembly module.
    Code {
        /// The translation in Cretonne IL.
        il: Function,
        /// The mappings between Cretonne imports and indexes in the function index space.
        imports: ImportMappings,
    },
    /// An imported function.
    Import(),
}

#[derive(Clone,Debug)]
/// Mappings describing the relations between imports of the Cretonne IL functions and the
/// functions in the WebAssembly module.
pub struct ImportMappings {
    /// Find the index of a function in the WebAssembly module thanks to a `FuncRef`.
    pub functions: HashMap<FuncRef, FunctionIndex>,
    /// Find the index of a signature in the WebAssembly module thanks to a `SigRef`.
    pub signatures: HashMap<SigRef, SignatureIndex>,
}

impl ImportMappings {
    /// Create a new empty `ImportMappings`.
    pub fn new() -> ImportMappings {
        ImportMappings {
            functions: HashMap::new(),
            signatures: HashMap::new(),
        }
    }
}

/// Translate a sequence of bytes forming a valid Wasm binary into a list of valid Cretonne IL
/// [`Function`](../cretonne/ir/function/struct.Function.html).
/// Returns the functions and also the mappings for imported functions and signature between the
/// indexes in the wasm module and the indexes inside each functions.
pub fn translate_module(data: &Vec<u8>,
                        runtime: &mut WasmRuntime)
                        -> Result<TranslationResult, String> {
    let mut parser = Parser::new(data.as_slice());
    match *parser.read() {
        ParserState::BeginWasm { .. } => {}
        ParserState::Error(BinaryReaderError { message, offset }) => {
            return Err(format!("at offset {}: {}", offset, message));
        }
        ref s @ _ => panic!("modules should begin properly: {:?}", s),
    }
    let mut signatures = None;
    let mut functions: Option<Vec<SignatureIndex>> = None;
    let mut globals = Vec::new();
    let mut exports: Option<HashMap<FunctionIndex, String>> = None;
    let mut next_input = ParserInput::Default;
    let mut function_index: FunctionIndex = 0;
    let mut function_imports_count = 0;
    let mut start_index: Option<FunctionIndex> = None;
    loop {
        match *parser.read_with_input(next_input) {
            ParserState::BeginSection { code: SectionCode::Type, .. } => {
                match parse_function_signatures(&mut parser) {
                    Ok(sigs) => signatures = Some(sigs),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the type section: {}", s))
                    }
                };
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Import, .. } => {
                match parse_import_section(&mut parser) {
                    Ok(imps) => {
                        for import in imps {
                            match import {
                                Import::Function { sig_index } => {
                                    functions = match functions {
                                        None => Some(vec![sig_index as SignatureIndex]),
                                        Some(mut funcs) => {
                                            funcs.push(sig_index as SignatureIndex);
                                            Some(funcs)
                                        }
                                    };
                                    function_index += 1;
                                }
                                Import::Memory(mem) => {
                                    runtime.declare_memory(mem);
                                }
                                Import::Global(glob) => {
                                    runtime.declare_global(glob.clone());
                                    globals.push(glob);
                                }
                                Import::Table(tab) => {
                                    runtime.declare_table(tab);
                                }
                            }
                        }
                    }
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the import section: {}", s))
                    }
                }
                function_imports_count = function_index;
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Function, .. } => {
                match parse_function_section(&mut parser) {
                    Ok(funcs) => {
                        match functions {
                            None => functions = Some(funcs),
                            Some(ref mut imps) => imps.extend(funcs),
                        }
                    }
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the function section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Table, .. } => {
                match parse_table_section(&mut parser, runtime) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the table section: {}", s))
                    }
                }
            }
            ParserState::BeginSection { code: SectionCode::Memory, .. } => {
                match parse_memory_section(&mut parser) {
                    Ok(mems) => {
                        for mem in mems {
                            runtime.declare_memory(mem);
                        }
                    }
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the memory section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Global, .. } => {
                match parse_global_section(&mut parser, runtime) {
                    Ok(mut globs) => globals.append(&mut globs),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the global section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Export, .. } => {
                match parse_export_section(&mut parser) {
                    Ok(exps) => exports = Some(exps),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the export section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Start, .. } => {
                match *parser.read() {
                    ParserState::StartSectionEntry(index) => {
                        start_index = Some(index as FunctionIndex)
                    }
                    _ => return Err(String::from("wrong content in the start section")),
                }
                match *parser.read() {
                    ParserState::EndSection => {}
                    _ => return Err(String::from("wrong content in the start section")),
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Element, .. } => {
                match parse_elements_section(&mut parser, runtime, &globals) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the element section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Code, .. } => {
                // The code section begins
                break;
            }
            ParserState::EndSection => {
                next_input = ParserInput::Default;
            }
            ParserState::EndWasm => {
                return Ok(TranslationResult {
                              functions: Vec::new(),
                              start_index: None,
                          })
            }
            ParserState::BeginSection { code: SectionCode::Data, .. } => {
                match parse_data_section(&mut parser, runtime, &globals) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the data section: {}", s))
                    }
                }
            }
            _ => return Err(String::from("wrong content in the preamble")),
        };
    }
    // At this point we've entered the code section
    // First we check that we have all that is necessary to translate a function.
    let signatures = match signatures {
        None => Vec::new(),
        Some(sigs) => sigs,
    };
    let functions = match functions {
        None => return Err(String::from("missing a function section")),
        Some(functions) => functions,
    };
    let mut il_functions: Vec<FunctionTranslation> = Vec::new();
    il_functions.resize(function_imports_count, FunctionTranslation::Import());
    let mut il_builder = ILBuilder::new();
    runtime.begin_translation();
    loop {
        let locals: Vec<(usize, Type)> = match *parser.read() {
            ParserState::BeginFunctionBody { ref locals, .. } => {
                locals
                    .iter()
                    .map(|&(index, ref ty)| {
                             (index as usize,
                              match type_to_type(ty) {
                                  Ok(ty) => ty,
                                  Err(()) => panic!("unsupported type for local variable"),
                              })
                         })
                    .collect()
            }
            ParserState::EndSection => break,
            _ => return Err(String::from(format!("wrong content in code section"))),
        };
        let signature = signatures[functions[function_index as usize] as usize].clone();
        match translate_function_body(&mut parser,
                                      function_index,
                                      signature,
                                      &locals,
                                      &exports,
                                      &signatures,
                                      &functions,
                                      &mut il_builder,
                                      runtime) {
            Ok((il_func, imports)) => {
                il_functions.push(FunctionTranslation::Code {
                                      il: il_func,
                                      imports: invert_hashmaps(imports),
                                  })
            }
            Err(s) => return Err(s),
        }
        function_index += 1;
    }
    loop {
        match *parser.read() {
            ParserState::BeginSection { code: SectionCode::Data, .. } => {
                match parse_data_section(&mut parser, runtime, &globals) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the data section: {}", s))
                    }
                }
            }
            ParserState::EndWasm => {
                return Ok(TranslationResult {
                              functions: il_functions,
                              start_index,
                          })
            }
            _ => (),
        }
    }
}
