use anyhow::Result;
use host::{add_to_linker, WasiCtx};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
test_programs_macros::reactor_tests!();

wasmtime::component::bindgen!({
    path: "../test-programs/reactor-tests/wit",
    world: "test-reactor",
    async: true,
});

async fn instantiate(path: &str) -> Result<(Store<WasiCtx>, TestReactor)> {
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

    let (wasi, _instance) = TestReactor::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, wasi))
}

async fn run_reactor_tests(mut store: Store<WasiCtx>, reactor: TestReactor) -> Result<()> {
    store
        .data_mut()
        .env
        .push(("GOOD_DOG".to_owned(), "gussie".to_owned()));

    let r = reactor
        .call_add_strings(&mut store, &["hello", "$GOOD_DOG"])
        .await?;
    assert_eq!(r, 2);

    // Redefine the env, show that the adapter only fetches it once
    // even if the libc ctors copy it in multiple times:
    store.data_mut().env.clear();
    store
        .data_mut()
        .env
        .push(("GOOD_DOG".to_owned(), "cody".to_owned()));
    // Cody is indeed good but this should be "hello again" "gussie"
    let r = reactor
        .call_add_strings(&mut store, &["hello again", "$GOOD_DOG"])
        .await?;
    assert_eq!(r, 4);

    let contents = reactor.call_get_strings(&mut store).await?;
    assert_eq!(contents, &["hello", "gussie", "hello again", "gussie"]);
    Ok(())
}
