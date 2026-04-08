#![cfg(arc_try_new)]

use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[tokio::test]
async fn component_linker_instantiate_async() -> Result<()> {
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

    OomTest::new()
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let _instance = linker.instantiate_async(&mut store, &component).await?;
            Ok(())
        })
        .await
}

#[test]
fn component_linker_instantiate() -> Result<()> {
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

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let _instance = linker.instantiate(&mut store, &component)?;
        Ok(())
    })
}
