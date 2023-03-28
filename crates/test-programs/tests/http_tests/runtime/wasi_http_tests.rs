use anyhow::Context;
use std::path::Path;
use wasi_http::WasiHttp;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::{sync::WasiCtxBuilder, WasiCtx};

pub fn instantiate_inherit_stdio(
    data: &[u8],
    bin_name: &str,
    workspace: Option<&Path>,
) -> anyhow::Result<()> {
    let mut config = Config::new();
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &data).context("failed to create wasm module")?;
    let mut linker = Linker::new(&engine);

    struct Ctx {
        wasi: WasiCtx,
        http: WasiHttp,
    }

    wasmtime_wasi::sync::add_to_linker(&mut linker, |cx: &mut Ctx| &mut cx.wasi)?;
    wasi_http::add_to_linker(&mut linker, |cx: &mut Ctx| &mut cx.http)?;

    // Create our wasi context.
    let mut builder = WasiCtxBuilder::new().inherit_stdio().arg(bin_name)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: builder.build(),
            http: WasiHttp::new(),
        },
    );

    let instance = linker.instantiate(&mut store, &module)?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call(&mut store, ())
}
