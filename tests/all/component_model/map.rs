#![cfg(not(miri))]

use super::{make_echo_component, map_engine};
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, DefaultHasher};
use wasmtime::component::{Component, Lift, Linker, ResourceAny};
use wasmtime::{Engine, Result, Store};

/// A `HashMap` with a hasher other than the default `RandomState`.
type CustomHashMap<K, V> = HashMap<K, V, BuildHasherDefault<DefaultHasher>>;

#[test]
fn map_types() -> Result<()> {
    let engine = map_engine();
    let mut store = Store::new(&engine, ());
    let component = Component::new(&engine, make_echo_component("(map u32 string)", 8))?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let entry = |k: u32, v: &str| (k, v.to_string());

    // `IndexMap` keeps the order of its entries.
    let input = IndexMap::<u32, String>::from([entry(3, "c"), entry(1, "a"), entry(2, "b")]);
    let echo = instance.get_typed_func::<(&IndexMap<u32, String>,), (IndexMap<u32, String>,)>(
        &mut store, "echo",
    )?;
    let output = echo.call(&mut store, (&input,))?.0;
    assert!(output.iter().eq(input.iter()));

    // `BTreeMap` is lowered in sorted order.
    let input = BTreeMap::from([entry(2, "b"), entry(1, "a")]);
    let echo = instance.get_typed_func::<(&BTreeMap<u32, String>,), (IndexMap<u32, String>,)>(
        &mut store, "echo",
    )?;
    let output = echo.call(&mut store, (&input,))?.0;
    assert!(output.iter().eq(input.iter()));

    // `HashMap` works with any hasher.
    let input = CustomHashMap::from_iter([entry(1, "a"), entry(2, "b")]);
    let echo = instance
        .get_typed_func::<(&CustomHashMap<u32, String>,), (CustomHashMap<u32, String>,)>(
            &mut store, "echo",
        )?;
    assert_eq!(echo.call(&mut store, (&input,))?.0, input);
    Ok(())
}

#[test]
fn duplicate_keys_with_resources() -> Result<()> {
    let engine = map_engine();
    // `f` returns two new resources, with the reps 100 and 200, both under the
    // key `1`. `drops` returns how many times the destructor of `r` has run.
    let component = Component::new(
        &engine,
        r#"
        (component
            (core module $a
                (memory (export "memory") 1)
                (global $drops (mut i32) (i32.const 0))
                (func (export "dtor") (param i32)
                    global.get $drops i32.const 1 i32.add global.set $drops)
                (func (export "drops") (result i32) global.get $drops)
                (func (export "rep") (param i32) (result i32) local.get 0)
            )
            (core instance $a (instantiate $a))
            (type $r (resource (rep i32) (dtor (core func $a "dtor"))))
            (core func $new (canon resource.new $r))
            (core module $b
                (import "a" "memory" (memory 1))
                (import "" "new" (func $new (param i32) (result i32)))
                (func (export "f") (result i32)
                    (i32.store (i32.const 64) (i32.const 1))
                    (i32.store (i32.const 68) (call $new (i32.const 100)))
                    (i32.store (i32.const 72) (i32.const 1))
                    (i32.store (i32.const 76) (call $new (i32.const 200)))
                    (i32.store (i32.const 32) (i32.const 64))
                    (i32.store (i32.const 36) (i32.const 2))
                    i32.const 32)
            )
            (core instance $b (instantiate $b
                (with "a" (instance $a))
                (with "" (instance (export "new" (func $new))))
            ))
            (export $r2 "r" (type $r))
            (func (export "f") (result (map u32 (own $r2)))
                (canon lift (core func $b "f") (memory (core memory $a "memory"))))
            (func (export "rep") (param "r" (borrow $r2)) (result u32)
                (canon lift (core func $a "rep")))
            (func (export "drops") (result u32) (canon lift (core func $a "drops")))
        )
        "#,
    )?;

    // The replaced resource is dropped by the time the call returns, and every
    // destructor runs exactly once.
    fn check<M>(engine: &Engine, component: &Component) -> Result<()>
    where
        M: Lift + IntoIterator<Item = (u32, ResourceAny)> + 'static,
    {
        let mut store = Store::new(engine, ());
        let instance = Linker::new(engine).instantiate(&mut store, component)?;
        let f = instance.get_typed_func::<(), (M,)>(&mut store, "f")?;
        let rep = instance.get_typed_func::<(ResourceAny,), (u32,)>(&mut store, "rep")?;
        let drops = instance.get_typed_func::<(), (u32,)>(&mut store, "drops")?;

        let (map,) = f.call(&mut store, ())?;
        assert_eq!(drops.call(&mut store, ())?, (1,));
        let entries = map.into_iter().collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        let (key, resource) = entries[0];
        assert_eq!((key, rep.call(&mut store, (resource,))?.0), (1, 200));
        resource.resource_drop(&mut store)?;
        assert_eq!(drops.call(&mut store, ())?, (2,));
        Ok(())
    }
    check::<HashMap<u32, ResourceAny>>(&engine, &component)?;
    check::<BTreeMap<u32, ResourceAny>>(&engine, &component)?;
    check::<IndexMap<u32, ResourceAny>>(&engine, &component)?;
    Ok(())
}
