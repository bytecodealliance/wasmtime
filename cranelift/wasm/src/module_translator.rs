//! Translation skeleton that traverses the whole WebAssembly module and call helper functions
//! to deal with each part of it.
use crate::environ::{ModuleEnvironment, WasmResult};
use crate::sections_translator::{
    parse_code_section, parse_data_section, parse_element_section, parse_export_section,
    parse_function_section, parse_global_section, parse_import_section, parse_memory_section,
    parse_name_section, parse_start_section, parse_table_section, parse_type_section,
};
use crate::state::ModuleTranslationState;
use cranelift_codegen::timing;
use wasmparser::{CustomSectionContent, ModuleReader, SectionContent};

/// Translate a sequence of bytes forming a valid Wasm binary into a list of valid Cranelift IR
/// [`Function`](cranelift_codegen::ir::Function).
pub fn translate_module<'data>(
    data: &'data [u8],
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<ModuleTranslationState> {
    let _tt = timing::wasm_translate_module();
    let mut reader = ModuleReader::new(data)?;
    let mut module_translation_state = ModuleTranslationState::new();

    while !reader.eof() {
        let section = reader.read()?;
        match section.content()? {
            SectionContent::Type(types) => {
                parse_type_section(types, &mut module_translation_state, environ)?;
            }

            SectionContent::Import(imports) => {
                parse_import_section(imports, environ)?;
            }

            SectionContent::Function(functions) => {
                parse_function_section(functions, environ)?;
            }

            SectionContent::Table(tables) => {
                parse_table_section(tables, environ)?;
            }

            SectionContent::Memory(memories) => {
                parse_memory_section(memories, environ)?;
            }

            SectionContent::Global(globals) => {
                parse_global_section(globals, environ)?;
            }

            SectionContent::Export(exports) => {
                parse_export_section(exports, environ)?;
            }

            SectionContent::Start(start) => {
                parse_start_section(start, environ)?;
            }

            SectionContent::Element(elements) => {
                parse_element_section(elements, environ)?;
            }

            SectionContent::Code(code) => {
                parse_code_section(code, &module_translation_state, environ)?;
            }

            SectionContent::Data(data) => {
                parse_data_section(data, environ)?;
            }

            SectionContent::DataCount(count) => {
                environ.reserve_passive_data(count)?;
            }

            SectionContent::Module(_)
            | SectionContent::ModuleCode(_)
            | SectionContent::Instance(_)
            | SectionContent::Alias(_) => unimplemented!("module linking not implemented yet"),

            SectionContent::Custom {
                name,
                binary,
                content,
            } => match content {
                Some(CustomSectionContent::Name(names)) => {
                    parse_name_section(names, environ)?;
                }
                _ => {
                    let mut reader = binary.clone();
                    let len = reader.bytes_remaining();
                    let payload = reader.read_bytes(len)?;
                    environ.custom_section(name, payload)?;
                }
            },
        }
    }

    Ok(module_translation_state)
}
