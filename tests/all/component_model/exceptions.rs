//! Tests for the interaction of Wasm exceptions and the component model.

#![cfg(not(miri))]

use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Result, Store, Trap};

/// Two components linked together: `$A` exports a function that throws an
/// (uncaught) exception, and `$B` calls it through the component boundary
/// with a `try_table` whose `catch_all` would observe any exception that
/// leaks across the boundary. `run` returns 1 if the exception was caught in
/// `$B` and 0 if the call returned normally; per the canonical ABI neither
/// should happen and the cross-component call should trap instead.
const THROW_ACROSS_BOUNDARY: &str = r#"
(component
  (component $A
    (core module $m
      (tag $t)
      (func (export "throw") (throw $t))
    )
    (core instance $i (instantiate $m))
    (func (export "throw") (canon lift (core func $i "throw")))
  )
  (component $B
    (import "f" (func $f))
    (core func $f-core (canon lower (func $f)))
    (core module $m
      (import "" "f" (func $f))
      (func (export "run") (result i32)
        (block $caught
          (try_table (catch_all $caught)
            (call $f))
          (return (i32.const 0)))
        (i32.const 1))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f-core))))))
    (func (export "run") (result u32) (canon lift (core func $i "run")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "throw"))))
  (export "run" (func $b "run"))
)
"#;

/// Same shape as above, but `$A` catches its own exception and returns
/// normally: nothing crosses the component boundary, so the call must
/// succeed and `$B` must see a normal return (0).
const CATCH_WITHIN_COMPONENT: &str = r#"
(component
  (component $A
    (core module $m
      (tag $t)
      (func $throw (throw $t))
      (func (export "f")
        (block $caught
          (try_table (catch_all $caught)
            (call $throw))))
    )
    (core instance $i (instantiate $m))
    (func (export "f") (canon lift (core func $i "f")))
  )
  (component $B
    (import "f" (func $f))
    (core func $f-core (canon lower (func $f)))
    (core module $m
      (import "" "f" (func $f))
      (func (export "run") (result i32)
        (block $caught
          (try_table (catch_all $caught)
            (call $f))
          (return (i32.const 0)))
        (i32.const 1))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f-core))))))
    (func (export "run") (result u32) (canon lift (core func $i "run")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "run" (func $b "run"))
)
"#;

fn run_component(wat: &str) -> Result<u32> {
    let config = super::config();
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_typed_func::<(), (u32,)>(&mut store, "run")?;
    let (result,) = run.call(&mut store, ())?;
    Ok(result)
}

#[test]
fn exception_does_not_cross_component_boundary() -> Result<()> {
    let err = run_component(THROW_ACROSS_BOUNDARY).unwrap_err();
    assert_eq!(
        err.downcast_ref::<Trap>(),
        Some(&Trap::UncaughtException),
        "expected uncaught-exception trap, got: {err:?}"
    );
    Ok(())
}

#[test]
fn exception_caught_within_component_is_ok() -> Result<()> {
    // The exception is caught within component $A, so the cross-component
    // call returns normally and $B's catch_all is not reached.
    assert_eq!(run_component(CATCH_WITHIN_COMPONENT)?, 0);
    Ok(())
}
