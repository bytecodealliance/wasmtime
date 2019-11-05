//! Translation of multi example

extern crate alloc;

use alloc::rc::Rc;
use anyhow::{ensure, format_err, Context as _, Result};
use core::cell::Ref;
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

fn main() -> Result<()> {
    // Initialize.
    println!("Initializing...");
    let engine = HostRef::new(Engine::new(Config::default()));
    let store = HostRef::new(Store::new(engine));

    // Load binary.
    println!("Loading binary...");
    let binary = read("examples/multi.wasm")?;

    // Compile.
    println!("Compiling module...");
    let module =
        HostRef::new(Module::new(store.clone(), &binary).context("Error compiling module!")?);

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
            .context("Error instantiating module!")?,
    );

    // Extract export.
    println!("Extracting export...");
    let exports = Ref::map(instance.borrow(), |instance| instance.exports());
    ensure!(!exports.is_empty(), "Error accessing exports!");
    let run_func = exports[0].func().context("Error accessing exports!")?;

    // Call.
    println!("Calling export...");
    let args = vec![Val::I32(1), Val::I64(3)];
    let results = run_func
        .borrow()
        .call(&args)
        .map_err(|e| format_err!("> Error calling function: {:?}", e))?;

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
