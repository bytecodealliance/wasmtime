use anyhow::{Context, Result};
use host::{add_to_linker, Wasi, WasiCtx};
use std::path::PathBuf;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wit_component::ComponentEncoder;

fn main() -> Result<()> {
    let input = PathBuf::from(
        std::env::args()
            .collect::<Vec<String>>()
            .get(1)
            .context("must provide an input file")?,
    );
    let input = std::fs::read(input).context("reading input")?;

    let adapter =
        PathBuf::from(env!("OUT_DIR")).join("wasm32-wasi/release/wasi_snapshot_preview1.wasm");

    let component = ComponentEncoder::default()
        .module(input.as_slice())
        .context("pull any custom sections from module")?
        .validate(true)
        .adapter_file(&adapter)
        .context("wasi adapter")?
        .encode()
        .context("encoding module to component")?;

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_binary(&engine, &component)?;
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker, |x| x)?;

    let mut store = Store::new(&engine, WasiCtx::default());

    let (wasi, _instance) = Wasi::instantiate(&mut store, &component, &linker)?;

    wasi.command(&mut store)?;

    Ok(())
}
