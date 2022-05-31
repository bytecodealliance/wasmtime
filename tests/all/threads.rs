use anyhow::Result;
use std::{
    collections::{hash_map::RandomState, HashSet},
    sync::{Arc, RwLock},
};
use wasmtime::*;

#[test]
fn test_instantiate_shared_memory() -> Result<()> {
    let wat = r#"(module (memory 1 1 shared))"#;
    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let _instance = Instance::new(&mut store, &module, &[])?;
    Ok(())
}

#[test]
fn test_import_shared_memory() -> Result<()> {
    let wat = r#"(module (import "env" "memory" (memory 1 5 shared)))"#;
    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, 5))?;
    let import = shared_memory.as_extern(&mut store)?;
    let _instance = Instance::new(&mut store, &module, &[import])?;
    Ok(())
}

#[test]
fn test_export_shared_memory() -> Result<()> {
    let wat = r#"(module (memory (export "memory") 1 5 shared))"#;
    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let shared_memory = instance.get_shared_memory(&mut store, "memory").unwrap();

    assert_eq!(shared_memory.size(), 1);
    assert!(shared_memory.ty().is_shared());
    assert_eq!(shared_memory.ty().maximum(), Some(5));

    Ok(())
}

#[test]
fn test_sharing_of_shared_memory() -> Result<()> {
    let wat = r#"(module
        (import "env" "memory" (memory 1 5 shared))
        (func (export "first_word") (result i32) (i32.load (i32.const 0)))
    )"#;
    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let mut shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, 5))?;
    let import1 = shared_memory.as_extern(&mut store)?;
    let instance1 = Instance::new(&mut store, &module, &[import1])?;
    let import2 = shared_memory.as_extern(&mut store)?;
    let instance2 = Instance::new(&mut store, &module, &[import2])?;

    // Modify the memory in one place.
    unsafe {
        (*shared_memory.data_mut())[0] = 42;
    }

    // Verify that the memory is the same in all shared locations.
    let shared_memory_first_word =
        i32::from_le_bytes(unsafe { (*shared_memory.data())[0..4].try_into()? });
    let instance1_first_word = instance1
        .get_typed_func::<(), i32, _>(&mut store, "first_word")?
        .call(&mut store, ())?;
    let instance2_first_word = instance2
        .get_typed_func::<(), i32, _>(&mut store, "first_word")?
        .call(&mut store, ())?;
    assert_eq!(shared_memory_first_word, 42);
    assert_eq!(instance1_first_word, 42);
    assert_eq!(instance2_first_word, 42);

    Ok(())
}

#[test]
fn test_probe_shared_memory_size() -> Result<()> {
    let wat = r#"(module
        (memory (export "memory") 1 2 shared)
        (func (export "size") (result i32) (memory.size))
    )"#;
    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let size_fn = instance.get_typed_func::<(), i32, _>(&mut store, "size")?;
    let mut shared_memory = instance.get_shared_memory(&mut store, "memory").unwrap();

    assert_eq!(size_fn.call(&mut store, ())?, 1);
    assert_eq!(shared_memory.size(), 1);

    shared_memory.grow(1)?;

    assert_eq!(shared_memory.size(), 2);
    assert_eq!(size_fn.call(&mut store, ())?, 2);

    Ok(())
}

#[test]
fn test_grow_memory_in_multiple_threads() -> Result<()> {
    const NUM_THREADS: usize = 4;
    const NUM_GROW_OPS: usize = 1000;

    let wat = r#"(module
        (import "env" "memory" (memory 1 4000 shared))
        (func (export "grow") (param $delta i32) (result i32) (memory.grow (local.get $delta)))
    )"#;

    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, NUM_GROW_OPS as u32))?;
    let mut threads = vec![];
    let observed_sizes = Arc::new(RwLock::new(vec![]));

    // Spawn several threads using a single shared memory and grow the memory
    // concurrently on all threads.
    for _ in 0..NUM_THREADS {
        let engine = engine.clone();
        let module = module.clone();
        let observed_sizes = observed_sizes.clone();
        let shared_memory = shared_memory.clone();
        let thread = std::thread::spawn(move || {
            let mut store = Store::new(&engine, ());
            let import = shared_memory.as_extern(&mut store).unwrap();
            let instance = Instance::new(&mut store, &module, &[import]).unwrap();
            let grow_fn = instance
                .get_typed_func::<i32, i32, _>(&mut store, "grow")
                .unwrap();
            let mut thread_local_observed_sizes: Vec<_> = (0..NUM_GROW_OPS / NUM_THREADS)
                .map(|_| grow_fn.call(&mut store, 1).unwrap() as u32)
                .collect();
            println!(
                "Returned memory sizes for {:?}: {:?}",
                std::thread::current().id(),
                thread_local_observed_sizes
            );
            assert!(is_sorted(thread_local_observed_sizes.as_slice()));
            observed_sizes
                .write()
                .unwrap()
                .append(&mut thread_local_observed_sizes);
        });
        threads.push(thread);
    }

    // Wait for all threads to finish.
    for t in threads {
        t.join().unwrap()
    }

    // Ensure the returned "old memory sizes" are all unique--i.e., we have not
    // observed the same growth twice.
    let unique_observed_sizes: HashSet<u32, RandomState> =
        HashSet::from_iter(observed_sizes.read().unwrap().iter().cloned());
    assert_eq!(
        observed_sizes.read().unwrap().len(),
        unique_observed_sizes.len()
    );

    Ok(())
}

fn is_sorted(data: &[u32]) -> bool {
    data.windows(2).all(|d| d[0] <= d[1])
}
