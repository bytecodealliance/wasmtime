use anyhow::Result;
use std::sync::{Arc, RwLock};
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
    let memory = Memory::from_shared_memory(&mut store, &shared_memory)?;
    let _instance = Instance::new(&mut store, &module, &[memory.into()])?;
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
    let shared_memory = instance
        .get_memory(&mut store, "memory")
        .unwrap()
        .into_shared_memory(&mut store)?;
    shared_memory.data();
    Ok(())
}

#[test]
fn test_construct_memory_with_shared_type() -> Result<()> {
    // let memory = Memory::new(&mut store, MemoryType::shared(1, 5))?;
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
    let memory = Memory::from_shared_memory(&mut store, &shared_memory)?;
    let instance1 = Instance::new(&mut store, &module, &[memory.into()])?;
    let instance2 = Instance::new(&mut store, &module, &[memory.into()])?;

    // Modify the memory in one place.
    shared_memory.data_mut()[0] = 42;

    // Verify that the memory is the same in all shared locations.
    let shared_memory_first_word = i32::from_le_bytes(shared_memory.data()[0..4].try_into()?);
    let memory_first_word = i32::from_le_bytes(memory.data(&store)[0..4].try_into()?);
    let instance1_first_word = instance1
        .get_typed_func::<(), i32, _>(&mut store, "first_word")?
        .call(&mut store, ())?;
    let instance2_first_word = instance2
        .get_typed_func::<(), i32, _>(&mut store, "first_word")?
        .call(&mut store, ())?;
    assert_eq!(shared_memory_first_word, 42);
    assert_eq!(memory_first_word, 42);
    assert_eq!(instance1_first_word, 42);
    assert_eq!(instance2_first_word, 42);

    Ok(())
}

#[test]
fn test_probe_shared_memory_size() -> Result<()> {
    let wat = r#"(module
        (memory (export "memory") 1 1 shared)
        (func (export "size") (result i32) (memory.size))
    )"#;
    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let size_fn = instance.get_typed_func::<(), i32, _>(&mut store, "size")?;

    assert_eq!(size_fn.call(&mut store, ())?, 1);
    assert_eq!(
        instance
            .get_memory(&mut store, "memory")
            .unwrap()
            .size(&store),
        1
    );

    Ok(())
}

#[test]
fn test_grow_memory_in_multiple_threads() -> Result<()> {
    let wat = r#"(module
        (import "env" "memory" (memory 1 10 shared))
        (func (export "grow") (param $delta i32) (result i32) (memory.grow (local.get $delta)))
    )"#;

    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Arc::new(Engine::new(&config)?);
    let module = Arc::new(Module::new(&engine, wat)?);
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, 10))?;
    let mut threads = vec![];
    let sizes = Arc::new(RwLock::new(vec![]));

    // Spawn several threads using a single shared memory and grow the memory
    // concurrently on all threads.
    for _ in 0..4 {
        let engine = engine.clone();
        let module = module.clone();
        let sizes = sizes.clone();
        let shared_memory = shared_memory.clone();
        let thread = std::thread::spawn(move || {
            let mut store = Store::new(&engine, ());
            let memory = Memory::from_shared_memory(&mut store, &shared_memory).unwrap();
            let instance = Instance::new(&mut store, &module, &[memory.into()]).unwrap();
            let grow = instance
                .get_typed_func::<i32, i32, _>(&mut store, "grow")
                .unwrap();
            for _ in 0..4 {
                let old_size = grow.call(&mut store, 1).unwrap();
                sizes.write().unwrap().push(old_size as u32);
            }
        });
        threads.push(thread);
    }

    // Wait for all threads to finish.
    for t in threads {
        t.join().unwrap()
    }

    // Ensure the returned "old memory sizes" were pushed in increasing order,
    // indicating that the lock worked.
    println!("Returned memory sizes: {:?}", sizes);
    assert!(is_sorted(sizes.read().unwrap().as_slice()));

    Ok(())
}

fn is_sorted(data: &[u32]) -> bool {
    data.windows(2).all(|d| d[0] <= d[1])
}
