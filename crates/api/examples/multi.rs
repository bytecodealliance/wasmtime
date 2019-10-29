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
    let engine = HostRef::new(Engine::default());
    let store = HostRef::new(Store::new(&engine));

    // Load binary.
    println!("Loading binary...");
    let binary = read("examples/multi.wasm")?;

    // Compile.
    println!("Compiling module...");
    let module = HostRef::new(Module::new(&store, &binary).context("Error compiling module!")?);

    // Create external print functions.
    println!("Creating callback...");
    let callback_type = FuncType::new(
        Box::new([ValType::I32, ValType::I64]),
        Box::new([ValType::I64, ValType::I32]),
    );
    let callback_func = HostRef::new(Func::new(&store, callback_type, Rc::new(Callback)));

    // Instantiate.
    println!("Instantiating module...");
    let imports = vec![callback_func.into()];
    let instance = HostRef::new(
        Instance::new(&store, &module, imports.as_slice())
            .context("Error instantiating module!")?,
    );

    // Extract exports.
    println!("Extracting export...");
    let exports = Ref::map(instance.borrow(), |instance| instance.exports());
    ensure!(!exports.is_empty(), "Error accessing exports!");
    let g = exports[0].func().context("> Error accessing export $g!")?;
    let round_trip_many = exports[1]
        .func()
        .context("> Error accessing export $round_trip_many")?;

    // Call `$g`.
    println!("Calling export \"g\"...");
    let args = vec![Val::I32(1), Val::I64(3)];
    let results = g
        .borrow()
        .call(&args)
        .map_err(|e| format_err!("> Error calling g! {:?}", e))?;

    println!("Printing result...");
    println!("> {} {}", results[0].i64(), results[1].i32());

    debug_assert_eq!(results[0].i64(), 4);
    debug_assert_eq!(results[1].i32(), 2);

    // Call `$round_trip_many`.
    println!("Calling export \"round_trip_many\"...");
    let args = vec![
        Val::I64(0),
        Val::I64(1),
        Val::I64(2),
        Val::I64(3),
        Val::I64(4),
        Val::I64(5),
        Val::I64(6),
        Val::I64(7),
        Val::I64(8),
        Val::I64(9),
    ];
    let results = round_trip_many
        .borrow()
        .call(&args)
        .map_err(|e| format_err!("> Error calling round_trip_many! {:?}", e))?;

    println!("Printing result...");
    print!(">");
    for r in results.iter() {
        print!(" {}", r.i64());
    }
    println!();

    debug_assert_eq!(results.len(), 10);
    debug_assert!(args
        .iter()
        .zip(results.iter())
        .all(|(a, r)| a.i64() == r.i64()));

    // Shut down.
    println!("Shutting down...");
    drop(store);

    // All done.
    println!("Done.");
    Ok(())
}
