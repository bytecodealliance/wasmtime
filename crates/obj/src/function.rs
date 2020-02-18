use anyhow::Result;
use faerie::{Artifact, Decl, Link};
use wasmtime_environ::entity::EntityRef;
use wasmtime_environ::settings;
use wasmtime_environ::settings::Configurable;
use wasmtime_environ::{Compilation, Module, RelocationTarget, Relocations};

/// Defines module functions
pub fn declare_functions(
    obj: &mut Artifact,
    module: &Module,
    relocations: &Relocations,
) -> Result<()> {
    for i in 0..module.imported_funcs.len() {
        let string_name = format!("_wasm_function_{}", i);
        obj.declare(string_name, Decl::function_import())?;
    }
    for (i, _function_relocs) in relocations.iter().rev() {
        let func_index = module.func_index(i);
        let string_name = format!("_wasm_function_{}", func_index.index());
        obj.declare(string_name, Decl::function().global())?;
    }
    Ok(())
}

/// Emits module functions
pub fn emit_functions(
    obj: &mut Artifact,
    module: &Module,
    compilation: &Compilation,
    relocations: &Relocations,
) -> Result<()> {
    debug_assert!(
        module.start_func.is_none()
            || module.start_func.unwrap().index() >= module.imported_funcs.len(),
        "imported start functions not supported yet"
    );

    let mut shared_builder = settings::builder();
    shared_builder
        .enable("enable_verifier")
        .expect("Missing enable_verifier setting");

    for (i, _function_relocs) in relocations.iter() {
        let body = &compilation.get(i).body;
        let func_index = module.func_index(i);
        let string_name = format!("_wasm_function_{}", func_index.index());

        obj.define(string_name, body.clone())?;
    }

    for (i, function_relocs) in relocations.iter() {
        let func_index = module.func_index(i);
        let string_name = format!("_wasm_function_{}", func_index.index());
        for r in function_relocs {
            debug_assert_eq!(r.addend, 0);
            match r.reloc_target {
                RelocationTarget::UserFunc(target_index) => {
                    let target_name = format!("_wasm_function_{}", target_index.index());
                    obj.link(Link {
                        from: &string_name,
                        to: &target_name,
                        at: r.offset as u64,
                    })?;
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
