use criterion::{criterion_group, criterion_main, Criterion};
use std::thread;
use std::time::{Duration, Instant};
use wasmtime::*;

fn measure_execution_time(c: &mut Criterion) {
    // Baseline performance: a single measurment covers both initializing
    // thread local resources and executing the first call.
    //
    // The other two bench functions should sum to this duration.
    c.bench_function("lazy initialization at call", move |b| {
        let (engine, module) = test_setup();
        b.iter_custom(move |iters| {
            (0..iters)
                .into_iter()
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
                .into_iter()
                .map(|_| {
                    let (init, _call) = eager_thread_instantiate(engine.clone(), module.clone());
                    init
                })
                .sum()
        })
    });

    // Measure how long the first call takes on a thread after it has been
    // eagerly initialized.
    c.bench_function("call after eager initialization", move |b| {
        let (engine, module) = test_setup();
        b.iter_custom(move |iters| {
            (0..iters)
                .into_iter()
                .map(|_| {
                    let (_init, call) = eager_thread_instantiate(engine.clone(), module.clone());
                    call
                })
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
    let f = f.typed::<(), (), _>(&store).expect("type f");

    let call = Instant::now();
    f.call(&mut store, ()).expect("call f");
    call.elapsed()
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
/// This benchmark spawns a new thread, and records the duration to eagerly
/// initializes the thread local resources. It then creates a store and
/// instance, and records the duration it took to execute the first function
/// call.
fn eager_thread_instantiate(engine: Engine, module: Module) -> (Duration, Duration) {
    thread::spawn(move || {
        let init_start = Instant::now();
        Engine::tls_eager_initialize();
        let init_duration = init_start.elapsed();

        (init_duration, duration_of_call(&engine, &module))
    })
    .join()
    .expect("thread joins")
}

fn test_setup() -> (Engine, Module) {
    // We only expect to create one Instance at a time, with a single memory.
    let pool_count = 10;

    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: pool_count,
            memory_pages: 1,
            ..Default::default()
        },
    });
    let engine = Engine::new(&config).unwrap();

    // The module has a memory (shouldn't matter) and a single function which is a no-op.
    let module = Module::new(&engine, r#"(module (memory 1) (func (export "f")))"#).unwrap();
    (engine, module)
}

criterion_group!(benches, measure_execution_time);
criterion_main!(benches);
