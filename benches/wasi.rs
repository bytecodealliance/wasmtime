//! Measure some common WASI call scenarios.

use criterion::{Criterion, criterion_group, criterion_main};
use std::{fs::File, path::Path, time::Instant};
use wasi_common::{WasiCtx, sync::WasiCtxBuilder};
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};

criterion_group!(benches, bench_wasi);
criterion_main!(benches);

fn bench_wasi(c: &mut Criterion) {
    let _ = env_logger::try_init();

    // Build a zero-filled test file if it does not yet exist.
    let test_file = Path::new("benches/wasi/test.bin");
    if !test_file.is_file() {
        let file = File::create(test_file).unwrap();
        file.set_len(4096).unwrap();
    }

    // Benchmark each `*.wat` file in the `wasi` directory.
    for file in std::fs::read_dir("benches/wasi").unwrap() {
        let path = file.unwrap().path();
        if path.extension().map(|e| e == "wat").unwrap_or(false) {
            let wat = std::fs::read(&path).unwrap();
            let (mut store, run_fn) = instantiate(&wat);
            let bench_name = format!("wasi/{}", path.file_name().unwrap().to_string_lossy());
            // To avoid overhead, the module itself must iterate the expected
            // number of times in a specially-crafted `run` function (see
            // `instantiate` for details).
            c.bench_function(&bench_name, move |b| {
                b.iter_custom(|iters| {
                    let start = Instant::now();
                    let result = run_fn.call(&mut store, iters).unwrap();
                    assert_eq!(iters, result);
                    start.elapsed()
                })
            });
        }
    }
}

/// Compile and instantiate the Wasm module, returning the exported `run`
/// function. This function expects `run` to:
/// - have a single `u64` parameter indicating the number of loop iterations to
///   execute
/// - execute the body of the function for that number of loop iterations
/// - return a single `u64` indicating how many loop iterations were executed
///   (to double-check)
fn instantiate(wat: &[u8]) -> (Store<WasiCtx>, TypedFunc<u64, u64>) {
    let engine = Engine::default();
    let wasi = wasi_context();
    let mut store = Store::new(&engine, wasi);
    let module = Module::new(&engine, wat).unwrap();
    let mut linker = Linker::new(&engine);
    wasi_common::sync::add_to_linker(&mut linker, |cx| cx).unwrap();
    let instance = linker.instantiate(&mut store, &module).unwrap();
    let run = instance.get_typed_func(&mut store, "run").unwrap();
    (store, run)
}

/// Build a WASI context with some actual data to retrieve.
fn wasi_context() -> WasiCtx {
    WasiCtxBuilder::new()
        .envs(&[
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
            ("c".to_string(), "d".to_string()),
        ])
        .unwrap()
        .args(&[
            "exe".to_string(),
            "--flag1".to_string(),
            "--flag2".to_string(),
            "--flag3".to_string(),
            "--flag4".to_string(),
        ])
        .unwrap()
        .preopened_dir(
            wasi_common::sync::Dir::open_ambient_dir(
                "benches/wasi",
                wasi_common::sync::ambient_authority(),
            )
            .unwrap(),
            "/",
        )
        .unwrap()
        .build()
}
