use cretonne::ir;
use cretonne::settings;
use cretonne::settings::Configurable;
use faerie::Artifact;
use wasmstandalone_runtime;

/// Emits a module that has been emitted with the `WasmRuntime` runtime
/// implementation to a native object file.
pub fn emit_module<'module>(
    obj: &mut Artifact,
    compilation: &wasmstandalone_runtime::Compilation<'module>,
    relocations: &wasmstandalone_runtime::Relocations,
) -> Result<(), String> {
    debug_assert!(
        compilation.module.start_func.is_none() ||
            compilation.module.start_func.unwrap() >= compilation.module.imported_funcs.len(),
        "imported start functions not supported yet"
    );

    let mut shared_builder = settings::builder();
    shared_builder.enable("enable_verifier").expect(
        "Missing enable_verifier setting",
    );

    for (i, function_relocs) in relocations.iter().enumerate() {
        assert!(function_relocs.is_empty(), "relocations not supported yet");
        let body = &compilation.functions[i];
        let external_name =
            wasmstandalone_runtime::get_func_name(compilation.module.imported_funcs.len() + i);
        let func_index = match external_name {
            ir::ExternalName::User { namespace, index } => {
                debug_assert!(namespace == 0);
                index
            }
            _ => panic!(),
        };

        let string_name = format!("wasm_function[{}]", func_index);

        obj.define(string_name, body.clone()).map_err(|err| {
            format!("{}", err)
        })?;
    }

    Ok(())
}
