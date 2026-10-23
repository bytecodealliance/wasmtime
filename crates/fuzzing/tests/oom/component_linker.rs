#![cfg(arc_try_new)]

use wasmtime::component::{Component, Linker, ResourceType};
use wasmtime::{Config, Engine, Module, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[tokio::test]
async fn component_linker_instantiate_async() -> Result<()> {
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
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"
            (component
                (import "f" (func $f))
                (core func $f (canon lower (func $f)))
                (core module $m
                    (import "" "f" (func))
                    (func (export "id") (param i32) (result i32) (local.get 0))
                )
                (core instance $i (instantiate $m
                    (with "" (instance (export "f" (func $f))))
                ))
                (func (export "id") (param "x" s32) (result s32)
                    (canon lift (core func $i "id"))
                )
            )
        "#,
    )?;
    let mut linker = Linker::<()>::new(&engine);
    linker.root().func_wrap("f", |_, (): ()| Ok(()))?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let _instance = linker.instantiate(&mut store, &component)?;
        Ok(())
    })
}

#[test]
fn component_linker_instantiate_pre() -> Result<()> {
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

    OomTest::new().test(|| {
        let _instance_pre = linker.instantiate_pre(&component)?;
        Ok(())
    })
}

#[test]
fn component_linker_substituted_component_type() -> Result<()> {
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

    OomTest::new().test(|| {
        let _ty = linker.substituted_component_type(&component)?;
        Ok(())
    })
}

#[test]
fn component_linker_define_unknown_imports_as_traps() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"
            (component
                (import "f" (func))
            )
        "#,
    )?;

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
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, "(module)")?;

    // Error propagation via anyhow allocates after OOM.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker.root().module("m", &module)?;
        Ok(())
    })
}

#[test]
fn component_linker_instance_resource() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    // Error propagation from HostFunc::wrap allocates via anyhow.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker
            .root()
            .resource("r", ResourceType::host::<()>(), |_, _| Ok(()))?;
        Ok(())
    })
}

/// Check that an OOM partway through defining items leaves a component
/// `Linker`'s string pool and map consistent.
///
/// `define_unknown_imports_as_traps` is the worst case: a loop with no
/// rollback.
#[test]
fn component_linker_state_reuse_linker() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, "(module)")?;

    // Several imports, so the loop is long enough for an OOM to land mid-way.
    let unknown_imports = Component::new(
        &engine,
        r#"
            (component
                (import "a" (func))
                (import "b" (func (param "x" u32)))
                (import "c" (func (result u32)))
                (import "d" (func (param "x" string) (result u32)))
            )
        "#,
    )?;

    let component = Component::new(
        &engine,
        r#"
            (component
                (import "post-wrapped" (func $f))
                (core func $f (canon lower (func $f)))
                (core module $m
                    (import "" "f" (func))
                    (func (export "run") (call 0))
                )
                (core instance $i (instantiate $m
                    (with "" (instance (export "f" (func $f))))
                ))
                (func (export "run") (canon lift (core func $i "run")))
            )
        "#,
    )?;

    OomTest::new()
        // Error propagation via anyhow allocates after OOM.
        .allow_alloc_after_oom(true)
        .test_with_reuse(
            || Ok(Linker::<()>::new(&engine)),
            |linker: &mut Linker<()>| {
                // Every API that interns into the pool and inserts into the
                // map.
                let mut root = linker.root();
                root.func_wrap(
                    "wrapped",
                    |_cx: wasmtime::StoreContextMut<'_, ()>, (): ()| Ok(()),
                )?;
                root.func_new("new", |_cx, _func_ty, _params, _results| Ok(()))?;
                root.module("m", &module)?;
                root.resource("r", ResourceType::host::<()>(), |_, _| Ok(()))?;

                linker.define_unknown_imports_as_traps(&unknown_imports)?;

                Ok(())
            },
            |mut linker: Linker<()>| {
                let mut root = linker.root();
                root.func_wrap(
                    "post-wrapped",
                    |_cx: wasmtime::StoreContextMut<'_, ()>, (): ()| Ok(()),
                )?;
                root.func_new("post-new", |_cx, _func_ty, _params, _results| Ok(()))?;
                root.module("post-m", &module)?;
                root.resource("post-r", ResourceType::host::<()>(), |_, _| Ok(()))?;

                // Instantiating only succeeds if *every* import got defined.
                linker.define_unknown_imports_as_traps(&unknown_imports)?;
                let mut store = Store::try_new(&engine, ())?;
                let _ = linker.instantiate(&mut store, &unknown_imports)?;

                let instance = linker.instantiate(&mut store, &component)?;
                let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
                run.call(&mut store, ())?;

                Ok(())
            },
        )
}
