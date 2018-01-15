//! Translation skeletton that traverses the whole WebAssembly module and call helper functions
//! to deal with each part of it.
use cretonne::timing;
use wasmparser::{ParserState, SectionCode, ParserInput, Parser, WasmDecoder, BinaryReaderError};
use sections_translator::{SectionParsingError, parse_function_signatures, parse_import_section,
                          parse_function_section, parse_export_section, parse_start_section,
                          parse_memory_section, parse_global_section, parse_table_section,
                          parse_elements_section, parse_data_section};
use environ::ModuleEnvironment;

// this is for no_std builds, but has no affect on regular builds
use std::string::String;

/// Translate a sequence of bytes forming a valid Wasm binary into a list of valid Cretonne IL
/// [`Function`](../cretonne/ir/function/struct.Function.html).
/// Returns the functions and also the mappings for imported functions and signature between the
/// indexes in the wasm module and the indexes inside each functions.
pub fn translate_module<'data>(
    data: &'data [u8],
    environ: &mut ModuleEnvironment<'data>,
) -> Result<(), String> {
    let _tt = timing::wasm_translate_module();
    let mut parser = Parser::new(data);
    match *parser.read() {
        ParserState::BeginWasm { .. } => {}
        ParserState::Error(BinaryReaderError { message, offset }) => {
            return Err(format!("at offset {}: {}", offset, message));
        }
        ref s => panic!("modules should begin properly: {:?}", s),
    }
    let mut next_input = ParserInput::Default;
    loop {
        match *parser.read_with_input(next_input) {
            ParserState::BeginSection { code: SectionCode::Type, .. } => {
                match parse_function_signatures(&mut parser, environ) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the type section: {}", s))
                    }
                };
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Import, .. } => {
                match parse_import_section(&mut parser, environ) {
                    Ok(()) => {}
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the import section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Function, .. } => {
                match parse_function_section(&mut parser, environ) {
                    Ok(()) => {}
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the function section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Table, .. } => {
                match parse_table_section(&mut parser, environ) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the table section: {}", s))
                    }
                }
            }
            ParserState::BeginSection { code: SectionCode::Memory, .. } => {
                match parse_memory_section(&mut parser, environ) {
                    Ok(()) => {}
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the memory section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Global, .. } => {
                match parse_global_section(&mut parser, environ) {
                    Ok(()) => {}
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the global section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Export, .. } => {
                match parse_export_section(&mut parser, environ) {
                    Ok(()) => {}
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the export section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Start, .. } => {
                match parse_start_section(&mut parser, environ) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the start section: {}", s))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Element, .. } => {
                match parse_elements_section(&mut parser, environ) {
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
            ParserState::EndWasm => return Ok(()),
            ParserState::BeginSection { code: SectionCode::Data, .. } => {
                match parse_data_section(&mut parser, environ) {
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
    loop {
        match *parser.read() {
            ParserState::BeginFunctionBody { .. } => {}
            ParserState::EndSection => break,
            _ => return Err(String::from("wrong content in code section")),
        }
        let mut reader = parser.create_binary_reader();
        let size = reader.bytes_remaining();
        environ.define_function_body(
            reader.read_bytes(size).map_err(|e| {
                format!("at offset {}: {}", e.offset, e.message)
            })?,
        )?;
    }
    loop {
        match *parser.read() {
            ParserState::BeginSection { code: SectionCode::Data, .. } => {
                match parse_data_section(&mut parser, environ) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent(s)) => {
                        return Err(format!("wrong content in the data section: {}", s))
                    }
                }
            }
            ParserState::EndWasm => break,
            _ => (),
        }
    }
    Ok(())
}
