use criterion::{Criterion, criterion_group, criterion_main};
use std::convert::TryFrom;
use wasmtime_wasi::p1::WasiP1Ctx;

fn run_iter(linker: &wasmtime::Linker<WasiP1Ctx>, module: &wasmtime::Module) {
    let wasi = wasmtime_wasi::WasiCtxBuilder::new().build_p1();
    let mut store = wasmtime::Store::new(linker.engine(), wasi);
    let instance = linker.instantiate(&mut store, module).unwrap();

    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let data = memory.data_mut(&mut store);
    let ptr = data.len() - 5;
    data[ptr..].copy_from_slice(b"hello");

    let run = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "run")
        .unwrap();
    let result = run
        .call(&mut store, (i32::try_from(ptr).unwrap(), 5))
        .unwrap();
    assert_eq!(result, 0);
}

fn bench_regex(c: &mut Criterion) {
    let mut group = c.benchmark_group("regex");
    group.bench_function("control", |b| {
        let engine = wasmtime::Engine::default();
        let module =
            wasmtime::Module::new(&engine, &include_bytes!("regex_bench.control.wasm")).unwrap();
        let mut linker = wasmtime::Linker::new(&engine);
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s| s).unwrap();

        b.iter(|| run_iter(&linker, &module));
    });
    group.bench_function("wizer", |b| {
        let engine = wasmtime::Engine::default();
        let module =
            wasmtime::Module::new(&engine, &include_bytes!("regex_bench.wizer.wasm")).unwrap();
        let mut linker = wasmtime::Linker::new(&engine);
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s| s).unwrap();

        b.iter(|| run_iter(&linker, &module));
    });
    group.finish();
}

criterion_group!(benches, bench_regex);
criterion_main!(benches);
