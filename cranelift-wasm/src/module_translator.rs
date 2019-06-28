//! Translation skeleton that traverses the whole WebAssembly module and call helper functions
//! to deal with each part of it.
use crate::environ::{ModuleEnvironment, WasmError, WasmResult};
use crate::sections_translator::{
    parse_code_section, parse_data_section, parse_element_section, parse_export_section,
    parse_function_section, parse_global_section, parse_import_section, parse_memory_section,
    parse_name_section, parse_start_section, parse_table_section, parse_type_section,
};
use cranelift_codegen::timing;
use wasmparser::{CustomSectionKind, ModuleReader, SectionCode};

/// Translate a sequence of bytes forming a valid Wasm binary into a list of valid Cranelift IR
/// [`Function`](cranelift_codegen::ir::Function).
pub fn translate_module<'data>(
    data: &'data [u8],
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    let _tt = timing::wasm_translate_module();
    let mut reader = ModuleReader::new(data)?;

    while !reader.eof() {
        let section = reader.read()?;
        match section.code {
            SectionCode::Type => {
                let types = section.get_type_section_reader()?;
                parse_type_section(types, environ)?;
            }

            SectionCode::Import => {
                let imports = section.get_import_section_reader()?;
                parse_import_section(imports, environ)?;
            }

            SectionCode::Function => {
                let functions = section.get_function_section_reader()?;
                parse_function_section(functions, environ)?;
            }

            SectionCode::Table => {
                let tables = section.get_table_section_reader()?;
                parse_table_section(tables, environ)?;
            }

            SectionCode::Memory => {
                let memories = section.get_memory_section_reader()?;
                parse_memory_section(memories, environ)?;
            }

            SectionCode::Global => {
                let globals = section.get_global_section_reader()?;
                parse_global_section(globals, environ)?;
            }

            SectionCode::Export => {
                let exports = section.get_export_section_reader()?;
                parse_export_section(exports, environ)?;
            }

            SectionCode::Start => {
                let start = section.get_start_section_content()?;
                parse_start_section(start, environ)?;
            }

            SectionCode::Element => {
                let elements = section.get_element_section_reader()?;
                parse_element_section(elements, environ)?;
            }

            SectionCode::Code => {
                let code = section.get_code_section_reader()?;
                parse_code_section(code, environ)?;
            }

            SectionCode::Data => {
                let data = section.get_data_section_reader()?;
                parse_data_section(data, environ)?;
            }

            SectionCode::DataCount => {
                return Err(WasmError::InvalidWebAssembly {
                    message: "don't know how to handle the data count section yet",
                    offset: reader.current_position(),
                });
            }

            SectionCode::Custom {
                kind: CustomSectionKind::Name,
                name: _,
            } => {
                let names = section.get_name_section_reader()?;
                parse_name_section(names, environ)?;
            }

            SectionCode::Custom { name, kind: _ } => {
                let mut reader = section.get_binary_reader();
                let len = reader.bytes_remaining();
                let payload = reader.read_bytes(len)?;
                environ.custom_section(name, payload)?;
            }
        }
    }

    Ok(())
}
