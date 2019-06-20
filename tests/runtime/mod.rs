mod utils;
mod wasi;

use cranelift_codegen::settings;
use cranelift_native;
use std::path::Path;
use wasmtime_jit::Context;

pub fn run_test<P: AsRef<Path>>(path: P) -> Result<(), String> {
    // Load in the wasm testcase
    let data = utils::read_wasm(path.as_ref())?;
    let bin_name = utils::extract_exec_name_from_path(path.as_ref())?;

    // Prepare workspace
    let workspace = utils::prepare_workspace(&bin_name)?;

    // Prepare runtime
    let isa_builder =
        cranelift_native::builder().map_err(|_| "host machine is not a supported target")?;
    let flag_builder = settings::builder();
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut context = Context::with_isa(isa);
    let global_exports = context.get_global_exports();
    let preopen_dir = wasi_common::preopen_dir(&workspace)
        .map_err(|e| format!("error while preopening directory '{}': {}", workspace, e))?;

    context.name_instance(
        "wasi_unstable".to_owned(),
        wasi::instantiate_wasi(
            "",
            global_exports,
            &[(".".to_owned(), preopen_dir)],
            &[bin_name.clone(), ".".to_owned()],
            &[],
        )
        .expect("instantiating wasi"),
    );

    // Compile and instantiating a wasm module.
    context
        .instantiate_module(None, &data)
        .map(|_| ())
        .map_err(|e| format!("error while processing main module '{}': {}", bin_name, e))
}
