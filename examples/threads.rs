// You can execute this example with `cargo run --example threads`

use anyhow::Result;
use std::thread;
use std::time;
use wasmtime::*;

const N_THREADS: i32 = 10;
const N_REPS: i32 = 3;

fn run(engine: &Engine, module: Module, id: i32) -> Result<()> {
    let store = Store::new(&engine);

    // Create external print functions.
    println!("Creating callback...");
    let callback_func = Func::wrap(&store, |arg: i32| {
        println!("> Thread {} is running", arg);
    });

    let id_type = GlobalType::new(ValType::I32, Mutability::Const);
    let id_global = Global::new(&store, id_type, Val::I32(id))?;

    // Instantiate.
    println!("Instantiating module...");
    let instance = Instance::new(&store, &module, &[callback_func.into(), id_global.into()])?;

    // Extract exports.
    println!("Extracting export...");
    let g = instance.get_typed_func::<(), ()>("run")?;

    for _ in 0..N_REPS {
        thread::sleep(time::Duration::from_millis(100));
        // Call `$run`.
        drop(g.call(())?);
    }

    Ok(())
}

fn main() -> Result<()> {
    println!("Initializing...");
    let engine = Engine::default();

    // Compile.
    println!("Compiling module...");
    let module = Module::from_file(&engine, "examples/threads.wat")?;

    let mut children = Vec::new();
    for id in 0..N_THREADS {
        let engine = engine.clone();
        let module = module.clone();
        children.push(thread::spawn(move || {
            run(&engine, module, id).expect("Success");
        }));
    }

    for (i, child) in children.into_iter().enumerate() {
        if let Err(_) = child.join() {
            println!("Thread #{} errors", i);
        }
    }

    Ok(())
}
