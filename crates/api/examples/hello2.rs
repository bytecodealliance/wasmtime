//! Translation of hello example

use anyhow::{bail, format_err, Result};
use std::cell::Ref;
use std::fs::read;
use wasmtime_api::*;

#[macro_use]
extern crate wasmtime_bindings_macro;

use wasmtime_bindings::*;

#[wasmtime_method(module(callback_mod))]
fn callback() {
    println!("Calling back...");
    println!("> Hello World!");
}

#[wasmtime_method(module(hello_mod))]
fn hello() {
    unimplemented!();
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
    let hello_func = HostRef::new(wrap_wasmtime_func!(&store; module(callback_mod)));

    // Instantiate.
    println!("Instantiating module...");
    let imports = vec![hello_func.into()];
    let instance = HostRef::new(
        Instance::new(&store, &module, imports.as_slice())
            .map_err(|_| format_err!("> Error instantiating module!"))?,
    );

    // Extract export.
    println!("Extracting export...");
    let exports = Ref::map(instance.borrow(), |instance| instance.exports());
    if exports.len() == 0 {
        bail!("> Error accessing exports!");
    }
    let run_func = exports[0]
        .func()
        .ok_or_else(|| format_err!("> Error accessing exports!"))?;

    // Call.
    println!("Calling export...");
    let f = get_wasmtime_func!(run_func; module(hello_mod));
    f.call();

    // Shut down.
    println!("Shutting down...");
    drop(store);

    // All done.
    println!("Done.");
    Ok(())
}
