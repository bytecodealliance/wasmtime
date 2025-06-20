//! Tuning Wasmtime for fast instantiation.

use anyhow::anyhow;
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, Linker, Module, PoolingAllocationConfig, Result,
    Store,
};

fn main() -> Result<()> {
    let mut config = Config::new();

    // Configure and enable the pooling allocator with space for 100 memories of
    // up to 2GiB in size, 100 tables holding up to 5000 elements, and with a
    // limit of no more than 100 concurrent instances.
    let mut pool = PoolingAllocationConfig::new();
    pool.total_memories(100);
    pool.max_memory_size(1 << 31); // 2 GiB
    pool.total_tables(100);
    pool.table_elements(5000);
    pool.total_core_instances(100);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    // Enable copy-on-write heap images.
    config.memory_init_cow(true);

    // Create an engine with our configuration.
    let engine = Engine::new(&config)?;

    // Create a linker and populate it with all the imports needed for the Wasm
    // programs we will run. In a more realistic Wasmtime embedding, this would
    // probably involve adding WASI functions to the linker, for example.
    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("math", "add", |a: u32, b: u32| -> u32 { a + b })?;

    // Create a new module, load a pre-compiled module from disk, or etc...
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "math" "add" (func $add (param i32 i32) (result i32)))
                (func (export "run")
                    (call $add (i32.const 29) (i32.const 13))
                )
            )
        "#,
    )?;

    // Create an `InstancePre` for our module, doing import resolution and
    // type-checking ahead-of-time and removing it from the instantiation
    // critical path.
    let instance_pre = linker.instantiate_pre(&module)?;

    // Now we can very quickly instantiate our module, so long as we have no
    // more than 100 concurrent instances at a time!
    //
    // For example, we can spawn 100 threads and have each of them instantiate
    // and run our Wasm module in a loop.
    //
    // In a real Wasmtime embedding, this would be doing something like handling
    // new HTTP requests, game events, or etc... instead of just calling the
    // same function. A production embedding would likely also be using async,
    // in which case it would want some sort of back-pressure mechanism (like a
    // semaphore) on incoming tasks to avoid attempting to allocate more than
    // the pool's maximum-supported concurrent instances (at which point,
    // instantiation will start returning errors).
    let handles: Vec<std::thread::JoinHandle<Result<()>>> = (0..100)
        .map(|_| {
            let engine = engine.clone();
            let instance_pre = instance_pre.clone();
            std::thread::spawn(move || -> Result<()> {
                for _ in 0..999 {
                    // Create a new store for this instance.
                    let mut store = Store::new(&engine, ());
                    // Instantiate our module in this store.
                    let instance = instance_pre.instantiate(&mut store)?;
                    // Call the instance's `run` function!
                    let _result = instance
                        .get_typed_func::<(), i32>(&mut store, "run")?
                        .call(&mut store, ());
                }
                Ok(())
            })
        })
        .collect();

    // Wait for the threads to finish.
    for h in handles.into_iter() {
        h.join().map_err(|_| anyhow!("thread panicked!"))??;
    }

    Ok(())
}
