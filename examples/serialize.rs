//! Small example of how to instantiate a wasm module that imports one function,
//! showing how you can fill in host functionality for a wasm module.

// You can execute this example with `cargo run --example hello`

use anyhow::Result;
use std::fs::{self, File};
use wasmtime::*;

fn main() -> Result<()> {
    // Configure the initial compilation environment, creating the global
    // `Store` structure. Note that you can also tweak configuration settings
    // with a `Config` and an `Engine` if desired.
    println!("Initializing...");
    let engine = Engine::default();

    // Compile the wasm binary into an in-memory instance of a `Module`.
    println!("Compiling module...");
    let wasm = fs::read("examples/hello.wat")?;
    let yaml = File::create("test.yaml")?;
    Module::compile_and_serialize(&engine, wasm, yaml)?;

    println!("Done");
    Ok(())
}
