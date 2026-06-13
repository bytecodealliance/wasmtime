#![cfg(not(miri))]

use wasmtime::Result;
use wasmtime::Store;
use wasmtime::component::{Component, Linker};

#[test]
fn fixed_length_list() -> Result<()> {
    // nested roundtrip returns the parameters in a tuple
    // this is the same component as in tests/misc_testsuite/component-model/fixed_length_lists.wast
    // as this test was compiled from C(++) the assembly is less human readable
    let component = r##"
(component
  (type $ty-test:fixed-size-lists/to-test (;0;)
    (instance
      (type (;0;) (list u32 2))
      (type (;1;) (list 0 2))
      (type (;2;) (list s32 2))
      (type (;3;) (list 2 2))
      (type (;4;) (tuple 1 3))
      (type (;5;) (func (param "a" 1) (param "b" 3) (result 4)))
      (export (;0;) "nested-roundtrip" (func (type 5)))
    )
  )
  (import "test:fixed-size-lists/to-test" (instance $test:fixed-size-lists/to-test (;0;) (type $ty-test:fixed-size-lists/to-test)))
  (core module $main (;0;)
    (type (;0;) (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32)))
    (type (;1;) (func (result i32)))
    (import "test:fixed-size-lists/to-test" "nested-roundtrip" (func (;0;) (type 0)))
    (memory (;0;) 2)
    (global (;0;) (mut i32) i32.const 67232)
    (export "memory" (memory 0))
    (export "run" (func 1))
    (func (;1;) (type 1) (result i32)
      (local i32 i32 i32 i32 i32 i32 i32 i32 i32)
      global.get 0
      i32.const -64
      i32.add
      local.tee 0
      global.set 0
      local.get 0
      i32.const 16
      i32.add
      local.tee 1
      i32.const 8
      i32.add
      i32.const 1032
      i64.load align=4
      i64.store
      local.get 0
      i32.const 8
      i32.add
      i32.const 1048
      i64.load align=4
      i64.store
      local.get 0
      i32.const 1024
      i64.load align=4
      i64.store offset=16
      local.get 0
      i32.const 1040
      i64.load align=4
      i64.store
      global.get 0
      i32.const 32
      i32.sub
      local.tee 2
      global.set 0
      local.get 1
      i32.load
      local.get 1
      i32.load offset=4
      local.get 1
      i32.load offset=8
      local.get 1
      i32.load offset=12
      local.get 0
      i32.load
      local.get 0
      i32.load offset=4
      local.get 0
      i32.load offset=8
      local.get 0
      i32.load offset=12
      local.get 2
      call 0
      local.get 0
      i32.const 32
      i32.add
      local.tee 1
      local.get 2
      i64.load offset=24
      i64.store offset=24 align=4
      local.get 1
      local.get 2
      i64.load offset=16
      i64.store offset=16 align=4
      local.get 1
      local.get 2
      i64.load offset=8
      i64.store offset=8 align=4
      local.get 1
      local.get 2
      i64.load
      i64.store align=4
      local.get 2
      i32.const 32
      i32.add
      global.set 0
      local.get 0
      i32.load offset=48
      local.set 2
      local.get 0
      i32.load offset=32
      local.get 0
      i32.load offset=36
      local.set 3
      local.get 0
      i32.load offset=52
      local.set 4
      local.get 0
      i32.load offset=40
      local.set 5
      local.get 0
      i32.load offset=56
      local.set 6
      local.get 0
      i32.load offset=44
      local.set 7
      local.get 0
      i32.load offset=60
      local.set 8
      local.get 0
      i32.const -64
      i32.sub
      global.set 0
      i32.const 1
      i32.ne
      local.get 2
      i32.const -1
      i32.ne
      i32.add
      local.get 3
      i32.const 2
      i32.ne
      i32.add
      local.get 4
      i32.const -2
      i32.ne
      i32.add
      local.get 5
      i32.const 3
      i32.ne
      i32.add
      local.get 6
      i32.const -3
      i32.ne
      i32.add
      local.get 7
      i32.const 4
      i32.ne
      i32.add
      local.get 8
      i32.const -4
      i32.ne
      i32.add
    )
    (data (;0;) (i32.const 1024) "\01\00\00\00\02\00\00\00\03\00\00\00\04\00\00\00\ff\ff\ff\ff\fe\ff\ff\ff\fd\ff\ff\ff\fc\ff\ff\ff")
    (data (;1;) (i32.const 1056) "\ff\ff\ff\ff\00\00\02")
    (@producers
      (processed-by "wit-component" "0.243.0")
    )
  )
)
"##;
    wasmtime::component::bindgen!({
        inline: "
        package root:root;

world root {
  export test:fixed-size-lists/to-test;
}
package test:fixed-size-lists {
  interface to-test {
    nested-roundtrip: func(a: list<list<u32, 2>, 2>, b: list<list<s32, 2>, 2>) -> tuple<list<list<u32, 2>, 2>, list<list<s32, 2>, 2>>;
  }
}
",
        imports: {default: trappable},
    });

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, 0);
    let mut linker = Linker::new(&engine);
    let to_test = ToTest::instantiate(&mut store, &component, &linker)?;
    let inputa: [[u32; 2]; 2] = [[1, 2], [3, 4]];
    let inputb: [[i32; 2]; 2] = [[-1, -2], [-3, -4]];
    let result = to_test
        .call_nested_roundtrip(&mut store, &inputa, &inputb)
        .expect("no trap")
        .expect("no errror returned");
    assert_eq!(result, ([[1, 2], [3, 4]], [[-1, -2], [-3, -4]]));
    Ok(())
}
