use backend::TranslatedCodeSection;
use error::Error;
use std::mem;
use translate_sections;
use wasmparser::{FuncType, ModuleReader, SectionCode};

pub trait FunctionArgs {
    unsafe fn call<T>(self, start: *const u8) -> T;
}

macro_rules! impl_function_args {
    ($first:ident $(, $rest:ident)*) => {
        impl<$first, $($rest),*> FunctionArgs for ($first, $($rest),*) {
            #[allow(non_snake_case)]
            unsafe fn call<T>(self, start: *const u8) -> T {
                let func = mem::transmute::<_, extern "sysv64" fn($first, $($rest),*) -> T>(start);
                {
                    let ($first, $($rest),*) = self;
                    func($first, $($rest),*)
                }
            }
        }

        impl_function_args!($($rest),*);
    };
    () => {
        impl FunctionArgs for () {
            unsafe fn call<T>(self, start: *const u8) -> T {
                let func = mem::transmute::<_, extern "sysv64" fn() -> T>(start);
                func()
            }
        }
    };
}

impl_function_args!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);

#[derive(Default)]
pub struct TranslatedModule {
    translated_code_section: Option<TranslatedCodeSection>,
}

impl TranslatedModule {
    // For testing only.
    // TODO: Handle generic signatures.
    pub unsafe fn execute_func<Args: FunctionArgs, T>(&self, func_idx: u32, args: Args) -> T {
        let code_section = self
            .translated_code_section
            .as_ref()
            .expect("no code section");
        let start_buf = code_section.func_start(func_idx as usize);

        args.call(start_buf)
    }
}

#[derive(Default)]
pub struct TranslationContext {
    types: Vec<FuncType>,
    func_ty_indicies: Vec<u32>,
}

impl TranslationContext {
    pub fn func_type(&self, func_idx: u32) -> &FuncType {
        // TODO: This assumes that there is no imported functions.
        let func_ty_idx = self.func_ty_indicies[func_idx as usize];
        &self.types[func_ty_idx as usize]
    }

    // TODO: type of a global
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

    let mut ctx = TranslationContext::default();

    if let SectionCode::Type = section.code {
        let types_reader = section.get_type_section_reader()?;
        ctx.types = translate_sections::type_(types_reader)?;

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
        ctx.func_ty_indicies = translate_sections::function(functions)?;

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
        output.translated_code_section = Some(translate_sections::code(code, &ctx)?);

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
