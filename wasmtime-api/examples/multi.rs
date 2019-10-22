//! Translation of multi example

extern crate alloc;

use alloc::rc::Rc;
use core::cell::Ref;
use failure::{bail, format_err, Error};
use std::fs::read;
use wasmtime_api::*;

struct Callback;

impl Callable for Callback {
    fn call(&self, args: &[Val], results: &mut [Val]) -> Result<(), HostRef<Trap>> {
        println!("Calling back...");
        println!("> {} {}", args[0].i32(), args[1].i64());

        results[0] = Val::I64(args[1].i64() + 1);
        results[1] = Val::I32(args[0].i32() + 1);
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    // Initialize.
    println!("Initializing...");
    let engine = HostRef::new(Engine::new(Config::default()));
    let store = HostRef::new(Store::new(engine));

    // Load binary.
    println!("Loading binary...");
    let binary = read("examples/multi.wasm")?;

    // Compile.
    println!("Compiling module...");
    let module = HostRef::new(
        Module::new(store.clone(), &binary)
            .map_err(|_| format_err!("> Error compiling module!"))?,
    );

    // Create external print functions.
    println!("Creating callback...");
    let callback_type = FuncType::new(
        Box::new([ValType::I32, ValType::I64]),
        Box::new([ValType::I64, ValType::I32]),
    );
    let callback_func = HostRef::new(Func::new(store.clone(), callback_type, Rc::new(Callback)));

    // Instantiate.
    println!("Instantiating module...");
    let imports = vec![callback_func.into()];
    let instance = HostRef::new(
        Instance::new(store.clone(), module, imports.as_slice())
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
    let args = vec![Val::I32(1), Val::I64(3)];
    let results = run_func.borrow().call(&args);
    if let Err(_) = results {
        bail!("> Error calling function!");
    }

    let results = results.unwrap();
    println!("Printing result...");
    println!("> {} {}", results[0].i64(), results[1].i32());

    debug_assert!(results[0].i64() == 4);
    debug_assert!(results[1].i32() == 2);

    // Shut down.
    println!("Shutting down...");
    drop(store);

    // All done.
    println!("Done.");
    Ok(())
}
