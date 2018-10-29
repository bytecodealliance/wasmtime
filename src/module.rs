use error::Error;
use translate_sections;
use wasmparser::{ModuleReader, SectionCode};

/// Translate from a slice of bytes holding a wasm module.
pub fn translate(data: &[u8]) -> Result<(), Error> {
    let mut reader = ModuleReader::new(data)?;

    reader.skip_custom_sections()?;
    if reader.eof() {
        return Ok(());
    }
    let mut section = reader.read()?;

    if let SectionCode::Type = section.code {
        let types = section.get_type_section_reader()?;
        translate_sections::type_(types)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Import = section.code {
        let imports = section.get_import_section_reader()?;
        translate_sections::import(imports)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Function = section.code {
        let functions = section.get_function_section_reader()?;
        translate_sections::function(functions)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Table = section.code {
        let tables = section.get_table_section_reader()?;
        translate_sections::table(tables)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Memory = section.code {
        let memories = section.get_memory_section_reader()?;
        translate_sections::memory(memories)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Global = section.code {
        let globals = section.get_global_section_reader()?;
        translate_sections::global(globals)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Export = section.code {
        let exports = section.get_export_section_reader()?;
        translate_sections::export(exports)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Start = section.code {
        let start = section.get_start_section_content()?;
        translate_sections::start(start)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Element = section.code {
        let elements = section.get_element_section_reader()?;
        translate_sections::element(elements)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Code = section.code {
        let code = section.get_code_section_reader()?;
        translate_sections::code(code)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(());
        }
        section = reader.read()?;
    }

    if let SectionCode::Data = section.code {
        let data = section.get_data_section_reader()?;
        translate_sections::data(data)?;
    }

    Ok(())
}
