#![no_std]

#[macro_use]
extern crate alloc;

use alloc::sync::Arc;
use wasmtime::{
    Config, CustomCodeMemory, Engine, Instance, Linker, Module, Result, Store, Trap, ensure,
};

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

/// Entrypoint for a test which is run concurrently on multiple threads.
///
/// Each invocation creates its own `Engine` and then repeatedly loads,
/// instantiates, runs, and drops the given precompiled `module`, including
/// executing some traps. The intent is to verify that when Wasmtime is not
/// using native signals for traps, separate engines on separate threads do not
/// contend on any process-global state (such as a global registry of loaded
/// code), since the default `no_std` synchronization primitives in Wasmtime
/// panic on contention.
///
/// This is only expected to work without the `custom` feature (i.e. without
/// signals-based traps).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_concurrent_module_load_test(
    error_buf: *mut u8,
    error_size: usize,
    module: *const u8,
    module_size: usize,
    iterations: usize,
) -> usize {
    unsafe {
        let buf = core::slice::from_raw_parts_mut(error_buf, error_size);
        let module = core::slice::from_raw_parts(module, module_size);
        match run_concurrent_module_load_test_result(module, iterations) {
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

fn run_concurrent_module_load_test_result(module: &[u8], iterations: usize) -> Result<()> {
    let engine = Engine::new(&config())?;

    for i in 0..iterations {
        // Load a fresh copy of the module each iteration so that its code is
        // registered and unregistered every time.
        let module = deserialize(&engine, module)?;
        let mut store = Store::new(&engine, ());
        let instance = Linker::new(&engine).instantiate(&mut store, &module)?;

        let add = instance.get_typed_func::<(u32, u32), u32>(&mut store, "add")?;
        ensure!(add.call(&mut store, (i as u32, 3))? == (i as u32).wrapping_add(3));

        let unreachable = instance.get_typed_func::<(), ()>(&mut store, "unreachable")?;
        let err = unreachable.call(&mut store, ()).unwrap_err();
        ensure!(err.downcast_ref::<Trap>() == Some(&Trap::UnreachableCodeReached));

        let load = instance.get_typed_func::<u32, u32>(&mut store, "load")?;
        ensure!(load.call(&mut store, 0)? == 0);
        let err = load.call(&mut store, 0x10000).unwrap_err();
        ensure!(err.downcast_ref::<Trap>() == Some(&Trap::MemoryOutOfBounds));
    }
    Ok(())
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

    // Without the `custom` feature Wasmtime has no virtual memory support, so
    // it can't make loaded native code executable itself. Instead the embedding
    // provides that capability through a custom code memory.
    if !cfg!(feature = "custom") {
        config.with_custom_code_memory(Some(Arc::new(CodeMemory)));
    }

    config
}

/// A `CustomCodeMemory` implementation which delegates to the
/// `embedding_{publish,unpublish}_executable` functions provided by this
/// embedding's platform (see `wasmtime-platform.c`).
///
/// Note that the code images given to this are allocated from this embedding's
/// heap, so the page-granular protection changes made here rely on the
/// allocator not sharing pages of code images with other allocations (see
/// `allocator.rs`).
struct CodeMemory;

unsafe extern "C" {
    fn embedding_publish_executable(ptr: *const u8, len: usize) -> i32;
    fn embedding_unpublish_executable(ptr: *const u8, len: usize) -> i32;
}

impl CustomCodeMemory for CodeMemory {
    fn required_alignment(&self) -> usize {
        allocator::PAGE_SIZE
    }

    fn publish_executable(&self, ptr: *const u8, len: usize) -> Result<()> {
        let rc = unsafe { embedding_publish_executable(ptr, len) };
        ensure!(rc == 0, "failed to publish executable code memory");
        Ok(())
    }

    fn unpublish_executable(&self, ptr: *const u8, len: usize) -> Result<()> {
        let rc = unsafe { embedding_unpublish_executable(ptr, len) };
        ensure!(rc == 0, "failed to unpublish executable code memory");
        Ok(())
    }
}

fn smoke(module: &[u8]) -> Result<()> {
    let engine = Engine::new(&config())?;
    let module = deserialize(&engine, module)?;
    Instance::new(&mut Store::new(&engine, ()), &module, &[])?;
    Ok(())
}

fn simple_add(module: &[u8]) -> Result<()> {
    let engine = Engine::new(&config())?;
    let module = deserialize(&engine, module)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(u32, u32), u32>(&mut store, "add")?;
    ensure!(func.call(&mut store, (2, 3))? == 5);
    Ok(())
}

fn simple_host_fn(module: &[u8]) -> Result<()> {
    let engine = Engine::new(&config())?;
    let module = deserialize(&engine, module)?;
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
    let module = deserialize(&engine, module)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(f32, f32), f32>(&mut store, "frob")?;
    ensure!(func.call(&mut store, (1.4, 3.2))? == 5.);
    Ok(())
}

fn deserialize(engine: &Engine, module: &[u8]) -> Result<Module> {
    // Note that this copies `module` into memory owned by Wasmtime and aligned
    // per `CustomCodeMemory::required_alignment`, if configured, which is then
    // made executable.
    unsafe { Module::deserialize(engine, module) }
}
