//! Running pre-compiled Wasm programs.

use wasmtime::{Config, Engine, Instance, Module, Result, Store};

fn main() -> Result<()> {
    // Create the default configuration for this host platform. Note that this
    // configuration must match the configuration used to pre-compile the Wasm
    // program. We cannot run Wasm programs pre-compiled for configurations that
    // do not match our own, therefore if you enabled or disabled any particular
    // Wasm proposals or tweaked memory knobs when pre-compiling, you should
    // make identical adjustments to this config.
    let config = Config::default();

    // Create an `Engine` with that configuration.
    let engine = Engine::new(&config)?;

    // Create a runtime `Module` from a Wasm program that was pre-compiled and
    // written to the `add.cwasm` file by `wasmtime/examples/pre_compile.rs`.
    //
    // **Warning:** Wasmtime does not (and in general cannot) fully validate
    // pre-compiled modules for safety -- only create `Module`s and `Component`s
    // from pre-compiled bytes you control and trust! Passing unknown or
    // untrusted bytes will lead to arbitrary code execution vulnerabilities in
    // your system!
    let module = match unsafe { Module::deserialize_file(&engine, "add.cwasm") } {
        Ok(module) => module,
        Err(error) => {
            println!("failed to deserialize pre-compiled module: {error:?}");
            return Ok(());
        }
    };

    // Instantiate the module and invoke its `add` function!
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let add = instance.get_typed_func::<(i32, i32), i32>(&mut store, "add")?;
    let sum = add.call(&mut store, (3, 8))?;
    println!("the sum of 3 and 8 is {sum}");

    Ok(())
}
