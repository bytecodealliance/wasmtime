//! Simplify testing of `wasi-parallel` modules.
//!
//! Due to several factors (toolchains, APIs, etc.) the set up for running
//! `wasi-parallel` code is still quite complicated. This module helps set up
//! the required bits for testing and benchmarking.

#![allow(dead_code)]

use anyhow::{Context, Result};
use wasmtime::{Config, Engine, Func, Linker, Module, SharedMemory, Store, Val};

/// Helper structure for setting up a wasi-parallel environment.
pub struct TestCase {
    store: Store<TestEnvironment>,
    linker: Linker<TestEnvironment>,
    memory: Option<SharedMemory>,
}

impl TestCase {
    /// Compile the WebAssembly at path.
    pub fn new(path: &str, engine: Engine, import_memory: Option<SharedMemory>) -> Result<Self> {
        let _ = pretty_env_logger::try_init();

        // Import the WASI definitions.
        let mut store = Store::new(&engine, TestEnvironment::new());
        let mut linker = Linker::<TestEnvironment>::new(&engine);
        wasmtime_wasi_parallel::add_to_linker(&mut linker, |cx| &mut cx.parallel)?;
        wasmtime_wasi::add_to_linker(&mut linker, |cx| &mut cx.common)?;

        // Import the shared memory, if provided.
        if let Some(import_memory) = &import_memory {
            linker.define("", "memory", import_memory.clone())?;
        }

        // Compile the module.
        let module = Module::from_file(&engine, path)?;
        linker.module(&mut store, "", &module)?;

        // Gather up either the imported or exported memory for later use.
        let memory = if let Some(import_memory) = import_memory {
            Some(import_memory)
        } else if let Some(export) = linker.get(&mut store, "", "memory") {
            let export_memory = export
                .into_shared_memory()
                .context("expect the 'memory' export to be a shared memory")?;
            Some(export_memory)
        } else {
            None
        };

        Ok(Self {
            store,
            linker,
            memory,
        })
    }

    /// Provide access to the store.
    pub fn store(&mut self) -> &mut Store<TestEnvironment> {
        &mut self.store
    }

    /// Conveniently return the shared memory as a slice.
    pub fn memory_as_slice(&self) -> &[u8] {
        let memory = self
            .memory
            .as_ref()
            .expect("expected an imported or exported shared memory");
        unsafe { std::slice::from_raw_parts_mut(memory.data() as *mut u8, memory.data_size()) }
    }

    /// Provide a convenient way to invoke a function.
    pub fn invoke(&mut self, name: &str, args: &[Val]) -> Result<Vec<Val>> {
        let func = self.get_function(name)?;
        let num_results = func.ty(&self.store).results().len();
        let mut results = vec![Val::I32(-1); num_results];
        func.call(&mut self.store, &args, &mut results)?;
        Ok(results)
    }

    /// Retrieve the exported `name` function.
    fn get_function(&mut self, name: &str) -> Result<Func> {
        Ok(self
            .linker
            .get(&mut self.store, "", name)
            .context("unable to find function of the given name")?
            .into_func()
            .context("the export was not a function")?)
    }

    /// Retrieve the default entry function.
    fn get_default_function(&mut self) -> Result<Func> {
        self.linker.get_default(&mut self.store, "")
    }
}

pub struct TestEnvironment {
    common: wasmtime_wasi::WasiCtx,
    parallel: wasmtime_wasi_parallel::WasiParallel,
}

impl TestEnvironment {
    fn new() -> Self {
        Self {
            common: wasmtime_wasi::WasiCtxBuilder::new().inherit_stdio().build(),
            parallel: wasmtime_wasi_parallel::WasiParallel::new(),
        }
    }
}

/// Configure the engine to use the threads proposal (i.e., to enable shared
/// memory).
pub fn default_engine() -> Engine {
    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config).unwrap();
    engine
}

/// Execute the default function of a test case and return its exit code.
pub fn exec(path: &str) -> Result<i32> {
    let mut test_case = TestCase::new(path, default_engine(), None).unwrap();
    let func = test_case.get_default_function().unwrap();
    let func = func.typed::<(), i32, _>(&mut test_case.store).unwrap();
    func.call(&mut test_case.store, ())
        .context("failed to execute default function")
}

/// Helper for describing the kinds of parallel devices.
#[derive(Clone, Copy, Debug)]
pub enum Parallelism {
    Cpu,
    Sequential,
}

impl Parallelism {
    pub fn as_device_kind(&self) -> i32 {
        // These values must be kept up-to-date with
        // https://github.com/WebAssembly/wasi-parallel/blob/main/wasi-parallel.witx.
        match self {
            Parallelism::Sequential => 0,
            Parallelism::Cpu => 1,
        }
    }
}
