#![cfg(not(miri))]

use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Result, Store};

#[test]
fn fixed_length_list() -> Result<()> {
    // nested roundtrip returns the parameters in a tuple
    // this is the same component as in tests/misc_testsuite/component-model/fixed_length_lists.wast
    // as this test was compiled from C(++) the assembly is less human readable
    let component = r##"
(component
  (core module $main (;0;)
    (type (;0;) (func (param i32 i32 i32 i32 i32 i32 i32 i32) (result i32)))
    (memory (;0;) 2)
    (global (;0;) (mut i32) i32.const 67248)
    (export "memory" (memory 0))
    (export "test:fixed-length-lists/to-test#nested-roundtrip" (func 0))
    (func (;0;) (type 0) (param i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
      (local i32 i32 i64 i64)
      global.get 0
      i32.const 96
      i32.sub
      local.tee 8
      global.set 0
      local.get 8
      i32.const 56
      i32.add
      local.tee 9
      local.get 2
      i32.store
      local.get 8
      i32.const 40
      i32.add
      local.tee 2
      local.get 6
      i32.store
      local.get 8
      local.get 3
      i32.store offset=60
      local.get 8
      i32.const 16
      i32.add
      local.tee 3
      i32.const 8
      i32.add
      local.tee 6
      local.get 9
      i64.load
      i64.store
      local.get 8
      local.get 7
      i32.store offset=44
      local.get 8
      i32.const 8
      i32.add
      local.tee 7
      local.get 2
      i64.load
      i64.store
      local.get 8
      local.get 0
      i64.extend_i32_u
      local.get 1
      i64.extend_i32_u
      i64.const 32
      i64.shl
      i64.or
      local.tee 10
      i64.store offset=48
      local.get 8
      local.get 4
      i64.extend_i32_u
      local.get 5
      i64.extend_i32_u
      i64.const 32
      i64.shl
      i64.or
      local.tee 11
      i64.store offset=32
      local.get 8
      local.get 10
      i64.store offset=16
      local.get 8
      local.get 11
      i64.store
      local.get 8
      i32.const -64
      i32.sub
      local.tee 0
      local.get 3
      i64.load align=4
      i64.store align=4
      local.get 0
      local.get 8
      i64.load align=4
      i64.store offset=16 align=4
      local.get 0
      i32.const 8
      i32.add
      local.tee 1
      local.get 6
      i64.load align=4
      i64.store align=4
      local.get 0
      i32.const 24
      i32.add
      local.get 7
      i64.load align=4
      i64.store align=4
      i32.const 1064
      local.get 1
      i64.load align=4
      i64.store
      i32.const 1056
      local.get 8
      i64.load offset=64 align=4
      i64.store
      i32.const 1072
      local.get 8
      i64.load offset=80 align=4
      i64.store
      i32.const 1080
      local.get 8
      i32.const 88
      i32.add
      i64.load align=4
      i64.store
      local.get 8
      i32.const 96
      i32.add
      global.set 0
      i32.const 1056
    )
    (data (;0;) (i32.const 1024) "\ff\ff\ff\ff\00\00\02")
    (@producers
      (processed-by "wit-component" "0.243.0")
    )
  )
  (core instance $main (;0;) (instantiate $main))
  (alias core export $main "memory" (core memory $memory (;0;)))
  (type (;0;) (list u32 2))
  (type (;1;) (list 0 2))
  (type (;2;) (list s32 2))
  (type (;3;) (list 2 2))
  (type (;4;) (tuple 1 3))
  (type (;5;) (func (param "a" 1) (param "b" 3) (result 4)))
  (alias core export $main "test:fixed-length-lists/to-test#nested-roundtrip" (core func $test:fixed-length-lists/to-test#nested-roundtrip (;0;)))
  (func $nested-roundtrip (;0;) (type 5) (canon lift (core func $test:fixed-length-lists/to-test#nested-roundtrip) (memory $memory)))
  (component $test:fixed-length-lists/to-test-shim-component (;0;)
    (type (;0;) (list u32 2))
    (type (;1;) (list 0 2))
    (type (;2;) (list s32 2))
    (type (;3;) (list 2 2))
    (type (;4;) (tuple 1 3))
    (type (;5;) (func (param "a" 1) (param "b" 3) (result 4)))
    (import "import-func-nested-roundtrip" (func (;0;) (type 5)))
    (type (;6;) (list u32 2))
    (type (;7;) (list 6 2))
    (type (;8;) (list s32 2))
    (type (;9;) (list 8 2))
    (type (;10;) (tuple 7 9))
    (type (;11;) (func (param "a" 7) (param "b" 9) (result 10)))
    (export (;1;) "nested-roundtrip" (func 0) (func (type 11)))
  )
  (instance $test:fixed-length-lists/to-test-shim-instance (;0;) (instantiate $test:fixed-length-lists/to-test-shim-component
      (with "import-func-nested-roundtrip" (func $nested-roundtrip))
    )
  )
  (export $test:fixed-length-lists/to-test (;1;) "test:fixed-length-lists/to-test" (instance $test:fixed-length-lists/to-test-shim-instance))
  (@producers
    (processed-by "wit-component" "0.243.0")
  )
)
"##;
    wasmtime::component::bindgen!({
        inline: "
        package root:root;

world root {
  export test:fixed-length-lists/to-test;
}
package test:fixed-length-lists {
  interface to-test {
    nested-roundtrip: func(a: list<list<u32, 2>, 2>, b: list<list<s32, 2>, 2>) -> tuple<list<list<u32, 2>, 2>, list<list<s32, 2>, 2>>;
  }
}
",
        imports: {default: trappable},
    });

    let mut config = Config::new();
    config.wasm_component_model_fixed_length_lists(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, 0);
    let linker = Linker::new(&engine);
    let to_test = Root::instantiate(&mut store, &component, &linker)?;
    let inputa: [[u32; 2]; 2] = [[1, 2], [3, 4]];
    let inputb: [[i32; 2]; 2] = [[-1, -2], [-3, -4]];
    let result = to_test
        .interface0
        .call_nested_roundtrip(&mut store, inputa, inputb)
        .expect("no trap");
    assert_eq!(result, ([[1, 2], [3, 4]], [[-1, -2], [-3, -4]]));
    Ok(())
}

#[test]
fn fixed_length_list_length_mismatch_rejected() -> Result<()> {
    let wat = r#"
(component
  (core module $m
    (func (export "run")
      (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
      (result i32)
      local.get 15)
    (memory (export "memory") 1)
  )
  (core instance $i (instantiate $m))
  (type $lst (list u32 16))
  (type $ft (func (param "x" $lst) (result u32)))
  (alias core export $i "run" (core func $run))
  (func (export "run") (type $ft)
    (canon lift (core func $run) (memory (core memory $i "memory"))))
)
"#;
    let mut config = Config::new();
    config.wasm_component_model_fixed_length_lists(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<([u32; 2],), (u32,)>(&mut store, "run")
            .is_err()
    );
    assert!(
        instance
            .get_typed_func::<([u32; 32],), (u32,)>(&mut store, "run")
            .is_err(),
    );

    let func = instance.get_typed_func::<([u32; 16],), (u32,)>(&mut store, "run")?;
    let mut input = [0u32; 16];
    input[15] = 0x1234_5678;
    let (out,) = func.call(&mut store, (input,))?;
    assert_eq!(out, 0x1234_5678);
    Ok(())
}
