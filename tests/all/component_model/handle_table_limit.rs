//! Tests for the store-level cap on the number of simultaneously-live
//! component-model handles.

#![cfg(not(miri))]

use super::engine;
use crate::ErrorExt;
use wasmtime::component::{Component, Linker};
use wasmtime::{Result, Store};

/// Two instances of one sub-component, each with its own handle table, so the
/// per-store nature of the cap can be exercised. `sprayN(n)` allocates `n` live
/// handles; `churnN(n)` allocates and drops one `n` times, staying at one live.
const COMPONENT: &str = r#"
(component
  (component $C
    (type $r (resource (rep i32)))
    (core func $new (canon resource.new $r))
    (core func $drop (canon resource.drop $r))
    (core module $m
      (import "" "new" (func $new (param i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))
      (func (export "spray") (param $n i32) (result i32)
        (local $i i32)
        (loop $l
          (if (i32.lt_u (local.get $i) (local.get $n))
            (then
              (drop (call $new (local.get $i)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $l))))
        (local.get $i))
      (func (export "churn") (param $n i32) (result i32)
        (local $i i32)
        (loop $l
          (if (i32.lt_u (local.get $i) (local.get $n))
            (then
              (call $drop (call $new (local.get $i)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $l))))
        (local.get $i)))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "new" (func $new))
        (export "drop" (func $drop))))))
    (func (export "spray") (param "n" u32) (result u32)
      (canon lift (core func $i "spray")))
    (func (export "churn") (param "n" u32) (result u32)
      (canon lift (core func $i "churn"))))

  (instance $i0 (instantiate $C))
  (export "spray0" (func $i0 "spray"))
  (export "churn0" (func $i0 "churn"))
  (instance $i1 (instantiate $C))
  (export "spray1" (func $i1 "spray"))
  (export "churn1" (func $i1 "churn"))
)
"#;

/// The cap is per store, not per table: two instances whose combined (but not
/// individual) allocations cross the limit. The old per-table `MAX_HANDLE` check
/// would let this pass.
#[test]
fn cap_is_per_store_not_per_table() -> Result<()> {
    let engine = engine();
    let component = Component::new(&engine, COMPONENT)?;
    let linker = Linker::new(&engine);

    let mut store = Store::new(&engine, ());
    store.set_max_component_handles(20);

    let instance = linker.instantiate(&mut store, &component)?;
    let spray0 = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "spray0")?;
    let spray1 = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "spray1")?;

    // 12 is under the limit of 20, so this alone succeeds.
    let (n,) = spray0.call(&mut store, (12,))?;
    assert_eq!(n, 12);

    // Another 12 in the second instance's own table trips the shared limit; a
    // per-table cap would allow it.
    spray1
        .call(&mut store, (12,))
        .unwrap_err()
        .assert_contains("store handle limit exceeded");

    Ok(())
}

/// Raising the limit admits a spray a tight limit rejects.
#[test]
fn knob_raises_the_limit() -> Result<()> {
    let engine = engine();
    let component = Component::new(&engine, COMPONENT)?;
    let linker = Linker::new(&engine);

    // Tight limit: spraying past it traps.
    let mut store = Store::new(&engine, ());
    store.set_max_component_handles(10);
    assert_eq!(store.max_component_handles(), 10);
    let instance = linker.instantiate(&mut store, &component)?;
    let spray0 = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "spray0")?;
    spray0
        .call(&mut store, (50,))
        .unwrap_err()
        .assert_contains("store handle limit exceeded");

    // Generous limit: the same spray succeeds.
    let mut store = Store::new(&engine, ());
    store.set_max_component_handles(1_000);
    assert_eq!(store.max_component_handles(), 1_000);
    let instance = linker.instantiate(&mut store, &component)?;
    let spray0 = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "spray0")?;
    let (n,) = spray0.call(&mut store, (50,))?;
    assert_eq!(n, 50);

    Ok(())
}

/// The cap counts live handles, not cumulative allocations: churning well past
/// the limit succeeds because each handle is dropped. Catches a missing
/// decrement in `remove`.
#[test]
fn freeing_handles_returns_budget() -> Result<()> {
    let engine = engine();
    let component = Component::new(&engine, COMPONENT)?;
    let linker = Linker::new(&engine);

    let mut store = Store::new(&engine, ());
    let limit = 100;
    store.set_max_component_handles(limit);

    let instance = linker.instantiate(&mut store, &component)?;
    let churn0 = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "churn0")?;

    // Far more allocations than the limit, each dropped, so live count stays low.
    let total = u32::try_from(limit).unwrap() * 100;
    let (n,) = churn0.call(&mut store, (total,))?;
    assert_eq!(n, total);

    // The budget is fully available again afterward.
    let spray0 = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "spray0")?;
    let (n,) = spray0.call(&mut store, (50,))?;
    assert_eq!(n, 50);

    Ok(())
}
