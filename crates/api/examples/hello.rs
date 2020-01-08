//! Translation of hello example

use anyhow::{ensure, Context as _, Result};
use std::rc::Rc;
use wasmtime::*;

struct HelloCallback;

impl Callable for HelloCallback {
    fn call(&self, _params: &[Val], _results: &mut [Val]) -> Result<(), Trap> {
        println!("Calling back...");
        println!("> Hello World!");
        Ok(())
    }
}

fn main() -> Result<()> {
    // Configure the initial compilation environment, creating the global
    // `Store` structure. Note that you can also tweak configuration settings
    // with a `Config` and an `Engine` if desired.
    println!("Initializing...");
    let store = Store::default();

    // Next upload the `*.wasm` binary file, which in this case we're going to
    // be parsing an inline text format into a binary.
    println!("Loading binary...");
    let binary = wat::parse_str(
        r#"
            (module
              (func $hello (import "" "hello"))
              (func (export "run") (call $hello))
            )
        "#,
    )?;

    // Compiler the `*.wasm` binary into an in-memory instance of a `Module`.
    println!("Compiling module...");
    let module = Module::new(&store, &binary).context("> Error compiling module!")?;

    // Here we handle the imports of the module, which in this case is our
    // `HelloCallback` type and its associated implementation of `Callback.
    println!("Creating callback...");
    let hello_type = FuncType::new(Box::new([]), Box::new([]));
    let hello_func = HostRef::new(Func::new(&store, hello_type, Rc::new(HelloCallback)));

    // Once we've got that all set up we can then move to the instantiation
    // phase, pairing together a compiled module as well as a set of imports.
    // Note that this is where the wasm `start` function, if any, would run.
    println!("Instantiating module...");
    let imports = vec![hello_func.into()];
    let instance = Instance::new(&store, &module, imports.as_slice())
        .context("> Error instantiating module!")?;

    // Next we poke around a bit to extract the `run` function from the module.
    println!("Extracting export...");
    let exports = instance.exports();
    ensure!(!exports.is_empty(), "> Error accessing exports!");
    let run_func = exports[0].func().context("> Error accessing exports!")?;

    // And last but not least we can call it!
    println!("Calling export...");
    run_func.borrow().call(&[])?;

    println!("Done.");
    Ok(())
}
