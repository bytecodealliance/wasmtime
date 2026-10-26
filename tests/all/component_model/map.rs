#![cfg(not(miri))]

use super::{make_echo_component, map_engine};
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, DefaultHasher};
use wasmtime::component::{Component, Linker};
use wasmtime::{Result, Store};

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
