use anyhow::Result;
use host::{add_to_linker, Wasi, WasiCtx};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

test_programs_macros::tests!();

fn run(path: &str) -> Result<()> {
    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &path)?;
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker, |x| x)?;

    let mut store = Store::new(&engine, WasiCtx::default());

    let (wasi, _instance) = Wasi::instantiate(&mut store, &component, &linker)?;

    wasi.command(&mut store)?;

    Ok(())
}
