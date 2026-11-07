use criterion::{Criterion, criterion_group, criterion_main};
use std::thread;
use std::time::Duration;
use wasmtime::*;
use wasmtime_bench_util::Timer;

fn measure_execution_time(c: &mut Criterion) {
    // Baseline performance: a single measurement covers both initializing
    // thread local resources and executing the first call.
    //
    // The other two bench functions should sum to this duration.
    c.bench_function("lazy initialization at call", move |b| {
        let (engine, module) = test_setup();
        b.iter_custom(move |iters| {
            (0..iters)
                .map(|_| lazy_thread_instantiate(engine.clone(), module.clone()))
                .sum()
        })
    });

    // Using Engine::tls_eager_initialize: measure how long eager
    // initialization takes on a new thread.
    c.bench_function("eager initialization", move |b| {
        let (engine, module) = test_setup();
        b.iter_custom(move |iters| {
            (0..iters)
                .map(|_| eager_thread_instantiate(engine.clone(), module.clone(), Measure::Init))
                .sum()
        })
    });

    // Measure how long the first call takes on a thread after it has been
    // eagerly initialized.
    c.bench_function("call after eager initialization", move |b| {
        let (engine, module) = test_setup();
        b.iter_custom(move |iters| {
            (0..iters)
                .map(|_| eager_thread_instantiate(engine.clone(), module.clone(), Measure::Call))
                .sum()
        })
    });
}

/// Creating a store and measuring the time to perform a call is the same behavior
/// in both setups.
fn duration_of_call(engine: &Engine, module: &Module) -> Duration {
    let mut store = Store::new(engine, ());
    let inst = Instance::new(&mut store, module, &[]).expect("instantiate");
    let f = inst.get_func(&mut store, "f").expect("get f");
    let f = f.typed::<(), ()>(&store).expect("type f");

    let call = Timer::start();
    f.call(&mut store, ()).expect("call f");
    call.stop()
}

/// When wasmtime first runs a function on a thread, it needs to initialize
/// some thread-local resources and install signal handlers. This benchmark
/// spawns a new thread, and returns the duration it took to execute the first
/// function call made on that thread.
fn lazy_thread_instantiate(engine: Engine, module: Module) -> Duration {
    thread::spawn(move || duration_of_call(&engine, &module))
        .join()
        .expect("thread joins")
}
/// Which region of [`eager_thread_instantiate`] to time.
#[derive(Clone, Copy)]
enum Measure {
    Init,
    Call,
}

/// This benchmark spawns a new thread, and eagerly initializes the thread local
/// resources. For [`Measure::Init`] it returns how long that took. For
/// [`Measure::Call`] it then creates a store and instance, and returns how long
/// the first function call took. A benchmark times only one region, so that
/// under Callgrind it makes one measurement.
fn eager_thread_instantiate(engine: Engine, module: Module, measure: Measure) -> Duration {
    thread::spawn(move || match measure {
        Measure::Init => {
            let init = Timer::start();
            Engine::tls_eager_initialize();
            init.stop()
        }
        Measure::Call => {
            Engine::tls_eager_initialize();
            duration_of_call(&engine, &module)
        }
    })
    .join()
    .expect("thread joins")
}

fn test_setup() -> (Engine, Module) {
    // We only expect to create one Instance at a time, with a single memory.
    let pool_count = 10;

    let mut pool = PoolingAllocationConfig::default();
    pool.total_memories(pool_count)
        .total_stacks(pool_count)
        .total_tables(pool_count);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config).unwrap();

    // The module has a memory (shouldn't matter) and a single function which is a no-op.
    let module = Module::new(&engine, r#"(module (memory 1) (func (export "f")))"#).unwrap();
    (engine, module)
}

criterion_group!(benches, measure_execution_time);
criterion_main!(benches);
