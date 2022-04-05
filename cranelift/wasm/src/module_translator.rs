//! Translation skeleton that traverses the whole WebAssembly module and call helper functions
//! to deal with each part of it.
use crate::environ::ModuleEnvironment;
use crate::sections_translator::{
    parse_data_section, parse_element_section, parse_export_section, parse_function_section,
    parse_global_section, parse_import_section, parse_memory_section, parse_name_section,
    parse_start_section, parse_table_section, parse_tag_section, parse_type_section,
};
use crate::state::ModuleTranslationState;
use crate::WasmResult;
use cranelift_codegen::timing;
use std::prelude::v1::*;
use wasmparser::{NameSectionReader, Parser, Payload, Validator};

/// Translate a sequence of bytes forming a valid Wasm binary into a list of valid Cranelift IR
/// [`Function`](cranelift_codegen::ir::Function).
pub fn translate_module<'data>(
    data: &'data [u8],
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<ModuleTranslationState> {
    let _tt = timing::wasm_translate_module();
    let mut module_translation_state = ModuleTranslationState::new();
    let mut validator = Validator::new_with_features(environ.wasm_features());

    for payload in Parser::new(0).parse_all(data) {
        match payload? {
            Payload::Version {
                num,
                encoding,
                range,
            } => {
                validator.version(num, encoding, &range)?;
            }
            Payload::End(offset) => {
                validator.end(offset)?;
            }

            Payload::TypeSection(types) => {
                validator.type_section(&types)?;
                parse_type_section(types, &mut module_translation_state, environ)?;
            }

            Payload::ImportSection(imports) => {
                validator.import_section(&imports)?;
                parse_import_section(imports, environ)?;
            }

            Payload::FunctionSection(functions) => {
                validator.function_section(&functions)?;
                parse_function_section(functions, environ)?;
            }

            Payload::TableSection(tables) => {
                validator.table_section(&tables)?;
                parse_table_section(tables, environ)?;
            }

            Payload::MemorySection(memories) => {
                validator.memory_section(&memories)?;
                parse_memory_section(memories, environ)?;
            }

            Payload::TagSection(tags) => {
                validator.tag_section(&tags)?;
                parse_tag_section(tags, environ)?;
            }

            Payload::GlobalSection(globals) => {
                validator.global_section(&globals)?;
                parse_global_section(globals, environ)?;
            }

            Payload::ExportSection(exports) => {
                validator.export_section(&exports)?;
                parse_export_section(exports, environ)?;
            }

            Payload::StartSection { func, range } => {
                validator.start_section(func, &range)?;
                parse_start_section(func, environ)?;
            }

            Payload::ElementSection(elements) => {
                validator.element_section(&elements)?;
                parse_element_section(elements, environ)?;
            }

            Payload::CodeSectionStart { count, range, .. } => {
                validator.code_section_start(count, &range)?;
                environ.reserve_function_bodies(count, range.start as u64);
            }

            Payload::CodeSectionEntry(body) => {
                let func_validator = validator.code_section_entry(&body)?;
                environ.define_function_body(func_validator, body)?;
            }

            Payload::DataSection(data) => {
                validator.data_section(&data)?;
                parse_data_section(data, environ)?;
            }

            Payload::DataCountSection { count, range } => {
                validator.data_count_section(count, &range)?;

                // NOTE: the count here is the total segment count, not the passive segment count
                environ.reserve_passive_data(count)?;
            }

            Payload::CustomSection {
                name: "name",
                data,
                data_offset,
                range: _,
            } => {
                let result = NameSectionReader::new(data, data_offset)
                    .map_err(|e| e.into())
                    .and_then(|s| parse_name_section(s, environ));
                if let Err(e) = result {
                    log::warn!("failed to parse name section {:?}", e);
                }
            }

            Payload::CustomSection { name, data, .. } => environ.custom_section(name, data)?,

            other => {
                validator.payload(&other)?;
                panic!("unimplemented section {:?}", other);
            }
        }
    }

    Ok(module_translation_state)
}
