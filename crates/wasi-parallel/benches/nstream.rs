//! Compare the memory throughput of parallel and sequential executions using
//! `nstream`.
//!
//! `nstream` is a memory throughput benchmark and will likely hit memory
//! bandwidth limitations as more threads are used. Do not expect a linear speed
//! up between the parallel and sequential versions. For reference, the
//! [Parallel Research Kernels] (PRK) repository has examples of `nstream` in
//! many languages.
//!
//! [Parallel Research Kernels]: https://github.com/ParRes/Kernels
mod test_case;

use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use test_case::*;

fn bench_nstream(c: &mut Criterion) {
    let mut group = c.benchmark_group("nstream");
    measure(&mut group, Parallelism::Cpu);
    measure(&mut group, Parallelism::Sequential);
}

fn measure(group: &mut BenchmarkGroup<WallTime>, parallelism: Parallelism) {
    let name = format!("{:?}", parallelism);
    let mut test_case = TestCase::new("tests/wat/nstream.wat", default_engine(), None).unwrap();
    let num_threads = num_cpus::get() as i32;
    // The guidance for nstream is to pick a number of items equivalent to a
    // buffer that is `4 * LLC size`. Since we use 4-byte floating point numbers
    // and assuming a very tame 2MB for the LLC, we set the number of items to
    // ~8 million. Note what the nstream PRK does here:
    // https://github.com/ParRes/Kernels/blob/default/scripts/small/runopenmp#L9.
    let num_items = 8_000_000;
    let device_kind = parallelism.as_device_kind();

    let _ = test_case
        .invoke(
            "setup",
            &[num_threads.into(), num_items.into(), device_kind.into()],
        )
        .expect("failed in benchmark `setup()`");

    group.bench_function(name, |b| {
        b.iter(|| {
            let _ = test_case
                .invoke("execute", &[])
                .expect("failed in benchmark `execute()`");
        });
    });

    // We cannot invoke `finish` here since the check will necessarily fail:
    // criterion will invoke `execute` repeatedly and since the nstream work
    // includes `A[i] += A[i] ...` then the result region `A` will quickly
    // diverge from its single-run value, which is what `finish` checks.
}

criterion_group!(benches, bench_nstream);
criterion_main!(benches);
