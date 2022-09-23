//! Add 100 million integers for each available CPU.
//!
//! This benchmark measures the upper ends of the speed-up available with
//! `wasi-parallel`. Each iteration of the kernel performs CPU-bound work, so
//! the difference between the sequential and parallel versions should be
//! related to the number of cores available.

mod test_case;

use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use test_case::*;

fn bench_nstream(c: &mut Criterion) {
    let mut group = c.benchmark_group("sum");
    measure(&mut group, Parallelism::Cpu);
    measure(&mut group, Parallelism::Sequential);
}

fn measure(group: &mut BenchmarkGroup<WallTime>, parallelism: Parallelism) {
    let name = format!("{:?}", parallelism);
    let mut test_case = TestCase::new("tests/wat/sum.wat", default_engine(), None).unwrap();
    let num_threads = num_cpus::get() as i32;
    let buffer_size = 100_000_000 as i32;
    let device_kind = parallelism.as_device_kind();

    let _ = test_case
        .invoke(
            "setup",
            &[num_threads.into(), buffer_size.into(), device_kind.into()],
        )
        .expect("failed in benchmark `setup()`");

    group.bench_function(name, |b| {
        b.iter(|| {
            let _ = test_case
                .invoke("execute", &[])
                .expect("failed in benchmark `execute()`");
        });
    });

    let results = test_case
        .invoke("finish", &[])
        .expect("failed in benchmark `finish`");
    assert_eq!(results[0].i32().unwrap(), 0);
}

criterion_group!(benches, bench_nstream);
criterion_main!(benches);
