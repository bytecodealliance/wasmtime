use anyhow::Result;
use criterion::*;
use wasmtime::*;

criterion_main!(benches);
criterion_group!(benches, bench_traps);

fn bench_traps(c: &mut Criterion) {
    bench_multi_threaded_traps(c);
    bench_many_modules_registered_traps(c);
    bench_many_stack_frames_traps(c);
}

fn bench_multi_threaded_traps(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi-threaded-traps");

    for num_bg_threads in vec![0, 1, 2, 4, 8, 16] {
        group.throughput(Throughput::Elements(num_bg_threads));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_bg_threads),
            &num_bg_threads,
            |b, &num_bg_threads| {
                let engine = Engine::default();
                let module = module(&engine, 10).unwrap();

                b.iter_custom(|iters| {
                    let (started_sender, started_receiver) = std::sync::mpsc::channel();

                    // Spawn threads in the background doing infinite work.
                    let threads = (0..num_bg_threads)
                        .map(|_| {
                            let (done_sender, done_receiver) = std::sync::mpsc::channel();
                            let handle = std::thread::spawn({
                                let engine = engine.clone();
                                let module = module.clone();
                                let started_sender = started_sender.clone();
                                move || {
                                    let mut store = Store::new(&engine, ());
                                    let instance = Instance::new(&mut store, &module, &[]).unwrap();
                                    let f = instance
                                        .get_typed_func::<(), (), _>(&mut store, "")
                                        .unwrap();

                                    // Notify the parent thread that we are
                                    // doing background work now.
                                    started_sender.send(()).unwrap();

                                    // Keep doing background work until the
                                    // parent tells us to stop.
                                    loop {
                                        if let Ok(()) = done_receiver.try_recv() {
                                            return;
                                        }
                                        assert!(f.call(&mut store, ()).is_err());
                                    }
                                }
                            });
                            (handle, done_sender)
                        })
                        .collect::<Vec<_>>();

                    // Wait on all the threads to start up.
                    for _ in 0..num_bg_threads {
                        let _ = started_receiver.recv().unwrap();
                    }

                    let mut store = Store::new(&engine, ());
                    let instance = Instance::new(&mut store, &module, &[]).unwrap();
                    let f = instance
                        .get_typed_func::<(), (), _>(&mut store, "")
                        .unwrap();

                    // Measure how long it takes to do `iters` worth of traps
                    // while there is a bunch of background work going on.
                    let start = std::time::Instant::now();
                    for _ in 0..iters {
                        assert!(f.call(&mut store, ()).is_err());
                    }
                    let elapsed = start.elapsed();

                    // Clean up all of our background threads.
                    threads.into_iter().for_each(|(handle, done_sender)| {
                        done_sender.send(()).unwrap();
                        handle.join().unwrap();
                    });

                    elapsed
                });
            },
        );
    }

    group.finish();
}

fn bench_many_modules_registered_traps(c: &mut Criterion) {
    let mut group = c.benchmark_group("many-modules-registered-traps");

    for num_modules in vec![1, 8, 64, 512, 4096] {
        group.throughput(Throughput::Elements(num_modules));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_modules),
            &num_modules,
            |b, &num_modules| {
                let engine = Engine::default();
                let modules = (0..num_modules)
                    .map(|_| module(&engine, 10).unwrap())
                    .collect::<Vec<_>>();

                b.iter_custom(|iters| {
                    let mut store = Store::new(&engine, ());
                    let instance = Instance::new(&mut store, modules.last().unwrap(), &[]).unwrap();
                    let f = instance
                        .get_typed_func::<(), (), _>(&mut store, "")
                        .unwrap();

                    let start = std::time::Instant::now();
                    for _ in 0..iters {
                        assert!(f.call(&mut store, ()).is_err());
                    }
                    start.elapsed()
                });
            },
        );
    }

    group.finish()
}

fn bench_many_stack_frames_traps(c: &mut Criterion) {
    let mut group = c.benchmark_group("many-stack-frames-traps");

    for num_stack_frames in vec![1, 8, 64, 512] {
        group.throughput(Throughput::Elements(num_stack_frames));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_stack_frames),
            &num_stack_frames,
            |b, &num_stack_frames| {
                let engine = Engine::default();
                let module = module(&engine, num_stack_frames).unwrap();

                b.iter_custom(|iters| {
                    let mut store = Store::new(&engine, ());
                    let instance = Instance::new(&mut store, &module, &[]).unwrap();
                    let f = instance
                        .get_typed_func::<(), (), _>(&mut store, "")
                        .unwrap();

                    let start = std::time::Instant::now();
                    for _ in 0..iters {
                        assert!(f.call(&mut store, ()).is_err());
                    }
                    start.elapsed()
                });
            },
        );
    }

    group.finish()
}

fn module(engine: &Engine, num_funcs: u64) -> Result<Module> {
    let mut wat = String::new();
    wat.push_str("(module\n");
    for i in 0..num_funcs {
        let j = i + 1;
        wat.push_str(&format!("(func $f{i} call $f{j})\n"));
    }
    wat.push_str(&format!("(func $f{num_funcs} unreachable)\n"));
    wat.push_str(&format!("(export \"\" (func $f0))\n"));
    wat.push_str(")\n");

    Module::new(engine, &wat)
}
