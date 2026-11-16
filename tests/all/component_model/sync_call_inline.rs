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
