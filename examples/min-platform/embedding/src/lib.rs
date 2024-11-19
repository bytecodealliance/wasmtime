#![no_std]

#[macro_use]
extern crate alloc;

use alloc::string::ToString;
use anyhow::Result;
use wasmtime::{Engine, Instance, Linker, Module, Store};

mod allocator;
mod panic;

/// Entrypoint of this embedding.
///
/// This takes a number of parameters which are the precompiled module AOT
/// images that are run for each of the various tests below. The first parameter
/// is also where to put an error string, if any, if anything fails.
#[no_mangle]
pub unsafe extern "C" fn run(
    error_buf: *mut u8,
    error_size: usize,
    smoke_module: *const u8,
    smoke_size: usize,
    simple_add_module: *const u8,
    simple_add_size: usize,
    simple_host_fn_module: *const u8,
    simple_host_fn_size: usize,
) -> usize {
    let buf = core::slice::from_raw_parts_mut(error_buf, error_size);
    let smoke = core::slice::from_raw_parts(smoke_module, smoke_size);
    let simple_add = core::slice::from_raw_parts(simple_add_module, simple_add_size);
    let simple_host_fn = core::slice::from_raw_parts(simple_host_fn_module, simple_host_fn_size);
    match run_result(smoke, simple_add, simple_host_fn) {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{e:?}");
            let len = buf.len().min(msg.len());
            buf[..len].copy_from_slice(&msg.as_bytes()[..len]);
            len
        }
    }
}

fn run_result(
    smoke_module: &[u8],
    simple_add_module: &[u8],
    simple_host_fn_module: &[u8],
) -> Result<()> {
    smoke(smoke_module)?;
    simple_add(simple_add_module)?;
    simple_host_fn(simple_host_fn_module)?;
    Ok(())
}

fn smoke(module: &[u8]) -> Result<()> {
    let engine = Engine::default();
    let module = match deserialize(&engine, module)? {
        Some(module) => module,
        None => return Ok(()),
    };
    Instance::new(&mut Store::new(&engine, ()), &module, &[])?;
    Ok(())
}

fn simple_add(module: &[u8]) -> Result<()> {
    let engine = Engine::default();
    let module = match deserialize(&engine, module)? {
        Some(module) => module,
        None => return Ok(()),
    };
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(u32, u32), u32>(&mut store, "add")?;
    assert_eq!(func.call(&mut store, (2, 3))?, 5);
    Ok(())
}

fn simple_host_fn(module: &[u8]) -> Result<()> {
    let engine = Engine::default();
    let module = match deserialize(&engine, module)? {
        Some(module) => module,
        None => return Ok(()),
    };
    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("host", "multiply", |a: u32, b: u32| a.saturating_mul(b))?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(u32, u32, u32), u32>(&mut store, "add_and_mul")?;
    assert_eq!(func.call(&mut store, (2, 3, 4))?, 10);
    Ok(())
}

fn deserialize(engine: &Engine, module: &[u8]) -> Result<Option<Module>> {
    match unsafe { Module::deserialize(engine, module) } {
        Ok(module) => Ok(Some(module)),
        Err(e) => {
            // Currently if signals-based-traps are disabled then this example
            // is expected to fail to load since loading native code requires
            // virtual memory. In the future this will go away as when
            // signals-based-traps is disabled then that means that the
            // interpreter should be used which should work here.
            if !cfg!(feature = "signals-based-traps")
                && e.to_string()
                    .contains("requires virtual memory to be enabled")
            {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}
