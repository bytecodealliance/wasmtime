//! Translation skeleton that traverses the whole WebAssembly module and call helper functions
//! to deal with each part of it.
use cretonne_codegen::timing;
use environ::{ModuleEnvironment, WasmError, WasmResult};
use sections_translator::{
    parse_data_section, parse_elements_section, parse_export_section, parse_function_section,
    parse_function_signatures, parse_global_section, parse_import_section, parse_memory_section,
    parse_start_section, parse_table_section,
};
use wasmparser::{Parser, ParserInput, ParserState, SectionCode, WasmDecoder};

/// Translate a sequence of bytes forming a valid Wasm binary into a list of valid Cretonne IR
/// [`Function`](../codegen/ir/function/struct.Function.html).
/// Returns the functions and also the mappings for imported functions and signature between the
/// indexes in the wasm module and the indexes inside each functions.
pub fn translate_module<'data>(
    data: &'data [u8],
    environ: &mut ModuleEnvironment<'data>,
) -> WasmResult<()> {
    let _tt = timing::wasm_translate_module();
    let mut parser = Parser::new(data);
    match *parser.read() {
        ParserState::BeginWasm { .. } => {}
        ParserState::Error(e) => {
            return Err(WasmError::from_binary_reader_error(e));
        }
        ref s => panic!("modules should begin properly: {:?}", s),
    }
    let mut next_input = ParserInput::Default;
    loop {
        match *parser.read_with_input(next_input) {
            ParserState::BeginSection {
                code: SectionCode::Type,
                ..
            } => {
                parse_function_signatures(&mut parser, environ)?;
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection {
                code: SectionCode::Import,
                ..
            } => {
                parse_import_section(&mut parser, environ)?;
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection {
                code: SectionCode::Function,
                ..
            } => {
                parse_function_section(&mut parser, environ)?;
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection {
                code: SectionCode::Table,
                ..
            } => {
                parse_table_section(&mut parser, environ)?;
            }
            ParserState::BeginSection {
                code: SectionCode::Memory,
                ..
            } => {
                parse_memory_section(&mut parser, environ)?;
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection {
                code: SectionCode::Global,
                ..
            } => {
                parse_global_section(&mut parser, environ)?;
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection {
                code: SectionCode::Export,
                ..
            } => {
                parse_export_section(&mut parser, environ)?;
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection {
                code: SectionCode::Start,
                ..
            } => {
                parse_start_section(&mut parser, environ)?;
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection {
                code: SectionCode::Element,
                ..
            } => {
                parse_elements_section(&mut parser, environ)?;
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection {
                code: SectionCode::Code,
                ..
            } => {
                // The code section begins
                break;
            }
            ParserState::EndSection => {
                next_input = ParserInput::Default;
            }
            ParserState::EndWasm => return Ok(()),
            ParserState::BeginSection {
                code: SectionCode::Data,
                ..
            } => {
                parse_data_section(&mut parser, environ)?;
            }
            ParserState::BeginSection {
                code: SectionCode::Custom { .. },
                ..
            } => {
                // Ignore unknown custom sections.
                next_input = ParserInput::SkipSection;
            }
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            _ => panic!("wrong content in the preamble"),
        };
    }
    // At this point we've entered the code section
    loop {
        match *parser.read() {
            ParserState::BeginFunctionBody { .. } => {}
            ParserState::EndSection => break,
            ParserState::Error(e) => return Err(WasmError::from_binary_reader_error(e)),
            ref s => panic!("wrong content in code section: {:?}", s),
        }
        let mut reader = parser.create_binary_reader();
        let size = reader.bytes_remaining();
        environ.define_function_body(reader
            .read_bytes(size)
            .map_err(WasmError::from_binary_reader_error)?)?;
    }
    loop {
        match *parser.read() {
            ParserState::BeginSection {
                code: SectionCode::Data,
                ..
            } => {
                parse_data_section(&mut parser, environ)?;
            }
            ParserState::EndWasm => break,
            _ => (),
        }
    }
    Ok(())
}
