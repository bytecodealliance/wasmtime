//! Example of limiting a WebAssembly function's runtime using "fuel consumption".

// You can execute this example with `cargo run --example fuel`

use wasmtime::*;

fn main() -> Result<()> {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000)?;
    let module = Module::from_file(store.engine(), "examples/fuel.wat")?;
    let instance = Instance::new(&mut store, &module, &[])?;

    // Invoke `fibonacci` export with higher and higher numbers until we exhaust our fuel.
    let fibonacci = instance.get_typed_func::<i32, i32>(&mut store, "fibonacci")?;
    for n in 1.. {
        let fuel_before = store.get_fuel().unwrap();
        let output = match fibonacci.call(&mut store, n) {
            Ok(v) => v,
            Err(e) => {
                assert_eq!(e.downcast::<Trap>()?, Trap::OutOfFuel);
                println!("Exhausted fuel computing fib({n})");
                break;
            }
        };
        let fuel_consumed = fuel_before - store.get_fuel().unwrap();
        println!("fib({n}) = {output} [consumed {fuel_consumed} fuel]");
        store.set_fuel(10_000)?;
    }
    Ok(())
}
