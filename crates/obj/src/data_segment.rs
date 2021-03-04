use anyhow::Result;
use object::write::{Object, StandardSection, Symbol, SymbolSection};
use object::{SymbolFlags, SymbolKind, SymbolScope};
use wasmtime_environ::MemoryInitializer;

/// Declares data segment symbol
pub fn declare_data_segment(
    obj: &mut Object,
    _memory_initializer: &MemoryInitializer,
    index: usize,
) -> Result<()> {
    let name = format!("_memory_{}", index);
    let _symbol_id = obj.add_symbol(Symbol {
        name: name.as_bytes().to_vec(),
        value: 0,
        size: 0,
        kind: SymbolKind::Data,
        scope: SymbolScope::Linkage,
        weak: false,
        section: SymbolSection::Undefined,
        flags: SymbolFlags::None,
    });
    Ok(())
}

/// Emit segment data and initialization location
pub fn emit_data_segment(
    obj: &mut Object,
    memory_initializer: &MemoryInitializer,
    index: usize,
) -> Result<()> {
    let name = format!("_memory_{}", index);
    let symbol_id = obj.symbol_id(name.as_bytes()).unwrap();
    let section_id = obj.section_id(StandardSection::Data);
    obj.add_symbol_data(symbol_id, section_id, &memory_initializer.data, 1);
    Ok(())
}
