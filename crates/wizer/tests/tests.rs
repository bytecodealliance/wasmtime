use wat::parse_str as wat_to_wasm;
use wizer::Wizer;

fn run_wat(args: &[wasmtime::Val], expected: i32, wat: &str) -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let wasm = wat_to_wasm(wat)?;
    run_wasm(args, expected, &wasm)
}

fn run_wasm(args: &[wasmtime::Val], expected: i32, wasm: &[u8]) -> anyhow::Result<()> {
    let _ = env_logger::try_init();

    let mut wizer = Wizer::new();
    wizer.allow_wasi(true);
    let wasm = wizer.run(&wasm)?;

    let mut config = wasmtime::Config::new();
    config.wasm_multi_memory(true);
    config.wasm_multi_value(true);

    let engine = wasmtime::Engine::new(&config);
    let store = wasmtime::Store::new(&engine);
    let module = wasmtime::Module::new(store.engine(), wasm)?;

    let mut linker = wasmtime::Linker::new(&store);
    let ctx = wasmtime_wasi::WasiCtx::new(None::<String>)?;
    let wasi = wasmtime_wasi::Wasi::new(&store, ctx);
    wasi.add_to_linker(&mut linker)?;
    let instance = linker.instantiate(&module)?;

    let run = instance
        .get_func("run")
        .ok_or_else(|| anyhow::anyhow!("the test Wasm module does not export a `run` function"))?;

    let actual = run.call(args)?;
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

#[test]
fn basic_global() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (global $g (mut i32) i32.const 0)
  (func (export "wizer.initialize")
    i32.const 42
    global.set $g)
  (func (export "run") (result i32)
    global.get $g))
        "#,
    )
}

#[test]
fn basic_memory() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (memory 1)
  (func (export "wizer.initialize")
    i32.const 0
    i32.const 42
    i32.store offset=1337)
  (func (export "run") (result i32)
    i32.const 0
    i32.load offset=1337))
        "#,
    )
}

#[test]
fn multi_memory() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (memory $m1 1)
  (memory $m2 1)
  (func (export "wizer.initialize")
    i32.const 0
    i32.const 41
    i32.store (memory $m1) offset=1337
    i32.const 0
    i32.const 1
    i32.store (memory $m2) offset=1337)
  (func (export "run") (result i32)
    i32.const 0
    i32.load (memory $m1) offset=1337
    i32.const 0
    i32.load (memory $m2) offset=1337
    i32.add))
"#,
    )
}

#[test]
fn rust_regex() -> anyhow::Result<()> {
    run_wasm(
        &[wasmtime::Val::I32(13)],
        42,
        &include_bytes!("./regex_test.wasm")[..],
    )
}

#[test]
fn rename_functions() -> anyhow::Result<()> {
    let wat = r#"
(module
 (func (export "wizer.initialize"))
 (func (export "func_a") (result i32)
  i32.const 1)
 (func (export "func_b") (result i32)
  i32.const 2)
 (func (export "func_c") (result i32)
  i32.const 3))
  "#;

    let wasm = wat_to_wasm(wat)?;
    let mut wizer = Wizer::new();
    wizer.allow_wasi(true);
    wizer.func_rename("func_a", "func_b");
    wizer.func_rename("func_b", "func_c");
    let wasm = wizer.run(&wasm)?;

    let expected_wat = r#"
(module
  (type (;0;) (func))
  (type (;1;) (func (result i32)))
  (func (;0;) (type 0))
  (func (;1;) (type 1) (result i32)
    i32.const 1)
  (func (;2;) (type 1) (result i32)
    i32.const 2)
  (func (;3;) (type 1) (result i32)
    i32.const 3)
  (export "func_a" (func 2))
  (export "func_b" (func 3)))
  "#;

    let expected_wasm = wat_to_wasm(expected_wat)?;

    assert_eq!(expected_wasm, wasm);

    Ok(())
}
