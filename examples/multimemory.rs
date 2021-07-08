//! An example of how to interact with multiple memories.
//!
//! Here a small wasm module with multiple memories is used to show how memory
//! is initialized, how to read and write memory through the `Memory` object,
//! and how wasm functions can trap when dealing with out-of-bounds addresses.

// You can execute this example with `cargo run --example example`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    // Enable the multi-memory feature.
    let mut config = Config::new();
    config.wasm_multi_memory(true);

    let engine = Engine::new(&config)?;

    // Create our `store_fn` context and then compile a module and create an
    // instance from the compiled module all in one go.
    let mut store = Store::new(&engine, ());
    let module = Module::from_file(store.engine(), "examples/multimemory.wat")?;
    let instance = Instance::new(&mut store, &module, &[])?;

    let memory0 = instance
        .get_memory(&mut store, "memory0")
        .ok_or(anyhow::format_err!("failed to find `memory0` export"))?;
    let size0 = instance.get_typed_func::<(), i32, _>(&mut store, "size0")?;
    let load0 = instance.get_typed_func::<i32, i32, _>(&mut store, "load0")?;
    let store0 = instance.get_typed_func::<(i32, i32), (), _>(&mut store, "store0")?;

    let memory1 = instance
        .get_memory(&mut store, "memory1")
        .ok_or(anyhow::format_err!("failed to find `memory1` export"))?;
    let size1 = instance.get_typed_func::<(), i32, _>(&mut store, "size1")?;
    let load1 = instance.get_typed_func::<i32, i32, _>(&mut store, "load1")?;
    let store1 = instance.get_typed_func::<(i32, i32), (), _>(&mut store, "store1")?;

    println!("Checking memory...");
    assert_eq!(memory0.size(&store), 2);
    assert_eq!(memory0.data_size(&store), 0x20000);
    assert_eq!(memory0.data_mut(&mut store)[0], 0);
    assert_eq!(memory0.data_mut(&mut store)[0x1000], 1);
    assert_eq!(memory0.data_mut(&mut store)[0x1001], 2);
    assert_eq!(memory0.data_mut(&mut store)[0x1002], 3);
    assert_eq!(memory0.data_mut(&mut store)[0x1003], 4);

    assert_eq!(size0.call(&mut store, ())?, 2);
    assert_eq!(load0.call(&mut store, 0)?, 0);
    assert_eq!(load0.call(&mut store, 0x1000)?, 1);
    assert_eq!(load0.call(&mut store, 0x1001)?, 2);
    assert_eq!(load0.call(&mut store, 0x1002)?, 3);
    assert_eq!(load0.call(&mut store, 0x1003)?, 4);
    assert_eq!(load0.call(&mut store, 0x1ffff)?, 0);
    assert!(load0.call(&mut store, 0x20000).is_err()); // out of bounds trap

    assert_eq!(memory1.size(&store), 2);
    assert_eq!(memory1.data_size(&store), 0x20000);
    assert_eq!(memory1.data_mut(&mut store)[0], 0);
    assert_eq!(memory1.data_mut(&mut store)[0x1000], 4);
    assert_eq!(memory1.data_mut(&mut store)[0x1001], 3);
    assert_eq!(memory1.data_mut(&mut store)[0x1002], 2);
    assert_eq!(memory1.data_mut(&mut store)[0x1003], 1);

    assert_eq!(size1.call(&mut store, ())?, 2);
    assert_eq!(load1.call(&mut store, 0)?, 0);
    assert_eq!(load1.call(&mut store, 0x1000)?, 4);
    assert_eq!(load1.call(&mut store, 0x1001)?, 3);
    assert_eq!(load1.call(&mut store, 0x1002)?, 2);
    assert_eq!(load1.call(&mut store, 0x1003)?, 1);
    assert_eq!(load1.call(&mut store, 0x1ffff)?, 0);
    assert!(load0.call(&mut store, 0x20000).is_err()); // out of bounds trap

    println!("Mutating memory...");
    memory0.data_mut(&mut store)[0x1003] = 5;

    store0.call(&mut store, (0x1002, 6))?;
    assert!(store0.call(&mut store, (0x20000, 0)).is_err()); // out of bounds trap

    assert_eq!(memory0.data(&store)[0x1002], 6);
    assert_eq!(memory0.data(&store)[0x1003], 5);
    assert_eq!(load0.call(&mut store, 0x1002)?, 6);
    assert_eq!(load0.call(&mut store, 0x1003)?, 5);

    memory1.data_mut(&mut store)[0x1003] = 7;

    store1.call(&mut store, (0x1002, 8))?;
    assert!(store1.call(&mut store, (0x20000, 0)).is_err()); // out of bounds trap

    assert_eq!(memory1.data(&store)[0x1002], 8);
    assert_eq!(memory1.data(&store)[0x1003], 7);
    assert_eq!(load1.call(&mut store, 0x1002)?, 8);
    assert_eq!(load1.call(&mut store, 0x1003)?, 7);

    println!("Growing memory...");
    memory0.grow(&mut store, 1)?;
    assert_eq!(memory0.size(&store), 3);
    assert_eq!(memory0.data_size(&store), 0x30000);

    assert_eq!(load0.call(&mut store, 0x20000)?, 0);
    store0.call(&mut store, (0x20000, 0))?;
    assert!(load0.call(&mut store, 0x30000).is_err());
    assert!(store0.call(&mut store, (0x30000, 0)).is_err());

    assert!(memory0.grow(&mut store, 1).is_err());
    assert!(memory0.grow(&mut store, 0).is_ok());

    memory1.grow(&mut store, 2)?;
    assert_eq!(memory1.size(&store), 4);
    assert_eq!(memory1.data_size(&store), 0x40000);

    assert_eq!(load1.call(&mut store, 0x30000)?, 0);
    store1.call(&mut store, (0x30000, 0))?;
    assert!(load1.call(&mut store, 0x40000).is_err());
    assert!(store1.call(&mut store, (0x40000, 0)).is_err());

    assert!(memory1.grow(&mut store, 1).is_err());
    assert!(memory1.grow(&mut store, 0).is_ok());

    Ok(())
}
