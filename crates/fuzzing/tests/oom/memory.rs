#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, MemoryType, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn memory_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new()
        // `IndexMap::reserve` will try to allocate double space, but if that
        // fails, will attempt to allocate the minimal space necessary.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut store = Store::try_new(&engine, ())?;
            let _memory = wasmtime::Memory::new(&mut store, MemoryType::new(1, None))?;
            Ok(())
        })
}
