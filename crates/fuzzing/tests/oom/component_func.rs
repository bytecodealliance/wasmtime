#![cfg(arc_try_new)]

use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Config, Engine, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[tokio::test]
async fn component_instance_pre_instantiate_async() -> Result<()> {
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

    OomTest::new()
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let _instance = instance_pre.instantiate_async(&mut store).await?;
            Ok(())
        })
        .await
}

#[tokio::test]
async fn component_func_call_async() -> Result<()> {
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

    OomTest::new()
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let instance = instance_pre.instantiate_async(&mut store).await?;
            let func = instance.get_func(&mut store, "id")?.unwrap();
            let mut results = [Val::S32(0)];
            func.call_async(&mut store, &[Val::S32(42)], &mut results)
                .await?;
            assert_eq!(results[0], Val::S32(42));
            Ok(())
        })
        .await
}
