use anyhow::Result;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::cell::LazyCell;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::thread;
use wasi_common::{WasiCtx, sync::WasiCtxBuilder};
use wasmtime::*;

fn store(engine: &Engine) -> Store<WasiCtx> {
    let wasi = WasiCtxBuilder::new().build();
    Store::new(engine, wasi)
}

fn instantiate(pre: &InstancePre<WasiCtx>, engine: &Engine) -> Result<()> {
    let mut store = store(engine);
    let _instance = pre.instantiate(&mut store)?;
    Ok(())
}

fn benchmark_name(strategy: &InstanceAllocationStrategy) -> &'static str {
    match strategy {
        InstanceAllocationStrategy::OnDemand => "default",
        InstanceAllocationStrategy::Pooling { .. } => "pooling",
        _ => unreachable!(),
    }
}

fn bench_sequential(c: &mut Criterion, path: &Path) {
    let mut group = c.benchmark_group("sequential");

    for strategy in strategies() {
        let id = BenchmarkId::new(
            benchmark_name(&strategy),
            path.file_name().unwrap().to_str().unwrap(),
        );
        let state = LazyCell::new(|| {
            let mut config = Config::default();
            config.allocation_strategy(strategy.clone());

            let engine = Engine::new(&config).expect("failed to create engine");
            let module = Module::from_file(&engine, path).unwrap_or_else(|e| {
                panic!("failed to load benchmark `{}`: {:?}", path.display(), e)
            });
            let mut linker = Linker::new(&engine);
            // Add these imports so we can benchmark instantiation of Sightglass
            // benchmark programs.
            linker.func_wrap("bench", "start", || {}).unwrap();
            linker.func_wrap("bench", "end", || {}).unwrap();
            wasi_common::sync::add_to_linker(&mut linker, |cx| cx).unwrap();
            let pre = linker
                .instantiate_pre(&module)
                .expect("failed to pre-instantiate");
            (engine, pre)
        });

        group.bench_function(id, |b| {
            let (engine, pre) = &*state;
            b.iter(|| {
                instantiate(&pre, &engine).expect("failed to instantiate module");
            });
        });
    }

    group.finish();
}

fn bench_parallel(c: &mut Criterion, path: &Path) {
    let mut group = c.benchmark_group("parallel");

    for strategy in strategies() {
        let state = LazyCell::new(|| {
            let mut config = Config::default();
            config.allocation_strategy(strategy.clone());

            let engine = Engine::new(&config).expect("failed to create engine");
            let module =
                Module::from_file(&engine, path).expect("failed to load WASI example module");
            let mut linker = Linker::new(&engine);
            // Add these imports so we can benchmark instantiation of Sightglass
            // benchmark programs.
            linker.func_wrap("bench", "start", || {}).unwrap();
            linker.func_wrap("bench", "end", || {}).unwrap();
            wasi_common::sync::add_to_linker(&mut linker, |cx| cx).unwrap();
            let pre = Arc::new(
                linker
                    .instantiate_pre(&module)
                    .expect("failed to pre-instantiate"),
            );
            (engine, pre)
        });

        for threads in 1..=num_cpus::get_physical().min(16) {
            let name = format!(
                "{}: with {} thread{}",
                path.file_name().unwrap().to_str().unwrap(),
                threads,
                if threads == 1 { "" } else { "s" }
            );
            let id = BenchmarkId::new(benchmark_name(&strategy), name);
            group.bench_function(id, |b| {
                let (engine, pre) = &*state;
                // Spin up N-1 threads doing background instantiations to
                // simulate concurrent instantiations.
                let done = Arc::new(AtomicBool::new(false));
                let count = Arc::new(AtomicUsize::new(0));
                let workers = (0..threads - 1)
                    .map(|_| {
                        let pre = pre.clone();
                        let done = done.clone();
                        let engine = engine.clone();
                        let count = count.clone();
                        thread::spawn(move || {
                            count.fetch_add(1, SeqCst);
                            while !done.load(SeqCst) {
                                instantiate(&pre, &engine).unwrap();
                            }
                        })
                    })
                    .collect::<Vec<_>>();

                // Wait for our workers to all get started and have
                // instantiated their first module, at which point they'll
                // all be spinning.
                while count.load(SeqCst) != threads - 1 {
                    thread::yield_now();
                }

                // Now that our background work is configured we can
                // benchmark the amount of time it takes to instantiate this
                // module.
                b.iter(|| {
                    instantiate(&pre, &engine).expect("failed to instantiate module");
                });

                // Shut down this benchmark iteration by signalling to
                // worker threads they should exit and then wait for them to
                // have reached the exit point.
                done.store(true, SeqCst);
                for t in workers {
                    t.join().unwrap();
                }
            });
        }
    }

    group.finish();
}

fn bench_deserialize_module(c: &mut Criterion, path: &Path) {
    let mut group = c.benchmark_group("deserialize");

    let name = path.file_name().unwrap().to_str().unwrap();
    let tmpfile = tempfile::NamedTempFile::new().unwrap();
    let state = LazyCell::new(|| {
        let engine = Engine::default();
        let module = Module::from_file(&engine, path).expect("failed to load WASI example module");
        let bytes = module.serialize().unwrap();
        std::fs::write(tmpfile.path(), bytes.clone()).unwrap();
        (engine, bytes, tmpfile.path())
    });
    group.bench_function(BenchmarkId::new("deserialize", name), |b| {
        let (engine, bytes, _) = &*state;
        b.iter(|| unsafe {
            Module::deserialize(&engine, bytes).unwrap();
        });
    });
    group.bench_function(BenchmarkId::new("deserialize_file", name), |b| {
        let (engine, _, path) = &*state;
        b.iter(|| unsafe {
            Module::deserialize_file(&engine, path).unwrap();
        });
    });

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
            "wasm32-wasip1",
        ])
        .spawn()
        .expect("failed to run cargo to build WASI example")
        .wait()
        .expect("failed to wait for cargo to build")
        .success()
    {
        panic!("failed to build WASI example for target `wasm32-wasip1`");
    }

    std::fs::copy(
        "target/wasm32-wasip1/release/wasi.wasm",
        "benches/instantiation/wasi.wasm",
    )
    .expect("failed to copy WASI example module");
}

fn bench_instantiation(c: &mut Criterion) {
    build_wasi_example();

    for file in std::fs::read_dir("benches/instantiation").unwrap() {
        let path = file.unwrap().path();
        bench_sequential(c, &path);
        bench_parallel(c, &path);
        bench_deserialize_module(c, &path);
    }
}

fn strategies() -> impl Iterator<Item = InstanceAllocationStrategy> {
    [
        InstanceAllocationStrategy::OnDemand,
        InstanceAllocationStrategy::Pooling({
            let mut config = PoolingAllocationConfig::default();
            config.max_memory_size(10_000 << 16);
            config
        }),
    ]
    .into_iter()
}

criterion_group!(benches, bench_instantiation);
criterion_main!(benches);
