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
#[cfg_attr(miri, ignore)]
fn linker_defines_unknown_imports_as_default_values() -> Result<()> {
    let engine = Engine::default();

    // Cover every aggregate, so the traversal is exercised in full.
    let component = Component::new(
        &engine,
        r#"(component
            (type (;0;) (record (field "a" u32) (field "b" string)))
            (import "rec" (type $rec (eq 0)))
            (type (;2;) (variant (case "x" s64) (case "y")))
            (import "var" (type $var (eq 2)))
            (type (;4;) (flags "p" "q"))
            (import "fl" (type $fl (eq 4)))
            (type (;6;) (enum "e1" "e2"))
            (import "en" (type $en (eq 6)))

            (type $ret (tuple u32 float64 string bool char (list u8)
                              (option u64) (result u32 (error string))
                              $rec $var $fl $en))
            (type $f (func (result $ret)))
            (import "f" (func $fn (type $f)))
        )"#,
    )?;

    let mut linker = Linker::<()>::new(&engine);
    let mut store = Store::new(&engine, ());
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;
    linker.instantiate(&mut store, &component)?;

    // Check the values the stub returns by asking the import's type.
    let ty = component.component_type();
    let func = ty
        .imports(&engine)
        .find_map(|(name, item)| match item.ty {
            ComponentItem::ComponentFunc(f) if name == "f" => Some(f),
            _ => None,
        })
        .expect("component imports a function named `f`");
    let results = func.results().collect::<Vec<_>>();
    assert_eq!(results.len(), 1);

    let Val::Tuple(fields) = results[0].default_value(&mut store)? else {
        panic!("expected a tuple");
    };
    assert_eq!(
        fields,
        [
            Val::U32(0),
            Val::Float64(0.0),
            Val::String(String::new()),
            Val::Bool(false),
            Val::Char('\0'),
            Val::List(Vec::new()),
            Val::Option(None),
            Val::Result(Ok(Some(Box::new(Val::U32(0))))),
            Val::Record(vec![
                ("a".to_string(), Val::U32(0)),
                ("b".to_string(), Val::String(String::new())),
            ]),
            Val::Variant("x".to_string(), Some(Box::new(Val::S64(0)))),
            Val::Flags(Vec::new()),
            Val::Enum("e1".to_string()),
        ]
    );

    Ok(())
}

/// A defaulted import is callable from wasm, not merely present.
#[test]
#[cfg_attr(miri, ignore)]
fn linker_default_value_imports_are_callable() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"(component
            (import "f" (func $f (result u32)))
            (core func $lowered (canon lower (func $f)))
            (core module $m
                (import "" "f" (func $f (result i32)))
                (func (export "run") (result i32) call $f)
            )
            (core instance $i (instantiate $m
                (with "" (instance (export "f" (func $lowered))))))
            (func (export "run") (result u32) (canon lift (core func $i "run")))
        )"#,
    )?;

    let mut linker = Linker::<()>::new(&engine);
    let mut store = Store::new(&engine, ());
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_func(&mut store, "run").unwrap();

    let mut results = [Val::Bool(false)];
    run.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0], Val::U32(0));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn linker_cannot_default_guest_defined_resources() -> Result<()> {
    let engine = Engine::default();

    // Instantiate a component which defines a resource, to get a live
    // guest-defined `ResourceType`.
    let producer = Component::new(
        &engine,
        r#"(component
            (core module $m (func (export "dtor") (param i32)))
            (core instance $mi (instantiate $m))
            (type $r (resource (rep i32) (dtor (core func $mi "dtor"))))
            (export "r" (type $r))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let producer = Linker::<()>::new(&engine).instantiate(&mut store, &producer)?;
    let guest_ty = producer.get_resource(&mut store, "r").unwrap();

    let component = Component::new(
        &engine,
        r#"(component
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
        )"#,
    )?;

    let mut linker = Linker::<()>::new(&engine);
    linker
        .instance("types")?
        .resource("r", guest_ty, |_, _| Ok(()))?;
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;

    // Instantiating still works; only calling the import fails.
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_func(&mut store, "run").unwrap();
    let mut results = [Val::Bool(false)];
    let err = run.call(&mut store, &[], &mut results).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("cannot synthesize a resource of a non-host-defined type"),
        "unexpected error: {err}"
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn linker_default_value_resources_use_the_instances_type() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"(component
            (import "r" (type $r (sub resource)))
            (type $f (func (result (own $r))))
            (import "f" (func $fn (type $f)))

            (core func $lowered (canon lower (func $fn)))
            (core module $m
                (import "" "f" (func $f (result i32)))
                (func (export "run") (result i32) call $f)
            )
            (core instance $i (instantiate $m
                (with "" (instance (export "f" (func $lowered))))))
            (func (export "run") (result (own $r))
                (canon lift (core func $i "run")))
        )"#,
    )?;

    let mut linker = Linker::<()>::new(&engine);
    linker
        .root()
        .resource("r", ResourceType::host::<u32>(), |_, _| Ok(()))?;

    let mut store = Store::new(&engine, ());
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_func(&mut store, "run").unwrap();

    let mut results = [Val::Bool(false)];
    run.call(&mut store, &[], &mut results)?;
    let Val::Resource(resource) = results[0] else {
        panic!("expected a resource");
    };
    assert_eq!(resource.ty(), ResourceType::host::<u32>());

    Ok(())
}

/// A function returning an imported resource gets a dummy handle.
#[test]
#[cfg_attr(miri, ignore)]
fn linker_defines_unknown_resource_imports_as_default_values() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"(component
            (import "r" (type $r (sub resource)))
            (type $f (func (result (own $r))))
            (import "f" (func $fn (type $f)))

            (core func $lowered (canon lower (func $fn)))
            (core module $m
                (import "" "f" (func $f (result i32)))
                (func (export "run") (result i32)
                    (drop (call $f))
                    (call $f))
            )
            (core instance $i (instantiate $m
                (with "" (instance (export "f" (func $lowered))))))
            (func (export "run") (result (own $r))
                (canon lift (core func $i "run")))
        )"#,
    )?;

    let mut linker = Linker::<()>::new(&engine);
    let mut store = Store::new(&engine, ());
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_func(&mut store, "run").unwrap();

    let mut results = [Val::Bool(false)];
    run.call(&mut store, &[], &mut results)?;
    assert!(matches!(results[0], Val::Resource(_)));

    Ok(())
}

/// Nested instances and resource imports are stubbed out.
#[test]
fn linker_defines_unknown_nested_imports_as_default_values() -> Result<()> {
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
    let mut store = Store::new(&engine, ());
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;
    linker.instantiate(&mut store, &component)?;

    Ok(())
}

/// A type with no default does not prevent instantiation, but instead gets a
/// trapping stub.
#[tokio::test]
async fn linker_defines_undefaultable_imports_as_traps() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(
        &engine,
        r#"(component
            (type $f (func (result (future u32))))
            (import "f" (func $fn (type $f)))

            (core func $lowered (canon lower (func $fn)))
            (core module $m
                (import "" "f" (func $f (result i32)))
                (func (export "run") (result i32) call $f)
            )
            (core instance $i (instantiate $m
                (with "" (instance (export "f" (func $lowered))))))
            (func (export "run") (result (future u32))
                (canon lift (core func $i "run")))
        )"#,
    )?;

    let mut linker = Linker::<()>::new(&engine);

    // Defining succeeds even though `future<u32>` has no default...
    let mut store = Store::new(&engine, ());
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;
    let instance = linker.instantiate_async(&mut store, &component).await?;

    // ...but calling the import traps.
    let run = instance.get_func(&mut store, "run").unwrap();
    let mut results = [Val::Bool(false)];
    let err = run
        .call_async(&mut store, &[], &mut results)
        .await
        .unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("unknown import: `f` has no default value"),
        "unexpected error: {err}"
    );
    assert!(
        err.contains("no default value exists for type `future`"),
        "unexpected error: {err}"
    );

    Ok(())
}

#[tokio::test]
async fn linker_defines_unknown_async_imports_as_default_values() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::<()>::new(&engine);

    let component = Component::new(
        &engine,
        r#"(component
            (import "foo" (func async (result u32)))
            (import "bar" (instance (export "baz" (func async))))
            (import "qux" (type (sub resource)))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;
    let _ = linker.instantiate_async(&mut store, &component).await?;

    Ok(())
}

/// Build default values for a deeply nested type.
#[test]
#[cfg_attr(miri, ignore)]
fn linker_defines_deeply_nested_imports_as_default_values() -> Result<()> {
    // Stay within the max component type depth of 100.
    const DEPTH: usize = 97;

    let engine = Engine::default();

    // A chain of `DEPTH` tuple types, each wrapping the previous.
    let mut wat = String::from("(component\n  (type $t0 u32)\n");
    for i in 1..=DEPTH {
        wat.push_str(&format!("  (type $t{i} (tuple $t{}))\n", i - 1));
    }
    wat.push_str(&format!("  (type $f (func (result $t{DEPTH})))\n"));
    wat.push_str("  (import \"f\" (func $fn (type $f)))\n)");

    let component = Component::new(&engine, &wat)?;

    let mut linker = Linker::<()>::new(&engine);
    let mut store = Store::new(&engine, ());
    linker.define_unknown_imports_as_default_values(&mut store, &component)?;
    // Should not overflow the stack.
    linker.instantiate(&mut store, &component)?;

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
