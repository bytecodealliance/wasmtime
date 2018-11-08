use error::Error;
use function_body;
#[allow(unused_imports)] // for now
use wasmparser::{
    CodeSectionReader, Data, DataSectionReader, Element, ElementSectionReader, Export,
    ExportSectionReader, ExternalKind, FuncType, FunctionSectionReader, Global,
    GlobalSectionReader, GlobalType, Import, ImportSectionEntryType, ImportSectionReader,
    MemorySectionReader, MemoryType, Operator, TableSectionReader, Type, TypeSectionReader,
};
use backend::{CodeGenSession, TranslatedCodeSection};

/// Parses the Type section of the wasm module.
pub fn type_(types: TypeSectionReader) -> Result<(), Error> {
    for entry in types {
        entry?; // TODO
    }
    Ok(())
}

/// Parses the Import section of the wasm module.
pub fn import(imports: ImportSectionReader) -> Result<(), Error> {
    for entry in imports {
        entry?; // TODO
    }
    Ok(())
}

/// Parses the Function section of the wasm module.
pub fn function(functions: FunctionSectionReader) -> Result<(), Error> {
    for entry in functions {
        entry?; // TODO
    }
    Ok(())
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
pub fn code(code: CodeSectionReader) -> Result<TranslatedCodeSection, Error> {
    let mut session = CodeGenSession::new();
    for body in code {
        function_body::translate(&mut session, &body?)?;
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
