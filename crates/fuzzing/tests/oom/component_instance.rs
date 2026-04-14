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

#[test]
fn component_instance_get_export() -> Result<()> {
    let component_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Component::new(
            &engine,
            r#"
                (component
                    (core module $m
                        (func (export "id") (param i32) (result i32) (local.get 0))
                    )
                    (core instance $i (instantiate $m))
                    (func (export "id") (param "x" s32) (result s32)
                        (canon lift (core func $i "id"))
                    )
                )
            "#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = unsafe { Component::deserialize(&engine, &component_bytes)? };
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&component)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let _export = instance.get_export(&mut store, None, "id");
        Ok(())
    })
}
