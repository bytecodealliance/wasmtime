use anyhow::{Context, Result};
use host::{add_to_linker, Wasi, WasiCtx};
use std::path::PathBuf;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

fn main() -> Result<()> {
    let input = PathBuf::from(
        std::env::args()
            .collect::<Vec<String>>()
            .get(1)
            .context("must provide an input file")?,
    );

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &input)?;
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker, |x| x)?;

    let mut store = Store::new(&engine, WasiCtx::default());

    let (wasi, _instance) = Wasi::instantiate(&mut store, &component, &linker)?;

    wasi.command(&mut store, 0, 0, Vec::new())?;

    Ok(())
}
