//! Translation of hello example

use anyhow::{ensure, format_err, Context as _, Result};
use std::cell::Ref;
use std::rc::Rc;
use wasmtime::*;

struct HelloCallback;

impl Callable for HelloCallback {
    fn call(&self, _params: &[Val], _results: &mut [Val]) -> Result<(), HostRef<Trap>> {
        println!("Calling back...");
        println!("> Hello World!");
        Ok(())
    }
}

fn main() -> Result<()> {
    // Configure the initial compilation environment, creating more global
    // structures such as an `Engine` and a `Store`.
    println!("Initializing...");
    let engine = HostRef::new(Engine::default());
    let store = HostRef::new(Store::new(&engine));

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
    let module = HostRef::new(Module::new(&store, &binary).context("> Error compiling module!")?);

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
    let instance = HostRef::new(
        Instance::new(&store, &module, imports.as_slice())
            .context("> Error instantiating module!")?,
    );

    // Next we poke around a bit to extract the `run` function from the module.
    println!("Extracting export...");
    let exports = Ref::map(instance.borrow(), |instance| instance.exports());
    ensure!(!exports.is_empty(), "> Error accessing exports!");
    let run_func = exports[0].func().context("> Error accessing exports!")?;

    // And last but not least we can call it!
    println!("Calling export...");
    run_func
        .borrow()
        .call(&[])
        .map_err(|e| format_err!("> Error calling function: {:?}", e))?;

    println!("Done.");
    Ok(())
}
