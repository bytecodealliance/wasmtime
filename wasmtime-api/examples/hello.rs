//! Translation of hello example

extern crate alloc;

use alloc::rc::Rc;
use anyhow::{ensure, format_err, Context as _, Result};
use core::cell::Ref;
use std::fs::read;
use wasmtime_api::*;

struct HelloCallback;

impl Callable for HelloCallback {
    fn call(&self, _params: &[Val], _results: &mut [Val]) -> Result<(), HostRef<Trap>> {
        println!("Calling back...");
        println!("> Hello World!");
        Ok(())
    }
}

fn main() -> Result<()> {
    // Initialize.
    println!("Initializing...");
    let engine = HostRef::new(Engine::new(Config::default()));
    let store = HostRef::new(Store::new(engine));

    // Load binary.
    println!("Loading binary...");
    let binary = read("examples/hello.wasm")?;

    // Compile.
    println!("Compiling module...");
    let module =
        HostRef::new(Module::new(store.clone(), &binary).context("> Error compiling module!")?);

    // Create external print functions.
    println!("Creating callback...");
    let hello_type = FuncType::new(Box::new([]), Box::new([]));
    let hello_func = HostRef::new(Func::new(store.clone(), hello_type, Rc::new(HelloCallback)));

    // Instantiate.
    println!("Instantiating module...");
    let imports = vec![hello_func.into()];
    let instance = HostRef::new(
        Instance::new(store.clone(), module, imports.as_slice())
            .context("> Error instantiating module!")?,
    );

    // Extract export.
    println!("Extracting export...");
    let exports = Ref::map(instance.borrow(), |instance| instance.exports());
    ensure!(!exports.is_empty(), "> Error accessing exports!");
    let run_func = exports[0].func().context("> Error accessing exports!")?;

    // Call.
    println!("Calling export...");
    run_func
        .borrow()
        .call(&[])
        .map_err(|e| format_err!("> Error calling function: {:?}", e))?;

    // Shut down.
    println!("Shutting down...");
    drop(store);

    // All done.
    println!("Done.");
    Ok(())
}
