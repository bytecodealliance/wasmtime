//! Runtime tests for the guest-to-guest sync-call fast path (issue #12311).
//!
//! When concurrency support is enabled a fused sync-to-sync adapter lowers its
//! `enter-sync-call`/`exit-sync-call` intrinsics inline: `enter` pushes an
//! on-stack `VMDeferredThread` (saving and zeroing the caller's component
//! `context.{get,set}` slots), and `exit` pops it inline -- unless host code has
//! read the current thread in the meantime, in which case the deferred thread
//! is "forced" into a real one and `exit` falls back to the out-of-line libcall.
//!
//! Calling an imported *host* function from inside the callee is the canonical
//! way to force the deferred thread: every guest->host boundary runs
//! `StoreOpaque::force_current_thread`. These tests drive that path through the
//! public `TypedFunc` API and use `context.{get,set}` as a guest-observable
//! witness that the save / zero / restore / replay logic is correct. The
//! sibling `.wast` tests cover the pure fast path (no host calls); here we
//! deliberately force the slow path with a real host import.

#![cfg(not(miri))]

use wasmtime::component::*;
use wasmtime::{Config, Engine, Result, Store, StoreContextMut};

/// An engine whose stores have component-model concurrency support (and thus
/// the inline sync-to-sync adapter optimization) enabled.
fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_async(true);
    Engine::new(&config).unwrap()
}

/// A single guest-to-guest sync call whose callee forces the deferred thread by
/// calling an imported host function. The slow `exit-sync-call` path must still
/// restore the caller's context slot, and the callee's mutation must not leak.
#[tokio::test]
async fn host_call_forces_slow_path_preserves_context() -> Result<()> {
    let component = r#"
(component
  (import "poke" (func $poke))

  (component $Inner
    (import "poke" (func $poke))
    (core func $poke' (canon lower (func $poke)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "poke" (func $poke'))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "f'") (param i32) (result i32)
        ;; Freshly entered deferred thread: context starts zeroed.
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x5678))
        ;; Force the deferred thread via a guest->host call.
        (call $poke')
        ;; Our context survives the force.
        (if (i32.ne (call $cget) (i32.const 0x5678)) (then unreachable))
        (i32.add (local.get 0) (i32.const 42))))
    (core instance $m (instantiate $M (with "" (instance
      (export "poke" (func $poke'))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))))

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "g'") (result i32) (local $r i32)
        (call $cset (i32.const 0x1234))
        (local.set $r (call $f' (i32.const 1234)))
        ;; Restored after the callee forced the slow exit path.
        (if (i32.ne (call $cget) (i32.const 0x1234)) (then unreachable))
        (local.get $r)))
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))
    (func (export "g") (result u32)
      (canon lift (core func $n "g'"))))

  (instance $inner (instantiate $Inner (with "poke" (func $poke))))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
)
    "#;

    let engine = engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, 0u32);
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("poke", |mut cx: StoreContextMut<u32>, (): ()| {
            *cx.data_mut() += 1;
            Ok(())
        })?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let g = instance.get_typed_func::<(), (u32,)>(&mut store, "g")?;

    let (result,) = g.call_async(&mut store, ()).await?;
    assert_eq!(result, 1276);
    assert_eq!(*store.data(), 1, "host import should have been called once");
    Ok(())
}

/// A three-deep guest-to-guest chain (Root -> Mid -> Leaf) where the innermost
/// callee forces. Forcing must walk and replay *both* suspended deferred frames
/// and every level's context slot must come back intact.
#[tokio::test]
async fn nested_chain_host_force_preserves_all_contexts() -> Result<()> {
    let component = r#"
(component
  (import "poke" (func $poke))

  (component $Leaf
    (import "poke" (func $poke))
    (core func $poke' (canon lower (func $poke)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "poke" (func $poke'))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "leaf'") (param i32) (result i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x0c0ffee0))
        (call $poke')
        (if (i32.ne (call $cget) (i32.const 0x0c0ffee0)) (then unreachable))
        (i32.add (local.get 0) (i32.const 1))))
    (core instance $m (instantiate $M (with "" (instance
      (export "poke" (func $poke'))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf'"))))

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (core func $leaf' (canon lower (func $leaf)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "leaf'" (func $leaf' (param i32) (result i32)))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "mid'") (param i32) (result i32) (local $r i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x0d00d100))
        (local.set $r (call $leaf' (local.get 0)))
        (if (i32.ne (call $cget) (i32.const 0x0d00d100)) (then unreachable))
        (i32.add (local.get $r) (i32.const 10))))
    (core instance $m (instantiate $M (with "" (instance
      (export "leaf'" (func $leaf'))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))
    (func (export "mid") (param "x" u32) (result u32)
      (canon lift (core func $m "mid'"))))

  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid' (canon lower (func $mid)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "mid'" (func $mid' (param i32) (result i32)))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "root'") (result i32) (local $r i32)
        (call $cset (i32.const 0x0badf00d))
        (local.set $r (call $mid' (i32.const 100)))
        (if (i32.ne (call $cget) (i32.const 0x0badf00d)) (then unreachable))
        (i32.add (local.get $r) (i32.const 1000))))
    (core instance $m (instantiate $M (with "" (instance
      (export "mid'" (func $mid'))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))
    (func (export "root") (result u32)
      (canon lift (core func $m "root'"))))

  (instance $leaf (instantiate $Leaf (with "poke" (func $poke))))
  (instance $mid (instantiate $Mid (with "leaf" (func $leaf "leaf"))))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
    "#;

    let engine = engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, 0u32);
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("poke", |mut cx: StoreContextMut<u32>, (): ()| {
            *cx.data_mut() += 1;
            Ok(())
        })?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let root = instance.get_typed_func::<(), (u32,)>(&mut store, "root")?;

    let (result,) = root.call_async(&mut store, ()).await?;
    assert_eq!(result, 1111);
    assert_eq!(*store.data(), 1);
    Ok(())
}

/// Repeated top-level calls must each set up and tear down their own deferred
/// frame independently: the callee keeps observing a freshly zeroed context and
/// the caller's context is restored every time, with no state left dangling
/// between adapter invocations.
#[tokio::test]
async fn repeated_calls_have_no_state_leak() -> Result<()> {
    let component = r#"
(component
  (import "poke" (func $poke))

  (component $Inner
    (import "poke" (func $poke))
    (core func $poke' (canon lower (func $poke)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "poke" (func $poke'))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "f'") (param i32) (result i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (local.get 0))
        (call $poke')
        (if (i32.ne (call $cget) (local.get 0)) (then unreachable))
        (i32.add (local.get 0) (i32.const 42))))
    (core instance $m (instantiate $M (with "" (instance
      (export "poke" (func $poke'))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))))

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "g'") (param i32) (result i32) (local $r i32)
        (call $cset (i32.const 0x4321))
        (local.set $r (call $f' (local.get 0)))
        (if (i32.ne (call $cget) (i32.const 0x4321)) (then unreachable))
        (local.get $r)))
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))
    (func (export "g") (param "x" u32) (result u32)
      (canon lift (core func $n "g'"))))

  (instance $inner (instantiate $Inner (with "poke" (func $poke))))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
)
    "#;

    let engine = engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, 0u32);
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("poke", |mut cx: StoreContextMut<u32>, (): ()| {
            *cx.data_mut() += 1;
            Ok(())
        })?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let g = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "g")?;

    for x in [7u32, 100, 0x10000, 1] {
        let (result,) = g.call_async(&mut store, (x,)).await?;
        assert_eq!(result, x + 42);
    }
    assert_eq!(*store.data(), 4, "host import called once per top-level call");
    Ok(())
}

/// Regression test for a use-after-free in the inline sync-to-sync fast path
/// (issue #12311).
///
/// The inline `enter-sync-call` publishes `VMStoreContext::current_thread`
/// pointing at a `VMDeferredThread` that lives in the *fused adapter's stack
/// frame*. If the callee traps, the stack unwinds and discards that frame; the
/// teardown `exit_guest_sync_call` runs only on the success path (via
/// post-return), so without a fix-up `current_thread` would be left dangling in
/// the store.
///
/// Re-*entering* the component is gated by `may_enter`, which short-circuits on
/// `trapped()` before forcing -- but *instantiation* is not gated. Instantiating
/// another component in the same store runs `enter_guest_sync_call` ->
/// `force_current_thread`, whose `unsafe { &*ptr }` parent-chain walk would then
/// dereference the dangling deferred-thread pointer (a freed stack frame). A
/// two-deep deferred chain (Root -> Mid -> Leaf, where Leaf traps) leaves two
/// freed `VMDeferredThread` records linked by `parent`, so the walk follows a
/// `parent` pointer read out of freed stack memory.
///
/// Before the fix this aborted/segfaulted reliably (the freed `parent` slot
/// being read as a misaligned/garbage pointer); `invoke_wasm_and_catch_traps`
/// now resets `current_thread` to the "forced" sentinel on the trap path, so the
/// later force is a cheap no-op and this instantiation completes cleanly.
#[tokio::test]
async fn trap_then_instantiate_uses_freed_deferred_thread() -> Result<()> {
    // A nested guest-to-guest sync chain (Root -> Mid -> Leaf) whose innermost
    // callee traps mid-flight. Each hop's fused adapter has published a
    // `VMDeferredThread` in its own (now unwound) stack frame, and
    // `current_thread` is left pointing at the innermost one with a `parent`
    // link to the next -- two dangling records for `force_current_thread` to
    // walk.
    let trapping = r#"
(component
  (component $Leaf
    (core module $M (func (export "leaf'") (param i32) (result i32) unreachable))
    (core instance $m (instantiate $M))
    (func (export "leaf") (param "x" u32) (result u32) (canon lift (core func $m "leaf'"))))
  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (core func $leaf' (canon lower (func $leaf)))
    (core module $M
      (import "" "leaf'" (func $leaf' (param i32) (result i32)))
      (func (export "mid'") (param i32) (result i32) (call $leaf' (local.get 0))))
    (core instance $m (instantiate $M (with "" (instance (export "leaf'" (func $leaf'))))))
    (func (export "mid") (param "x" u32) (result u32) (canon lift (core func $m "mid'"))))
  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid' (canon lower (func $mid)))
    (core module $M
      (import "" "mid'" (func $mid' (param i32) (result i32)))
      (func (export "root'") (result i32) (call $mid' (i32.const 1))))
    (core instance $m (instantiate $M (with "" (instance (export "mid'" (func $mid'))))))
    (func (export "root") (result u32) (canon lift (core func $m "root'"))))
  (instance $leaf (instantiate $Leaf))
  (instance $mid (instantiate $Mid (with "leaf" (func $leaf "leaf"))))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
    "#;

    // Any second component whose instantiation drives a core instance runs
    // `enter_guest_sync_call` -> `force_current_thread`, which reads the dangling
    // pointer left behind by the trap above.
    let other = r#"
(component
  (core module $m (func (export "x")))
  (core instance (instantiate $m))
)
    "#;

    let engine = engine();
    let trapping = Component::new(&engine, trapping)?;
    let other = Component::new(&engine, other)?;
    let mut store = Store::new(&engine, 0u32);
    let linker = Linker::new(&engine);

    let instance = linker.instantiate_async(&mut store, &trapping).await?;
    let root = instance.get_typed_func::<(), (u32,)>(&mut store, "root")?;
    let err = root.call_async(&mut store, ()).await.unwrap_err();
    assert!(
        err.downcast_ref::<wasmtime::Trap>().is_some(),
        "expected a trap, got: {err:?}"
    );

    // BUG: this instantiation forces the dangling deferred thread (use-after-free).
    // With the bug fixed it instantiates cleanly and the test passes.
    let _ = linker.instantiate_async(&mut store, &other).await?;
    Ok(())
}
