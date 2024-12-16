#![cfg(not(miri))]

use crate::threads::engine;
use std::time::Instant;
use wasmtime::*;

#[test]
fn atomic_wait_timeout_length() -> Result<()> {
    let sleep_nanoseconds = 500000000;
    let wat = format!(
        r#"(module
        (import "env" "memory" (memory 1 1 shared))

        (func (export "func1") (result i32)
            (memory.atomic.wait32 (i32.const 0) (i32.const 0) (i64.const {sleep_nanoseconds}))
        )

        (data (i32.const 0) "\00\00\00\00")
    )"#
    );
    let Some(engine) = engine() else {
        return Ok(());
    };
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, 1))?;
    let instance = Instance::new(&mut store, &module, &[shared_memory.clone().into()])?;
    let now = Instant::now();
    let func_ret = instance
        .get_typed_func::<(), i32>(&mut store, "func1")
        .unwrap()
        .call(&mut store, ())
        .unwrap();
    let duration = now.elapsed();
    assert!(
        duration.as_nanos() >= sleep_nanoseconds,
        "duration: {duration:?} < {sleep_nanoseconds:?}"
    );
    assert_eq!(func_ret, 2);
    Ok(())
}

#[test]
fn atomic_wait_notify_basic() -> Result<()> {
    let wat = r#"(module
        (import "env" "memory" (memory 1 1 shared))

        (func (export "first_thread") (result i32)
            (drop (memory.atomic.wait32 (i32.const 4) (i32.const 0) (i64.const -1)))
            (i32.atomic.store (i32.const 0) (i32.const 42))
            (drop (memory.atomic.notify (i32.const 0) (i32.const -1)))
            (i32.atomic.load (i32.const 0))
        )

        (func (export "second_thread") (result i32)
            (i32.atomic.store (i32.const 4) (i32.const 21))
            (drop (memory.atomic.notify (i32.const 4) (i32.const -1)))
            (drop (memory.atomic.wait32 (i32.const 0) (i32.const 0) (i64.const -1)))
            (i32.atomic.load (i32.const 0))
        )

        (data (i32.const 0) "\00\00\00\00")
        (data (i32.const 4) "\00\00\00\00")
    )"#;
    let Some(engine) = engine() else {
        return Ok(());
    };
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, 1))?;
    let instance1 = Instance::new(&mut store, &module, &[shared_memory.clone().into()])?;

    let thread = {
        let engine = engine.clone();
        let module = module.clone();
        let shared_memory = shared_memory.clone();
        std::thread::spawn(move || {
            let mut store = Store::new(&engine, ());
            let instance2 = Instance::new(&mut store, &module, &[shared_memory.into()]).unwrap();

            let instance2_first_word = instance2
                .get_typed_func::<(), i32>(&mut store, "second_thread")
                .unwrap()
                .call(&mut store, ())
                .unwrap();

            assert_eq!(instance2_first_word, 42);
        })
    };

    let instance1_first_word = instance1
        .get_typed_func::<(), i32>(&mut store, "first_thread")
        .unwrap()
        .call(&mut store, ())
        .unwrap();
    assert_eq!(instance1_first_word, 42);

    thread.join().unwrap();

    let data = shared_memory.data();
    // Verify that the memory is the same in all shared locations.
    let shared_memory_first_word = i32::from_le_bytes(unsafe {
        [
            *data[0].get(),
            *data[1].get(),
            *data[2].get(),
            *data[3].get(),
        ]
    });
    assert_eq!(shared_memory_first_word, 42);

    let shared_memory_second_word = i32::from_le_bytes(unsafe {
        [
            *data[4].get(),
            *data[5].get(),
            *data[6].get(),
            *data[7].get(),
        ]
    });
    assert_eq!(shared_memory_second_word, 21);
    Ok(())
}
