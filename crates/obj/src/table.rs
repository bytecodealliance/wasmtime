use anyhow::Result;
use object::write::{Object, StandardSection, Symbol, SymbolSection};
use object::{SymbolFlags, SymbolKind, SymbolScope};

/// Declares data segment symbol
pub fn declare_table(obj: &mut Object, index: usize) -> Result<()> {
    let name = format!("_table_{}", index);
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
pub fn emit_table(obj: &mut Object, index: usize) -> Result<()> {
    let name = format!("_table_{}", index);
    let symbol_id = obj.symbol_id(name.as_bytes()).unwrap();
    let section_id = obj.section_id(StandardSection::Data);
    // FIXME: We need to initialize table using function symbols
    obj.add_symbol_data(symbol_id, section_id, &[], 1);
    Ok(())
}
