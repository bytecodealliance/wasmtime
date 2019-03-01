use backend::{CodeGenSession, TranslatedCodeSection};
use error::Error;
use function_body;
use microwasm::{MicrowasmConv, Type as MWType};
use module::{ModuleContext, SimpleContext};
#[allow(unused_imports)] // for now
use wasmparser::{
    CodeSectionReader, Data, DataSectionReader, Element, ElementSectionReader, Export,
    ExportSectionReader, ExternalKind, FuncType, FunctionSectionReader, Global,
    GlobalSectionReader, GlobalType, Import, ImportSectionEntryType, ImportSectionReader,
    MemorySectionReader, MemoryType, Operator, TableSectionReader, TableType, Type,
    TypeSectionReader,
};

/// Parses the Type section of the wasm module.
pub fn type_(types_reader: TypeSectionReader) -> Result<Vec<FuncType>, Error> {
    types_reader
        .into_iter()
        .map(|r| r.map_err(Into::into))
        .collect()
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
    functions
        .into_iter()
        .map(|r| r.map_err(Into::into))
        .collect()
}

/// Parses the Table section of the wasm module.
pub fn table(tables: TableSectionReader) -> Result<Vec<TableType>, Error> {
    tables.into_iter().map(|r| r.map_err(Into::into)).collect()
}

/// Parses the Memory section of the wasm module.
pub fn memory(memories: MemorySectionReader) -> Result<Vec<MemoryType>, Error> {
    memories
        .into_iter()
        .map(|r| r.map_err(Into::into))
        .collect()
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
pub fn element(elements: ElementSectionReader) -> Result<Vec<u32>, Error> {
    let mut out = Vec::new();

    for entry in elements {
        let entry = entry?;

        assert_eq!(entry.table_index, 0);
        let offset = {
            let mut reader = entry.init_expr.get_operators_reader();
            let out = match reader.read() {
                Ok(Operator::I32Const { value }) => value,
                _ => panic!("We only support i32.const table init expressions right now"),
            };

            //reader.ensure_end()?;

            out
        };

        assert_eq!(offset, out.len() as i32);

        let elements = entry
            .items
            .get_items_reader()?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        out.extend(elements);
    }

    Ok(out)
}

/// Parses the Code section of the wasm module.
pub fn code(
    code: CodeSectionReader,
    translation_ctx: &SimpleContext,
) -> Result<TranslatedCodeSection, Error> {
    let func_count = code.get_count();
    let mut session = CodeGenSession::new(func_count, translation_ctx);

    for (idx, body) in code.into_iter().enumerate() {
        let body = body?;

        function_body::translate_wasm(
            &mut session,
            idx as u32,
            &body,
        )?;
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
