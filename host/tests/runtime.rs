use anyhow::Result;
use host::{add_to_linker, Wasi, WasiCtx};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

test_programs_macros::tests!();

async fn instantiate(path: &str) -> Result<(Store<WasiCtx>, Wasi)> {
    println!("{}", path);

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &path)?;
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker, |x| x)?;

    let mut store = Store::new(&engine, WasiCtxBuilder::new().build());

    let (wasi, _instance) = Wasi::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, wasi))
}

async fn run_hello_stdout(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    wasi.command(
        &mut store,
        0 as host::Descriptor,
        1 as host::Descriptor,
        &["gussie", "sparky", "willa"],
    )
    .await?;
    Ok(())
}

async fn run_panic(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    let r = wasi
        .command(
            &mut store,
            0 as host::Descriptor,
            1 as host::Descriptor,
            &[
                "diesel",
                "the",
                "cat",
                "scratched",
                "me",
                "real",
                "good",
                "yesterday",
            ],
        )
        .await;
    assert!(r.is_err());
    println!("{:?}", r);
    Ok(())
}

async fn run_args(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    wasi.command(
        &mut store,
        0 as host::Descriptor,
        1 as host::Descriptor,
        &["hello", "this", "", "is an argument", "with 🚩 emoji"],
    )
    .await?;
    Ok(())
}
