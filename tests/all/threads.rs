use std::{
    collections::{HashSet, hash_map::RandomState},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
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
    let _instance = Instance::new(&mut store, &module, &[shared_memory.into()])?;
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
#[cfg_attr(miri, ignore)]
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
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, 5))?;
    let instance1 = Instance::new(&mut store, &module, &[shared_memory.clone().into()])?;
    let instance2 = Instance::new(&mut store, &module, &[shared_memory.clone().into()])?;
    let data = shared_memory.data();

    // Modify the memory in one place.
    unsafe {
        *data[0].get() = 42;
    }

    // Verify that the memory is the same in all shared locations.
    let shared_memory_first_word = i32::from_le_bytes(unsafe {
        [
            *data[0].get(),
            *data[1].get(),
            *data[2].get(),
            *data[3].get(),
        ]
    });
    let instance1_first_word = instance1
        .get_typed_func::<(), i32>(&mut store, "first_word")?
        .call(&mut store, ())?;
    let instance2_first_word = instance2
        .get_typed_func::<(), i32>(&mut store, "first_word")?
        .call(&mut store, ())?;
    assert_eq!(shared_memory_first_word, 42);
    assert_eq!(instance1_first_word, 42);
    assert_eq!(instance2_first_word, 42);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
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
    let size_fn = instance.get_typed_func::<(), i32>(&mut store, "size")?;
    let shared_memory = instance.get_shared_memory(&mut store, "memory").unwrap();

    assert_eq!(size_fn.call(&mut store, ())?, 1);
    assert_eq!(shared_memory.size(), 1);

    shared_memory.grow(1)?;

    assert_eq!(shared_memory.size(), 2);
    assert_eq!(size_fn.call(&mut store, ())?, 2);

    Ok(())
}

#[test]
fn test_multi_memory() -> Result<()> {
    let wat = r#"(module
        (import "env" "imported" (memory $imported 5 10 shared))
        (memory (export "owned") 10 20)
        (memory (export "shared") 1 2 shared)
        (export "imported" (memory $imported))
    )"#;
    let mut config = Config::new();
    config.wasm_threads(true);
    config.wasm_multi_memory(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let incoming_shared_memory = SharedMemory::new(&engine, MemoryType::shared(5, 10))?;
    let instance = Instance::new(&mut store, &module, &[incoming_shared_memory.into()])?;
    let owned_memory = instance.get_memory(&mut store, "owned").unwrap();
    let shared_memory = instance.get_shared_memory(&mut store, "shared").unwrap();
    let imported_memory = instance.get_shared_memory(&mut store, "imported").unwrap();

    assert_eq!(owned_memory.size(&store), 10);
    assert_eq!(owned_memory.ty(&store).minimum(), 10);
    assert_eq!(owned_memory.ty(&store).maximum(), Some(20));
    assert_eq!(owned_memory.ty(&store).is_shared(), false);
    assert_eq!(shared_memory.size(), 1);
    assert_eq!(shared_memory.ty().minimum(), 1);
    assert_eq!(shared_memory.ty().maximum(), Some(2));
    assert_eq!(shared_memory.ty().is_shared(), true);
    assert_eq!(imported_memory.size(), 5);
    assert_eq!(imported_memory.ty().minimum(), 5);
    assert_eq!(imported_memory.ty().maximum(), Some(10));
    assert_eq!(imported_memory.ty().is_shared(), true);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
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
            let instance = Instance::new(&mut store, &module, &[shared_memory.into()]).unwrap();
            let grow_fn = instance
                .get_typed_func::<i32, i32>(&mut store, "grow")
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

#[test]
#[cfg_attr(miri, ignore)]
fn test_memory_size_accessibility() -> Result<()> {
    const NUM_GROW_OPS: usize = 1000;
    let wat = r#"(module
        (import "env" "memory" (memory $memory 1 1000 shared))
        (func (export "probe_last_available") (result i32)
            (local $last_address i32)
            (local.set $last_address (i32.sub (i32.mul (memory.size) (i32.const 0x10000)) (i32.const 4)))
            (i32.load $memory (local.get $last_address))
        )
    )"#;

    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, NUM_GROW_OPS as u32))?;
    let done = Arc::new(AtomicBool::new(false));

    let grow_memory = shared_memory.clone();
    let grow_thread = std::thread::spawn(move || {
        for i in 0..NUM_GROW_OPS {
            if grow_memory.grow(1).is_err() {
                println!("stopping at grow operation #{i}");
                break;
            }
        }
    });

    let probe_memory = shared_memory.clone();
    let probe_done = done.clone();
    let probe_thread = std::thread::spawn(move || {
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[probe_memory.into()]).unwrap();
        let probe_fn = instance
            .get_typed_func::<(), i32>(&mut store, "probe_last_available")
            .unwrap();
        while !probe_done.load(Ordering::SeqCst) {
            let value = probe_fn.call(&mut store, ()).unwrap() as u32;
            assert_eq!(value, 0);
        }
    });

    grow_thread.join().unwrap();
    done.store(true, Ordering::SeqCst);
    probe_thread.join().unwrap();

    Ok(())
}
