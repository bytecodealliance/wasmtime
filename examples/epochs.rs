//! Example of interrupting a WebAssembly function's runtime via epoch
//! changes ("epoch interruption") in a synchronous context.  To see
//! an example of setup for asynchronous usage, see
//! `tests/all/epoch_interruption.rs`

use anyhow::Error;
use std::sync::Arc;
use wasmtime::{Config, Engine, Instance, Module, Store};

fn main() -> Result<(), Error> {
    // Set up an engine configured with epoch interruption enabled.
    let mut config = Config::new();
    config.epoch_interruption(true);
    let engine = Arc::new(Engine::new(&config)?);

    let mut store = Store::new(&engine, ());
    // Configure the store to trap on reaching the epoch deadline.
    // This is the default, but we do it explicitly here to
    // demonstrate.
    store.epoch_deadline_trap();
    // Configure the store to have an initial epoch deadline one tick
    // in the future.
    store.set_epoch_deadline(1);

    // Reuse the fibonacci function from the Fuel example. This is a
    // long-running function that we will want to interrupt.
    let module = Module::from_file(store.engine(), "examples/fuel.wat")?;
    let instance = Instance::new(&mut store, &module, &[])?;

    // Start a thread that will bump the epoch after 1 second.
    let engine_clone = engine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        engine_clone.increment_epoch();
    });

    // Invoke `fibonacci` with a large argument such that a normal
    // invocation would take many seconds to complete.
    let fibonacci = instance.get_typed_func::<i32, i32, _>(&mut store, "fibonacci")?;
    match fibonacci.call(&mut store, 100) {
        Ok(_) => panic!("Somehow we computed recursive fib(100) in less than a second!"),
        Err(_) => {
            println!("Trapped out of fib(100) after epoch increment");
        }
    };

    Ok(())
}
