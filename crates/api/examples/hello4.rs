//! Translation of hello example

use anyhow::{format_err, Result};
use std::fs::read;
use wasmtime_api::*;

#[macro_use]
extern crate wasmtime_bindings_macro;

struct Callback;

#[wasmtime_impl(module(callback_mod))]
impl Callback {
    fn callback(&self) {
        println!("Calling back...");
        println!("> Hello World!");
    }
}

#[wasmtime_trait(module(hello_mod))]
trait Hello {
    fn run(&self);
}

fn main() -> Result<()> {
    // Initialize.
    println!("Initializing...");
    let engine = HostRef::new(Engine::default());
    let store = HostRef::new(Store::new(&engine));

    // Load binary.
    println!("Loading binary...");
    let binary = read("examples/hello.wasm")?;

    // Compile.
    println!("Compiling module...");
    let module = HostRef::new(
        Module::new(&store, &binary).map_err(|_| format_err!("> Error compiling module!"))?,
    );

    // Create external print functions.
    println!("Creating callback...");
    let callback_mod = HostRef::new(
        wrap_wasmtime_module!(
            &store, |_imports| Callback; module(callback_mod)
        )
        .map_err(|_| format_err!("> Error compiling callback module!"))?,
    );
    let callback_instance = Instance::new(&store, &callback_mod, &[])
        .map_err(|_| format_err!("> Error instantiating callback module!"))?;
    let hello_func = &callback_instance.exports()[0];

    // Instantiate.
    println!("Instantiating module...");
    let imports = vec![hello_func.clone()];
    let instance = HostRef::new(
        Instance::new(&store, &module, imports.as_slice())
            .map_err(|_| format_err!("> Error instantiating module!"))?,
    );

    // Extract export.
    println!("Extracting export...");
    let hello = map_to_wasmtime_trait!(&instance; module(hello_mod));

    // Call.
    println!("Calling export...");
    hello.run();

    // Shut down.
    println!("Shutting down...");
    drop(store);

    // All done.
    println!("Done.");
    Ok(())
}
