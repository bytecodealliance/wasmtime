#![cfg(not(miri))]

use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{Store, Trap};

#[test]
fn host_resource_types() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))
                (import "u" (type $u (sub resource)))

                (export "t1" (type $t))
                (export "t2" (type $t))
                (export "u1" (type $u))
                (export "u2" (type $u))

                (component $c
                    (import "r" (type $r (sub resource)))
                    (export "r1" (type $r))
                )
                (instance $i1 (instantiate $c (with "r" (type $t))))
                (instance $i2 (instantiate $c (with "r" (type $t))))
                (export "t3" (type $i1 "r1"))
                (export "t4" (type $i2 "r1"))
            )
        "#,
    )?;

    struct T;
    struct U;
    assert!(ResourceType::host::<T>() != ResourceType::host::<U>());

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<T>(), |_, _| Ok(()))?;
    linker
        .root()
        .resource("u", ResourceType::host::<U>(), |_, _| Ok(()))?;
    let i = linker.instantiate(&mut store, &c)?;
    let t1 = i.get_resource(&mut store, "t1").unwrap();
    let t2 = i.get_resource(&mut store, "t2").unwrap();
    let t3 = i.get_resource(&mut store, "t3").unwrap();
    let t4 = i.get_resource(&mut store, "t4").unwrap();
    let u1 = i.get_resource(&mut store, "u1").unwrap();
    let u2 = i.get_resource(&mut store, "u2").unwrap();

    assert_eq!(t1, ResourceType::host::<T>());
    assert_eq!(t2, ResourceType::host::<T>());
    assert_eq!(t3, ResourceType::host::<T>());
    assert_eq!(t4, ResourceType::host::<T>());
    assert_eq!(u1, ResourceType::host::<U>());
    assert_eq!(u2, ResourceType::host::<U>());
    Ok(())
}

#[test]
fn guest_resource_types() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (type $t (resource (rep i32)))
                (type $u (resource (rep i32)))

                (export "t1" (type $t))
                (export "t2" (type $t))
                (export "u1" (type $u))
                (export "u2" (type $u))

                (component $c
                    (import "r" (type $r (sub resource)))
                    (export "r1" (type $r))
                )
                (instance $i1 (instantiate $c (with "r" (type $t))))
                (instance $i2 (instantiate $c (with "r" (type $t))))
                (export "t3" (type $i1 "r1"))
                (export "t4" (type $i2 "r1"))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let i = linker.instantiate(&mut store, &c)?;
    let t1 = i.get_resource(&mut store, "t1").unwrap();
    let t2 = i.get_resource(&mut store, "t2").unwrap();
    let t3 = i.get_resource(&mut store, "t3").unwrap();
    let t4 = i.get_resource(&mut store, "t4").unwrap();
    let u1 = i.get_resource(&mut store, "u1").unwrap();
    let u2 = i.get_resource(&mut store, "u2").unwrap();

    assert_ne!(t1, u1);
    assert_eq!(t1, t2);
    assert_eq!(t1, t3);
    assert_eq!(t1, t4);
    assert_eq!(u1, u2);
    Ok(())
}

#[test]
fn resource_any() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (type $t' (resource (rep i32)))
                (type $u' (resource (rep i32)))

                (export $t "t" (type $t'))
                (export $u "u" (type $u'))

                (core func $t_ctor (canon resource.new $t))
                (core func $u_ctor (canon resource.new $u))

                (func (export "[constructor]t") (param "x" u32) (result (own $t))
                    (canon lift (core func $t_ctor)))
                (func (export "[constructor]u") (param "x" u32) (result (own $u))
                    (canon lift (core func $u_ctor)))

                (core func $t_drop (canon resource.drop $t))
                (core func $u_drop (canon resource.drop $u))

                (func (export "drop-t") (param "x" (own $t))
                    (canon lift (core func $t_drop)))
                (func (export "drop-u") (param "x" (own $u))
                    (canon lift (core func $u_drop)))
            )
        "#,
    )?;

    let linker = Linker::new(&engine);
    {
        let mut store = Store::new(&engine, ());
        let i = linker.instantiate(&mut store, &c)?;
        let t = i.get_resource(&mut store, "t").unwrap();
        let u = i.get_resource(&mut store, "u").unwrap();

        assert_ne!(t, u);

        let t_ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "[constructor]t")?;
        let u_ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "[constructor]u")?;
        let t_dtor = i.get_typed_func::<(ResourceAny,), ()>(&mut store, "drop-t")?;
        let u_dtor = i.get_typed_func::<(ResourceAny,), ()>(&mut store, "drop-u")?;

        let (t1,) = t_ctor.call(&mut store, (100,))?;
        t_ctor.post_return(&mut store)?;
        let (t2,) = t_ctor.call(&mut store, (200,))?;
        t_ctor.post_return(&mut store)?;
        let (u1,) = u_ctor.call(&mut store, (300,))?;
        u_ctor.post_return(&mut store)?;
        let (u2,) = u_ctor.call(&mut store, (400,))?;
        u_ctor.post_return(&mut store)?;

        assert_eq!(t1.ty(), t);
        assert_eq!(t2.ty(), t);
        assert_eq!(u1.ty(), u);
        assert_eq!(u2.ty(), u);

        u_dtor.call(&mut store, (u2,))?;
        u_dtor.post_return(&mut store)?;

        u_dtor.call(&mut store, (u1,))?;
        u_dtor.post_return(&mut store)?;

        t_dtor.call(&mut store, (t1,))?;
        t_dtor.post_return(&mut store)?;

        t_dtor.call(&mut store, (t2,))?;
        t_dtor.post_return(&mut store)?;
    }

    {
        let mut store = Store::new(&engine, ());
        let i = linker.instantiate(&mut store, &c)?;
        let t_ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "[constructor]t")?;
        let u_ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "[constructor]u")?;
        let t_dtor = i.get_typed_func::<(ResourceAny,), ()>(&mut store, "drop-t")?;

        // `t` is placed at host index 0
        let (t,) = t_ctor.call(&mut store, (100,))?;
        t_ctor.post_return(&mut store)?;
        t_dtor.call(&mut store, (t,))?;
        t_dtor.post_return(&mut store)?;

        // `u` is also placed at host index 0 since `t` was deallocated
        let (_u,) = u_ctor.call(&mut store, (100,))?;
        u_ctor.post_return(&mut store)?;

        // reuse of `t` should fail, despite it pointing to a valid resource
        assert_eq!(
            t_dtor.call(&mut store, (t,)).unwrap_err().to_string(),
            "host-owned resource is being used with the wrong type"
        );
    }

    Ok(())
}

#[test]
fn mismatch_intrinsics() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (type $t' (resource (rep i32)))
                (type $u' (resource (rep i32)))

                (export $t "t" (type $t'))
                (export $u "u" (type $u'))

                ;; note the mismatch where this is an intrinsic for `u` but
                ;; we're typing it as `t`
                (core func $t_ctor (canon resource.new $u))

                (func (export "ctor") (param "x" u32) (result (own $t))
                    (canon lift (core func $t_ctor)))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let i = Linker::new(&engine).instantiate(&mut store, &c)?;
    let ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "ctor")?;
    assert_eq!(
        ctor.call(&mut store, (100,)).unwrap_err().to_string(),
        "unknown handle index 1"
    );

    Ok(())
}

#[test]
fn mismatch_resource_types() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (type $t' (resource (rep i32)))
                (type $u' (resource (rep i32)))

                (export $t "t" (type $t'))
                (export $u "u" (type $u'))

                (core func $t_ctor (canon resource.new $t))
                (func (export "ctor") (param "x" u32) (result (own $t))
                    (canon lift (core func $t_ctor)))

                (core func $u_dtor (canon resource.drop $u))
                (func (export "dtor") (param "x" (own $u))
                    (canon lift (core func $u_dtor)))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let i = Linker::new(&engine).instantiate(&mut store, &c)?;
    let ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "ctor")?;
    let dtor = i.get_typed_func::<(ResourceAny,), ()>(&mut store, "dtor")?;

    let (t,) = ctor.call(&mut store, (100,))?;
    ctor.post_return(&mut store)?;
    assert_eq!(
        dtor.call(&mut store, (t,)).unwrap_err().to_string(),
        "mismatched resource types"
    );

    Ok(())
}

#[test]
fn drop_in_different_places() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (type $t' (resource (rep i32)))

                (export $t "t" (type $t'))

                (core func $ctor (canon resource.new $t))
                (func (export "ctor") (param "x" u32) (result (own $t))
                    (canon lift (core func $ctor)))

                (core func $dtor (canon resource.drop $t))
                (func (export "dtor1") (param "x" (own $t))
                    (canon lift (core func $dtor)))

                (component $c
                    (import "t" (type $t (sub resource)))
                    (core func $dtor (canon resource.drop $t))
                    (func (export "dtor") (param "x" (own $t))
                        (canon lift (core func $dtor)))
                )
                (instance $i1 (instantiate $c (with "t" (type $t))))
                (instance $i2 (instantiate $c (with "t" (type $t))))

                (export "dtor2" (func $i1 "dtor"))
                (export "dtor3" (func $i2 "dtor"))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let i = Linker::new(&engine).instantiate(&mut store, &c)?;
    let ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "ctor")?;
    let dtor1 = i.get_typed_func::<(ResourceAny,), ()>(&mut store, "dtor1")?;
    let dtor2 = i.get_typed_func::<(ResourceAny,), ()>(&mut store, "dtor2")?;
    let dtor3 = i.get_typed_func::<(ResourceAny,), ()>(&mut store, "dtor3")?;

    let (t,) = ctor.call(&mut store, (100,))?;
    ctor.post_return(&mut store)?;
    dtor1.call(&mut store, (t,))?;
    dtor1.post_return(&mut store)?;

    let (t,) = ctor.call(&mut store, (200,))?;
    ctor.post_return(&mut store)?;
    dtor2.call(&mut store, (t,))?;
    dtor2.post_return(&mut store)?;

    let (t,) = ctor.call(&mut store, (300,))?;
    ctor.post_return(&mut store)?;
    dtor3.call(&mut store, (t,))?;
    dtor3.post_return(&mut store)?;

    Ok(())
}

#[test]
fn drop_guest_twice() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (type $t' (resource (rep i32)))

                (export $t "t" (type $t'))

                (core func $ctor (canon resource.new $t))
                (func (export "ctor") (param "x" u32) (result (own $t))
                    (canon lift (core func $ctor)))

                (core func $dtor (canon resource.drop $t))
                (func (export "dtor") (param "x" (own $t))
                    (canon lift (core func $dtor)))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let i = Linker::new(&engine).instantiate(&mut store, &c)?;
    let ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "ctor")?;
    let dtor = i.get_typed_func::<(&ResourceAny,), ()>(&mut store, "dtor")?;

    let (t,) = ctor.call(&mut store, (100,))?;
    ctor.post_return(&mut store)?;
    dtor.call(&mut store, (&t,))?;
    dtor.post_return(&mut store)?;

    assert_eq!(
        dtor.call(&mut store, (&t,)).unwrap_err().to_string(),
        "unknown handle index 1"
    );

    Ok(())
}

#[test]
fn drop_host_twice() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))

                (core func $dtor (canon resource.drop $t))
                (func (export "dtor") (param "x" (own $t))
                    (canon lift (core func $dtor)))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    let i = linker.instantiate(&mut store, &c)?;
    let dtor = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "dtor")?;

    let t = Resource::new_own(100);
    dtor.call(&mut store, (&t,))?;
    dtor.post_return(&mut store)?;

    assert_eq!(
        dtor.call(&mut store, (&t,)).unwrap_err().to_string(),
        "host resource already consumed"
    );

    Ok(())
}

#[test]
fn manually_destroy() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t1" (type $t1 (sub resource)))

                (core module $m
                  (global $drops (mut i32) i32.const 0)
                  (global $last-drop (mut i32) i32.const 0)

                  (func (export "dtor") (param i32)
                    (global.set $drops (i32.add (global.get $drops) (i32.const 1)))
                    (global.set $last-drop (local.get 0))
                  )
                  (func (export "drops") (result i32) global.get $drops)
                  (func (export "last-drop") (result i32) global.get $last-drop)
                  (func (export "pass") (param i32) (result i32) local.get 0)
                )
                (core instance $i (instantiate $m))
                (type $t2' (resource (rep i32) (dtor (func $i "dtor"))))
                (export $t2 "t2" (type $t2'))
                (core func $ctor (canon resource.new $t2))
                (func (export "[constructor]t2") (param "rep" u32) (result (own $t2))
                  (canon lift (core func $ctor)))
                (func (export "[static]t2.drops") (result u32)
                  (canon lift (core func $i "drops")))
                (func (export "[static]t2.last-drop") (result u32)
                  (canon lift (core func $i "last-drop")))

                (func (export "t1-pass") (param "t" (own $t1)) (result (own $t1))
                  (canon lift (core func $i "pass")))
            )
        "#,
    )?;

    struct MyType;

    #[derive(Default)]
    struct Data {
        drops: u32,
        last_drop: Option<u32>,
    }

    let mut store = Store::new(&engine, Data::default());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t1", ResourceType::host::<MyType>(), |mut cx, rep| {
            let data: &mut Data = cx.data_mut();
            data.drops += 1;
            data.last_drop = Some(rep);
            Ok(())
        })?;
    let i = linker.instantiate(&mut store, &c)?;
    let t2_ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "[constructor]t2")?;
    let t2_drops = i.get_typed_func::<(), (u32,)>(&mut store, "[static]t2.drops")?;
    let t2_last_drop = i.get_typed_func::<(), (u32,)>(&mut store, "[static]t2.last-drop")?;
    let t1_pass = i.get_typed_func::<(Resource<MyType>,), (ResourceAny,)>(&mut store, "t1-pass")?;

    // Host resources can be destroyed through `resource_drop`
    let t1 = Resource::new_own(100);
    let (t1,) = t1_pass.call(&mut store, (t1,))?;
    t1_pass.post_return(&mut store)?;
    assert_eq!(store.data().drops, 0);
    assert_eq!(store.data().last_drop, None);
    t1.resource_drop(&mut store)?;
    assert_eq!(store.data().drops, 1);
    assert_eq!(store.data().last_drop, Some(100));

    // Guest resources can be destroyed through `resource_drop`
    let (t2,) = t2_ctor.call(&mut store, (200,))?;
    t2_ctor.post_return(&mut store)?;
    assert_eq!(t2_drops.call(&mut store, ())?, (0,));
    t2_drops.post_return(&mut store)?;
    assert_eq!(t2_last_drop.call(&mut store, ())?, (0,));
    t2_last_drop.post_return(&mut store)?;
    t2.resource_drop(&mut store)?;
    assert_eq!(t2_drops.call(&mut store, ())?, (1,));
    t2_drops.post_return(&mut store)?;
    assert_eq!(t2_last_drop.call(&mut store, ())?, (200,));
    t2_last_drop.post_return(&mut store)?;

    // Wires weren't crossed to drop more resources
    assert_eq!(store.data().drops, 1);
    assert_eq!(store.data().last_drop, Some(100));

    Ok(())
}

#[test]
fn dynamic_type() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t1" (type $t1 (sub resource)))
                (type $t2' (resource (rep i32)))
                (export $t2 "t2" (type $t2'))
                (core func $f (canon resource.drop $t2))

                (func (export "a") (param "x" (own $t1))
                    (canon lift (core func $f)))
                (func (export "b") (param "x" (tuple (own $t2)))
                    (canon lift (core func $f)))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t1", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    let i = linker.instantiate(&mut store, &c)?;

    let a = i.get_func(&mut store, "a").unwrap();
    let b = i.get_func(&mut store, "b").unwrap();
    let t2 = i.get_resource(&mut store, "t2").unwrap();

    let a_params = a.params(&store);
    assert_eq!(
        a_params[0],
        ("x".to_string(), Type::Own(ResourceType::host::<MyType>()))
    );
    let b_params = b.params(&store);
    match &b_params[0] {
        (name, Type::Tuple(t)) => {
            assert_eq!(name, "x");
            assert_eq!(t.types().len(), 1);
            let t0 = t.types().next().unwrap();
            assert_eq!(t0, Type::Own(t2));
        }
        _ => unreachable!(),
    }

    Ok(())
}

#[test]
fn dynamic_val() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t1" (type $t1 (sub resource)))
                (type $t2' (resource (rep i32)))
                (export $t2 "t2" (type $t2'))
                (core func $f (canon resource.new $t2))

                (core module $m
                    (func (export "pass") (param i32) (result i32)
                        (local.get 0)))
                (core instance $i (instantiate $m))

                (func (export "a") (param "x" (own $t1)) (result (own $t1))
                    (canon lift (core func $i "pass")))
                (func (export "b") (param "x" u32) (result (own $t2))
                    (canon lift (core func $f)))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t1", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    let i_pre = linker.instantiate_pre(&c)?;
    let i = i_pre.instantiate(&mut store)?;

    let a = i.get_func(&mut store, "a").unwrap();
    let a_typed = i.get_typed_func::<(Resource<MyType>,), (ResourceAny,)>(&mut store, "a")?;
    let a_typed_result =
        i.get_typed_func::<(Resource<MyType>,), (Resource<MyType>,)>(&mut store, "a")?;
    let b = i.get_func(&mut store, "b").unwrap();
    let t2 = i.get_resource(&mut store, "t2").unwrap();

    let t1 = Resource::new_own(100);
    let (t1,) = a_typed.call(&mut store, (t1,))?;
    a_typed.post_return(&mut store)?;
    assert_eq!(t1.ty(), ResourceType::host::<MyType>());

    let mut results = [Val::Bool(false)];
    a.call(&mut store, &[Val::Resource(t1)], &mut results)?;
    a.post_return(&mut store)?;
    match &results[0] {
        Val::Resource(resource) => {
            assert_eq!(resource.ty(), ResourceType::host::<MyType>());
            assert!(resource.owned());

            let resource = resource.try_into_resource::<MyType>(&mut store)?;
            assert_eq!(resource.rep(), 100);
            assert!(resource.owned());

            let resource = resource.try_into_resource_any(&mut store)?;
            assert_eq!(resource.ty(), ResourceType::host::<MyType>());
            assert!(resource.owned());
        }
        _ => unreachable!(),
    }

    let t1_any = Resource::<MyType>::new_own(100).try_into_resource_any(&mut store)?;
    let mut results = [Val::Bool(false)];
    a.call(&mut store, &[Val::Resource(t1_any)], &mut results)?;
    a.post_return(&mut store)?;
    match &results[0] {
        Val::Resource(resource) => {
            assert_eq!(resource.ty(), ResourceType::host::<MyType>());
            assert!(resource.owned());

            let resource = resource.try_into_resource::<MyType>(&mut store)?;
            assert_eq!(resource.rep(), 100);
            assert!(resource.owned());

            let resource = resource.try_into_resource_any(&mut store)?;
            assert_eq!(resource.ty(), ResourceType::host::<MyType>());
            assert!(resource.owned());
        }
        _ => unreachable!(),
    }

    let t1 = Resource::<MyType>::new_own(100)
        .try_into_resource_any(&mut store)?
        .try_into_resource(&mut store)?;
    let (t1,) = a_typed_result.call(&mut store, (t1,))?;
    a_typed_result.post_return(&mut store)?;
    assert_eq!(t1.rep(), 100);
    assert!(t1.owned());

    let t1_any = t1
        .try_into_resource_any(&mut store)?
        .try_into_resource::<MyType>(&mut store)?
        .try_into_resource_any(&mut store)?;
    let mut results = [Val::Bool(false)];
    a.call(&mut store, &[Val::Resource(t1_any)], &mut results)?;
    a.post_return(&mut store)?;
    match &results[0] {
        Val::Resource(resource) => {
            assert_eq!(resource.ty(), ResourceType::host::<MyType>());
            assert!(resource.owned());

            let resource = resource.try_into_resource::<MyType>(&mut store)?;
            assert_eq!(resource.rep(), 100);
            assert!(resource.owned());

            let resource = resource.try_into_resource_any(&mut store)?;
            assert_eq!(resource.ty(), ResourceType::host::<MyType>());
            assert!(resource.owned());
        }
        _ => unreachable!(),
    }

    b.call(&mut store, &[Val::U32(200)], &mut results)?;
    match &results[0] {
        Val::Resource(resource) => {
            assert_eq!(resource.ty(), t2);
        }
        _ => unreachable!(),
    }

    Ok(())
}

#[test]
fn cannot_reenter_during_import() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "f" (func $f))

                (core func $f (canon lower (func $f)))

                (core module $m
                    (import "" "f" (func $f))
                    (func (export "call") call $f)
                    (func (export "dtor") (param i32) unreachable)
                )

                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "f" (func $f))
                    ))
                ))

                (type $t2' (resource (rep i32) (dtor (func $i "dtor"))))
                (export $t2 "t" (type $t2'))
                (core func $ctor (canon resource.new $t2))
                (func (export "ctor") (param "x" u32) (result (own $t2))
                    (canon lift (core func $ctor)))

                (func (export "call") (canon lift (core func $i "call")))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, None);
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap("f", |mut cx, ()| {
        let data: &mut Option<ResourceAny> = cx.data_mut();
        let err = data.take().unwrap().resource_drop(cx).unwrap_err();
        assert_eq!(
            err.downcast_ref(),
            Some(&Trap::CannotEnterComponent),
            "bad error: {err:?}"
        );
        Ok(())
    })?;
    let i = linker.instantiate(&mut store, &c)?;

    let ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "ctor")?;
    let call = i.get_typed_func::<(), ()>(&mut store, "call")?;

    let (resource,) = ctor.call(&mut store, (100,))?;
    ctor.post_return(&mut store)?;
    *store.data_mut() = Some(resource);
    call.call(&mut store, ())?;

    Ok(())
}

#[test]
fn active_borrows_at_end_of_call() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))

                (core module $m
                    (func (export "f") (param i32))
                )
                (core instance $i (instantiate $m))

                (func (export "f") (param "x" (borrow $t))
                    (canon lift (core func $i "f")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    let i = linker.instantiate(&mut store, &c)?;

    let f = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f")?;

    let resource = Resource::new_own(1);
    f.call(&mut store, (&resource,))?;
    let err = f.post_return(&mut store).unwrap_err();
    assert_eq!(
        err.to_string(),
        "borrow handles still remain at the end of the call",
    );

    Ok(())
}

#[test]
fn thread_through_borrow() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))
                (import "f" (func $f (param "x" (borrow $t))))

                (core func $f (canon lower (func $f)))
                (core func $drop (canon resource.drop $t))

                (core module $m
                    (import "" "f" (func $f (param i32)))
                    (import "" "drop" (func $drop (param i32)))
                    (func (export "f2") (param i32)
                        (call $f (local.get 0))
                        (call $f (local.get 0))
                        (call $drop (local.get 0))
                    )
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "f" (func $f))
                        (export "drop" (func $drop))
                    ))
                ))

                (func (export "f2") (param "x" (borrow $t))
                    (canon lift (core func $i "f2")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    linker
        .root()
        .func_wrap("f", |_cx, (r,): (Resource<MyType>,)| {
            assert!(!r.owned());
            assert_eq!(r.rep(), 100);
            Ok(())
        })?;
    let i = linker.instantiate(&mut store, &c)?;

    let f = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f2")?;

    let resource = Resource::new_own(100);
    f.call(&mut store, (&resource,))?;
    f.post_return(&mut store)?;
    Ok(())
}

#[test]
fn cannot_use_borrow_for_own() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))

                (core module $m
                    (func (export "f") (param i32) (result i32)
                        local.get 0
                    )
                )
                (core instance $i (instantiate $m))

                (func (export "f") (param "x" (borrow $t)) (result (own $t))
                    (canon lift (core func $i "f")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    let i = linker.instantiate(&mut store, &c)?;

    let f = i.get_typed_func::<(&Resource<MyType>,), (Resource<MyType>,)>(&mut store, "f")?;

    let resource = Resource::new_own(100);
    let err = f.call(&mut store, (&resource,)).unwrap_err();
    assert_eq!(err.to_string(), "cannot lift own resource from a borrow");
    Ok(())
}

#[test]
fn can_use_own_for_borrow() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))

                (core func $drop (canon resource.drop $t))

                (core module $m
                    (import "" "drop" (func $drop (param i32)))
                    (func (export "f") (param i32)
                        (call $drop (local.get 0))
                    )
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "drop" (func $drop))
                    ))
                ))

                (func (export "f") (param "x" (borrow $t))
                    (canon lift (core func $i "f")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    let i_pre = linker.instantiate_pre(&c)?;
    let i = i_pre.instantiate(&mut store)?;

    let f = i.get_func(&mut store, "f").unwrap();
    let f_typed = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f")?;

    let resource = Resource::new_own(100);
    f_typed.call(&mut store, (&resource,))?;
    f_typed.post_return(&mut store)?;

    let resource = Resource::new_borrow(200);
    f_typed.call(&mut store, (&resource,))?;
    f_typed.post_return(&mut store)?;

    let resource = Resource::<MyType>::new_own(300).try_into_resource_any(&mut store)?;
    f.call(&mut store, &[Val::Resource(resource)], &mut [])?;
    f.post_return(&mut store)?;
    resource.resource_drop(&mut store)?;

    // TODO: Enable once https://github.com/bytecodealliance/wasmtime/issues/7793 is fixed
    //let resource =
    //    Resource::<MyType>::new_borrow(400).try_into_resource_any(&mut store, &i_pre, ty_idx)?;
    //f.call(&mut store, &[Val::Resource(resource)], &mut [])?;
    //f.post_return(&mut store)?;
    //resource.resource_drop(&mut store)?;

    Ok(())
}

#[test]
fn passthrough_wrong_type() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))
                (import "f" (func $f (param "a" (borrow $t)) (result (own $t))))

                (core func $f (canon lower (func $f)))

                (core module $m
                    (import "" "f" (func $f (param i32) (result i32)))
                    (func (export "f2") (param i32)
                        (drop (call $f (local.get 0)))
                    )
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "f" (func $f))
                    ))
                ))

                (func (export "f2") (param "x" (borrow $t))
                    (canon lift (core func $i "f2")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    linker
        .root()
        .func_wrap("f", |_cx, (r,): (Resource<MyType>,)| Ok((r,)))?;
    let i = linker.instantiate(&mut store, &c)?;

    let f = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f2")?;

    let resource = Resource::new_own(100);
    let err = f.call(&mut store, (&resource,)).unwrap_err();
    assert!(
        format!("{err:?}").contains("cannot lower a `borrow` resource into an `own`"),
        "bad error: {err:?}"
    );
    Ok(())
}

#[test]
fn pass_moved_resource() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))
                (core module $m
                    (func (export "f") (param i32 i32))
                )
                (core instance $i (instantiate $m))

                (func (export "f") (param "x" (own $t)) (param "y" (borrow $t))
                    (canon lift (core func $i "f")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    let i = linker.instantiate(&mut store, &c)?;

    let f = i.get_typed_func::<(&Resource<MyType>, &Resource<MyType>), ()>(&mut store, "f")?;

    let resource = Resource::new_own(100);
    let err = f.call(&mut store, (&resource, &resource)).unwrap_err();
    assert!(
        format!("{err:?}").contains("host resource already consumed"),
        "bad error: {err:?}"
    );
    Ok(())
}

#[test]
fn type_mismatch() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (type $t' (resource (rep i32)))
                (export $t "t" (type $t'))

                (core func $drop (canon resource.drop $t))

                (func (export "f1") (param "x" (own $t))
                    (canon lift (core func $drop)))
                (func (export "f2") (param "x" (borrow $t))
                    (canon lift (core func $drop)))
                (func (export "f3") (param "x" u32)
                    (canon lift (core func $drop)))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let i = Linker::new(&engine).instantiate(&mut store, &c)?;

    assert!(i
        .get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f1")
        .is_err());
    assert!(i
        .get_typed_func::<(&ResourceAny,), ()>(&mut store, "f1")
        .is_ok());

    assert!(i
        .get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f2")
        .is_err());
    assert!(i
        .get_typed_func::<(&ResourceAny,), ()>(&mut store, "f2")
        .is_ok());

    assert!(i
        .get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f3")
        .is_err());
    assert!(i
        .get_typed_func::<(&ResourceAny,), ()>(&mut store, "f3")
        .is_err());
    assert!(i.get_typed_func::<(u32,), ()>(&mut store, "f3").is_ok());

    Ok(())
}

#[test]
fn drop_no_dtor() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (type $t' (resource (rep i32)))
                (export $t "t" (type $t'))

                (core func $ctor (canon resource.new $t))

                (func (export "ctor") (param "x" u32) (result (own $t))
                    (canon lift (core func $ctor)))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let i = Linker::new(&engine).instantiate(&mut store, &c)?;
    let ctor = i.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "ctor")?;
    let (resource,) = ctor.call(&mut store, (100,))?;
    ctor.post_return(&mut store)?;
    resource.resource_drop(&mut store)?;

    Ok(())
}

#[test]
fn host_borrow_as_resource_any() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))
                (import "f" (func $f (param "f" (borrow $t))))

                (core func $f (canon lower (func $f)))

                (core module $m
                    (import "" "f" (func $f (param i32)))
                    (func (export "f2") (param i32)
                        (call $f (local.get 0))
                    )
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "f" (func $f))
                    ))
                ))

                (func (export "f2") (param "x" (borrow $t))
                    (canon lift (core func $i "f2")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());

    // First test the above component where the host properly drops the argument
    {
        let mut linker = Linker::new(&engine);
        linker
            .root()
            .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
        linker
            .root()
            .func_wrap("f", |mut cx, (r,): (ResourceAny,)| {
                r.resource_drop(&mut cx)?;
                Ok(())
            })?;
        let i = linker.instantiate(&mut store, &c)?;

        let f = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f2")?;

        let resource = Resource::new_own(100);
        f.call(&mut store, (&resource,))?;
    }

    // Then also test the case where the host forgets a drop
    {
        let mut linker = Linker::new(&engine);
        linker
            .root()
            .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
        linker.root().func_wrap("f", |_cx, (_r,): (ResourceAny,)| {
            // ... no drop here
            Ok(())
        })?;
        let i = linker.instantiate(&mut store, &c)?;

        let f = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f2")?;

        let resource = Resource::new_own(100);
        let err = f.call(&mut store, (&resource,)).unwrap_err();
        assert!(
            format!("{err:?}").contains("borrow handles still remain at the end of the call"),
            "bad error: {err:?}"
        );
    }
    Ok(())
}

#[test]
fn pass_guest_back_as_borrow() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (type $t' (resource (rep i32)))

                (export $t "t" (type $t'))

                (core func $new (canon resource.new $t))

                (core module $m
                    (import "" "new" (func $new (param i32) (result i32)))

                    (func (export "mk") (result i32)
                        (call $new (i32.const 100))
                    )

                    (func (export "take") (param i32)
                        (if (i32.ne (local.get 0) (i32.const 100)) (then (unreachable)))
                    )
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "new" (func $new))
                    ))
                ))

                (func (export "mk") (result (own $t))
                    (canon lift (core func $i "mk")))
                (func (export "take") (param "x" (borrow $t))
                    (canon lift (core func $i "take")))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let i = Linker::new(&engine).instantiate(&mut store, &c)?;
    let mk = i.get_typed_func::<(), (ResourceAny,)>(&mut store, "mk")?;
    let take = i.get_typed_func::<(&ResourceAny,), ()>(&mut store, "take")?;

    let (resource,) = mk.call(&mut store, ())?;
    mk.post_return(&mut store)?;
    take.call(&mut store, (&resource,))?;
    take.post_return(&mut store)?;

    resource.resource_drop(&mut store)?;

    // Should not be valid to use `resource` again
    let err = take.call(&mut store, (&resource,)).unwrap_err();
    assert_eq!(err.to_string(), "unknown handle index 1");

    Ok(())
}

#[test]
fn pass_host_borrow_to_guest() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))

                (core func $drop (canon resource.drop $t))

                (core module $m
                    (import "" "drop" (func $drop (param i32)))
                    (func (export "take") (param i32)
                      (call $drop (local.get 0))
                    )
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "drop" (func $drop))
                    ))
                ))

                (func (export "take") (param "x" (borrow $t))
                    (canon lift (core func $i "take")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    let i = linker.instantiate(&mut store, &c)?;
    let take = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "take")?;

    let resource = Resource::new_borrow(100);
    take.call(&mut store, (&resource,))?;
    take.post_return(&mut store)?;

    Ok(())
}

#[test]
fn drop_on_owned_resource() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))
                (import "[constructor]t" (func $ctor (result (own $t))))
                (import "[method]t.foo" (func $foo (param "self" (borrow $t)) (result (list u8))))

                (core func $ctor (canon lower (func $ctor)))
                (core func $drop (canon resource.drop $t))

                (core module $m1
                    (import "" "drop" (func $drop (param i32)))
                    (memory (export "memory") 1)
                    (global $to-drop (export "to-drop") (mut i32) (i32.const 0))
                    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                        (call $drop (global.get $to-drop))
                        unreachable)
                )
                (core instance $i1 (instantiate $m1
                    (with "" (instance
                        (export "drop" (func $drop))
                    ))
                ))

                (core func $foo (canon lower (func $foo)
                    (memory $i1 "memory")
                    (realloc (func $i1 "realloc"))))

                (core module $m2
                    (import "" "ctor" (func $ctor (result i32)))
                    (import "" "foo" (func $foo (param i32 i32)))
                    (import "i1" "to-drop" (global $to-drop (mut i32)))

                    (func (export "f")
                        (local $r i32)
                        (local.set $r (call $ctor))
                        (global.set $to-drop (local.get $r))
                        (call $foo
                            (local.get $r)
                            (i32.const 200))
                    )
                )
                (core instance $i2 (instantiate $m2
                    (with "" (instance
                        (export "ctor" (func $ctor))
                        (export "foo" (func $foo))
                    ))
                    (with "i1" (instance $i1))
                ))
                (func (export "f") (canon lift (core func $i2 "f")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    linker.root().func_wrap("[constructor]t", |_cx, ()| {
        Ok((Resource::<MyType>::new_own(300),))
    })?;
    linker
        .root()
        .func_wrap("[method]t.foo", |_cx, (r,): (Resource<MyType>,)| {
            assert!(!r.owned());
            Ok((vec![2u8],))
        })?;
    let i = linker.instantiate(&mut store, &c)?;
    let f = i.get_typed_func::<(), ()>(&mut store, "f")?;

    let err = f.call(&mut store, ()).unwrap_err();
    assert!(
        format!("{err:?}").contains("cannot remove owned resource while borrowed"),
        "bad error: {err:?}"
    );

    Ok(())
}

#[test]
fn guest_different_host_same() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t1" (type $t1 (sub resource)))
                (import "t2" (type $t2 (sub resource)))

                (import "f" (func $f (param "a" (borrow $t1)) (param "b" (borrow $t2))))

                (export $g1 "g1" (type $t1))
                (export $g2 "g2" (type $t2))

                (core func $f (canon lower (func $f)))
                (core func $drop1 (canon resource.drop $t1))
                (core func $drop2 (canon resource.drop $t2))

                (core module $m
                    (import "" "f" (func $f (param i32 i32)))
                    (import "" "drop1" (func $drop1 (param i32)))
                    (import "" "drop2" (func $drop2 (param i32)))

                    (func (export "f") (param i32 i32)
                        ;; separate tables both have initial index of 1
                        (if (i32.ne (local.get 0) (i32.const 1)) (then (unreachable)))
                        (if (i32.ne (local.get 1) (i32.const 1)) (then (unreachable)))

                        ;; host should end up getting the same resource
                        (call $f (local.get 0) (local.get 1))

                        ;; drop our borrows
                        (call $drop1 (local.get 0))
                        (call $drop2 (local.get 0))
                    )
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "f" (func $f))
                        (export "drop1" (func $drop1))
                        (export "drop2" (func $drop2))
                    ))
                ))

                (func (export "f2") (param "a" (borrow $g1)) (param "b" (borrow $g2))
                    (canon lift (core func $i "f")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t1", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    linker
        .root()
        .resource("t2", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    linker.root().func_wrap(
        "f",
        |_cx, (r1, r2): (Resource<MyType>, Resource<MyType>)| {
            assert!(!r1.owned());
            assert!(!r2.owned());
            assert_eq!(r1.rep(), 100);
            assert_eq!(r2.rep(), 100);
            Ok(())
        },
    )?;
    let i = linker.instantiate(&mut store, &c)?;
    let f = i.get_typed_func::<(&Resource<MyType>, &Resource<MyType>), ()>(&mut store, "f2")?;

    let t1 = i.get_resource(&mut store, "g1").unwrap();
    let t2 = i.get_resource(&mut store, "g2").unwrap();
    assert_eq!(t1, t2);
    assert_eq!(t1, ResourceType::host::<MyType>());

    let resource = Resource::new_own(100);
    f.call(&mut store, (&resource, &resource))?;
    f.post_return(&mut store)?;

    Ok(())
}

#[test]
fn resource_any_to_typed_handles_borrow() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))

                (import "f" (func $f (param "a" (borrow $t))))

                (core func $f (canon lower (func $f)))

                (core module $m
                    (import "" "f" (func $f (param i32)))

                    (func (export "f") (param i32)
                        (call $f (local.get 0))
                    )
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "f" (func $f))
                    ))
                ))

                (func (export "f") (param "a" (own $t))
                    (canon lift (core func $i "f")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .resource("t", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    linker
        .root()
        .func_wrap("f", |mut cx, (r,): (ResourceAny,)| {
            let r = r.try_into_resource::<MyType>(&mut cx).unwrap();
            assert_eq!(r.rep(), 100);
            assert!(!r.owned());
            Ok(())
        })?;
    let i = linker.instantiate(&mut store, &c)?;
    let f = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "f")?;

    let resource = Resource::new_own(100);
    f.call(&mut store, (&resource,))?;
    f.post_return(&mut store)?;

    Ok(())
}
