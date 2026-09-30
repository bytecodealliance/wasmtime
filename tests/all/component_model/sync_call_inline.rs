//! Runtime tests for the guest-to-guest sync-call inline fast path.

#![cfg(not(miri))]

use wasmtime::component::*;
use wasmtime::{Config, Engine, Result, Store, StoreContextMut};

fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_async(true);
    Engine::new(&config).unwrap()
}

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
    assert_eq!(
        *store.data(),
        4,
        "host import called once per top-level call"
    );
    Ok(())
}

#[tokio::test]
async fn transparent_frame_nested_inside_opaque_one() -> Result<()> {
    let component = r#"
(component
  (import "poke" (func $poke))

  (component $Leaf
    (core module $M
      (func (export "leaf'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 1))))
    (core instance $m (instantiate $M))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf'"))))

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (import "poke" (func $poke))
    (core func $leaf' (canon lower (func $leaf)))
    (core func $poke' (canon lower (func $poke)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "leaf'" (func $leaf' (param i32) (result i32)))
      (import "" "poke" (func $poke'))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "mid'") (param i32) (result i32) (local $r i32)
        ;; Freshly entered deferred thread: context starts zeroed.
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x0d00d100))
        ;; Call through the transparent adapter, which pushes no frame.
        (local.set $r (call $leaf' (local.get 0)))
        ;; Only now force the deferred thread, after the omitted frame has
        ;; come and gone.
        (call $poke')
        (if (i32.ne (call $cget) (i32.const 0x0d00d100)) (then unreachable))
        (i32.add (local.get $r) (i32.const 10))))
    (core instance $m (instantiate $M (with "" (instance
      (export "leaf'" (func $leaf'))
      (export "poke" (func $poke'))
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

  (instance $leaf (instantiate $Leaf))
  (instance $mid (instantiate $Mid
    (with "leaf" (func $leaf "leaf"))
    (with "poke" (func $poke))))
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

    // Call more than once so that any state the omitted frame failed to clean
    // up would be visible on a later call.
    for i in 1..=3 {
        let (result,) = root.call_async(&mut store, ()).await?;
        assert_eq!(result, 1111);
        assert_eq!(*store.data(), i);
    }
    Ok(())
}

#[tokio::test]
async fn opaque_frame_nested_inside_transparent_one() -> Result<()> {
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
        ;; The maintained frame zeroed the context on the way in.
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x0feed000))
        ;; Force the deferred thread from the innermost frame.
        (call $poke')
        (if (i32.ne (call $cget) (i32.const 0x0feed000)) (then unreachable))
        (i32.add (local.get 0) (i32.const 1))))
    (core instance $m (instantiate $M (with "" (instance
      (export "poke" (func $poke'))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf'"))))

  ;; Nothing in here can observe or mutate thread state, so the adapter that
  ;; lifts `mid` is thread-transparent.
  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (core func $leaf' (canon lower (func $leaf)))
    (core module $M
      (import "" "leaf'" (func $leaf' (param i32) (result i32)))
      (func (export "mid'") (param i32) (result i32)
        (i32.add (call $leaf' (local.get 0)) (i32.const 10))))
    (core instance $m (instantiate $M (with "" (instance
      (export "leaf'" (func $leaf'))))))
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
        ;; Neither `$Mid` nor `$Leaf` disturbed our slot.
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

    for i in 1..=3 {
        let (result,) = root.call_async(&mut store, ()).await?;
        assert_eq!(result, 1111);
        assert_eq!(*store.data(), i);
    }
    Ok(())
}

#[tokio::test]
async fn trap_unwinds_through_transparent_frames() -> Result<()> {
    let component = r#"
(component
  (import "poke" (func $poke))

  (component $Leaf
    (core module $M
      (func (export "leaf'") (param i32) (result i32) unreachable))
    (core instance $m (instantiate $M))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf'"))))

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (import "poke" (func $poke))
    (core func $leaf' (canon lower (func $leaf)))
    (core func $poke' (canon lower (func $poke)))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "leaf'" (func $leaf' (param i32) (result i32)))
      (import "" "poke" (func $poke'))
      (import "" "cset" (func $cset (param i32)))
      (func (export "mid'") (param i32) (result i32)
        (call $cset (i32.const 0x0d00d100))
        ;; Force the deferred thread first so that the unwind below has a real
        ;; task to tear down rather than just a stack slot.
        (call $poke')
        (call $leaf' (local.get 0))))
    (core instance $m (instantiate $M (with "" (instance
      (export "leaf'" (func $leaf'))
      (export "poke" (func $poke'))
      (export "cset" (func $cset))))))
    (func (export "mid") (param "x" u32) (result u32)
      (canon lift (core func $m "mid'"))))

  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid' (canon lower (func $mid)))
    (core module $M
      (import "" "mid'" (func $mid' (param i32) (result i32)))
      (func (export "root'") (result i32) (call $mid' (i32.const 1))))
    (core instance $m (instantiate $M (with "" (instance
      (export "mid'" (func $mid'))))))
    (func (export "root") (result u32)
      (canon lift (core func $m "root'"))))

  (instance $leaf (instantiate $Leaf))
  (instance $mid (instantiate $Mid
    (with "leaf" (func $leaf "leaf"))
    (with "poke" (func $poke))))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
    "#;

    let other = r#"
(component
  (core module $m (func (export "x")))
  (core instance (instantiate $m))
)
    "#;

    let engine = engine();
    let component = Component::new(&engine, component)?;
    let other = Component::new(&engine, other)?;
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

    let err = root.call_async(&mut store, ()).await.unwrap_err();
    assert!(
        err.downcast_ref::<wasmtime::Trap>().is_some(),
        "expected a trap, got: {err:?}"
    );
    assert_eq!(*store.data(), 1);

    // The store is still usable afterwards.
    let _ = linker.instantiate_async(&mut store, &other).await?;
    Ok(())
}

#[tokio::test]
async fn trap_then_instantiate_uses_freed_deferred_thread() -> Result<()> {
    let trapping = [
        // Root -> Mid -> Leaf then trap
        r#"
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
        "#,
        // A -> B -> uncaught exception
        r#"
(component
  (component $A
    (core module $M
      (type $t (func))
      (tag $t (type $t))
      (func (export "run") throw $t)
    )
    (core instance $m (instantiate $M))
    (func (export "run") (canon lift (core func $m "run")))
  )
  (component $B
    (import "run" (func $run))
    (core func $run (canon lower (func $run)))
    (core module $M
      (import "" "run" (func $run))
      (func (export "root") (result i32)
        call $run
        i32.const 0
      )
    )
    (core instance $m (instantiate $M (with "" (instance (export "run" (func $run))))))
    (func (export "root") (result u32) (canon lift (core func $m "root")))
  )
  (instance $A (instantiate $A))
  (instance $B (instantiate $B (with "run" (func $A "run"))))
  (export "root" (func $B "root"))
)
        "#,
        // Trapping resource destructor
        r#"
(component
  (component $R
    (core module $M
      (func (export "dtor") (param i32) unreachable)
    )
    (core instance $i (instantiate $M))
    (type $R (resource (rep i32) (dtor (core func $i "dtor"))))
    (export $R' "R" (type $R))
    (core func $new (canon resource.new $R))
    (func (export "new") (param "x" u32) (result (own $R'))
      (canon lift (core func $new))
    )
  )
  (component $B
    (import "R" (instance $R
      (export "R" (type $R (sub resource)))
      (export "new" (func (param "x" u32) (result (own $R))))
    ))

    (core module $M
      (import "" "new" (func $new (param i32) (result i32)))
      (import "" "dtor" (func $dtor (param i32)))

      (func (export "run") (result i32)
        (call $dtor (call $new (i32.const 10)))

        i32.const 0
      )
    )
    (core func $new (canon lower (func $R "new")))
    (core func $dtor (canon resource.drop (type $R "R")))
    (core instance $i (instantiate $M
      (with "" (instance
        (export "new" (func $new))
        (export "dtor" (func $dtor))
      ))
    ))
    (func (export "run") (result u32)
      (canon lift (core func $i "run"))
    )
  )
  (instance $R (instantiate $R))
  (instance $B (instantiate $B (with "R" (instance $R))))
  (export "root" (func $B "run"))
)
        "#,
        // resource destructor with uncaught exception
        r#"
(component
  (component $R
    (core module $M
      (type $t (func))
      (tag $t (type $t))
      (func (export "dtor") (param i32) throw $t)
    )
    (core instance $i (instantiate $M))
    (type $R (resource (rep i32) (dtor (core func $i "dtor"))))
    (export $R' "R" (type $R))
    (core func $new (canon resource.new $R))
    (func (export "new") (param "x" u32) (result (own $R'))
      (canon lift (core func $new))
    )
  )
  (component $B
    (import "R" (instance $R
      (export "R" (type $R (sub resource)))
      (export "new" (func (param "x" u32) (result (own $R))))
    ))

    (core module $M
      (import "" "new" (func $new (param i32) (result i32)))
      (import "" "dtor" (func $dtor (param i32)))

      (func (export "run") (result i32)
        (call $dtor (call $new (i32.const 10)))

        i32.const 0
      )
    )
    (core func $new (canon lower (func $R "new")))
    (core func $dtor (canon resource.drop (type $R "R")))
    (core instance $i (instantiate $M
      (with "" (instance
        (export "new" (func $new))
        (export "dtor" (func $dtor))
      ))
    ))
    (func (export "run") (result u32)
      (canon lift (core func $i "run"))
    )
  )
  (instance $R (instantiate $R))
  (instance $B (instantiate $B (with "R" (instance $R))))
  (export "root" (func $B "run"))
)
        "#,
    ];

    let other = r#"
(component
  (core module $m (func (export "x")))
  (core instance (instantiate $m))
)
    "#;

    let engine = engine();
    let other = Component::new(&engine, other)?;
    for trapping in trapping {
        let trapping = Component::new(&engine, trapping)?;
        let mut store = Store::new(&engine, 0u32);
        let linker = Linker::new(&engine);

        let instance = linker.instantiate_async(&mut store, &trapping).await?;
        let root = instance.get_typed_func::<(), (u32,)>(&mut store, "root")?;
        let err = root.call_async(&mut store, ()).await.unwrap_err();
        assert!(
            err.downcast_ref::<wasmtime::Trap>().is_some(),
            "expected a trap, got: {err:?}"
        );

        let _ = linker.instantiate_async(&mut store, &other).await?;
        wasmtime::component::FutureReader::new(&mut store, async move { wasmtime::error::Ok(1) })?;
    }
    Ok(())
}

#[tokio::test]
async fn shared_table_does_not_leak_context() -> Result<()> {
    // The `$Victim`/`$Evil` pair, parameterized over which core module owns the
    // shared table and what else `$Victim` imports.
    fn component(inner: &str) -> String {
        format!(
            r#"
(component
  (component $Inner {inner})

  (component $Outer
    (import "f" (func $f (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $N
      (import "" "f" (func $f' (result i32)))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      ;; What the callee saw in *our* slot.
      (func (export "g") (result i32)
        (call $cset (i32.const 0x1234))
        (call $f'))
      ;; Our slot after the call returns.
      (func (export "h") (result i32)
        (call $cset (i32.const 0x1234))
        (drop (call $f'))
        (call $cget)))
    (core instance $n (instantiate $N (with "" (instance
      (export "f" (func $f'))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))
    (func (export "g") (result u32) (canon lift (core func $n "g")))
    (func (export "h") (result u32) (canon lift (core func $n "h"))))

  (instance $inner (instantiate $Inner))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
  (export "h" (func $outer "h"))
)
            "#
        )
    }

    // `$Evil`'s body: read the caller's slot, clobber it, return what was read.
    // The element segment writes into the *imported* table, so this happens
    // during instantiation with no start function or host call involved.
    const EVIL: &str = r#"
    (core module $Evil
      (import "" "t" (table 1 funcref))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func $leak (result i32) (local $t i32)
        (local.set $t (call $cget))
        (call $cset (i32.const 0x9999))
        (local.get $t))
      (elem (table 0) (i32.const 0) func $leak))
    "#;

    let cases = [
        // The table is defined by a third module that imports nothing at all.
        component(&format!(
            r#"
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))

    (core module $Shared (table (export "t") 1 funcref))
    (core instance $shared (instantiate $Shared))

    {EVIL}
    (core instance $evil (instantiate $Evil (with "" (instance
      (export "t" (table $shared "t"))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))

    (core module $Victim
      (import "" "t" (table 1 funcref))
      (type $sig (func (result i32)))
      (func (export "f") (result i32)
        (call_indirect (type $sig) (i32.const 0))))
    (core instance $victim (instantiate $Victim (with "" (instance
      (export "t" (table $shared "t"))))))

    (func (export "f") (result u32) (canon lift (core func $victim "f")))
            "#
        )),
        // Same thing with only two core modules: the table is defined and
        // exported by `$Victim` itself, which still imports nothing.
        component(&format!(
            r#"
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))

    (core module $Victim
      (table (export "t") 1 funcref)
      (type $sig (func (result i32)))
      (func (export "f") (result i32)
        (call_indirect (type $sig) (i32.const 0))))
    (core instance $victim (instantiate $Victim))

    {EVIL}
    (core instance $evil (instantiate $Evil (with "" (instance
      (export "t" (table $victim "t"))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))

    (func (export "f") (result u32) (canon lift (core func $victim "f")))
            "#
        )),
        // Control: `$Victim` also directly imports `context.get`, which even a
        // core-instance-granularity analysis would have seen.
        component(&format!(
            r#"
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))

    (core module $Shared (table (export "t") 1 funcref))
    (core instance $shared (instantiate $Shared))

    {EVIL}
    (core instance $evil (instantiate $Evil (with "" (instance
      (export "t" (table $shared "t"))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))

    (core module $Victim
      (import "" "t" (table 1 funcref))
      (import "" "cget" (func (result i32)))
      (type $sig (func (result i32)))
      (func (export "f") (result i32)
        (call_indirect (type $sig) (i32.const 0))))
    (core instance $victim (instantiate $Victim (with "" (instance
      (export "t" (table $shared "t"))
      (export "cget" (func $cget))))))

    (func (export "f") (result u32) (canon lift (core func $victim "f")))
            "#
        )),
    ];

    let engine = engine();
    for (i, wat) in cases.iter().enumerate() {
        let component = Component::new(&engine, wat)?;
        let mut store = Store::new(&engine, ());
        let linker = Linker::new(&engine);
        let instance = linker.instantiate_async(&mut store, &component).await?;

        let g = instance.get_typed_func::<(), (u32,)>(&mut store, "g")?;
        let (leaked,) = g.call_async(&mut store, ()).await?;
        assert_eq!(
            leaked, 0,
            "case {i}: callee read the caller's private context slot: {leaked:#x}",
        );

        let h = instance.get_typed_func::<(), (u32,)>(&mut store, "h")?;
        let (after,) = h.call_async(&mut store, ()).await?;
        assert_eq!(
            after, 0x1234,
            "case {i}: callee clobbered the caller's context slot: {after:#x}",
        );
    }
    Ok(())
}
