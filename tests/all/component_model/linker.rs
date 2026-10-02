use wasmtime::Result;
use wasmtime::component::types::ComponentItem;
use wasmtime::component::{Component, Linker, ResourceType};
use wasmtime::{Config, Engine, Module, Store};

#[test]
fn old_import_importing_new_item() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    let ty = ResourceType::host::<u32>();
    linker.root().resource("a:b/c@1.0.1", ty, |_, _| Ok(()))?;

    let component = Component::new(
        &engine,
        r#"(component
            (import "a:b/c@1.0.0" (type $t (sub resource)))
            (export "a" (type $t))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let i = linker.instantiate(&mut store, &component)?;

    assert_eq!(i.get_resource(&mut store, "a"), Some(ty));

    Ok(())
}

#[test]
fn new_import_importing_old_item() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    let ty = ResourceType::host::<u32>();
    linker.root().resource("a:b/c@1.0.0", ty, |_, _| Ok(()))?;

    let component = Component::new(
        &engine,
        r#"(component
            (import "a:b/c@1.0.1" (type $t (sub resource)))
            (export "a" (type $t))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let i = linker.instantiate(&mut store, &component)?;

    assert_eq!(i.get_resource(&mut store, "a"), Some(ty));

    Ok(())
}

#[test]
fn import_both_old_and_new() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    // There's one definition per semver track.
    let t1 = ResourceType::host::<u32>();
    let t2 = ResourceType::host::<i32>();
    linker.root().resource("a:b/c@1.0.0", t1, |_, _| Ok(()))?;
    assert!(
        linker
            .root()
            .resource("a:b/c@1.0.1", t2, |_, _| Ok(()))
            .is_err()
    );

    let component = Component::new(
        &engine,
        r#"(component
            (import "a:b/c@1.0.0" (type $t1 (sub resource)))
            (import "a:b/c@1.0.1" (type $t2 (sub resource)))
            (export "t1" (type $t1))
            (export "t2" (type $t2))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let i = linker.instantiate(&mut store, &component)?;

    assert_eq!(i.get_resource(&mut store, "t1"), Some(t1));
    assert_eq!(i.get_resource(&mut store, "t2"), Some(t1));

    Ok(())
}

#[test]
fn shadowing_replaces_same_track() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);
    linker.allow_shadowing(true);

    let t1 = ResourceType::host::<u32>();
    let t2 = ResourceType::host::<i32>();
    linker.root().resource("a:b/c@1.0.1", t1, |_, _| Ok(()))?;
    linker.root().resource("a:b/c@1.0.2", t2, |_, _| Ok(()))?;

    let component = Component::new(
        &engine,
        r#"(component
            (import "a:b/c@1.0.0" (type $t1 (sub resource)))
            (import "a:b/c@1.0.3" (type $t2 (sub resource)))
            (export "t1" (type $t1))
            (export "t2" (type $t2))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let i = linker.instantiate(&mut store, &component)?;

    assert_eq!(i.get_resource(&mut store, "t1"), Some(t2));
    assert_eq!(i.get_resource(&mut store, "t2"), Some(t2));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn linker_substituting_types_issue_8003() -> Result<()> {
    let engine = Engine::default();
    let linker = Linker::<()>::new(&engine);
    let component = Component::new(
        &engine,
        r#"
            (component
              (component $foo
                (type $_myres (resource (rep i32)))
                (export $myres "myres" (type $_myres))

                (core module $m
                  (func (export "make") (result i32) unreachable)
                )
                (core instance $m (instantiate $m))

                (func (export "make") (result (own $myres))
                  (canon lift (core func $m "make")))
              )
              (instance $foo (instantiate $foo))
              (export "foo" (instance $foo))
            )
        "#,
    )?;

    let component_ty = linker.substituted_component_type(&component)?;
    let exports = component_ty.exports(&engine);
    for (_name, item) in exports {
        match item.ty {
            ComponentItem::ComponentInstance(instance) => {
                for _ in instance.exports(&engine) {
                    // ..
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[test]
fn linker_defines_unknown_imports_as_traps() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    let component = Component::new(
        &engine,
        r#"(component
            (import "foo" (func))
            (import "bar" (instance (export "baz" (func))))
            (import "qux" (type (sub resource)))
        )"#,
    )?;
    linker.define_unknown_imports_as_traps(&component)?;
    let mut store = Store::new(&engine, ());
    let _ = linker.instantiate(&mut store, &component)?;

    Ok(())
}

/// Resource aliases must follow the real resource regardless of registration order.
#[test]
fn linker_defines_unknown_imports_as_traps_with_resource_aliases() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"(component
            (import "types" (instance $types
                (export "r" (type (sub resource)))
            ))
            (alias export $types "r" (type $r))
            (import "uses" (instance
                (alias outer 1 $r (type $r))
                (export "r" (type (eq $r)))
            ))
        )"#,
    )?;

    for stub_first in [false, true] {
        let mut linker = Linker::<()>::new(&engine);
        linker.allow_shadowing(stub_first);

        if stub_first {
            linker.define_unknown_imports_as_traps(&component)?;
        }

        linker
            .instance("types")?
            .resource("r", ResourceType::host::<u32>(), |_, _| Ok(()))?;

        if !stub_first {
            linker.define_unknown_imports_as_traps(&component)?;
        }

        let mut store = Store::new(&engine, ());
        linker.instantiate(&mut store, &component)?;
    }

    Ok(())
}

#[tokio::test]
async fn linker_defines_unknown_async_imports_as_traps() -> Result<()> {
    // `define_unknown_imports_as_traps` used to always stub with `func_new`,
    // which can never satisfy an `async func`-typed import - so a component
    // with an unsatisfied async import couldn't be stubbed (and thus
    // couldn't be instantiated) at all, even though nothing in the component
    // ever calls it. With concurrency support enabled, it should now stub
    // async imports with `func_new_concurrent` instead.
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::<()>::new(&engine);

    let component = Component::new(
        &engine,
        r#"(component
            (import "foo" (func async))
            (import "bar" (instance (export "baz" (func async))))
            (import "qux" (type (sub resource)))
        )"#,
    )?;
    linker.define_unknown_imports_as_traps(&component)?;

    let mut store = Store::new(&engine, ());
    let _ = linker.instantiate_async(&mut store, &component).await?;

    Ok(())
}

#[test]
fn linker_fails_to_define_unknown_core_module_imports_as_traps() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    let component = Component::new(
        &engine,
        r#"(component
            (import "foo" (core module))
        )"#,
    )?;
    assert!(linker.define_unknown_imports_as_traps(&component).is_err());

    Ok(())
}

#[test]
fn open_instance_twice() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    let module = Module::new(&engine, "(module)")?;
    linker.instance("foo")?.module("a", &module)?;
    linker.instance("foo")?.module("b", &module)?;
    assert!(linker.instance("foo")?.module("a", &module).is_err());
    assert!(linker.instance("foo")?.module("b", &module).is_err());
    linker.instance("foo")?.module("c", &module)?;
    assert!(linker.root().module("foo", &module).is_err());
    linker.instance("foo")?.module("d", &module)?;

    Ok(())
}

#[test]
fn semver_instances_share_track() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    let t1 = ResourceType::host::<u32>();
    let t2 = ResourceType::host::<i32>();
    let t3 = ResourceType::host::<u64>();
    linker
        .instance("a:b/c@0.2.0")?
        .resource("r1", t1, |_, _| Ok(()))?;
    linker
        .instance("a:b/c@0.2.1")?
        .resource("r2", t2, |_, _| Ok(()))?;
    assert!(
        linker
            .instance("a:b/c@0.2.1")?
            .resource("r1", t2, |_, _| Ok(()))
            .is_err()
    );
    assert!(
        linker
            .instance("a:b/c@0.2.0")?
            .resource("r2", t1, |_, _| Ok(()))
            .is_err()
    );
    // A different semver track is a separate instance.
    linker
        .instance("a:b/c@0.3.0")?
        .resource("r1", t3, |_, _| Ok(()))?;

    for version in ["0.2.0", "0.2.1", "0.2.2"] {
        let component = Component::new(
            &engine,
            format!(
                r#"(component
                    (import "a:b/c@{version}" (instance $i
                        (export "r1" (type (sub resource)))
                        (export "r2" (type (sub resource)))
                    ))
                    (import "a:b/c@0.3.0" (instance $j
                        (export "r1" (type (sub resource)))
                    ))
                    (alias export $i "r1" (type $r1))
                    (alias export $i "r2" (type $r2))
                    (alias export $j "r1" (type $r3))
                    (export "r1" (type $r1))
                    (export "r2" (type $r2))
                    (export "r3" (type $r3))
                )"#
            ),
        )?;
        let mut store = Store::new(&engine, ());
        let i = linker.instantiate(&mut store, &component)?;
        assert_eq!(i.get_resource(&mut store, "r1"), Some(t1));
        assert_eq!(i.get_resource(&mut store, "r2"), Some(t2));
        assert_eq!(i.get_resource(&mut store, "r3"), Some(t3));
    }

    Ok(())
}

#[test]
fn semver_instances_share_track_with_shadowing() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);
    linker.allow_shadowing(true);

    let t1 = ResourceType::host::<u32>();
    let t2 = ResourceType::host::<i32>();
    linker
        .instance("a:b/c@0.2.0")?
        .resource("r", t1, |_, _| Ok(()))?;
    linker
        .instance("a:b/c@0.2.1")?
        .resource("r", t2, |_, _| Ok(()))?;

    let component = Component::new(
        &engine,
        r#"(component
            (import "a:b/c@0.2.0" (instance $i
                (export "r" (type (sub resource)))
            ))
            (alias export $i "r" (type $r))
            (export "r" (type $r))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let i = linker.instantiate(&mut store, &component)?;
    assert_eq!(i.get_resource(&mut store, "r"), Some(t2));

    Ok(())
}

#[test]
fn canonical_instance_names() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_canonical_names(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::<()>::new(&engine);

    let t1 = ResourceType::host::<u32>();
    linker
        .instance("a:b/c@0.2.1")?
        .resource("r", t1, |_, _| Ok(()))?;
    // A canonical name reopens the instance on its semver track.
    assert!(
        linker
            .instance("a:b/c@0.2")?
            .resource("r", t1, |_, _| Ok(()))
            .is_err()
    );

    for import in [
        r#""a:b/c@0.2" (versionsuffix ".1")"#,
        r#""a:b/c@0.2" (versionsuffix ".0")"#,
        r#""a:b/c@0.2.1""#,
    ] {
        let component = Component::new(
            &engine,
            format!(
                r#"(component
                    (import {import} (instance $i
                        (export "r" (type (sub resource)))
                    ))
                    (alias export $i "r" (type $r))
                    (export "r" (type $r))
                )"#
            ),
        )?;
        let mut store = Store::new(&engine, ());
        let i = linker.instantiate(&mut store, &component)?;
        assert_eq!(i.get_resource(&mut store, "r"), Some(t1));
    }

    Ok(())
}

#[test]
fn invalid_instance_names() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    assert!(linker.instance("[method]a.b").is_err());
    assert!(linker.root().instance("[static]a.b").is_err());
    assert!(linker.instance("a:b/c@1.2").is_err());
    assert!(linker.instance("a:b/c@").is_err());

    // Invalid names are rejected even when a same-track instance exists.
    linker.instance("a:b/c@1.2.0")?;
    assert!(linker.instance("a:b/c@1.2").is_err());
    linker.instance("a:b/c@1")?;

    Ok(())
}

#[test]
fn unknown_imports_as_traps_share_semver_track() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    let t1 = ResourceType::host::<u32>();
    linker
        .instance("a:b/c@0.2.1")?
        .resource("r", t1, |_, _| Ok(()))?;

    let component = Component::new(
        &engine,
        r#"(component
            (import "a:b/c@0.2.0" (instance $i
                (export "r" (type (sub resource)))
                (export "f" (func))
            ))
            (alias export $i "r" (type $r))
            (export "r" (type $r))
        )"#,
    )?;
    linker.define_unknown_imports_as_traps(&component)?;
    let mut store = Store::new(&engine, ());
    let i = linker.instantiate(&mut store, &component)?;
    assert_eq!(i.get_resource(&mut store, "r"), Some(t1));

    // The stub for `f` was added to the existing `@0.2.1` instance.
    assert!(
        linker
            .instance("a:b/c@0.2.1")?
            .func_wrap("f", |_, (): ()| Ok(()))
            .is_err()
    );

    Ok(())
}
