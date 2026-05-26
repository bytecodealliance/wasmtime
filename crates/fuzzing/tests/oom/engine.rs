#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Result};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn engine_new() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        let _ = Engine::new(&config)?;
        Ok(())
    })
}
