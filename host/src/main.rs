use anyhow::{Context, Result};
use host::{add_to_linker, Wasi};
use std::path::PathBuf;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
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

    let mut store = Store::new(&engine, WasiCtxBuilder::new().build());

    let (wasi, _instance) = Wasi::instantiate_async(&mut store, &component, &linker).await?;

    let result: Result<(), ()> = wasi.command(&mut store, 0, 0, &[], &[], &[]).await?;

    if result.is_err() {
        anyhow::bail!("command returned with failing exit status");
    }

    Ok(())
}
