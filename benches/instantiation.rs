use anyhow::Result;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::{prelude::*, ThreadPoolBuilder};
use std::{path::PathBuf, process::Command};
use wasmtime::*;
use wasmtime_wasi::{sync::WasiCtxBuilder, WasiCtx};

fn instantiate(linker: &Linker<WasiCtx>, module: &Module) -> Result<()> {
    let wasi = WasiCtxBuilder::new().build();
    let mut store = Store::new(module.engine(), wasi);
    let _instance = linker.instantiate(&mut store, module)?;

    Ok(())
}

fn benchmark_name<'a>(strategy: &InstanceAllocationStrategy) -> &'static str {
    match strategy {
        InstanceAllocationStrategy::OnDemand => "default",
        #[cfg(any(not(feature = "uffd"), not(target_os = "linux")))]
        InstanceAllocationStrategy::Pooling { .. } => "pooling",
        #[cfg(all(feature = "uffd", target_os = "linux"))]
        InstanceAllocationStrategy::Pooling { .. } => "uffd",
    }
}

fn bench_sequential(c: &mut Criterion, modules: &[&str]) {
    let mut group = c.benchmark_group("sequential");

    for strategy in &[
        // Skip the on-demand allocator when uffd is enabled
        #[cfg(any(not(feature = "uffd"), not(target_os = "linux")))]
        InstanceAllocationStrategy::OnDemand,
        InstanceAllocationStrategy::pooling(),
    ] {
        for file_name in modules {
            let mut path = PathBuf::new();
            path.push("benches");
            path.push("instantiation");
            path.push(file_name);

            let mut config = Config::default();
            config.allocation_strategy(strategy.clone());

            let engine = Engine::new(&config).expect("failed to create engine");
            let module = Module::from_file(&engine, &path)
                .expect(&format!("failed to load benchmark `{}`", path.display()));
            let mut linker = Linker::new(&engine);
            wasmtime_wasi::add_to_linker(&mut linker, |cx| cx).unwrap();

            group.bench_function(BenchmarkId::new(benchmark_name(strategy), file_name), |b| {
                b.iter(|| instantiate(&linker, &module).expect("failed to instantiate module"));
            });
        }
    }

    group.finish();
}

fn bench_parallel(c: &mut Criterion) {
    const PARALLEL_INSTANCES: usize = 1000;

    let mut group = c.benchmark_group("parallel");

    for strategy in &[
        // Skip the on-demand allocator when uffd is enabled
        #[cfg(any(not(feature = "uffd"), not(target_os = "linux")))]
        InstanceAllocationStrategy::OnDemand,
        InstanceAllocationStrategy::pooling(),
    ] {
        let mut config = Config::default();
        config.allocation_strategy(strategy.clone());

        let engine = Engine::new(&config).expect("failed to create engine");
        let module = Module::from_file(&engine, "benches/instantiation/wasi.wasm")
            .expect("failed to load WASI example module");
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |cx| cx).unwrap();

        for threads in 1..=num_cpus::get_physical() {
            let pool = ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();

            group.bench_function(
                BenchmarkId::new(
                    benchmark_name(strategy),
                    format!(
                        "{} instances with {} thread{}",
                        PARALLEL_INSTANCES,
                        threads,
                        if threads == 1 { "" } else { "s" }
                    ),
                ),
                |b| {
                    b.iter(|| {
                        pool.install(|| {
                            (0..PARALLEL_INSTANCES).into_par_iter().for_each(|_| {
                                instantiate(&linker, &module)
                                    .expect("failed to instantiate module");
                            })
                        })
                    });
                },
            );
        }
    }

    group.finish();
}

fn build_wasi_example() {
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

fn bench_instantiation(c: &mut Criterion) {
    build_wasi_example();
    bench_sequential(
        c,
        &[
            "empty.wat",
            "small_memory.wat",
            "data_segments.wat",
            "wasi.wasm",
        ],
    );
    bench_parallel(c);
}

criterion_group!(benches, bench_instantiation);
criterion_main!(benches);
