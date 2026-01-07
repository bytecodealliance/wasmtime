#![no_std]

#[macro_use]
extern crate alloc;

use alloc::string::ToString;
use core::ptr;
use wasmtime::{Config, Engine, Instance, Linker, Module, Result, Store, ensure};

mod allocator;
mod panic;

#[cfg(feature = "wasi")]
mod wasi;

/// Entrypoint of this embedding.
///
/// This takes a number of parameters which are the precompiled module AOT
/// images that are run for each of the various tests below. The first parameter
/// is also where to put an error string, if any, if anything fails.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(
    error_buf: *mut u8,
    error_size: usize,
    smoke_module: *const u8,
    smoke_size: usize,
    simple_add_module: *const u8,
    simple_add_size: usize,
    simple_host_fn_module: *const u8,
    simple_host_fn_size: usize,
    simple_floats_module: *const u8,
    simple_floats_size: usize,
) -> usize {
    unsafe {
        let buf = core::slice::from_raw_parts_mut(error_buf, error_size);
        let smoke = core::slice::from_raw_parts(smoke_module, smoke_size);
        let simple_add = core::slice::from_raw_parts(simple_add_module, simple_add_size);
        let simple_host_fn =
            core::slice::from_raw_parts(simple_host_fn_module, simple_host_fn_size);
        let simple_floats = core::slice::from_raw_parts(simple_floats_module, simple_floats_size);
        match run_result(smoke, simple_add, simple_host_fn, simple_floats) {
            Ok(()) => 0,
            Err(e) => {
                let msg = format!("{e:?}");
                let len = buf.len().min(msg.len());
                buf[..len].copy_from_slice(&msg.as_bytes()[..len]);
                len
            }
        }
    }
}

fn run_result(
    smoke_module: &[u8],
    simple_add_module: &[u8],
    simple_host_fn_module: &[u8],
    simple_floats_module: &[u8],
) -> Result<()> {
    smoke(smoke_module)?;
    simple_add(simple_add_module)?;
    simple_host_fn(simple_host_fn_module)?;
    simple_floats(simple_floats_module)?;
    Ok(())
}

fn config() -> Config {
    let mut config = Config::new();
    let _ = &mut config;

    #[cfg(target_arch = "x86_64")]
    {
        // This example runs in a Linux process where it's valid to use
        // floating point registers. Additionally sufficient x86 features are
        // enabled during compilation to avoid float-related libcalls. Thus
        // despite the host being configured for "soft float" it should be
        // valid to turn this on.
        unsafe {
            config.x86_float_abi_ok(true);
        }

        // To make the float ABI above OK it requires CPU features above
        // baseline to be enabled. Wasmtime needs to be able to check to ensure
        // that the feature is actually supplied at runtime, but a default check
        // isn't possible in no_std. For x86_64 we can use the cpuid instruction
        // bound through an external crate.
        //
        // Note that CPU support for these features has existed since 2013
        // (Haswell) on Intel chips and 2012 (Piledriver) on AMD chips.
        unsafe {
            config.detect_host_feature(move |feature| {
                let id = raw_cpuid::CpuId::new();
                match feature {
                    "sse3" => Some(id.get_feature_info()?.has_sse3()),
                    "ssse3" => Some(id.get_feature_info()?.has_sse3()),
                    "sse4.1" => Some(id.get_feature_info()?.has_sse41()),
                    "sse4.2" => Some(id.get_feature_info()?.has_sse42()),
                    "fma" => Some(id.get_feature_info()?.has_fma()),
                    _ => None,
                }
            });
        }
    }

    config
}

fn smoke(module: &[u8]) -> Result<()> {
    let engine = Engine::new(&config())?;
    let module = match deserialize(&engine, module)? {
        Some(module) => module,
        None => return Ok(()),
    };
    Instance::new(&mut Store::new(&engine, ()), &module, &[])?;
    Ok(())
}

fn simple_add(module: &[u8]) -> Result<()> {
    let engine = Engine::new(&config())?;
    let module = match deserialize(&engine, module)? {
        Some(module) => module,
        None => return Ok(()),
    };
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(u32, u32), u32>(&mut store, "add")?;
    ensure!(func.call(&mut store, (2, 3))? == 5);
    Ok(())
}

fn simple_host_fn(module: &[u8]) -> Result<()> {
    let engine = Engine::new(&config())?;
    let module = match deserialize(&engine, module)? {
        Some(module) => module,
        None => return Ok(()),
    };
    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("host", "multiply", |a: u32, b: u32| a.saturating_mul(b))?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(u32, u32, u32), u32>(&mut store, "add_and_mul")?;
    ensure!(func.call(&mut store, (2, 3, 4))? == 10);
    Ok(())
}

fn simple_floats(module: &[u8]) -> Result<()> {
    let engine = Engine::new(&config())?;
    let module = match deserialize(&engine, module)? {
        Some(module) => module,
        None => return Ok(()),
    };
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(f32, f32), f32>(&mut store, "frob")?;
    ensure!(func.call(&mut store, (1.4, 3.2))? == 5.);
    Ok(())
}

fn deserialize(engine: &Engine, module: &[u8]) -> Result<Option<Module>> {
    let result = if cfg!(feature = "custom") {
        // If a custom virtual memory system is in use use the raw `deserialize`
        // API to let Wasmtime handle publishing the executable and such.
        unsafe { Module::deserialize(engine, module) }
    } else {
        // NOTE: deserialize_raw avoids creating a copy of the module code. See
        // the safety notes before using in your embedding.
        //
        // Also note that this will only work for native code with a custom code
        // publisher which isn't configured in this example. Such custom code
        // publisher will need to handle making this executable for example.
        let memory_ptr = ptr::slice_from_raw_parts(module.as_ptr(), module.len());
        let module_memory = ptr::NonNull::new(memory_ptr.cast_mut()).unwrap();
        unsafe { Module::deserialize_raw(engine, module_memory) }
    };
    match result {
        Ok(module) => Ok(Some(module)),
        Err(e) => {
            // Currently if custom signals/virtual memory are disabled then this
            // example is expected to fail to load since loading native code
            // requires virtual memory. In the future this will go away as when
            // signals-based-traps is disabled then that means that the
            // interpreter should be used which should work here.
            if !cfg!(feature = "custom")
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
