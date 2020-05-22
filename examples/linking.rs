//! Example of instantiating two modules which link to each other.

// You can execute this example with `cargo run --example linking`

use anyhow::Result;
use wasmtime::*;
use wasmtime_wasi::{Wasi, WasiCtx};

fn main() -> Result<()> {
    let store = Store::default();

    // First set up our linker which is going to be linking modules together. We
    // want our linker to have wasi available, so we set that up here as well.
    let mut linker = Linker::new(&store);
    let wasi = Wasi::new(&store, WasiCtx::new(std::env::args())?);
    wasi.add_to_linker(&mut linker)?;

    // Load and compile our two modules
    let linking1 = Module::from_file(&store, "examples/linking1.wat")?;
    let linking2 = Module::from_file(&store, "examples/linking2.wat")?;

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
