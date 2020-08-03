use crate::builder::{ObjectBuilder, ObjectBuilderTarget};
use crate::context::layout_vmcontext;
use crate::data_segment::{declare_data_segment, emit_data_segment};
use crate::table::{declare_table, emit_table};
use anyhow::Result;
use object::write::{Object, Relocation, StandardSection, Symbol, SymbolSection};
use object::{RelocationEncoding, RelocationKind, SymbolFlags, SymbolKind, SymbolScope};
use wasmtime_debug::DwarfSection;
use wasmtime_environ::isa::TargetFrontendConfig;
use wasmtime_environ::{CompiledFunctions, DataInitializer, Module};

fn emit_vmcontext_init(
    obj: &mut Object,
    module: &Module,
    target_config: &TargetFrontendConfig,
) -> Result<()> {
    let (data, table_relocs) = layout_vmcontext(module, target_config);
    let symbol_id = obj.add_symbol(Symbol {
        name: "_vmcontext_init".as_bytes().to_vec(),
        value: 0,
        size: 0,
        kind: SymbolKind::Data,
        scope: SymbolScope::Linkage,
        weak: false,
        section: SymbolSection::Undefined,
        flags: SymbolFlags::None,
    });
    let section_id = obj.section_id(StandardSection::Data);
    let section_offset = obj.add_symbol_data(symbol_id, section_id, &data, 1);

    for reloc in table_relocs.iter() {
        let target_name = format!("_table_{}", reloc.index);
        let target_symbol = obj.symbol_id(target_name.as_bytes()).unwrap();
        obj.add_relocation(
            section_id,
            Relocation {
                offset: section_offset + reloc.offset as u64,
                size: 64, // FIXME for all targets
                kind: RelocationKind::Absolute,
                encoding: RelocationEncoding::Generic,
                symbol: target_symbol,
                addend: 0,
            },
        )?;
    }
    Ok(())
}

/// Emits a module that has been emitted with the `wasmtime-environ` environment
/// implementation to a native object file.
pub fn emit_module(
    target: ObjectBuilderTarget,
    module: &Module,
    target_config: &TargetFrontendConfig,
    compilation: CompiledFunctions,
    dwarf_sections: Vec<DwarfSection>,
    data_initializers: &[DataInitializer],
) -> Result<Object> {
    let mut builder = ObjectBuilder::new(target, module, &compilation);
    builder.set_dwarf_sections(dwarf_sections);
    let mut obj = builder.build()?;

    // Append data, table and vmcontext_init code to the object file.

    for (i, initializer) in data_initializers.iter().enumerate() {
        declare_data_segment(&mut obj, initializer, i)?;
    }

    for i in 0..module.local.table_plans.len() {
        declare_table(&mut obj, i)?;
    }

    for (i, initializer) in data_initializers.iter().enumerate() {
        emit_data_segment(&mut obj, initializer, i)?;
    }

    for i in 0..module.local.table_plans.len() {
        emit_table(&mut obj, i)?;
    }

    emit_vmcontext_init(&mut obj, module, target_config)?;

    Ok(obj)
}
