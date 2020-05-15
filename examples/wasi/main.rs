//! Example of instantiating of instantiating a wasm module which uses WASI
//! imports.

// You can execute this example with `cargo run --example wasi`

use anyhow::Result;
use wasmtime::*;
use wasmtime_wasi::wasi_linker;

fn main() -> Result<()> {
    let store = Store::default();
    let module = Module::from_file(&store, "target/wasm32-wasi/debug/wasi.wasm")?;

    // Create a new `Linker` with no preloaded directories, command-line arguments,
    // or environment variables.
    let linker = wasi_linker(&store, &[], &[], &[])?;

    // Instantiate and run our module with the imports we've created.
    let _instance = linker.instantiate_wasi_abi(&module)?;

    Ok(())
}
