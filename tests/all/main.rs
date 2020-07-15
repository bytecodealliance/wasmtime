mod cli_tests;
mod custom_signal_handler;
mod debug;
mod externals;
mod func;
mod funcref;
mod fuzzing;
mod gc;
mod globals;
mod iloop;
mod import_calling_export;
mod import_indexes;
mod instance;
mod invoke_func_via_table;
mod linker;
mod memory_creator;
mod name;
mod stack_overflow;
mod table;
mod traps;
mod use_after_drop;
mod wast;

pub(crate) fn ref_types_module(
    source: &str,
) -> anyhow::Result<(wasmtime::Store, wasmtime::Module)> {
    use wasmtime::*;

    let _ = env_logger::try_init();

    let mut config = Config::new();
    config.wasm_reference_types(true);

    let engine = Engine::new(&config);
    let store = Store::new(&engine);

    let module = Module::new(&engine, source)?;

    Ok((store, module))
}
