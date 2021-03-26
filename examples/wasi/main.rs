//! Example of instantiating of instantiating a wasm module which uses WASI
//! imports.

// You can execute this example with `cargo run --example wasi`

use anyhow::Result;
use wasmtime::*;
use wasmtime_wasi::{sync::WasiCtxBuilder, Wasi};

fn main() -> Result<()> {
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    // Define the WASI functions globally on the `Config`.
    let mut config = Config::default();
    Wasi::add_to_config(&mut config);

    let store = Store::new(&Engine::new(&config)?);

    // Set the WASI context in the store; all instances in the store share this context.
    // `WasiCtxBuilder` provides a number of ways to configure what the target program
    // will have access to.
    assert!(Wasi::set_context(
        &store,
        WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_args()?
            .build()?
    )
    .is_ok());

    let mut linker = Linker::new(&store);

    // Instantiate our module with the imports we've created, and run it.
    let module = Module::from_file(store.engine(), "target/wasm32-wasi/debug/wasi.wasm")?;
    linker.module("", &module)?;
    linker.get_default("")?.typed::<(), ()>()?.call(())?;

    Ok(())
}
