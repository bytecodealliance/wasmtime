use criterion::{Criterion, criterion_group, criterion_main};
use std::cell::LazyCell;
use std::convert::TryFrom;
use std::pin::pin;
use std::process::Command;
use std::task::{Context, Poll, Waker};
use wasmtime_wasi::p1::WasiP1Ctx;

fn run_iter(linker: &wasmtime::Linker<WasiP1Ctx>, module: &wasmtime::Module) {
    let wasi = wasmtime_wasi::WasiCtxBuilder::new()
        .inherit_stdio()
        .build_p1();
    let mut store = wasmtime::Store::new(linker.engine(), wasi);
    let instance = linker.instantiate(&mut store, module).unwrap();

    let ua = "Mozilla/5.0 (X11; Linux x86_64; rv:85.0) Gecko/20100101 Firefox/85.0";

    let alloc = instance
        .get_typed_func::<(u32, u32), u32>(&mut store, "alloc")
        .unwrap();
    let ptr = alloc.call(&mut store, (ua.len() as u32, 1)).unwrap() as usize;

    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let data = memory.data_mut(&mut store);
    data[ptr..ptr + ua.len()].copy_from_slice(ua.as_bytes());

    let run = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "run")
        .unwrap();
    let result = run
        .call(&mut store, (i32::try_from(ptr).unwrap(), 5))
        .unwrap();
    assert_eq!(result, 0);

    let dealloc = instance
        .get_typed_func::<(u32, u32, u32), ()>(&mut store, "dealloc")
        .unwrap();
    dealloc
        .call(&mut store, (ptr as u32, ua.len() as u32, 1))
        .unwrap();
}

fn bench_uap(c: &mut Criterion) {
    let mut group = c.benchmark_group("uap");

    let control_wasm = LazyCell::new(|| {
        let status = Command::new("cargo")
            .args(&["build", "--target", "wasm32-wasip1", "--release", "-q"])
            .current_dir("./benches/uap-bench")
            .status()
            .unwrap();
        assert!(status.success());
        std::fs::read("../../target/wasm32-wasip1/release/uap_bench.wasm").unwrap()
    });
    let mut config = wasmtime::Config::new();
    config.force_memory_init_memfd(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let mut linker = wasmtime::Linker::new(&engine);
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s| s).unwrap();
    let control = LazyCell::new(|| wasmtime::Module::new(&engine, &*control_wasm).unwrap());

    group.bench_function("control", |b| {
        LazyCell::force(&control);
        b.iter(|| run_iter(&linker, &control));
    });

    let wizer = LazyCell::new(|| {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build_p1();
        let mut store = wasmtime::Store::new(linker.engine(), wasi);
        let wizened = assert_ready(wasmtime_wizer::Wizer::new().run(
            &mut store,
            &control_wasm,
            async |store, module| linker.instantiate(store, module),
        ))
        .unwrap();
        wasmtime::Module::new(&engine, &wizened).unwrap()
    });
    group.bench_function("wizer", |b| {
        LazyCell::force(&wizer);
        b.iter(|| run_iter(&linker, &wizer));
    });
    group.finish();
}

fn assert_ready<F: Future>(f: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    match pin!(f).poll(&mut context) {
        Poll::Ready(ret) => ret,
        Poll::Pending => panic!("future wasn't ready"),
    }
}

criterion_group!(benches, bench_uap);
criterion_main!(benches);
