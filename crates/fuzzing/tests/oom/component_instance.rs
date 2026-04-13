#![cfg(arc_try_new)]

use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, PoolingAllocationConfig, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn instantiate() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, "(component)")?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&component)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let _instance = instance_pre.instantiate(&mut store)?;
        Ok(())
    })
}

#[test]
fn instantiate_in_pooling_allocator() -> Result<()> {
    let mut pool_config = PoolingAllocationConfig::default();
    pool_config.total_component_instances(1);

    let mut config = Config::new();
    config.concurrency_support(false);
    config.allocation_strategy(pool_config);

    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, "(component)")?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&component)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let _instance = instance_pre.instantiate(&mut store)?;
        Ok(())
    })
}
