//! Example of limiting a WebAssembly function's runtime using "fuel consumption".

// You can execute this example with `cargo run --example fuel`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);
    store.add_fuel(10_000)?;
    let module = Module::from_file(store.engine(), "examples/fuel.wat")?;
    let instance = Instance::new(&store, &module, &[])?;

    // Invoke `fibonacci` export with higher and higher numbers until we exhaust our fuel.
    let fibonacci = instance.get_typed_func::<i32, i32>("fibonacci")?;
    for n in 1.. {
        let fuel_before = store.fuel_consumed().unwrap();
        let output = match fibonacci.call(n) {
            Ok(v) => v,
            Err(_) => {
                println!("Exhausted fuel computing fib({})", n);
                break;
            }
        };
        let fuel_consumed = store.fuel_consumed().unwrap() - fuel_before;
        println!("fib({}) = {} [consumed {} fuel]", n, output, fuel_consumed);
        store.add_fuel(fuel_consumed)?;
    }
    Ok(())
}
