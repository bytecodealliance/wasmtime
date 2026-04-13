#![cfg(arc_try_new)]

use wasmtime::component::{Component, Linker, ResourceAny};
use wasmtime::{Config, Engine, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[tokio::test]
async fn component_resource_any_resource_drop_async() -> Result<()> {
    let component_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Component::new(
            &engine,
            r#"
                (component
                    (type $t' (resource (rep i32)))
                    (export $t "t" (type $t'))

                    (core func $new (canon resource.new $t))
                    (func (export "mk") (param "r" u32) (result (own $t))
                        (canon lift (core func $new))
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
        .allow_alloc_after_oom(true)
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let instance = instance_pre.instantiate_async(&mut store).await?;
            let mk = instance.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "mk")?;
            let (resource,) = mk.call_async(&mut store, (42,)).await?;
            resource.resource_drop_async(&mut store).await?;
            Ok(())
        })
        .await
}

#[test]
fn component_resource_any_resource_drop() -> Result<()> {
    let component_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Component::new(
            &engine,
            r#"
                (component
                    (type $t' (resource (rep i32)))
                    (export $t "t" (type $t'))

                    (core func $new (canon resource.new $t))
                    (func (export "mk") (param "r" u32) (result (own $t))
                        (canon lift (core func $new))
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

    // Error propagation via anyhow allocates after OOM.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let mk = instance.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "mk")?;
        let (resource,) = mk.call(&mut store, (42,))?;
        resource.resource_drop(&mut store)?;
        Ok(())
    })
}
