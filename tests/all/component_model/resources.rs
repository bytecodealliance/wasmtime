use anyhow::Result;
use wasmtime::component::*;
use wasmtime::Store;

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
    linker.root().resource::<T>("t", |_, _| {})?;
    linker.root().resource::<U>("u", |_, _| {})?;
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

                (core func $t_drop (canon resource.drop (own $t)))
                (core func $u_drop (canon resource.drop (own $u)))

                (func (export "drop-t") (param "x" (own $t))
                    (canon lift (core func $t_drop)))
                (func (export "drop-u") (param "x" (own $u))
                    (canon lift (core func $u_drop)))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
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
        "unknown handle index 0"
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

                (core func $u_dtor (canon resource.drop (own $u)))
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

                (core func $dtor (canon resource.drop (own $t)))
                (func (export "dtor1") (param "x" (own $t))
                    (canon lift (core func $dtor)))

                (component $c
                    (import "t" (type $t (sub resource)))
                    (core func $dtor (canon resource.drop (own $t)))
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

                (core func $dtor (canon resource.drop (own $t)))
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
        "resource already consumed"
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

                (core func $dtor (canon resource.drop (own $t)))
                (func (export "dtor") (param "x" (own $t))
                    (canon lift (core func $dtor)))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.root().resource::<MyType>("t", |_, _| {})?;
    let i = linker.instantiate(&mut store, &c)?;
    let dtor = i.get_typed_func::<(&Resource<MyType>,), ()>(&mut store, "dtor")?;

    let t = Resource::new(100);
    dtor.call(&mut store, (&t,))?;
    dtor.post_return(&mut store)?;

    assert_eq!(
        dtor.call(&mut store, (&t,)).unwrap_err().to_string(),
        "resource already consumed"
    );

    Ok(())
}
