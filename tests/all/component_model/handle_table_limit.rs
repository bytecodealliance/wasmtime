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

/// Borrows lent to the resource's owner count against the limit while the
/// call is active, and return their budget when it returns.
#[test]
fn lends_to_owner_count_against_the_limit() -> Result<()> {
    // `$D` defines a resource and `$U` passes a `list<borrow<R>>` of `n` copies
    // of one handle to `$D`'s `take`, `k` times. `$D` receives reps, so no
    // handles are created for the borrows, but each lend still counts against
    // the limit until `take` returns.
    const LEND_COMPONENT: &str = r#"
(component
  (component $D
    (type $R (resource (rep i32)))
    (core func $new (canon resource.new $R))
    (core module $m
      (import "" "new" (func $new (param i32) (result i32)))
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) (i32.const 16))
      (func (export "make") (result i32) (call $new (i32.const 42)))
      (func (export "take") (param i32 i32) (result i32) (local.get 1)))
    (core instance $i (instantiate $m (with "" (instance (export "new" (func $new))))))
    (export $Rx "r" (type $R))
    (func (export "make") (result (own $Rx)) (canon lift (core func $i "make")))
    (func (export "take") (param "xs" (list (borrow $Rx))) (result u32)
      (canon lift (core func $i "take") (memory (core memory $i "mem"))
        (realloc (core func $i "realloc")))))
  (instance $d (instantiate $D))

  (component $U
    (import "r" (type $R (sub resource)))
    (import "make" (func $make (result (own $R))))
    (import "take" (func $take (param "xs" (list (borrow $R))) (result u32)))
    (core module $mm (memory (export "mem") 1))
    (core instance $mi (instantiate $mm))
    (core func $make (canon lower (func $make)))
    (core func $take (canon lower (func $take) (memory (core memory $mi "mem"))))
    (core module $m
      (import "" "mem" (memory 1))
      (import "" "make" (func $make (result i32)))
      (import "" "take" (func $take (param i32 i32) (result i32)))
      (func (export "run") (param $n i32) (param $k i32) (result i32)
        (local $h i32) (local $i i32)
        (local.set $h (call $make))
        (loop $l
          (if (i32.lt_u (local.get $i) (local.get $n)) (then
            (i32.store (i32.shl (local.get $i) (i32.const 2)) (local.get $h))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $l))))
        (local.set $i (i32.const 0))
        (loop $l
          (if (i32.lt_u (local.get $i) (local.get $k)) (then
            (drop (call $take (i32.const 0) (local.get $n)))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $l))))
        (local.get $k)))
    (core instance $i (instantiate $m (with "" (instance
      (export "mem" (memory $mi "mem"))
      (export "make" (func $make)) (export "take" (func $take))))))
    (func (export "run") (param "n" u32) (param "k" u32) (result u32)
      (canon lift (core func $i "run"))))
  (instance $u (instantiate $U (with "r" (type $d "r"))
    (with "make" (func $d "make")) (with "take" (func $d "take"))))
  (export "run" (func $u "run"))
)
"#;
    let engine = engine();
    let component = Component::new(&engine, LEND_COMPONENT)?;
    let linker = Linker::new(&engine);

    let mut store = Store::new(&engine, ());
    store.set_max_component_handles(20);
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_typed_func::<(u32, u32), (u32,)>(&mut store, "run")?;

    // Ten lends at a time, many times over, fit.
    let (n,) = run.call(&mut store, (10, 100))?;
    assert_eq!(n, 100);

    let mut store = Store::new(&engine, ());
    store.set_max_component_handles(20);
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_typed_func::<(u32, u32), (u32,)>(&mut store, "run")?;

    // Fifty lends in one call don't.
    run.call(&mut store, (50, 1))
        .unwrap_err()
        .assert_contains("store handle limit exceeded");

    Ok(())
}
