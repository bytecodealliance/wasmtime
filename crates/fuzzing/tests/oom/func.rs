#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Func, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn func_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let _func = Func::try_wrap(&mut store, |x: i32| x * 2)?;
        Ok(())
    })
}
