#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn store_try_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    OomTest::new().test(|| {
        let _ = Store::try_new(&engine, ())?;
        Ok(())
    })
}
