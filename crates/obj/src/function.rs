use anyhow::Result;
use object::write::{Object, Relocation, StandardSection, Symbol, SymbolSection};
use object::{RelocationEncoding, RelocationKind, SymbolFlags, SymbolKind, SymbolScope};
use wasmtime_environ::entity::EntityRef;
use wasmtime_environ::settings;
use wasmtime_environ::settings::Configurable;
use wasmtime_environ::{Compilation, Module, RelocationTarget, Relocations};

/// Defines module functions
pub fn declare_functions(
    obj: &mut Object,
    module: &Module,
    relocations: &Relocations,
) -> Result<()> {
    for i in 0..module.local.num_imported_funcs {
        let string_name = format!("_wasm_function_{}", i);
        let _symbol_id = obj.add_symbol(Symbol {
            name: string_name.as_bytes().to_vec(),
            value: 0,
            size: 0,
            kind: SymbolKind::Text,
            scope: SymbolScope::Unknown,
            weak: false,
            section: SymbolSection::Undefined,
            flags: SymbolFlags::None,
        });
    }
    for (i, _function_relocs) in relocations.iter().rev() {
        let func_index = module.local.func_index(i);
        let string_name = format!("_wasm_function_{}", func_index.index());
        let _symbol_id = obj.add_symbol(Symbol {
            name: string_name.as_bytes().to_vec(),
            value: 0,
            size: 0,
            kind: SymbolKind::Text,
            scope: SymbolScope::Linkage,
            weak: false,
            section: SymbolSection::Undefined,
            flags: SymbolFlags::None,
        });
    }
    Ok(())
}

/// Emits module functions
pub fn emit_functions(
    obj: &mut Object,
    module: &Module,
    compilation: &Compilation,
    relocations: &Relocations,
) -> Result<()> {
    debug_assert!(
        module.start_func.is_none()
            || module.start_func.unwrap().index() >= module.local.num_imported_funcs,
        "imported start functions not supported yet"
    );

    let mut shared_builder = settings::builder();
    shared_builder
        .enable("enable_verifier")
        .expect("Missing enable_verifier setting");

    for (i, _function_relocs) in relocations.iter() {
        let body = &compilation.get(i).body;
        let func_index = module.local.func_index(i);
        let string_name = format!("_wasm_function_{}", func_index.index());

        let symbol_id = obj.symbol_id(string_name.as_bytes()).unwrap();
        let section_id = obj.section_id(StandardSection::Text);

        obj.add_symbol_data(symbol_id, section_id, body, 1);
    }

    for (i, function_relocs) in relocations.iter() {
        let func_index = module.local.func_index(i);
        let string_name = format!("_wasm_function_{}", func_index.index());
        let symbol_id = obj.symbol_id(string_name.as_bytes()).unwrap();
        let (_, section_offset) = obj.symbol_section_and_offset(symbol_id).unwrap();
        let section_id = obj.section_id(StandardSection::Text);
        for r in function_relocs {
            debug_assert_eq!(r.addend, 0);
            match r.reloc_target {
                RelocationTarget::UserFunc(target_index) => {
                    let target_name = format!("_wasm_function_{}", target_index.index());
                    let target_symbol = obj.symbol_id(target_name.as_bytes()).unwrap();
                    obj.add_relocation(
                        section_id,
                        Relocation {
                            offset: section_offset + r.offset as u64,
                            size: 64, // FIXME for all targets
                            kind: RelocationKind::Absolute,
                            encoding: RelocationEncoding::Generic,
                            symbol: target_symbol,
                            addend: 0,
                        },
                    )?;
                }
                RelocationTarget::JumpTable(_, _) => {
                    // ignore relocations for jump tables
                }
                _ => panic!("relocations target not supported yet: {:?}", r.reloc_target),
            };
        }
    }

    Ok(())
}
