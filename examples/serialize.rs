//! Small example of how to serialize compiled wasm module to the disk,
//! and then instantiate it from the compilation artifacts.

// You can execute this example with `cargo run --example serialize`

use anyhow::Result;
use wasmtime::*;

fn serialize() -> Result<Vec<u8>> {
    // Configure the initial compilation environment, creating the global
    // `Store` structure. Note that you can also tweak configuration settings
    // with a `Config` and an `Engine` if desired.
    println!("Initializing...");
    let engine = Engine::default();

    // Compile the wasm binary into an in-memory instance of a `Module`.
    println!("Compiling module...");
    let module = Module::from_file(&engine, "examples/hello.wat")?;
    let serialized = module.serialize()?;

    println!("Serialized.");
    Ok(serialized)
}

fn deserialize(buffer: &[u8]) -> Result<()> {
    // Configure the initial compilation environment, creating the global
    // `Store` structure. Note that you can also tweak configuration settings
    // with a `Config` and an `Engine` if desired.
    println!("Initializing...");
    let store = Store::default();

    // Compile the wasm binary into an in-memory instance of a `Module`.
    println!("Deserialize module...");
    let module = Module::new(store.engine(), buffer)?;

    // Here we handle the imports of the module, which in this case is our
    // `HelloCallback` type and its associated implementation of `Callback.
    println!("Creating callback...");
    let hello_func = Func::wrap(&store, || {
        println!("Calling back...");
        println!("> Hello World!");
    });

    // Once we've got that all set up we can then move to the instantiation
    // phase, pairing together a compiled module as well as a set of imports.
    // Note that this is where the wasm `start` function, if any, would run.
    println!("Instantiating module...");
    let imports = [hello_func.into()];
    let instance = Instance::new(&store, &module, &imports)?;

    // Next we poke around a bit to extract the `run` function from the module.
    println!("Extracting export...");
    let run = instance.get_typed_func::<(), ()>("run")?;

    // And last but not least we can call it!
    println!("Calling export...");
    run.call(())?;

    println!("Done.");
    Ok(())
}

fn main() -> Result<()> {
    let file = serialize()?;
    deserialize(&file)
}
