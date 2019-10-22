use cranelift_codegen::settings::{self, Configurable};
use std::{collections::HashMap, path::Path};
use wasmtime_api::{Config, Engine, HostRef, Instance, Module, Store};
use wasmtime_jit::{CompilationStrategy, Features};

pub fn instantiate(data: &[u8], bin_name: &str, workspace: Option<&Path>) -> Result<(), String> {
    // Prepare runtime
    let mut flag_builder = settings::builder();

    // Enable proper trap for division
    flag_builder
        .enable("avoid_div_traps")
        .map_err(|err| format!("error while enabling proper division trap: {}", err))?;

    let config = Config::new(
        settings::Flags::new(flag_builder),
        Features::default(),
        false,
        CompilationStrategy::Auto,
    );
    let engine = HostRef::new(Engine::new(config));
    let store = HostRef::new(Store::new(engine));

    let mut module_registry = HashMap::new();
    let global_exports = store.borrow().global_exports().clone();
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
    module_registry.insert(
        "wasi_unstable".to_owned(),
        Instance::from_handle(
            store.clone(),
            wasmtime_wasi::instantiate_wasi(
                "",
                global_exports.clone(),
                &get_preopens(workspace)?,
                &[bin_name.to_owned(), ".".to_owned()],
                &[],
            )
            .map_err(|e| format!("error instantiating WASI: {}", e))?,
        )
        .map_err(|err| format!("error instantiating from handle: {}", err))?,
    );

    let module = HostRef::new(
        Module::new(store.clone(), &data)
            .map_err(|err| format!("error while creating Wasm module '{}': {}", bin_name, err))?,
    );
    let imports = module
        .borrow()
        .imports()
        .iter()
        .map(|i| {
            let module_name = i.module().to_string();
            if let Some((instance, map)) = module_registry.get(&module_name) {
                let field_name = i.name().to_string();
                if let Some(export_index) = map.get(&field_name) {
                    Ok(instance.exports()[*export_index].clone())
                } else {
                    Err(format!(
                        "import {} was not found in module {}",
                        field_name, module_name
                    ))
                }
            } else {
                Err(format!("import module {} was not found", module_name))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let _ = HostRef::new(
        Instance::new(store.clone(), module.clone(), &imports).map_err(|err| {
            format!(
                "error while instantiating Wasm module '{}': {}",
                bin_name, err
            )
        })?,
    );

    Ok(())
}
