use criterion::{criterion_group, criterion_main, Criterion};
use std::convert::TryFrom;

fn run_iter(linker: &wasmtime::Linker, module: &wasmtime::Module) {
    let instance = linker.instantiate(module).unwrap();

    let ua = "Mozilla/5.0 (X11; Linux x86_64; rv:85.0) Gecko/20100101 Firefox/85.0";

    let alloc = instance
        .get_func("alloc")
        .unwrap()
        .get2::<u32, u32, u32>()
        .unwrap();
    let ptr = alloc(ua.len() as u32, 1).unwrap() as usize;

    let memory = instance.get_memory("memory").unwrap();
    let data = unsafe { memory.data_unchecked_mut() };
    data[ptr..ptr + ua.len()].copy_from_slice(ua.as_bytes());

    let run = instance.get_func("run").unwrap();
    let result = run
        .call(&[
            wasmtime::Val::I32(i32::try_from(ptr).unwrap()),
            wasmtime::Val::I32(5),
        ])
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].i32(), Some(0));

    let dealloc = instance
        .get_func("dealloc")
        .unwrap()
        .get3::<u32, u32, u32, ()>()
        .unwrap();
    dealloc(ptr as u32, ua.len() as u32, 1).unwrap();
}

fn linker(store: &wasmtime::Store) -> wasmtime::Linker {
    let mut linker = wasmtime::Linker::new(&store);
    let ctx = wasmtime_wasi::WasiCtx::new(None::<String>).unwrap();
    let wasi = wasmtime_wasi::Wasi::new(&store, ctx);
    wasi.add_to_linker(&mut linker).unwrap();
    linker
}

fn bench_uap(c: &mut Criterion) {
    let mut group = c.benchmark_group("uap");
    group.bench_function("control", |b| {
        let store = wasmtime::Store::default();
        let module =
            wasmtime::Module::new(store.engine(), &include_bytes!("uap_bench.control.wasm"))
                .unwrap();
        let linker = linker(&store);
        b.iter(|| run_iter(&linker, &module));
    });
    group.bench_function("wizer", |b| {
        let store = wasmtime::Store::default();
        let module =
            wasmtime::Module::new(store.engine(), &include_bytes!("uap_bench.wizer.wasm")).unwrap();
        let linker = linker(&store);
        b.iter(|| run_iter(&linker, &module));
    });
    group.finish();
}

criterion_group!(benches, bench_uap);
criterion_main!(benches);
