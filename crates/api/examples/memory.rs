//! Translation of the memory example

use anyhow::{bail, ensure, Context as _, Error};
use wasmtime::*;

fn get_export_memory(exports: &[Extern], i: usize) -> Result<Memory, Error> {
    if exports.len() <= i {
        bail!("> Error accessing memory export {}!", i);
    }
    Ok(exports[i]
        .memory()
        .with_context(|| format!("> Error accessing memory export {}!", i))?
        .clone())
}

fn get_export_func(exports: &[Extern], i: usize) -> Result<Func, Error> {
    if exports.len() <= i {
        bail!("> Error accessing function export {}!", i);
    }
    Ok(exports[i]
        .func()
        .with_context(|| format!("> Error accessing function export {}!", i))?
        .clone())
}

macro_rules! check {
    ($actual:expr, $expected:expr) => {
        if $actual != $expected {
            bail!("> Error on result, expected {}, got {}", $expected, $actual);
        }
    };
}

macro_rules! check_ok {
  ($func:expr, $($p:expr),*) => {
    if let Err(_) = $func.call(&[$($p.into()),*]) {
      bail!("> Error on result, expected return");
    }
  }
}

macro_rules! check_trap {
  ($func:expr, $($p:expr),*) => {
    if let Ok(_) = $func.call(&[$($p.into()),*]) {
      bail!("> Error on result, expected trap");
    }
  }
}

macro_rules! call {
  ($func:expr, $($p:expr),*) => {
    match $func.call(&[$($p.into()),*]) {
      Ok(result) => {
        let result: i32 = result[0].unwrap_i32();
        result
      }
      Err(_) => { bail!("> Error on result, expected return"); }
    }
  }
}

fn main() -> Result<(), Error> {
    // Initialize.
    println!("Initializing...");
    let store = Store::default();

    // Load binary.
    println!("Loading binary...");
    let binary = wat::parse_str(
        r#"
            (module
              (memory (export "memory") 2 3)

              (func (export "size") (result i32) (memory.size))
              (func (export "load") (param i32) (result i32)
                (i32.load8_s (local.get 0))
              )
              (func (export "store") (param i32 i32)
                (i32.store8 (local.get 0) (local.get 1))
              )

              (data (i32.const 0x1000) "\01\02\03\04")
            )
        "#,
    )?;

    // Compile.
    println!("Compiling module...");
    let module = Module::new(&store, &binary).context("> Error compiling module!")?;

    // Instantiate.
    println!("Instantiating module...");
    let instance = Instance::new(&module, &[]).context("> Error instantiating module!")?;

    // Extract export.
    println!("Extracting export...");
    let exports = instance.exports();
    ensure!(!exports.is_empty(), "> Error accessing exports!");
    let memory = get_export_memory(&exports, 0)?;
    let size_func = get_export_func(&exports, 1)?;
    let load_func = get_export_func(&exports, 2)?;
    let store_func = get_export_func(&exports, 3)?;

    // Check initial memory.
    println!("Checking memory...");
    check!(memory.size(), 2u32);
    check!(memory.data_size(), 0x20000usize);
    check!(unsafe { memory.data()[0] }, 0);
    check!(unsafe { memory.data()[0x1000] }, 1);
    check!(unsafe { memory.data()[0x1003] }, 4);

    check!(call!(size_func,), 2);
    check!(call!(load_func, 0), 0);
    check!(call!(load_func, 0x1000), 1);
    check!(call!(load_func, 0x1003), 4);
    check!(call!(load_func, 0x1ffff), 0);
    check_trap!(load_func, 0x20000);

    // Mutate memory.
    println!("Mutating memory...");
    unsafe {
        memory.data()[0x1003] = 5;
    }

    check_ok!(store_func, 0x1002, 6);
    check_trap!(store_func, 0x20000, 0);

    check!(unsafe { memory.data()[0x1002] }, 6);
    check!(unsafe { memory.data()[0x1003] }, 5);
    check!(call!(load_func, 0x1002), 6);
    check!(call!(load_func, 0x1003), 5);

    // Grow memory.
    println!("Growing memory...");
    check!(memory.grow(1), true);
    check!(memory.size(), 3u32);
    check!(memory.data_size(), 0x30000usize);

    check!(call!(load_func, 0x20000), 0);
    check_ok!(store_func, 0x20000, 0);
    check_trap!(load_func, 0x30000);
    check_trap!(store_func, 0x30000, 0);

    check!(memory.grow(1), false);
    check!(memory.grow(0), true);

    // Create stand-alone memory.
    // TODO(wasm+): Once Wasm allows multiple memories, turn this into import.
    println!("Creating stand-alone memory...");
    let memorytype = MemoryType::new(Limits::new(5, Some(5)));
    let memory2 = Memory::new(&store, memorytype);
    check!(memory2.size(), 5u32);
    check!(memory2.grow(1), false);
    check!(memory2.grow(0), true);

    // Shut down.
    println!("Shutting down...");
    drop(store);

    println!("Done.");
    Ok(())
}
