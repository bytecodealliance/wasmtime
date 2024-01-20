//! Example of instantiating a wasm module which uses WASI preview1 imports
//! implemented through the async preview2 WASI implementation.

/*
You can execute this example with:
    cmake examples/
    cargo run --example wasi-async
*/

use anyhow::Result;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::preview2;

struct WasiHostCtx {
    preview2_ctx: preview2::WasiCtx,
    preview2_table: wasmtime::component::ResourceTable,
    preview1_adapter: preview2::preview1::WasiPreview1Adapter,
}

impl preview2::WasiView for WasiHostCtx {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.preview2_table
    }

    fn ctx(&mut self) -> &mut preview2::WasiCtx {
        &mut self.preview2_ctx
    }
}

impl preview2::preview1::WasiPreview1View for WasiHostCtx {
    fn adapter(&self) -> &preview2::preview1::WasiPreview1Adapter {
        &self.preview1_adapter
    }

    fn adapter_mut(&mut self) -> &mut preview2::preview1::WasiPreview1Adapter {
        &mut self.preview1_adapter
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Construct the wasm engine with async support enabled.
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;

    // Add the WASI preview1 API to the linker (will be implemented in terms of
    // the preview2 API)
    let mut linker: Linker<WasiHostCtx> = Linker::new(&engine);
    preview2::preview1::add_to_linker_async(&mut linker)?;

    // Add capabilities (e.g. filesystem access) to the WASI preview2 context here.
    let wasi_ctx = preview2::WasiCtxBuilder::new().inherit_stdio().build();

    let host_ctx = WasiHostCtx {
        preview2_ctx: wasi_ctx,
        preview2_table: preview2::ResourceTable::new(),
        preview1_adapter: preview2::preview1::WasiPreview1Adapter::new(),
    };
    let mut store: Store<WasiHostCtx> = Store::new(&engine, host_ctx);

    // Instantiate our 'Hello World' wasm module.
    // Note: This is a module built against the preview1 WASI API.
    let module = Module::from_file(&engine, "target/wasm32-wasi/debug/wasi.wasm")?;
    let func = linker
        .module_async(&mut store, "", &module)
        .await?
        .get_default(&mut store, "")?
        .typed::<(), ()>(&store)?;

    // Invoke the WASI program default function.
    func.call_async(&mut store, ()).await?;

    Ok(())
}
