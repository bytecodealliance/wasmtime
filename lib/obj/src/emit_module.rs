use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_entity::EntityRef;
use faerie::Artifact;
use wasmtime_environ::{Compilation, Module, Relocations};

/// Emits a module that has been emitted with the `wasmtime-environ` environment
/// implementation to a native object file.
pub fn emit_module(
    obj: &mut Artifact,
    module: &Module,
    compilation: &Compilation,
    relocations: &Relocations,
) -> Result<(), String> {
    debug_assert!(
        module.start_func.is_none()
            || module.start_func.unwrap().index() >= module.imported_funcs.len(),
        "imported start functions not supported yet"
    );

    let mut shared_builder = settings::builder();
    shared_builder
        .enable("enable_verifier")
        .expect("Missing enable_verifier setting");

    for (i, function_relocs) in relocations.iter() {
        assert!(function_relocs.is_empty(), "relocations not supported yet");
        let body = &compilation.functions[i];
        let func_index = module.func_index(i);
        let string_name = format!("wasm_function[{}]", func_index.index());

        obj.define(string_name, body.clone())
            .map_err(|err| format!("{}", err))?;
    }

    Ok(())
}
