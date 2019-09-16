use cranelift_codegen::settings;
use std::path::Path;
use wasmtime_jit::Context;

pub fn instantiate(data: &[u8], bin_name: &str, workspace: Option<&Path>) -> Result<(), String> {
    // Prepare runtime
    let isa_builder =
        cranelift_native::builder().map_err(|_| "host machine is not a supported target")?;
    let flag_builder = settings::builder();
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut context = Context::with_isa(isa);
    let global_exports = context.get_global_exports();

    let get_preopens = |workspace: Option<&Path>| -> Result<Vec<_>, String> {
        if let Some(workspace) = workspace {
            let preopen_dir = wasi_common::preopen_dir(workspace).map_err(|e| {
                format!(
                    "error while preopening directory '{}': {}",
                    workspace.display(),
                    e
                )
            })?;

            Ok(vec![(".".to_owned(), preopen_dir)])
        } else {
            Ok(vec![])
        }
    };

    context.name_instance(
        "wasi_unstable".to_owned(),
        wasmtime_wasi::instantiate_wasi(
            "",
            global_exports,
            &get_preopens(workspace)?,
            &[bin_name.to_owned(), ".".to_owned()],
            &[],
        )
        .map_err(|e| format!("error instantiating WASI: {}", e))?,
    );

    // Compile and instantiating a wasm module.
    context
        .instantiate_module(None, &data)
        .map(|_| ())
        .map_err(|e| format!("error while processing main module '{}': {}", bin_name, e))
}
