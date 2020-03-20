//! Example of instantiating of instantiating a wasm module which uses WASI
//! imports.

// You can execute this example with `cargo run --example wasi`

use anyhow::Result;
use wasmtime::*;
use wasmtime_wasi::{Wasi, WasiCtx};

fn main() -> Result<()> {
    let store = Store::default();
    let module = Module::from_file(&store, "target/wasm32-wasi/debug/wasi.wasm")?;

    // Create an instance of `Wasi` which contains a `WasiCtx`. Note that
    // `WasiCtx` provides a number of ways to configure what the target program
    // will have access to.
    let wasi = Wasi::new(&store, WasiCtx::new(std::env::args())?);
    let mut imports = Vec::new();
    for import in module.imports() {
        if import.module() == "wasi_snapshot_preview1" {
            if let Some(export) = wasi.get_export(import.name()) {
                imports.push(Extern::from(export.clone()));
                continue;
            }
        }
        panic!(
            "couldn't find import for `{}::{}`",
            import.module(),
            import.name()
        );
    }

    // Instance our module with the imports we've created, then we can run the
    // standard wasi `_start` function.
    let instance = Instance::new(&module, &imports)?;
    let start = instance
        .get_export("_start")
        .and_then(|e| e.func())
        .unwrap();
    let start = start.get0::<()>()?;
    start()?;
    Ok(())
}
