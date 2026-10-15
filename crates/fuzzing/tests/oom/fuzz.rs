#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Func, Instance, Module, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

fn make_module(engine: &Engine) -> Result<Module> {
    Module::new(
        engine,
        "(module (import \"\" \"f\" (func)) (func (export \"g\") (call 0)))",
    )
}

#[test]
fn test_fuzz() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = make_module(&engine)?;

    OomTest::new().max_iters(100).fuzz(|| {
        let mut store = Store::try_new(&engine, ())?;
        let func = Func::try_wrap(&mut store, || {})?;
        let _instance = Instance::new(&mut store, &module, &[func.into()])?;
        Ok(())
    })
}

#[tokio::test]
async fn test_fuzz_async() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = make_module(&engine)?;

    OomTest::new()
        .max_iters(100)
        .fuzz_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let func = Func::try_wrap(&mut store, || {})?;
            let _instance = Instance::new_async(&mut store, &module, &[func.into()]).await?;
            Ok(())
        })
        .await
}
