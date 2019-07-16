//! Translation skeleton that traverses the whole WebAssembly module and call helper functions
//! to deal with each part of it.
use crate::environ::{ModuleEnvironment, WasmError, WasmResult};
use crate::sections_translator::{
    parse_code_section, parse_data_section, parse_element_section, parse_export_section,
    parse_function_section, parse_global_section, parse_import_section, parse_memory_section,
    parse_start_section, parse_table_section, parse_type_section,
};
use cranelift_codegen::timing;
use wasmparser::{ModuleReader, SectionCode};

/// Translate a sequence of bytes forming a valid Wasm binary into a list of valid Cranelift IR
/// [`Function`](../codegen/ir/function/struct.Function.html).
pub fn translate_module<'data>(
    data: &'data [u8],
    environ: &mut dyn ModuleEnvironment<'data>,
) -> WasmResult<()> {
    let _tt = timing::wasm_translate_module();
    let mut reader = ModuleReader::new(data)?;

    reader.skip_custom_sections()?;
    if reader.eof() {
        return Ok(());
    }
    let mut section = reader.read()?;

    if let SectionCode::Type = section.code {
        let types = section.get_type_section_reader()?;
        parse_type_section(types, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Import = section.code {
        let imports = section.get_import_section_reader()?;
        parse_import_section(imports, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Function = section.code {
        let functions = section.get_function_section_reader()?;
        parse_function_section(functions, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Table = section.code {
        let tables = section.get_table_section_reader()?;
        parse_table_section(tables, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Memory = section.code {
        let memories = section.get_memory_section_reader()?;
        parse_memory_section(memories, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Global = section.code {
        let globals = section.get_global_section_reader()?;
        parse_global_section(globals, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Export = section.code {
        let exports = section.get_export_section_reader()?;
        parse_export_section(exports, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Start = section.code {
        let start = section.get_start_section_content()?;
        parse_start_section(start, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Element = section.code {
        let elements = section.get_element_section_reader()?;
        parse_element_section(elements, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Code = section.code {
        let code = section.get_code_section_reader()?;
        parse_code_section(code, environ)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Data = section.code {
        let data = section.get_data_section_reader()?;
        parse_data_section(data, environ)?;
    }

    reader.skip_custom_sections()?;
    if !reader.eof() {
        return Err(WasmError::InvalidWebAssembly {
            message: "sections must occur at most once and in the prescribed order",
            offset: reader.current_position(),
        });
    }

    Ok(())
}
