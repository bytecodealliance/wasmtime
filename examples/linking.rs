//! Example of instantiating two modules which link to each other.

// You can execute this example with `cargo run --example linking`

use anyhow::Result;
use wasmtime::*;
use wasmtime_wasi::{Wasi, WasiCtx};

fn main() -> Result<()> {
    let store = Store::default();

    // Load and compile our two modules
    let linking1 = Module::from_file(&store, "examples/linking1.wat")?;
    let linking2 = Module::from_file(&store, "examples/linking2.wat")?;

    // Instantiate the first, `linking2`, which uses WASI imports
    let wasi = Wasi::new(&store, WasiCtx::new(std::env::args())?);
    let mut imports = Vec::new();
    for import in linking2.imports() {
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
    let linking2 = Instance::new(&linking2, &imports)?;

    // And using the previous instance we can create the imports for `linking1`,
    // using the previous exports.
    let mut imports = Vec::new();
    for import in linking1.imports() {
        if import.module() == "linking2" {
            if let Some(export) = linking2.get_export(import.name()) {
                imports.push(export.clone());
                continue;
            }
        }
        panic!(
            "couldn't find import for `{}::{}`",
            import.module(),
            import.name()
        );
    }
    let linking1 = Instance::new(&linking1, &imports)?;

    // And once everything is instantiated we can run!
    let run = linking1.get_export("run").and_then(|e| e.func()).unwrap();
    let run = run.get0::<()>()?;
    run()?;
    Ok(())
}
