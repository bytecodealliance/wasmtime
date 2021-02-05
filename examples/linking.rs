//! Example of instantiating two modules which link to each other.

// You can execute this example with `cargo run --example linking`

use anyhow::Result;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::*;
use wasmtime_wasi::Wasi;

fn main() -> Result<()> {
    let engine = Engine::default();
    let store = Store::new(&engine);

    // First set up our linker which is going to be linking modules together. We
    // want our linker to have wasi available, so we set that up here as well.
    let mut linker = Linker::new(&store);
    let wasi = Wasi::new(
        &store,
        WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_args()?
            .build()?,
    );
    wasi.add_to_linker(&mut linker)?;

    // Load and compile our two modules
    let linking1 = Module::from_file(&engine, "examples/linking1.wat")?;
    let linking2 = Module::from_file(&engine, "examples/linking2.wat")?;

    // Instantiate our first module which only uses WASI, then register that
    // instance with the linker since the next linking will use it.
    let linking2 = linker.instantiate(&linking2)?;
    linker.instance("linking2", &linking2)?;

    // And with that we can perform the final link and the execute the module.
    let linking1 = linker.instantiate(&linking1)?;
    let run = linking1.get_func("run").unwrap();
    let run = run.get0::<()>()?;
    run()?;
    Ok(())
}
