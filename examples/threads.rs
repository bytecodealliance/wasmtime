//! This program is an example of how Wasmtime can be used with multithreaded
//! runtimes and how various types and structures can be shared across threads.

// You can execute this example with `cargo run --example threads`

use anyhow::Result;
use std::sync::Arc;
use std::thread;
use std::time;
use wasmtime::*;

const N_THREADS: i32 = 10;
const N_REPS: i32 = 3;

fn main() -> Result<()> {
    println!("Initializing...");

    // Initialize global per-process state. This state will be shared amonst all
    // threads. Notably this includes the compiled module as well as a `Linker`,
    // which contains all our host functions we want to define.
    let engine = Engine::default();
    let module = Module::from_file(&engine, "examples/threads.wat")?;
    let mut linker = Linker::new(&engine);
    linker.func_wrap("global", "hello", || {
        println!("> Hello from {:?}", thread::current().id());
    })?;
    let linker = Arc::new(linker); // "finalize" the linker

    // Share this global state amongst a set of threads, each of which will
    // create stores and execute instances.
    let children = (0..N_THREADS)
        .map(|_| {
            let engine = engine.clone();
            let module = module.clone();
            let linker = linker.clone();
            thread::spawn(move || {
                run(&engine, &module, &linker).expect("Success");
            })
        })
        .collect::<Vec<_>>();

    for child in children {
        child.join().unwrap();
    }

    Ok(())
}

fn run(engine: &Engine, module: &Module, linker: &Linker<()>) -> Result<()> {
    // Each sub-thread we have starting out by instantiating the `module`
    // provided into a fresh `Store`.
    println!("Instantiating module...");
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, module)?;
    let run = instance.get_typed_func::<(), (), _>(&mut store, "run")?;

    println!("Executing...");
    for _ in 0..N_REPS {
        run.call(&mut store, ())?;
        thread::sleep(time::Duration::from_millis(100));
    }

    // Also note that that a `Store` can also move between threads:
    println!("> Moving {:?} to a new thread", thread::current().id());
    let child = thread::spawn(move || run.call(&mut store, ()));

    child.join().unwrap()?;

    Ok(())
}
