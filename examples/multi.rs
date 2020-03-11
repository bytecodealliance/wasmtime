//! This is an example of working with mulit-value modules and dealing with
//! multi-value functions.
//!
//! Note that the `Func::wrap*` interfaces cannot be used to return multiple
//! values just yet, so we need to use the more dynamic `Func::new` and
//! `Func::call` methods.

// You can execute this example with `cargo run --example multi`

use anyhow::{format_err, Result};
use std::rc::Rc;
use wasmtime::*;

struct Callback;

impl Callable for Callback {
    fn call(&self, args: &[Val], results: &mut [Val]) -> Result<(), Trap> {
        println!("Calling back...");
        println!("> {} {}", args[0].unwrap_i32(), args[1].unwrap_i64());

        results[0] = Val::I64(args[1].unwrap_i64() + 1);
        results[1] = Val::I32(args[0].unwrap_i32() + 1);
        Ok(())
    }
}

fn main() -> Result<()> {
    // Configure our `Store`, but be sure to use a `Config` that enables the
    // wasm multi-value feature since it's not stable yet.
    println!("Initializing...");
    let engine = Engine::new(Config::new().wasm_multi_value(true));
    let store = Store::new(&engine);

    // Compile.
    println!("Compiling module...");
    let module = Module::from_file(&store, "examples/multi.wat")?;

    // Create external print functions.
    println!("Creating callback...");
    let callback_type = FuncType::new(
        Box::new([ValType::I32, ValType::I64]),
        Box::new([ValType::I64, ValType::I32]),
    );
    let callback_func = Func::new(&store, callback_type, Rc::new(Callback));

    // Instantiate.
    println!("Instantiating module...");
    let instance = Instance::new(&module, &[callback_func.into()])?;

    // Extract exports.
    println!("Extracting export...");
    let g = instance
        .get_export("g")
        .and_then(|e| e.func())
        .ok_or(format_err!("failed to find export `g`"))?;

    // Call `$g`.
    println!("Calling export \"g\"...");
    let results = g.call(&[Val::I32(1), Val::I64(3)])?;

    println!("Printing result...");
    println!("> {} {}", results[0].unwrap_i64(), results[1].unwrap_i32());

    assert_eq!(results[0].unwrap_i64(), 4);
    assert_eq!(results[1].unwrap_i32(), 2);

    // Call `$round_trip_many`.
    println!("Calling export \"round_trip_many\"...");
    let round_trip_many = instance
        .get_export("round_trip_many")
        .and_then(|e| e.func())
        .ok_or(format_err!("failed to find export `round_trip_many`"))?;
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
    let results = round_trip_many.call(&args)?;

    println!("Printing result...");
    print!(">");
    for r in results.iter() {
        print!(" {}", r.unwrap_i64());
    }
    println!();

    assert_eq!(results.len(), 10);
    assert!(args
        .iter()
        .zip(results.iter())
        .all(|(a, r)| a.i64() == r.i64()));

    Ok(())
}
