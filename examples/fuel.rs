//! Example of limiting a WebAssembly function's runtime using "fuel consumption".

// You can execute this example with `cargo run --example fuel`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    let mut config = Config::new();
    config.consume_fuel(true);
    config.outband_fuel(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    store.add_fuel(114_057_726)?;
    let module = Module::from_file(store.engine(), "examples/fuel.wat")?;
    let instance = Instance::new(&mut store, &module, &[])?;

    init_outband_fuel_check(&store);

    // Invoke `fibonacci` export with higher and higher numbers until we exhaust our fuel.
    // HACK: I did not bother fixing the trampolines stuff for the typed funcs.
    let fibonacci = instance
        .get_func(&mut store, "fibonacci")
        .ok_or_else(|| anyhow::anyhow!("fuel.wat does not export 'fibonacci'"))?;
    for n in 1.. {
        let fuel_before = store.fuel_consumed().unwrap();
        let mut outputs = [Val::I32(0)];
        let output = match fibonacci.call(&mut store, &[Val::I32(n as i32)], &mut outputs) {
            Ok(()) => outputs[0].clone().unwrap_i32(),
            Err(_) => {
                let fuel_consumed = store.fuel_consumed().unwrap() - fuel_before;
                println!(
                    "Exhausted fuel computing fib({}), consumed: {} fuel",
                    n, fuel_consumed
                );
                break;
            }
        };
        let fuel_consumed = store.fuel_consumed().unwrap() - fuel_before;
        // println!("fib({}) = {} [consumed {} fuel]", n, output, fuel_consumed);
        store.add_fuel(fuel_consumed)?;
    }
    Ok(())
}

fn init_outband_fuel_check<T>(store: &Store<T>) {
    let checker = store.current_thread_outband_fuel_checker();
    std::thread::spawn(move || loop {
        #[allow(deprecated)]
        std::thread::sleep_ms(1000);
        checker.check();
    });

    // wait for some time to allow the spawned thread to start.
    #[allow(deprecated)]
    std::thread::sleep_ms(100);
}
