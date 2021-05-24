//! A C API for benchmarking Wasmtime's WebAssembly compilation, instantiation,
//! and execution.
//!
//! The API expects calls that match the following state machine:
//!
//! ```text
//!               |
//!               |
//!               V
//! .---> wasm_bench_create
//! |        |        |
//! |        |        |
//! |        |        V
//! |        |   wasm_bench_compile
//! |        |     |            |
//! |        |     |            |     .----.
//! |        |     |            |     |    |
//! |        |     |            V     V    |
//! |        |     |     wasm_bench_instantiate <------.
//! |        |     |            |        |             |
//! |        |     |            |        |             |
//! |        |     |            |        |             |
//! |        |     |     .------'        '-----> wasm_bench_execute
//! |        |     |     |                             |
//! |        |     |     |                             |
//! |        V     V     V                             |
//! '------ wasm_bench_free <--------------------------'
//!               |
//!               |
//!               V
//! ```
//!
//! All API calls must happen on the same thread.
//!
//! Functions which return pointers use null as an error value. Function which
//! return `int` use `0` as OK and non-zero as an error value.
//!
//! # Example
//!
//! ```
//! use std::ptr;
//! use wasmtime_bench_api::*;
//!
//! let working_dir = std::env::current_dir().unwrap().display().to_string();
//! let stdout_path = "./stdout.log";
//! let stderr_path = "./stderr.log";
//!
//! // Functions to start/end timers for compilation.
//! //
//! // The `compilation_timer` pointer configured in the `WasmBenchConfig` is
//! // passed through.
//! extern "C" fn compilation_start(timer: *mut u8) {
//!     // Start your compilation timer here.
//! }
//! extern "C" fn compilation_end(timer: *mut u8) {
//!     // End your compilation timer here.
//! }
//!
//! // Similar for instantiation.
//! extern "C" fn instantiation_start(timer: *mut u8) {
//!     // Start your instantiation timer here.
//! }
//! extern "C" fn instantiation_end(timer: *mut u8) {
//!     // End your instantiation timer here.
//! }
//!
//! // Similar for execution.
//! extern "C" fn execution_start(timer: *mut u8) {
//!     // Start your execution timer here.
//! }
//! extern "C" fn execution_end(timer: *mut u8) {
//!     // End your execution timer here.
//! }
//!
//! let config = WasmBenchConfig {
//!     working_dir_ptr: working_dir.as_ptr(),
//!     working_dir_len: working_dir.len(),
//!     stdout_path_ptr: stdout_path.as_ptr(),
//!     stdout_path_len: stdout_path.len(),
//!     stderr_path_ptr: stderr_path.as_ptr(),
//!     stderr_path_len: stderr_path.len(),
//!     stdin_path_ptr: ptr::null(),
//!     stdin_path_len: 0,
//!     compilation_timer: ptr::null_mut(),
//!     compilation_start,
//!     compilation_end,
//!     instantiation_timer: ptr::null_mut(),
//!     instantiation_start,
//!     instantiation_end,
//!     execution_timer: ptr::null_mut(),
//!     execution_start,
//!     execution_end,
//! };
//!
//! let mut bench_api = ptr::null_mut();
//! unsafe {
//!     let code = wasm_bench_create(config, &mut bench_api);
//!     assert_eq!(code, OK);
//!     assert!(!bench_api.is_null());
//! };
//!
//! let wasm = wat::parse_bytes(br#"
//!     (module
//!         (func $bench_start (import "bench" "start"))
//!         (func $bench_end (import "bench" "end"))
//!         (func $start (export "_start")
//!             call $bench_start
//!             i32.const 1
//!             i32.const 2
//!             i32.add
//!             drop
//!             call $bench_end
//!         )
//!     )
//! "#).unwrap();
//!
//! // This will call the `compilation_{start,end}` timing functions on success.
//! let code = unsafe { wasm_bench_compile(bench_api, wasm.as_ptr(), wasm.len()) };
//! assert_eq!(code, OK);
//!
//! // This will call the `instantiation_{start,end}` timing functions on success.
//! let code = unsafe { wasm_bench_instantiate(bench_api) };
//! assert_eq!(code, OK);
//!
//! // This will call the `execution_{start,end}` timing functions on success.
//! let code = unsafe { wasm_bench_execute(bench_api) };
//! assert_eq!(code, OK);
//!
//! unsafe {
//!     wasm_bench_free(bench_api);
//! }
//! ```

mod unsafe_send_sync;

use crate::unsafe_send_sync::UnsafeSendSync;
use anyhow::{anyhow, Context, Result};
use std::os::raw::{c_int, c_void};
use std::slice;
use std::{env, path::PathBuf};
use wasmtime::{Config, Engine, FuncType, Instance, Linker, Module, Store};
use wasmtime_wasi::{
    sync::{Wasi, WasiCtxBuilder},
    WasiCtx,
};

pub type ExitCode = c_int;
pub const OK: ExitCode = 0;
pub const ERR: ExitCode = -1;

// Randomize the location of heap objects to avoid accidental locality being an
// uncontrolled variable that obscures performance evaluation in our
// experiments.
#[cfg(feature = "shuffling-allocator")]
#[global_allocator]
static ALLOC: shuffling_allocator::ShufflingAllocator<std::alloc::System> =
    shuffling_allocator::wrap!(&std::alloc::System);

/// Configuration options for the benchmark.
#[repr(C)]
pub struct WasmBenchConfig {
    /// The working directory where benchmarks should be executed.
    pub working_dir_ptr: *const u8,
    pub working_dir_len: usize,

    /// The file path that should be created and used as `stdout`.
    pub stdout_path_ptr: *const u8,
    pub stdout_path_len: usize,

    /// The file path that should be created and used as `stderr`.
    pub stderr_path_ptr: *const u8,
    pub stderr_path_len: usize,

    /// The (optional) file path that should be opened and used as `stdin`. If
    /// not provided, then the WASI context will not have a `stdin` initialized.
    pub stdin_path_ptr: *const u8,
    pub stdin_path_len: usize,

    /// The functions to start and stop performance timers/counters during Wasm
    /// compilation.
    pub compilation_timer: *mut u8,
    pub compilation_start: extern "C" fn(*mut u8),
    pub compilation_end: extern "C" fn(*mut u8),

    /// The functions to start and stop performance timers/counters during Wasm
    /// instantiation.
    pub instantiation_timer: *mut u8,
    pub instantiation_start: extern "C" fn(*mut u8),
    pub instantiation_end: extern "C" fn(*mut u8),

    /// The functions to start and stop performance timers/counters during Wasm
    /// execution.
    pub execution_timer: *mut u8,
    pub execution_start: extern "C" fn(*mut u8),
    pub execution_end: extern "C" fn(*mut u8),
}

impl WasmBenchConfig {
    fn working_dir(&self) -> Result<PathBuf> {
        let working_dir =
            unsafe { std::slice::from_raw_parts(self.working_dir_ptr, self.working_dir_len) };
        let working_dir = std::str::from_utf8(working_dir)
            .context("given working directory is not valid UTF-8")?;
        Ok(working_dir.into())
    }

    fn stdout_path(&self) -> Result<PathBuf> {
        let stdout_path =
            unsafe { std::slice::from_raw_parts(self.stdout_path_ptr, self.stdout_path_len) };
        let stdout_path =
            std::str::from_utf8(stdout_path).context("given stdout path is not valid UTF-8")?;
        Ok(stdout_path.into())
    }

    fn stderr_path(&self) -> Result<PathBuf> {
        let stderr_path =
            unsafe { std::slice::from_raw_parts(self.stderr_path_ptr, self.stderr_path_len) };
        let stderr_path =
            std::str::from_utf8(stderr_path).context("given stderr path is not valid UTF-8")?;
        Ok(stderr_path.into())
    }

    fn stdin_path(&self) -> Result<Option<PathBuf>> {
        if self.stdin_path_ptr.is_null() {
            return Ok(None);
        }

        let stdin_path =
            unsafe { std::slice::from_raw_parts(self.stdin_path_ptr, self.stdin_path_len) };
        let stdin_path =
            std::str::from_utf8(stdin_path).context("given stdin path is not valid UTF-8")?;
        Ok(Some(stdin_path.into()))
    }
}

/// Exposes a C-compatible way of creating the engine from the bytes of a single
/// Wasm module.
///
/// On success, the `out_bench_ptr` is initialized to a pointer to a structure
/// that contains the engine's initialized state, and `0` is returned. On
/// failure, a non-zero status code is returned and `out_bench_ptr` is left
/// untouched.
#[no_mangle]
pub extern "C" fn wasm_bench_create(
    config: WasmBenchConfig,
    out_bench_ptr: *mut *mut c_void,
) -> ExitCode {
    let result = (|| -> Result<_> {
        let working_dir = config.working_dir()?;
        let working_dir = unsafe { cap_std::fs::Dir::open_ambient_dir(&working_dir) }
            .with_context(|| {
                format!(
                    "failed to preopen the working directory: {}",
                    working_dir.display(),
                )
            })?;

        let stdout_path = config.stdout_path()?;
        let stderr_path = config.stderr_path()?;
        let stdin_path = config.stdin_path()?;

        let state = Box::new(BenchState::new(
            config.compilation_timer,
            config.compilation_start,
            config.compilation_end,
            config.instantiation_timer,
            config.instantiation_start,
            config.instantiation_end,
            config.execution_timer,
            config.execution_start,
            config.execution_end,
            move || {
                let mut cx = WasiCtxBuilder::new();

                let stdout = std::fs::File::create(&stdout_path)
                    .with_context(|| format!("failed to create {}", stdout_path.display()))?;
                let stdout = unsafe { cap_std::fs::File::from_std(stdout) };
                let stdout = wasi_cap_std_sync::file::File::from_cap_std(stdout);
                cx = cx.stdout(Box::new(stdout));

                let stderr = std::fs::File::create(&stderr_path)
                    .with_context(|| format!("failed to create {}", stderr_path.display()))?;
                let stderr = unsafe { cap_std::fs::File::from_std(stderr) };
                let stderr = wasi_cap_std_sync::file::File::from_cap_std(stderr);
                cx = cx.stderr(Box::new(stderr));

                if let Some(stdin_path) = &stdin_path {
                    let stdin = std::fs::File::open(stdin_path)
                        .with_context(|| format!("failed to open {}", stdin_path.display()))?;
                    let stdin = unsafe { cap_std::fs::File::from_std(stdin) };
                    let stdin = wasi_cap_std_sync::file::File::from_cap_std(stdin);
                    cx = cx.stdin(Box::new(stdin));
                }

                // Allow access to the working directory so that the benchmark can read
                // its input workload(s).
                cx = cx.preopened_dir(working_dir.try_clone()?, ".")?;

                // Pass this env var along so that the benchmark program can use smaller
                // input workload(s) if it has them and that has been requested.
                if let Ok(val) = env::var("WASM_BENCH_USE_SMALL_WORKLOAD") {
                    cx = cx.env("WASM_BENCH_USE_SMALL_WORKLOAD", &val)?;
                }

                Ok(cx.build())
            },
        )?);
        Ok(Box::into_raw(state) as _)
    })();

    if let Ok(bench_ptr) = result {
        unsafe {
            assert!(!out_bench_ptr.is_null());
            *out_bench_ptr = bench_ptr;
        }
    }

    to_exit_code(result.map(|_| ()))
}

/// Free the engine state allocated by this library.
#[no_mangle]
pub extern "C" fn wasm_bench_free(state: *mut c_void) {
    assert!(!state.is_null());
    unsafe {
        Box::from_raw(state as *mut BenchState);
    }
}

/// Compile the Wasm benchmark module.
#[no_mangle]
pub extern "C" fn wasm_bench_compile(
    state: *mut c_void,
    wasm_bytes: *const u8,
    wasm_bytes_length: usize,
) -> ExitCode {
    let state = unsafe { (state as *mut BenchState).as_mut().unwrap() };
    let wasm_bytes = unsafe { slice::from_raw_parts(wasm_bytes, wasm_bytes_length) };
    let result = state.compile(wasm_bytes).context("failed to compile");
    to_exit_code(result)
}

/// Instantiate the Wasm benchmark module.
#[no_mangle]
pub extern "C" fn wasm_bench_instantiate(state: *mut c_void) -> ExitCode {
    let state = unsafe { (state as *mut BenchState).as_mut().unwrap() };
    let result = state.instantiate().context("failed to instantiate");
    to_exit_code(result)
}

/// Execute the Wasm benchmark module.
#[no_mangle]
pub extern "C" fn wasm_bench_execute(state: *mut c_void) -> ExitCode {
    let state = unsafe { (state as *mut BenchState).as_mut().unwrap() };
    let result = state.execute().context("failed to execute");
    to_exit_code(result)
}

/// Helper function for converting a Rust result to a C error code.
///
/// This will print an error indicating some information regarding the failure.
fn to_exit_code<T>(result: impl Into<Result<T>>) -> ExitCode {
    match result.into() {
        Ok(_) => OK,
        Err(error) => {
            eprintln!("{:?}", error);
            ERR
        }
    }
}

/// This structure contains the actual Rust implementation of the state required
/// to manage the Wasmtime engine between calls.
struct BenchState {
    engine: Engine,
    compilation_timer: *mut u8,
    compilation_start: extern "C" fn(*mut u8),
    compilation_end: extern "C" fn(*mut u8),
    instantiation_timer: *mut u8,
    instantiation_start: extern "C" fn(*mut u8),
    instantiation_end: extern "C" fn(*mut u8),
    make_wasi_cx: Box<dyn FnMut() -> Result<WasiCtx>>,
    module: Option<Module>,
    instance: Option<Instance>,
}

impl BenchState {
    fn new(
        compilation_timer: *mut u8,
        compilation_start: extern "C" fn(*mut u8),
        compilation_end: extern "C" fn(*mut u8),
        instantiation_timer: *mut u8,
        instantiation_start: extern "C" fn(*mut u8),
        instantiation_end: extern "C" fn(*mut u8),
        execution_timer: *mut u8,
        execution_start: extern "C" fn(*mut u8),
        execution_end: extern "C" fn(*mut u8),
        make_wasi_cx: impl FnMut() -> Result<WasiCtx> + 'static,
    ) -> Result<Self> {
        // NB: do not configure a code cache.
        let mut config = Config::new();
        config.wasm_simd(true);
        Wasi::add_to_config(&mut config);

        // Define the benchmarking start/end functions.
        let execution_timer = unsafe {
            // Safe because this bench API's contract requires that its methods
            // are only ever called from a single thread.
            UnsafeSendSync::new(execution_timer)
        };
        config.define_host_func(
            "bench",
            "start",
            FuncType::new(vec![], vec![]),
            move |_, _, _| {
                execution_start(*execution_timer.get());
                Ok(())
            },
        );
        config.define_host_func(
            "bench",
            "end",
            FuncType::new(vec![], vec![]),
            move |_, _, _| {
                execution_end(*execution_timer.get());
                Ok(())
            },
        );

        let engine = Engine::new(&config)?;

        Ok(Self {
            engine,
            compilation_timer,
            compilation_start,
            compilation_end,
            instantiation_timer,
            instantiation_start,
            instantiation_end,
            make_wasi_cx: Box::new(make_wasi_cx) as _,
            module: None,
            instance: None,
        })
    }

    fn compile(&mut self, bytes: &[u8]) -> Result<()> {
        assert!(
            self.module.is_none(),
            "create a new engine to repeat compilation"
        );

        (self.compilation_start)(self.compilation_timer);
        let module = Module::from_binary(&self.engine, bytes)?;
        (self.compilation_end)(self.compilation_timer);

        self.module = Some(module);
        Ok(())
    }

    fn instantiate(&mut self) -> Result<()> {
        let module = self
            .module
            .as_ref()
            .expect("compile the module before instantiating it");

        let wasi_cx = (self.make_wasi_cx)().context("failed to create a WASI context")?;

        // NB: Start measuring instantiation time *after* we've created the WASI
        // context, since that needs to do file I/O to setup
        // stdin/stdout/stderr.
        (self.instantiation_start)(self.instantiation_timer);

        let store = Store::new(&self.engine);
        assert!(Wasi::set_context(&store, wasi_cx).is_ok());

        let linker = Linker::new(&store);

        #[cfg(feature = "wasi-nn")]
        {
            use std::cell::RefCell;
            use std::rc::Rc;
            use wasmtime_wasi_nn::{WasiNn, WasiNnCtx};

            let wasi_nn = WasiNn::new(linker.store(), Rc::new(RefCell::new(WasiNnCtx::new()?)));
            wasi_nn.add_to_linker(&mut linker)?;
        }

        #[cfg(feature = "wasi-crypto")]
        {
            use std::cell::RefCell;
            use std::rc::Rc;
            use wasmtime_wasi_crypto::{
                WasiCryptoAsymmetricCommon, WasiCryptoCommon, WasiCryptoCtx, WasiCryptoSignatures,
                WasiCryptoSymmetric,
            };

            let cx_crypto = Rc::new(RefCell::new(WasiCryptoCtx::new()));
            WasiCryptoCommon::new(linker.store(), cx_crypto.clone()).add_to_linker(linker)?;
            WasiCryptoAsymmetricCommon::new(linker.store(), cx_crypto.clone())
                .add_to_linker(linker)?;
            WasiCryptoSignatures::new(linker.store(), cx_crypto.clone()).add_to_linker(linker)?;
            WasiCryptoSymmetric::new(linker.store(), cx_crypto).add_to_linker(linker)?;
        }

        let instance = linker.instantiate(&module)?;
        (self.instantiation_end)(self.instantiation_timer);

        self.instance = Some(instance);
        Ok(())
    }

    fn execute(&mut self) -> Result<()> {
        let instance = self
            .instance
            .take()
            .expect("instantiate the module before executing it");

        let start_func = instance.get_typed_func::<(), ()>("_start")?;
        match start_func.call(()) {
            Ok(_) => Ok(()),
            Err(trap) => {
                // Since _start will likely return by using the system `exit` call, we must
                // check the trap code to see if it actually represents a successful exit.
                match trap.i32_exit_status() {
                    Some(0) => Ok(()),
                    Some(n) => Err(anyhow!("_start exited with a non-zero code: {}", n)),
                    None => Err(anyhow!(
                        "executing the benchmark resulted in a trap: {}",
                        trap
                    )),
                }
            }
        }
    }
}
