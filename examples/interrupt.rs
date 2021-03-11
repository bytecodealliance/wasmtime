//! Small example of how you can interrupt the execution of a wasm module to
//! ensure that it doesn't run for too long.

// You can execute this example with `cargo run --example interrupt`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    // Enable interruptable code via `Config` and then create an interrupt
    // handle which we'll use later to interrupt running code.
    let engine = Engine::new(Config::new().interruptable(true))?;
    let store = Store::new(&engine);
    let interrupt_handle = store.interrupt_handle()?;

    // Compile and instantiate a small example with an infinite loop.
    let module = Module::from_file(&engine, "examples/interrupt.wat")?;
    let instance = Instance::new(&store, &module, &[])?;
    let run = instance.get_typed_func::<(), ()>("run")?;

    // Spin up a thread to send us an interrupt in a second
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("Interrupting!");
        interrupt_handle.interrupt();
    });

    println!("Entering infinite loop ...");
    let trap = run.call(()).unwrap_err();

    println!("trap received...");
    assert!(trap.to_string().contains("wasm trap: interrupt"));

    Ok(())
}
