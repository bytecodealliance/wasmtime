#![cfg(arc_try_new)]

use wasmtime::component::{Component, Linker, Resource, ResourceAny, ResourceType};
use wasmtime::{Config, Engine, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[tokio::test]
async fn component_resource_any_resource_drop_async() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = Component::new(
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
    )?;
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
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = Component::new(
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
    )?;
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

struct MyResource;

#[test]
fn component_resource_any_try_from_resource() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    // Error propagation via anyhow allocates after OOM.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let resource = Resource::<MyResource>::new_own(42);
        let any = ResourceAny::try_from_resource(resource, &mut store)?;
        let _typed: Resource<MyResource> = any.try_into_resource(&mut store)?;
        Ok(())
    })
}

#[test]
fn component_resource_any_try_from_resource_after_oom() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new()
        .allow_alloc_after_oom(true)
        .alloc_succeeds_after_oom(true)
        .test(|| {
            let mut store = Store::try_new(&engine, ())?;

            // Keep converting after a conversion fails with OOM, to check that
            // the failure left the store usable.
            let mut result = Ok(());
            for rep in 0..10 {
                let resource = Resource::<MyResource>::new_own(rep);
                if let Err(e) = ResourceAny::try_from_resource(resource, &mut store) {
                    result = result.and(Err(e));
                }
            }
            result
        })
}

#[test]
fn component_resource_lift_borrow() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"
            (component
                (import "r" (type $r (sub resource)))
                (core func $drop (canon resource.drop $r))
                (core module $m
                    (import "" "drop" (func $drop (param i32)))
                    (func (export "f") (param i32) (call $drop (local.get 0)))
                )
                (core instance $i (instantiate $m
                    (with "" (instance (export "drop" (func $drop))))
                ))
                (func (export "f") (param "r" (borrow $r))
                    (canon lift (core func $i "f"))
                )
            )
        "#,
    )?;
    let mut linker = Linker::<()>::new(&engine);
    linker
        .root()
        .resource("r", ResourceType::host::<MyResource>(), |_, _| Ok(()))?;
    let instance_pre = linker.instantiate_pre(&component)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let f = instance.get_typed_func::<(Resource<MyResource>,), ()>(&mut store, "f")?;
        f.call(&mut store, (Resource::new_own(42),))?;
        Ok(())
    })
}
