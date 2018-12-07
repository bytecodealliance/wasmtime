use backend::{CodeGenSession, TranslatedCodeSection};
use error::Error;
use function_body;
use module::TranslationContext;
#[allow(unused_imports)] // for now
use wasmparser::{
    CodeSectionReader, Data, DataSectionReader, Element, ElementSectionReader, Export,
    ExportSectionReader, ExternalKind, FuncType, FunctionSectionReader, Global,
    GlobalSectionReader, GlobalType, Import, ImportSectionEntryType, ImportSectionReader,
    MemorySectionReader, MemoryType, Operator, TableSectionReader, Type, TypeSectionReader,
};

/// Parses the Type section of the wasm module.
pub fn type_(types_reader: TypeSectionReader) -> Result<Vec<FuncType>, Error> {
    let mut types = vec![];
    for entry in types_reader {
        types.push(entry?);
    }
    Ok(types)
}

/// Parses the Import section of the wasm module.
pub fn import(imports: ImportSectionReader) -> Result<(), Error> {
    for entry in imports {
        entry?; // TODO
    }
    Ok(())
}

/// Parses the Function section of the wasm module.
pub fn function(functions: FunctionSectionReader) -> Result<Vec<u32>, Error> {
    let mut func_ty_indicies = vec![];
    for entry in functions {
        func_ty_indicies.push(entry?);
    }
    Ok(func_ty_indicies)
}

/// Parses the Table section of the wasm module.
pub fn table(tables: TableSectionReader) -> Result<(), Error> {
    for entry in tables {
        entry?; // TODO
    }
    Ok(())
}

/// Parses the Memory section of the wasm module.
pub fn memory(memories: MemorySectionReader) -> Result<(), Error> {
    for entry in memories {
        entry?; // TODO
    }
    Ok(())
}

/// Parses the Global section of the wasm module.
pub fn global(globals: GlobalSectionReader) -> Result<(), Error> {
    for entry in globals {
        entry?; // TODO
    }
    Ok(())
}

/// Parses the Export section of the wasm module.
pub fn export(exports: ExportSectionReader) -> Result<(), Error> {
    for entry in exports {
        entry?; // TODO
    }
    Ok(())
}

/// Parses the Start section of the wasm module.
pub fn start(_index: u32) -> Result<(), Error> {
    // TODO
    Ok(())
}

/// Parses the Element section of the wasm module.
pub fn element(elements: ElementSectionReader) -> Result<(), Error> {
    for entry in elements {
        entry?; // TODO
    }
    Ok(())
}

/// Parses the Code section of the wasm module.
pub fn code(
    code: CodeSectionReader,
    translation_ctx: &TranslationContext,
) -> Result<TranslatedCodeSection, Error> {
    let func_count = code.get_count();
    let mut session = CodeGenSession::new(func_count);
    for (idx, body) in code.into_iter().enumerate() {
        function_body::translate(&mut session, translation_ctx, idx as u32, &body?)?;
    }
    Ok(session.into_translated_code_section()?)
}

/// Parses the Data section of the wasm module.
pub fn data(data: DataSectionReader) -> Result<(), Error> {
    for entry in data {
        entry?; // TODO
    }
    Ok(())
}
