use anyhow::{anyhow, Context, Result};
use std::rc::Rc;
use wat::parse_str as wat_to_wasm;
use wizer::Wizer;

fn get_wizer() -> Wizer {
    let mut wizer = Wizer::new();
    wizer
        .make_linker(Some(Rc::new(|e: &wasmtime::Engine| {
            let mut linker = wasmtime::Linker::new(e);
            linker.func_wrap("foo", "bar", |x: i32| x + 1)?;
            Ok(linker)
        })))
        .unwrap();
    wizer
}

fn run_wasm(args: &[wasmtime::Val], expected: i32, wasm: &[u8]) -> Result<()> {
    let _ = env_logger::try_init();

    let wasm = get_wizer().run(&wasm)?;
    log::debug!(
        "=== Wizened Wasm ==========================================================\n\
       {}\n\
       ===========================================================================",
        wasmprinter::print_bytes(&wasm).unwrap()
    );
    if log::log_enabled!(log::Level::Debug) {
        std::fs::write("test.wasm", &wasm).unwrap();
    }

    let mut config = wasmtime::Config::new();
    config.cache_config_load_default().unwrap();
    config.wasm_multi_memory(true);
    config.wasm_multi_value(true);

    let engine = wasmtime::Engine::new(&config)?;
    let wasi_ctx = wasi_common::sync::WasiCtxBuilder::new().build();
    let mut store = wasmtime::Store::new(&engine, wasi_ctx);
    let module =
        wasmtime::Module::new(store.engine(), wasm).context("Wasm test case failed to compile")?;

    let mut linker = wasmtime::Linker::new(&engine);
    linker.func_wrap("foo", "bar", |_: i32| -> Result<i32> {
        Err(anyhow!("shouldn't be called"))
    })?;

    let instance = linker.instantiate(&mut store, &module)?;

    let run = instance
        .get_func(&mut store, "run")
        .ok_or_else(|| anyhow::anyhow!("the test Wasm module does not export a `run` function"))?;

    let mut actual = vec![wasmtime::Val::I32(0)];
    run.call(&mut store, args, &mut actual)?;
    anyhow::ensure!(actual.len() == 1, "expected one result");
    let actual = match actual[0] {
        wasmtime::Val::I32(x) => x,
        _ => anyhow::bail!("expected an i32 result"),
    };
    anyhow::ensure!(
        expected == actual,
        "expected `{}`, found `{}`",
        expected,
        actual,
    );

    Ok(())
}

fn run_wat(args: &[wasmtime::Val], expected: i32, wat: &str) -> Result<()> {
    let _ = env_logger::try_init();
    let wasm = wat_to_wasm(wat)?;
    run_wasm(args, expected, &wasm)
}

#[test]
fn custom_linker() -> Result<()> {
    run_wat(
        &[],
        1,
        r#"
(module
  (type (func (param i32) (result i32)))
  (import "foo" "bar" (func (type 0)))
  (global $g (mut i32) (i32.const 0))
  (func (export "wizer.initialize")
    global.get $g
    call 0
    global.set $g
  )
  (func (export "run") (result i32)
    (global.get $g)
  )
)"#,
    )
}

#[test]
#[should_panic]
fn linker_and_wasi() {
    Wizer::new()
        .make_linker(Some(Rc::new(|e: &wasmtime::Engine| {
            Ok(wasmtime::Linker::new(e))
        })))
        .unwrap()
        .allow_wasi(true)
        .unwrap();
}

#[test]
#[should_panic]
fn wasi_and_linker() {
    Wizer::new()
        .allow_wasi(true)
        .unwrap()
        .make_linker(Some(Rc::new(|e: &wasmtime::Engine| {
            Ok(wasmtime::Linker::new(e))
        })))
        .unwrap();
}
