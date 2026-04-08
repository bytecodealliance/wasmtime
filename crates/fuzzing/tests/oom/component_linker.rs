#![cfg(arc_try_new)]

use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Module, Result, Store};
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

#[test]
fn component_linker_instantiate_pre() -> Result<()> {
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
        let _instance_pre = linker.instantiate_pre(&component)?;
        Ok(())
    })
}

#[test]
fn component_linker_substituted_component_type() -> Result<()> {
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
        let _ty = linker.substituted_component_type(&component)?;
        Ok(())
    })
}

#[test]
fn component_linker_define_unknown_imports_as_traps() -> Result<()> {
    let component_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Component::new(
            &engine,
            r#"
                (component
                    (import "f" (func))
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

    // Error propagation via anyhow allocates after OOM.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker.define_unknown_imports_as_traps(&component)?;
        Ok(())
    })
}

#[test]
fn component_linker_instance_func_wrap() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    // Error propagation via anyhow allocates after OOM.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker
            .root()
            .func_wrap("f", |_cx: wasmtime::StoreContextMut<'_, ()>, (): ()| Ok(()))?;
        Ok(())
    })
}

#[test]
fn component_linker_instance_func_new() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    // Error propagation via anyhow allocates after OOM.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker
            .root()
            .func_new("f", |_cx, _func_ty, _params, _results| Ok(()))?;
        Ok(())
    })
}

#[test]
fn component_linker_instance_module() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(&engine, "(module)")?.serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    // Error propagation via anyhow allocates after OOM.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker.root().module("m", &module)?;
        Ok(())
    })
}
