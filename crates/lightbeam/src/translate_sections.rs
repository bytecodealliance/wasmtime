use crate::backend::TranslatedCodeSection;
use crate::error::Error;
use crate::module::SimpleContext;
use cranelift_codegen::{binemit, ir};
use wasmparser::{
    CodeSectionReader, DataSectionReader, ElementSectionReader, ExportSectionReader, FuncType,
    FunctionSectionReader, GlobalSectionReader, ImportSectionReader, MemorySectionReader,
    MemoryType, TableSectionReader, TableType, TypeDef, TypeSectionReader,
};

/// Parses the Type section of the wasm module.
pub fn type_(types_reader: TypeSectionReader) -> Result<Vec<FuncType>, Error> {
    types_reader
        .into_iter()
        .map(|r| match r {
            Ok(TypeDef::Func(ft)) => Ok(ft),
            Ok(_) => unimplemented!("module linking is not implemented yet"),
            Err(e) => Err(e.into()),
        })
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
#[allow(dead_code)]
pub fn code(
    _code: CodeSectionReader,
    _translation_ctx: &SimpleContext,
) -> Result<TranslatedCodeSection, Error> {
    // TODO: Remove the Lightbeam harness entirely, this is just to make this compile.
    //       We do all our testing through Wasmtime now, there's no reason to duplicate
    //       writing a WebAssembly environment in Lightbeam too.
    unimplemented!("Incomplete migration to wasm-reader");
}

/// Parses the Data section of the wasm module.
pub fn data(data: DataSectionReader) -> Result<(), Error> {
    for entry in data {
        entry?; // TODO
    }
    Ok(())
}
