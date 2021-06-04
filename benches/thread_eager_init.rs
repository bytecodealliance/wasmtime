use criterion::{criterion_group, criterion_main, Criterion};
use std::thread;
use std::time::{Duration, Instant};
use wasmtime::*;

fn measure_execution_time(c: &mut Criterion) {
    c.bench_function("lazy initialization at call", move |b| {
        let (engine, module) = test_engine();
        b.iter_custom(move |iters| {
            (0..iters)
                .into_iter()
                .map(|_| lazy_thread_instantiate(engine.clone(), module.clone()))
                .sum()
        })
    });

    c.bench_function("eager initialization", move |b| {
        let (engine, module) = test_engine();
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
    c.bench_function("call after eager initialization", move |b| {
        let (engine, module) = test_engine();
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

fn test_engine() -> (Engine, Module) {
    let pool_count = 1000;

    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 1,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: pool_count,
            memory_reservation_size: 1,
        },
    });

    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, r#"(module (memory 1) (func (export "f")))"#).unwrap();
    (engine, module)
}

fn lazy_thread_instantiate(engine: Engine, module: Module) -> Duration {
    thread::spawn(move || {
        let mut store = Store::new(&engine, ());
        let inst = Instance::new(&mut store, &module, &[]).expect("instantiate");
        let f = inst.get_func(&mut store, "f").expect("get f");
        let f = f.typed::<(), (), _>(&store).expect("type f");

        let call = Instant::now();
        f.call(&mut store, ()).expect("call f");
        call.elapsed()
    })
    .join()
    .expect("thread joins")
}

fn eager_thread_instantiate(engine: Engine, module: Module) -> (Duration, Duration) {
    thread::spawn(move || {
        let init_start = Instant::now();
        Engine::tls_eager_initialize().expect("eager init");
        let init_duration = init_start.elapsed();

        let mut store = Store::new(&engine, ());
        let inst = Instance::new(&mut store, &module, &[]).expect("instantiate");
        let f = inst.get_func(&mut store, "f").expect("get f");
        let f = f.typed::<(), (), _>(&store).expect("type f");

        let call = Instant::now();
        f.call(&mut store, ()).expect("call f");
        (init_duration, call.elapsed())
    })
    .join()
    .expect("thread joins")
}

criterion_group!(benches, measure_execution_time);
criterion_main!(benches);
