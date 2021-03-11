//! An example of how to interact with wasm memory.
//!
//! Here a small wasm module is used to show how memory is initialized, how to
//! read and write memory through the `Memory` object, and how wasm functions
//! can trap when dealing with out-of-bounds addresses.

// You can execute this example with `cargo run --example example`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    // Create our `Store` context and then compile a module and create an
    // instance from the compiled module all in one go.
    let wasmtime_store = Store::default();
    let module = Module::from_file(wasmtime_store.engine(), "examples/memory.wat")?;
    let instance = Instance::new(&wasmtime_store, &module, &[])?;

    // Load up our exports from the instance
    let memory = instance
        .get_memory("memory")
        .ok_or(anyhow::format_err!("failed to find `memory` export"))?;
    let size = instance.get_typed_func::<(), i32>("size")?;
    let load = instance.get_typed_func::<i32, i32>("load")?;
    let store = instance.get_typed_func::<(i32, i32), ()>("store")?;

    // Note that these memory reads are *unsafe* due to unknown knowledge about
    // aliasing with wasm memory. For more information about the safety
    // guarantees here and how to use `Memory` safely, see the API
    // documentation.
    println!("Checking memory...");
    assert_eq!(memory.size(), 2);
    assert_eq!(memory.data_size(), 0x20000);
    unsafe {
        assert_eq!(memory.data_unchecked_mut()[0], 0);
        assert_eq!(memory.data_unchecked_mut()[0x1000], 1);
        assert_eq!(memory.data_unchecked_mut()[0x1003], 4);
    }

    assert_eq!(size.call(())?, 2);
    assert_eq!(load.call(0)?, 0);
    assert_eq!(load.call(0x1000)?, 1);
    assert_eq!(load.call(0x1003)?, 4);
    assert_eq!(load.call(0x1ffff)?, 0);
    assert!(load.call(0x20000).is_err()); // out of bounds trap

    println!("Mutating memory...");
    unsafe {
        memory.data_unchecked_mut()[0x1003] = 5;
    }

    store.call((0x1002, 6))?;
    assert!(store.call((0x20000, 0)).is_err()); // out of bounds trap

    unsafe {
        assert_eq!(memory.data_unchecked_mut()[0x1002], 6);
        assert_eq!(memory.data_unchecked_mut()[0x1003], 5);
    }
    assert_eq!(load.call(0x1002)?, 6);
    assert_eq!(load.call(0x1003)?, 5);

    // Grow memory.
    println!("Growing memory...");
    memory.grow(1)?;
    assert_eq!(memory.size(), 3);
    assert_eq!(memory.data_size(), 0x30000);

    assert_eq!(load.call(0x20000)?, 0);
    store.call((0x20000, 0))?;
    assert!(load.call(0x30000).is_err());
    assert!(store.call((0x30000, 0)).is_err());

    assert!(memory.grow(1).is_err());
    assert!(memory.grow(0).is_ok());

    println!("Creating stand-alone memory...");
    let memorytype = MemoryType::new(Limits::new(5, Some(5)));
    let memory2 = Memory::new(&wasmtime_store, memorytype);
    assert_eq!(memory2.size(), 5);
    assert!(memory2.grow(1).is_err());
    assert!(memory2.grow(0).is_ok());

    Ok(())
}
