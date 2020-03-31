use crate::backend::{CodeGenSession, TranslatedCodeSection};
use crate::error::Error;
use crate::function_body;
use crate::module::SimpleContext;
use cranelift_codegen::{binemit, ir};
use wasmparser::{
    CodeSectionReader, DataSectionReader, ElementSectionReader, ExportSectionReader, FuncType,
    FunctionSectionReader, GlobalSectionReader, ImportSectionReader, MemorySectionReader,
    MemoryType, TableSectionReader, TableType, TypeSectionReader,
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
pub fn element(elements: ElementSectionReader) -> Result<(), Error> {
    for entry in elements {
        entry?;
    }

    Ok(())
}

struct UnimplementedRelocSink;

impl binemit::RelocSink for UnimplementedRelocSink {
    fn reloc_block(&mut self, _: binemit::CodeOffset, _: binemit::Reloc, _: binemit::CodeOffset) {
        unimplemented!()
    }

    fn reloc_external(
        &mut self,
        _: binemit::CodeOffset,
        _: ir::SourceLoc,
        _: binemit::Reloc,
        _: &ir::ExternalName,
        _: binemit::Addend,
    ) {
        unimplemented!()
    }

    fn reloc_constant(&mut self, _: binemit::CodeOffset, _: binemit::Reloc, _: ir::ConstantOffset) {
        unimplemented!()
    }

    fn reloc_jt(&mut self, _: binemit::CodeOffset, _: binemit::Reloc, _: ir::JumpTable) {
        unimplemented!()
    }
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
        let mut relocs = UnimplementedRelocSink;

        function_body::translate_wasm(&mut session, &mut relocs, idx as u32, &body)?;
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
