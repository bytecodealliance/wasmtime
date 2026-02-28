use wasmtime::Result;
use wasmtime::component::types::ComponentItem;
use wasmtime::component::{Component, Linker, ResourceType};
use wasmtime::{Engine, Store};

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
        match item {
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
fn implements_full_encoded_name() -> Result<()> {
    // Register using the full `[implements=<...>]label` name and verify
    // that a component importing that name instantiates successfully.
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    linker
        .root()
        .instance("[implements=<a:b/c>]primary")?
        .func_wrap(
            "get",
            |_: wasmtime::StoreContextMut<'_, ()>, (_key,): (String,)| Ok((String::new(),)),
        )?;

    let component = Component::new(
        &engine,
        r#"(component
            (type $store-type (instance
                (export "get" (func (param "key" string) (result string)))
            ))
            (import "[implements=<a:b/c>]primary" (instance (type $store-type)))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    linker.instantiate(&mut store, &component)?;

    Ok(())
}

#[test]
fn implements_label_fallback() -> Result<()> {
    // Register using the plain label name and verify that a component
    // importing via `[implements=<...>]label` matches via label fallback.
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    linker.root().instance("primary")?.func_wrap(
        "get",
        |_: wasmtime::StoreContextMut<'_, ()>, (_key,): (String,)| Ok((String::new(),)),
    )?;

    let component = Component::new(
        &engine,
        r#"(component
            (type $store-type (instance
                (export "get" (func (param "key" string) (result string)))
            ))
            (import "[implements=<a:b/c>]primary" (instance (type $store-type)))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    linker.instantiate(&mut store, &component)?;

    Ok(())
}

#[test]
fn implements_multiple_imports_same_interface() -> Result<()> {
    // Two imports of the same interface type under different labels.
    // Verify that each import binds to a distinct host implementation by
    // wrapping each imported function through a core module and exporting them.
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    linker
        .root()
        .instance("primary")?
        .func_wrap("id", |_: wasmtime::StoreContextMut<'_, ()>, (): ()| {
            Ok((1u32,))
        })?;

    linker
        .root()
        .instance("secondary")?
        .func_wrap("id", |_: wasmtime::StoreContextMut<'_, ()>, (): ()| {
            Ok((2u32,))
        })?;

    let component = Component::new(
        &engine,
        r#"(component
            (type $store-type (instance
                (export "id" (func (result u32)))
            ))
            (import "[implements=<test:kv/store>]primary" (instance $primary (type $store-type)))
            (import "[implements=<test:kv/store>]secondary" (instance $secondary (type $store-type)))

            (alias export $primary "id" (func $primary-id))
            (alias export $secondary "id" (func $secondary-id))

            (core func $primary-lowered (canon lower (func $primary-id)))
            (core func $secondary-lowered (canon lower (func $secondary-id)))

            (core module $m
                (import "" "p" (func $p (result i32)))
                (import "" "s" (func $s (result i32)))
                (func (export "call-primary") (result i32) (call $p))
                (func (export "call-secondary") (result i32) (call $s))
            )

            (core instance $i (instantiate $m
                (with "" (instance
                    (export "p" (func $primary-lowered))
                    (export "s" (func $secondary-lowered))
                ))
            ))

            (func $cp (result u32) (canon lift (core func $i "call-primary")))
            (func $cs (result u32) (canon lift (core func $i "call-secondary")))

            (export "call-primary" (func $cp))
            (export "call-secondary" (func $cs))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;

    let call_primary = instance.get_typed_func::<(), (u32,)>(&mut store, "call-primary")?;
    let call_secondary = instance.get_typed_func::<(), (u32,)>(&mut store, "call-secondary")?;

    let (result,) = call_primary.call(&mut store, ())?;
    assert_eq!(result, 1);

    let (result,) = call_secondary.call(&mut store, ())?;
    assert_eq!(result, 2);

    Ok(())
}

#[test]
fn implements_semver_compat() -> Result<()> {
    // Linker registers with `[implements=<a:b/c@1.0.1>]primary`, component
    // imports `[implements=<a:b/c@1.0.0>]primary` — should match via semver.
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    linker
        .root()
        .instance("[implements=<a:b/c@1.0.1>]primary")?
        .func_wrap(
            "get",
            |_: wasmtime::StoreContextMut<'_, ()>, (_key,): (String,)| Ok((String::new(),)),
        )?;

    let component = Component::new(
        &engine,
        r#"(component
            (type $store-type (instance
                (export "get" (func (param "key" string) (result string)))
            ))
            (import "[implements=<a:b/c@1.0.0>]primary" (instance (type $store-type)))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    linker.instantiate(&mut store, &component)?;

    Ok(())
}

#[test]
fn implements_missing_label_fails() -> Result<()> {
    // Only "primary" is defined; "secondary" is missing.
    // Instantiation should fail — the label fallback must not match
    // a different label.
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    linker.root().instance("primary")?.func_wrap(
        "get",
        |_: wasmtime::StoreContextMut<'_, ()>, (_key,): (String,)| Ok((String::new(),)),
    )?;

    let component = Component::new(
        &engine,
        r#"(component
            (type $store-type (instance
                (export "get" (func (param "key" string) (result string)))
            ))
            (import "[implements=<a:b/c>]primary" (instance (type $store-type)))
            (import "[implements=<a:b/c>]secondary" (instance (type $store-type)))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    assert!(linker.instantiate(&mut store, &component).is_err());

    Ok(())
}
