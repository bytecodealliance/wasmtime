//! An example of how to interact with wasm memory.
//!
//! Here a small wasm module is used to show how memory is initialized, how to
//! read and write memory through the `Memory` object, and how wasm functions
//! can trap when dealing with out-of-bounds addresses.

// You can execute this example with `cargo run --example example`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    // Create our `store_fn` context and then compile a module and create an
    // instance from the compiled module all in one go.
    let mut store: Store<()> = Store::default();
    let module = Module::from_file(store.engine(), "examples/memory.wat")?;
    let instance = Instance::new(&mut store, &module, &[])?;

    // load_fn up our exports from the instance
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or(anyhow::format_err!("failed to find `memory` export"))?;
    let size = instance.get_typed_func::<(), i32, _>(&mut store, "size")?;
    let load_fn = instance.get_typed_func::<i32, i32, _>(&mut store, "load")?;
    let store_fn = instance.get_typed_func::<(i32, i32), (), _>(&mut store, "store")?;

    println!("Checking memory...");
    assert_eq!(memory.size(&store), 2);
    assert_eq!(memory.data_size(&store), 0x20000);
    assert_eq!(memory.data_mut(&mut store)[0], 0);
    assert_eq!(memory.data_mut(&mut store)[0x1000], 1);
    assert_eq!(memory.data_mut(&mut store)[0x1003], 4);

    assert_eq!(size.call(&mut store, ())?, 2);
    assert_eq!(load_fn.call(&mut store, 0)?, 0);
    assert_eq!(load_fn.call(&mut store, 0x1000)?, 1);
    assert_eq!(load_fn.call(&mut store, 0x1003)?, 4);
    assert_eq!(load_fn.call(&mut store, 0x1ffff)?, 0);
    assert!(load_fn.call(&mut store, 0x20000).is_err()); // out of bounds trap

    println!("Mutating memory...");
    memory.data_mut(&mut store)[0x1003] = 5;

    store_fn.call(&mut store, (0x1002, 6))?;
    assert!(store_fn.call(&mut store, (0x20000, 0)).is_err()); // out of bounds trap

    assert_eq!(memory.data(&store)[0x1002], 6);
    assert_eq!(memory.data(&store)[0x1003], 5);
    assert_eq!(load_fn.call(&mut store, 0x1002)?, 6);
    assert_eq!(load_fn.call(&mut store, 0x1003)?, 5);

    // Grow memory.
    println!("Growing memory...");
    memory.grow(&mut store, 1)?;
    assert_eq!(memory.size(&store), 3);
    assert_eq!(memory.data_size(&store), 0x30000);

    assert_eq!(load_fn.call(&mut store, 0x20000)?, 0);
    store_fn.call(&mut store, (0x20000, 0))?;
    assert!(load_fn.call(&mut store, 0x30000).is_err());
    assert!(store_fn.call(&mut store, (0x30000, 0)).is_err());

    assert!(memory.grow(&mut store, 1).is_err());
    assert!(memory.grow(&mut store, 0).is_ok());

    println!("Creating stand-alone memory...");
    let memorytype = MemoryType::new(5, Some(5));
    let memory2 = Memory::new(&mut store, memorytype)?;
    assert_eq!(memory2.size(&store), 5);
    assert!(memory2.grow(&mut store, 1).is_err());
    assert!(memory2.grow(&mut store, 0).is_ok());

    Ok(())
}
