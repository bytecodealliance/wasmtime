//! Example of instantiating two modules which link to each other.

// You can execute this example with `cargo run --example linking`

use anyhow::Result;
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

fn main() -> Result<()> {
    let engine = Engine::default();

    // First set up our linker which is going to be linking modules together. We
    // want our linker to have wasi available, so we set that up here as well.
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

    // Load and compile our two modules
    let linking1 = Module::from_file(&engine, "examples/linking1.wat")?;
    let linking2 = Module::from_file(&engine, "examples/linking2.wat")?;

    // Configure WASI and insert it into a `Store`
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .build();
    let mut store = Store::new(&engine, wasi);

    // Instantiate our first module which only uses WASI, then register that
    // instance with the linker since the next linking will use it.
    let linking2 = linker.instantiate(&mut store, &linking2)?;
    linker.instance(&mut store, "linking2", linking2)?;

    // And with that we can perform the final link and the execute the module.
    let linking1 = linker.instantiate(&mut store, &linking1)?;
    let run = linking1.get_typed_func::<(), (), _>(&mut store, "run")?;
    run.call(&mut store, ())?;
    Ok(())
}
