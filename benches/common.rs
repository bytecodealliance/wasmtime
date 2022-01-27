//! Common benchmark helpers shared by multiple benchmarks.

// Not all helpers are used in all benchmarks.
#![allow(dead_code)]

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use wasmtime::*;
use wasmtime_wasi::{sync::WasiCtxBuilder, WasiCtx};

pub fn build_wasi_example() {
    println!("Building WASI example module...");
    if !Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "-p",
            "example-wasi-wasm",
            "--target",
            "wasm32-wasi",
        ])
        .spawn()
        .expect("failed to run cargo to build WASI example")
        .wait()
        .expect("failed to wait for cargo to build")
        .success()
    {
        panic!("failed to build WASI example for target `wasm32-wasi`");
    }

    std::fs::copy(
        "target/wasm32-wasi/release/wasi.wasm",
        "benches/instantiation/wasi.wasm",
    )
    .expect("failed to copy WASI example module");
}

pub fn strategies() -> Vec<InstanceAllocationStrategy> {
    vec![
        // Skip the on-demand allocator when uffd is enabled
        #[cfg(any(not(feature = "uffd"), not(target_os = "linux")))]
        InstanceAllocationStrategy::OnDemand,
        InstanceAllocationStrategy::pooling(),
    ]
}

pub fn modules() -> Vec<&'static str> {
    vec![
        "empty.wat",
        "small_memory.wat",
        "data_segments.wat",
        "wasi.wasm",
    ]
}

pub fn make_engine(strategy: &InstanceAllocationStrategy, is_async: bool) -> Result<Engine> {
    let mut config = Config::default();
    config.allocation_strategy(strategy.clone());
    config.async_support(is_async);
    Engine::new(&config)
}

pub fn load_module(engine: &Engine, module_name: &str) -> Result<(Module, Linker<WasiCtx>)> {
    let mut path = PathBuf::new();
    path.push("benches");
    path.push("instantiation");
    path.push(module_name);

    let module = Module::from_file(&engine, &path)
        .unwrap_or_else(|_| panic!("failed to load benchmark `{}`", path.display()));
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |cx| cx).unwrap();

    Ok((module, linker))
}

pub fn benchmark_name<'a>(strategy: &InstanceAllocationStrategy) -> &'static str {
    match strategy {
        InstanceAllocationStrategy::OnDemand => "default",
        #[cfg(any(not(feature = "uffd"), not(target_os = "linux")))]
        InstanceAllocationStrategy::Pooling { .. } => "pooling",
        #[cfg(all(feature = "uffd", target_os = "linux"))]
        InstanceAllocationStrategy::Pooling { .. } => "uffd",
    }
}

pub fn instantiate(linker: &Linker<WasiCtx>, module: &Module) -> Result<Instance> {
    let wasi = WasiCtxBuilder::new().build();
    let mut store = Store::new(module.engine(), wasi);
    linker.instantiate(&mut store, module)
}

pub fn instantiate_pre(linker: &Linker<WasiCtx>, module: &Module) -> Result<InstancePre<WasiCtx>> {
    let wasi = WasiCtxBuilder::new().build();
    let mut store = Store::new(module.engine(), wasi);
    linker.instantiate_pre(&mut store, module)
}
