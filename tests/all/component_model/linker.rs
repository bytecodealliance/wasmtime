use wasmtime::Result;
use wasmtime::component::types::ComponentItem;
use wasmtime::component::{Component, Linker, ResourceType, Val};
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

    let t1 = ResourceType::host::<u32>();
    let t2 = ResourceType::host::<i32>();
    linker.root().resource("a:b/c@1.0.0", t1, |_, _| Ok(()))?;
    linker.root().resource("a:b/c@1.0.1", t2, |_, _| Ok(()))?;

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
    assert_eq!(i.get_resource(&mut store, "t2"), Some(t2));

    Ok(())
}

#[test]
fn missing_import_selects_max() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

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
#[cfg_attr(miri, ignore)]
fn missing_implements_import_mentions_interface() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_implements(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::<()>::new(&engine);
    linker
        .instance("a:b/kv")?
        .func_wrap("get", |_, ()| Ok(()))?;

    let component = Component::new(
        &engine,
        r#"(component
            (import "primary" (implements "a:b/kv") (instance
                (export "get" (func))
            ))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let err = linker.instantiate(&mut store, &component).unwrap_err();
    let err = err.to_string();
    assert!(
        err.contains("`primary`") && err.contains("`a:b/kv`"),
        "{err}"
    );

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

/// A default `own` handle takes the resource type of the instance it flows
/// into, which is only possible for a host-defined type: a guest owns the
/// representation of its own resources, so one cannot be synthesized.
#[test]
#[cfg_attr(miri, ignore)]
fn linker_default_value_resource_types() -> Result<()> {
    const COMPONENT: &str = r#"(component
        (import "types" (instance $t (export "r" (type (sub resource)))))
        (alias export $t "r" (type $r))
        (import "f" (func $fn (result (own $r))))

        (core func $lowered (canon lower (func $fn)))
        (core module $m
            (import "" "f" (func $f (result i32)))
            (func (export "run") (result i32) call $f)
        )
        (core instance $i (instantiate $m
            (with "" (instance (export "f" (func $lowered))))))
        (func (export "run") (result (own $r))
            (canon lift (core func $i "run")))
    )"#;

    let engine = Engine::default();
    let component = Component::new(&engine, COMPONENT)?;
    let mut store = Store::new(&engine, ());

    // A host-defined resource: the handle comes back with the linker's type,
    // not one of the stub's choosing.
    let host_ty = ResourceType::host::<u32>();
    let mut linker = Linker::<()>::new(&engine);
    linker
        .instance("types")?
        .resource("r", host_ty, |_, _| Ok(()))?;
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;

    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_func(&mut store, "run").unwrap();
    let mut results = [Val::Bool(false)];
    run.call(&mut store, &[], &mut results)?;
    let Val::Resource(resource) = results[0] else {
        panic!("expected a resource");
    };
    assert_eq!(resource.ty(), host_ty);

    // A guest-defined resource, taken from a live instance of a component
    // which defines one. Instantiating still works; only the call fails.
    let producer = Component::new(
        &engine,
        r#"(component
            (core module $m (func (export "dtor") (param i32)))
            (core instance $mi (instantiate $m))
            (type $r (resource (rep i32) (dtor (core func $mi "dtor"))))
            (export "r" (type $r))
        )"#,
    )?;
    let producer = Linker::<()>::new(&engine).instantiate(&mut store, &producer)?;
    let guest_ty = producer.get_resource(&mut store, "r").unwrap();

    let mut linker = Linker::<()>::new(&engine);
    linker
        .instance("types")?
        .resource("r", guest_ty, |_, _| Ok(()))?;
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;

    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_func(&mut store, "run").unwrap();
    let err = run.call(&mut store, &[], &mut results).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("cannot synthesize a resource of a non-host-defined type"),
        "unexpected error: {err}"
    );

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
