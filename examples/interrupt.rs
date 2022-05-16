//! Small example of how you can interrupt the execution of a wasm module to
//! ensure that it doesn't run for too long.

// You can execute this example with `cargo run --example interrupt`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    // Enable epoch interruption code via `Config` which means that code will
    // get interrupted when `Engine::increment_epoch` happens.
    let engine = Engine::new(Config::new().epoch_interruption(true))?;
    let mut store = Store::new(&engine, ());
    store.set_epoch_deadline(1);

    // Compile and instantiate a small example with an infinite loop.
    let module = Module::from_file(&engine, "examples/interrupt.wat")?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_typed_func::<(), (), _>(&mut store, "run")?;

    // Spin up a thread to send us an interrupt in a second
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("Interrupting!");
        engine.increment_epoch();
    });

    println!("Entering infinite loop ...");
    let trap = run.call(&mut store, ()).unwrap_err();

    println!("trap received...");
    assert!(trap.trap_code().unwrap() == TrapCode::Interrupt);

    Ok(())
}
