#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, GlobalType, Mutability, Result, Store, Val, ValType};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn global_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = GlobalType::new(ValType::I32, Mutability::Var);
        let _global = wasmtime::Global::new(&mut store, ty, Val::I32(42))?;
        Ok(())
    })
}
