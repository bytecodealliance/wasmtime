use std::mem;
use error::Error;
use translate_sections;
use backend::TranslatedCodeSection;
use wasmparser::{ModuleReader, SectionCode};

#[derive(Default)]
pub struct TranslatedModule {
    translated_code_section: Option<TranslatedCodeSection>,
}

impl TranslatedModule {
    // For testing only.
    // Assume signature is (i32, i32) -> i32 for now.
    // TODO: Handle generic signatures.
    pub fn execute_func(&self, func_idx: u32, a: usize, b: usize) -> usize {
        let code_section = self.translated_code_section.as_ref().expect("no code section");
        let start_buf = code_section.func_start(func_idx as usize);

        unsafe {
            let func = mem::transmute::<_, extern "sysv64" fn(usize, usize) -> usize>(start_buf);
            func(a, b)
        }
    }
}

/// Translate from a slice of bytes holding a wasm module.
pub fn translate(data: &[u8]) -> Result<TranslatedModule, Error> {
    let mut reader = ModuleReader::new(data)?;
    let mut output = TranslatedModule::default();

    reader.skip_custom_sections()?;
    if reader.eof() {
        return Ok(output);
    }
    let mut section = reader.read()?;

    if let SectionCode::Type = section.code {
        let types = section.get_type_section_reader()?;
        translate_sections::type_(types)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Import = section.code {
        let imports = section.get_import_section_reader()?;
        translate_sections::import(imports)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Function = section.code {
        let functions = section.get_function_section_reader()?;
        translate_sections::function(functions)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Table = section.code {
        let tables = section.get_table_section_reader()?;
        translate_sections::table(tables)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Memory = section.code {
        let memories = section.get_memory_section_reader()?;
        translate_sections::memory(memories)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Global = section.code {
        let globals = section.get_global_section_reader()?;
        translate_sections::global(globals)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Export = section.code {
        let exports = section.get_export_section_reader()?;
        translate_sections::export(exports)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Start = section.code {
        let start = section.get_start_section_content()?;
        translate_sections::start(start)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Element = section.code {
        let elements = section.get_element_section_reader()?;
        translate_sections::element(elements)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Code = section.code {
        let code = section.get_code_section_reader()?;
        output.translated_code_section = Some(translate_sections::code(code)?);

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Data = section.code {
        let data = section.get_data_section_reader()?;
        translate_sections::data(data)?;
    }

    Ok(output)
}
