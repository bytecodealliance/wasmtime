use criterion::{criterion_group, criterion_main, Criterion};
use std::{sync::Arc, time::Instant};
use wasmtime::*;
use wasmtime_wasi::{sync::WasiCtxBuilder, WasiCtx};

// Tell rustfmt to skip this module reference; otherwise it can't seem to find it (`cargo fmt` says
// ".../wasmtime/benches does not exist".
#[rustfmt::skip]
mod common;

struct Server {
    permits: tokio::sync::Semaphore,
    engine: Engine,
    modules: Vec<Module>,
    instance_pres: Vec<InstancePre<WasiCtx>>,
}

impl Server {
    async fn job(self: Arc<Self>, index: usize) {
        let _permit = self.permits.acquire().await.unwrap();
        let ipre = &self.instance_pres[index % self.modules.len()];
        let wasi = WasiCtxBuilder::new().build();
        let mut store = Store::new(&self.engine, wasi);
        let instance = ipre.instantiate_async(&mut store).await.unwrap();
        let start_func = instance.get_func(&mut store, "_start").unwrap();
        start_func
            .call_async(&mut store, &[], &mut [])
            .await
            .unwrap();
    }
}

fn run_server(
    strategy: &InstanceAllocationStrategy,
    filenames: &[&str],
    occupancy: usize,
    instantiations: usize,
) {
    let engine = common::make_engine(strategy, /* async = */ true).unwrap();
    let mut instance_pres = vec![];
    let mut modules = vec![];
    for filename in filenames {
        let (module, linker) = common::load_module(&engine, filename).unwrap();
        let instance_pre = common::instantiate_pre(&linker, &module).unwrap();
        modules.push(module);
        instance_pres.push(instance_pre);
    }

    let server = Arc::new(Server {
        permits: tokio::sync::Semaphore::new(occupancy),
        engine,
        modules,
        instance_pres,
    });

    // Spawn an initial batch of jobs up to the
    let server_clone = server.clone();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        for i in 0..instantiations {
            let server = server_clone.clone();
            tokio::spawn(server.job(i));
        }
    });
}

fn bench_server(c: &mut Criterion) {
    common::build_wasi_example();

    let modules = vec!["wasi.wasm"];
    let occupancy = 1000;

    for strategy in common::strategies() {
        c.bench_function(
            &format!(
                "strategy {}, occupancy {}, benches {:?}",
                common::benchmark_name(&strategy),
                occupancy,
                modules,
            ),
            |b| {
                b.iter_custom(|iters| {
                    let start = Instant::now();
                    run_server(
                        &strategy,
                        &modules,
                        occupancy,
                        /* instantiations = */ iters as usize,
                    );
                    start.elapsed()
                });
            },
        );
    }
}

criterion_group!(benches, bench_server);
criterion_main!(benches);
