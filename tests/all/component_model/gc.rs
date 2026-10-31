#![cfg(not(miri))]

use std::fmt::Write;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Result, Store};
use wasmtime_test_util::component::config;

const INNER_LEN: usize = 1200;

#[test]
fn wide_nested_tuple_translation_is_outlined() -> Result<()> {
    let repeat = |s: &str| (0..INNER_LEN).map(|_| s).collect::<String>();
    let inner_tuple = repeat(" u32");
    let inner_fields = repeat(" (field (mut i32))");

    // Every element but the last is a constant, so the value read back out of
    // the last one is the value passed in.
    let inner_operands = (0..INNER_LEN).fold(String::new(), |mut s, i| {
        if i == INNER_LEN - 1 {
            s.push_str("local.get 0\n      ");
        } else {
            write!(s, "i32.const {i}\n      ").unwrap();
        }
        s
    });
    let last = INNER_LEN - 1;

    let component = format!(
        r#"
(component
  (component $A
    (core module $m
      (rec
        (type $inner (struct{inner_fields}))
        (type $outer (struct (field (mut i32)) (field (mut (ref null $inner)))))
      )
      (func (export "f") (param (ref null $outer)) (result (ref null $outer))
        local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f")
      (param "a" (tuple u32 (tuple{inner_tuple})))
      (result (tuple u32 (tuple{inner_tuple})))
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core rec
    (type $inner (struct{inner_fields}))
    (type $outer (struct (field (mut i32)) (field (mut (ref null $inner)))))
  )
  (core type $sig (func (param (ref null $outer)) (result (ref null $outer))))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (rec
      (type $inner (struct{inner_fields}))
      (type $outer (struct (field (mut i32)) (field (mut (ref null $inner)))))
    )
    (import "" "f" (func $f (param (ref null $outer)) (result (ref null $outer))))
    (func (export "g") (param i32) (result i32)
      i32.const 0
      {inner_operands}
      struct.new $inner
      struct.new $outer
      call $f
      struct.get $outer 1
      struct.get $inner {last})
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (result u32)
    (canon lift (core func $i "g") gc))
)
        "#,
    );

    let mut config = config();
    config.wasm_gc(true);
    config.wasm_function_references(true);
    config.wasm_component_model_gc(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(&engine, &component)?;
    let linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;
    let g = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "g")?;
    assert_eq!(g.call(&mut store, (42,))?, (42,));

    Ok(())
}

#[test]
fn wide_top_level_tuple_translation_is_outlined() -> Result<()> {
    let repeat = |s: &str| (0..INNER_LEN).map(|_| s).collect::<String>();
    let tuple = repeat(" u32");
    let fields = repeat(" (field (mut i32))");

    let operands = (0..INNER_LEN).fold(String::new(), |mut s, i| {
        if i == INNER_LEN - 1 {
            s.push_str("local.get 0\n      ");
        } else {
            write!(s, "i32.const {i}\n      ").unwrap();
        }
        s
    });
    let last = INNER_LEN - 1;

    let component = format!(
        r#"
(component
  (component $A
    (core module $m
      (rec (type $t (struct{fields})))
      (func (export "f") (param (ref null $t)) (param (ref null $t)) (result (ref null $t))
        local.get 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f")
      (param "a" (tuple{tuple})) (param "b" (tuple{tuple}))
      (result (tuple{tuple}))
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core rec (type $t (struct{fields})))
  (core type $sig
    (func (param (ref null $t)) (param (ref null $t)) (result (ref null $t))))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (rec (type $t (struct{fields})))
    (import "" "f"
      (func $f (param (ref null $t)) (param (ref null $t)) (result (ref null $t))))
    (func (export "g") (param i32) (result i32)
      {operands}
      struct.new $t
      {operands}
      struct.new $t
      call $f
      struct.get $t {last})
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" u32) (result u32)
    (canon lift (core func $i "g") gc))
)
        "#,
    );

    let mut config = config();
    config.wasm_gc(true);
    config.wasm_function_references(true);
    config.wasm_component_model_gc(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(&engine, &component)?;
    let linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;
    let g = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "g")?;
    assert_eq!(g.call(&mut store, (7,))?, (7,));

    Ok(())
}
