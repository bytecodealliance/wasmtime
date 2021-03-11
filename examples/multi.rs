//! This is an example of working with multi-value modules and dealing with
//! multi-value functions.
//!
//! Note that the `Func::wrap*` interfaces cannot be used to return multiple
//! values just yet, so we need to use the more dynamic `Func::new` and
//! `Func::call` methods.

// You can execute this example with `cargo run --example multi`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    println!("Initializing...");
    let engine = Engine::default();
    let store = Store::new(&engine);

    // Compile.
    println!("Compiling module...");
    let module = Module::from_file(&engine, "examples/multi.wat")?;

    // Create external print functions.
    println!("Creating callback...");
    let callback_type = FuncType::new(
        [ValType::I32, ValType::I64].iter().cloned(),
        [ValType::I64, ValType::I32].iter().cloned(),
    );
    let callback_func = Func::new(&store, callback_type, |_, args, results| {
        println!("Calling back...");
        println!("> {} {}", args[0].unwrap_i32(), args[1].unwrap_i64());

        results[0] = Val::I64(args[1].unwrap_i64() + 1);
        results[1] = Val::I32(args[0].unwrap_i32() + 1);
        Ok(())
    });

    // Instantiate.
    println!("Instantiating module...");
    let instance = Instance::new(&store, &module, &[callback_func.into()])?;

    // Extract exports.
    println!("Extracting export...");
    let g = instance.get_typed_func::<(i32, i64), (i64, i32)>("g")?;

    // Call `$g`.
    println!("Calling export \"g\"...");
    let (a, b) = g.call((1, 3))?;

    println!("Printing result...");
    println!("> {} {}", a, b);

    assert_eq!(a, 4);
    assert_eq!(b, 2);

    // Call `$round_trip_many`.
    println!("Calling export \"round_trip_many\"...");
    let round_trip_many = instance
        .get_typed_func::<
        (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64),
        (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64),
        >
        ("round_trip_many")?;
    let results = round_trip_many.call((0, 1, 2, 3, 4, 5, 6, 7, 8, 9))?;

    println!("Printing result...");
    println!("> {:?}", results);
    assert_eq!(results, (0, 1, 2, 3, 4, 5, 6, 7, 8, 9));

    Ok(())
}
