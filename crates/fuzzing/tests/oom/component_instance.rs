#![cfg(arc_try_new)]

use wasmtime::component::{Component, Linker};
use wasmtime::{
    Config, Engine, Instance, Module, PoolingAllocationConfig, Result, Store, error::OutOfMemory,
};
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
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = Component::new(
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
    )?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&component)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let _export = instance.get_export(&mut store, None, "id");
        Ok(())
    })
}

#[test]
fn instantiate_again_after_oom() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, "(module (func (export \"f\")))")?;
    let component = Component::new(
        &engine,
        r#"
            (component
                (core module $m
                    (func (export "id") (param i32) (result i32) (local.get 0))
                )
                (core instance $i1 (instantiate $m))
                (core instance $i2 (instantiate $m))
                (func (export "id") (param "x" s32) (result s32)
                    (canon lift (core func $i2 "id"))
                )
            )
        "#,
    )?;
    let component_pre = Linker::<()>::new(&engine).instantiate_pre(&component)?;

    let instantiate = |store: &mut Store<()>| -> Result<()> {
        Instance::new(&mut *store, &module, &[])?;
        component_pre.instantiate(&mut *store)?;
        Ok(())
    };
    OomTest::new()
        .allow_alloc_after_oom(true)
        .alloc_succeeds_after_oom(true)
        .test(|| {
            let mut store = Store::try_new(&engine, ())?;
            // If this succeeds, then bail out. Only do further things with the
            // store once this fails.
            let Err(e) = instantiate(&mut store) else {
                return Ok(());
            };
            // Only one OOM is injected per test run and allocations succeed
            // after it, so having seen it here, everything below should
            // succeed.
            assert!(e.is::<OutOfMemory>(), "{e:?}");
            let instance = Instance::new(&mut store, &module, &[]).unwrap();
            let f = instance.get_typed_func::<(), ()>(&mut store, "f").unwrap();
            f.call(&mut store, ()).unwrap();
            let instance = component_pre.instantiate(&mut store).unwrap();
            let id = instance
                .get_typed_func::<(i32,), (i32,)>(&mut store, "id")
                .unwrap();
            assert_eq!(id.call(&mut store, (42,)).unwrap(), (42,));
            Err(e)
        })
}
