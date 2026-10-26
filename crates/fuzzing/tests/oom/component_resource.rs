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

/// Check that an OOM partway through mutating a store's
/// `ComponentStoreData` leaves it consistent: the instance arena, handle
/// counter, host resource table, and generation metadata.
///
/// The test function deliberately does not call into the component: any
/// error from a call poisons the store via `set_trapped`, making every later
/// entry trap with `CannotEnterComponent` by design.
#[tokio::test]
async fn component_store_data_reuse_store() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    // Own resources and a host borrow exercise both resource tables.
    let component = Component::new(
        &engine,
        r#"
            (component
                (import "r" (type $hr (sub resource)))
                (core func $hdrop (canon resource.drop $hr))

                (type $t' (resource (rep i32)))
                (export $t "t" (type $t'))
                (core func $new (canon resource.new $t))

                (core module $m
                    (import "" "hdrop" (func $hdrop (param i32)))
                    (import "" "new" (func $new (param i32) (result i32)))
                    (func (export "mk") (param i32) (result i32)
                        (call $new (local.get 0)))
                    (func (export "take") (param i32)
                        (call $hdrop (local.get 0)))
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "hdrop" (func $hdrop))
                        (export "new" (func $new))
                    ))
                ))

                (func (export "mk") (param "r" u32) (result (own $t))
                    (canon lift (core func $i "mk")))
                (func (export "take") (param "r" (borrow $hr))
                    (canon lift (core func $i "take")))
            )
        "#,
    )?;
    let mut linker = Linker::<()>::new(&engine);
    linker
        .root()
        .resource("r", ResourceType::host::<MyResource>(), |_, _| Ok(()))?;
    let instance_pre = linker.instantiate_pre(&component)?;

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_with_reuse_async(
            async || Store::try_new(&engine, ()),
            async |store: &mut Store<()>| {
                // Arena registration, plus the two-step table insert.
                let _ = instance_pre.instantiate(&mut *store)?;
                let _ = instance_pre.instantiate_async(&mut *store).await?;

                let any = ResourceAny::try_from_resource(
                    Resource::<MyResource>::new_own(9),
                    &mut *store,
                )?;
                let _typed: Resource<MyResource> = any.try_into_resource(&mut *store)?;

                let any = ResourceAny::try_from_resource(
                    Resource::<MyResource>::new_own(11),
                    &mut *store,
                )?;
                any.resource_drop_async(&mut *store).await?;

                Ok(())
            },
            async |mut store: Store<()>| {
                let instance = instance_pre.instantiate_async(&mut store).await?;

                // Bumps the handle counter and the instance's table.
                let mk = instance.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "mk")?;
                let (resource,) = mk.call_async(&mut store, (42,)).await?;
                resource.resource_drop_async(&mut store).await?;

                // Goes through the host table and generation metadata.
                let take =
                    instance.get_typed_func::<(Resource<MyResource>,), ()>(&mut store, "take")?;
                take.call_async(&mut store, (Resource::new_own(7),)).await?;

                // A dropped handle must be rejected, not resolved to some
                // other resource.
                let any = ResourceAny::try_from_resource(
                    Resource::<MyResource>::new_own(11),
                    &mut store,
                )?;
                any.resource_drop_async(&mut store).await?;
                assert!(
                    any.try_into_resource::<MyResource>(&mut store).is_err(),
                    "a dropped resource handle should no longer be valid"
                );

                let any = ResourceAny::try_from_resource(
                    Resource::<MyResource>::new_own(13),
                    &mut store,
                )?;
                let typed: Resource<MyResource> = any.try_into_resource(&mut store)?;
                assert_eq!(typed.rep(), 13);

                Ok(())
            },
        )
        .await
}
