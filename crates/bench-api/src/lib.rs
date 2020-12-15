//! Expose a C-compatible API for controlling the Wasmtime engine during benchmarking. The API expects very sequential
//! use:
//!  - `engine_create`
//!  - `engine_compile_module`
//!  - `engine_instantiate_module`
//!  - `engine_execute_module`
//!  - `engine_free`
//!
//! An example of this C-style usage, without error checking, is shown below:
//!
//! ```
//! use wasmtime_bench_api::*;
//! let module = wat::parse_bytes(br#"(module
//!    (func $bench_start (import "bench" "start"))
//!    (func $bench_end (import "bench" "end"))
//!    (func $start (export "_start")
//!      (call $bench_start) (i32.const 2) (i32.const 2) (i32.add) (drop) (call $bench_end))
//! )"#).unwrap();
//! let engine = unsafe { engine_create(module.as_ptr(), module.len()) };
//!
//! // Start compilation timer.
//! unsafe { engine_compile_module(engine) };
//! // End compilation timer.
//!
//! // The Wasm benchmark will expect us to provide functions to start ("bench" "start") and stop ("bench" "stop") the
//! // measurement counters/timers during execution; here we provide a no-op implementation.
//! extern "C" fn noop() {}
//!
//! // Start instantiation timer.
//! unsafe { engine_instantiate_module(engine, noop, noop) };
//! // End instantiation timer.
//!
//! // No need to start timers for the execution since, by convention, the timer functions we passed during
//! // instantiation will be called by the benchmark at the appropriate time (before and after the benchmarked section).
//! unsafe { engine_execute_module(engine) };
//!
//! unsafe { engine_free(engine) }
//! ```
use anyhow::{anyhow, Result};
use core::slice;
use std::os::raw::c_int;
use wasi_common::WasiCtxBuilder;
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::Wasi;

/// Exposes a C-compatible way of creating the engine from the bytes of a single Wasm module. This function returns a
/// pointer to an opaque structure that contains the engine's initialized state.
#[no_mangle]
pub extern "C" fn engine_create(
    wasm_bytes: *const u8,
    wasm_bytes_length: usize,
) -> *mut OpaqueEngineState {
    let wasm_bytes = unsafe { slice::from_raw_parts(wasm_bytes, wasm_bytes_length) };
    let state = Box::new(EngineState::new(wasm_bytes));
    Box::into_raw(state) as *mut _
}

/// Free the engine state allocated by this library.
#[no_mangle]
pub extern "C" fn engine_free(state: *mut OpaqueEngineState) {
    unsafe {
        Box::from_raw(state);
    }
}

/// Compile the Wasm benchmark module.
#[no_mangle]
pub extern "C" fn engine_compile_module(state: *mut OpaqueEngineState) -> c_int {
    let result = unsafe { OpaqueEngineState::convert(state) }.compile();
    to_c_error(result, "failed to compile")
}

/// Instantiate the Wasm benchmark module.
#[no_mangle]
pub extern "C" fn engine_instantiate_module(
    state: *mut OpaqueEngineState,
    bench_start: extern "C" fn(),
    bench_end: extern "C" fn(),
) -> c_int {
    let result = unsafe { OpaqueEngineState::convert(state) }.instantiate(bench_start, bench_end);
    to_c_error(result, "failed to instantiate")
}

/// Execute the Wasm benchmark module.
#[no_mangle]
pub extern "C" fn engine_execute_module(state: *mut OpaqueEngineState) -> c_int {
    let result = unsafe { OpaqueEngineState::convert(state) }.execute();
    to_c_error(result, "failed to execute")
}

/// Helper function for converting a Rust result to a C error code (0 == success). Additionally, this will print an
/// error indicating some information regarding the failure.
fn to_c_error<T>(result: Result<T>, message: &str) -> c_int {
    match result {
        Ok(_) => 0,
        Err(error) => {
            println!("{}: {:?}", message, error);
            1
        }
    }
}

/// Opaque pointer type for hiding the engine state details.
#[repr(C)]
pub struct OpaqueEngineState {
    _private: [u8; 0],
}
impl OpaqueEngineState {
    unsafe fn convert(ptr: *mut OpaqueEngineState) -> &'static mut EngineState<'static> {
        assert!(!ptr.is_null());
        &mut *(ptr as *mut EngineState)
    }
}

/// This structure contains the actual Rust implementation of the state required to manage the Wasmtime engine between
/// calls.
struct EngineState<'a> {
    bytes: &'a [u8],
    engine: Engine,
    store: Store,
    module: Option<Module>,
    instance: Option<Instance>,
}

impl<'a> EngineState<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        // TODO turn off caching?
        let mut config = Config::new();
        config.wasm_simd(true);
        let engine = Engine::new(&config);
        let store = Store::new(&engine);
        Self {
            bytes,
            engine,
            store,
            module: None,
            instance: None,
        }
    }

    fn compile(&mut self) -> Result<()> {
        self.module = Some(Module::from_binary(&self.engine, self.bytes)?);
        Ok(())
    }

    fn instantiate(
        &mut self,
        bench_start: extern "C" fn(),
        bench_end: extern "C" fn(),
    ) -> Result<()> {
        // TODO instantiate WASI modules?
        match &self.module {
            Some(module) => {
                let mut linker = Linker::new(&self.store);

                // Import a very restricted WASI environment.
                let mut cx = WasiCtxBuilder::new();
                cx.inherit_stdio();
                let cx = cx.build()?;
                let wasi = Wasi::new(linker.store(), cx);
                wasi.add_to_linker(&mut linker)?;

                // Import the specialized benchmarking functions.
                linker.func("bench", "start", move || bench_start())?;
                linker.func("bench", "end", move || bench_end())?;

                self.instance = Some(linker.instantiate(module)?);
            }
            None => panic!("compile the module before instantiating it"),
        }
        Ok(())
    }

    fn execute(&self) -> Result<()> {
        match &self.instance {
            Some(instance) => {
                let start_func = instance.get_func("_start").expect("a _start function");
                let runnable_func = start_func.get0::<()>()?;
                match runnable_func() {
                    Ok(_) => {}
                    Err(trap) => {
                        // Since _start will likely return by using the system `exit` call, we must
                        // check the trap code to see if it actually represents a successful exit.
                        let status = trap.i32_exit_status();
                        if status != Some(0) {
                            return Err(anyhow!(
                                "_start exited with a non-zero code: {}",
                                status.unwrap()
                            ));
                        }
                    }
                };
            }
            None => panic!("instantiate the module before executing it"),
        }
        Ok(())
    }
}
