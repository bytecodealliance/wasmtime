//! Example of instantiating a wasm module which uses WASI preview1 imports
//! implemented through the async preview2 WASI implementation.

/*
You can execute this example with:
    cmake examples/
    cargo run --example wasip1-async
*/

use anyhow::Result;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtx;
use wasmtime_wasi::p1::{self, WasiP1Ctx};

#[tokio::main]
async fn main() -> Result<()> {
    // Construct the wasm engine with async support enabled.
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;

    // Add the WASIp1 APIs to the linker
    let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
    p1::add_to_linker_async(&mut linker, |t| t)?;

    // Add capabilities (e.g. filesystem access) to the WASI preview2 context
    // here. Here only stdio is inherited, but see docs of `WasiCtx` for
    // more.
    let wasi_ctx = WasiCtx::builder().inherit_stdio().build_p1();

    let mut store = Store::new(&engine, wasi_ctx);

    // Instantiate our 'Hello World' wasm module.
    // Note: This is a module built against the preview1 WASI API.
    let module = Module::from_file(&engine, "target/wasm32-wasip1/debug/wasi.wasm")?;
    let func = linker
        .module_async(&mut store, "", &module)
        .await?
        .get_default(&mut store, "")?
        .typed::<(), ()>(&store)?;

    // Invoke the WASI program default function.
    func.call_async(&mut store, ()).await?;

    Ok(())
}
