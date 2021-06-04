use anyhow::Result;
use std::thread;
use std::time::{Duration, Instant};
use wasmtime::*;

#[test]
fn measure_execution_time() -> Result<()> {
    let iterations = 1000;

    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 1,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: iterations * 2,
            memory_reservation_size: 1,
        },
    });

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, r#"(module (memory 1) (func (export "f")))"#)?;

    let lazy_call_time: Duration = (0..iterations)
        .into_iter()
        .map(|_| lazy_thread_instantiate(engine.clone(), module.clone()))
        .sum();

    let (eager_init_total, eager_call_total): (Duration, Duration) = (0..iterations)
        .into_iter()
        .map(|_| eager_thread_instantiate(engine.clone(), module.clone()))
        .fold(
            (Duration::default(), Duration::default()),
            |(s1, s2), (d1, d2)| (s1 + d1, s2 + d2),
        );

    println!(
        "lazy call: {:?}, eager init: {:?}, eager call: {:?}",
        lazy_call_time, eager_init_total, eager_call_total
    );

    Ok(())
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
