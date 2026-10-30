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
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, 1)?)?;
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
