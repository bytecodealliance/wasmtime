use wat::parse_str as wat_to_wasm;
use wizer::Wizer;

fn run_test(expected: i32, wat: &str) -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let wasm = wat_to_wasm(wat)?;
    let wasm = Wizer::new().run(&wasm)?;
    let store = wasmtime::Store::default();
    let module = wasmtime::Module::new(store.engine(), wasm)?;
    let instance = wasmtime::Instance::new(&store, &module, &[])?;
    let run = instance
        .get_func("run")
        .ok_or_else(|| anyhow::anyhow!("the test Wasm module does not export a `run` function"))?
        .get0::<i32>()?;
    let actual = run()?;
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
    run_test(
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
    run_test(
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
