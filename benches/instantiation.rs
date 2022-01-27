use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::{prelude::*, ThreadPoolBuilder};

// Tell rustfmt to skip this module reference; otherwise it can't seem to find it (`cargo fmt` says
// ".../wasmtime/benches does not exist".
#[rustfmt::skip]
mod common;

fn bench_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential");

    for strategy in common::strategies() {
        for file_name in common::modules() {
            let engine = common::make_engine(&strategy, false).unwrap();
            let (module, linker) = common::load_module(&engine, file_name).unwrap();

            group.bench_function(
                BenchmarkId::new(common::benchmark_name(&strategy), file_name),
                |b| {
                    b.iter(|| {
                        common::instantiate(&linker, &module).expect("failed to instantiate module")
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_parallel(c: &mut Criterion) {
    const PARALLEL_INSTANCES: usize = 1000;

    let mut group = c.benchmark_group("parallel");

    for strategy in common::strategies() {
        let engine = common::make_engine(&strategy, false).unwrap();
        let (module, linker) = common::load_module(&engine, "wasi.wasm").unwrap();

        for threads in 1..=num_cpus::get_physical() {
            let pool = ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();

            group.bench_function(
                BenchmarkId::new(
                    common::benchmark_name(&strategy),
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
                                common::instantiate(&linker, &module)
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

fn bench_instantiation(c: &mut Criterion) {
    common::build_wasi_example();
    bench_sequential(c);
    bench_parallel(c);
}

criterion_group!(benches, bench_instantiation);
criterion_main!(benches);
