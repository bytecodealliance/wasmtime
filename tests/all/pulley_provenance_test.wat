;; This file is run as part of `pulley_provenance_test` in
;; `tests/all/pulley.rs`. This is currently split out to be precompiled outside
;; of miri and to have the compiled bytecode loaded directly into miri.
(module
  (import "" "host-wrap" (func $host-wrap (result i32 i32 i32)))
  (import "" "host-new" (func $host-new (result i32 i32 i32)))

  (table 1 funcref)
  (elem (i32.const 0) func $some-wasm-func)

  (type $ret-triple (func (result i32 i32 i32)))

  (func $some-wasm-func (result i32 i32 i32)
    i32.const 1
    i32.const 2
    i32.const 3
  )
  (func (export "call-wasm") (result i32 i32 i32)
    call $some-wasm-func
  )
  (func (export "call-native-wrap") (result i32 i32 i32)
    call $host-wrap
  )
  (func (export "call-native-new") (result i32 i32 i32)
    call $host-new
  )
  (func (export "return-call-wasm") (result i32 i32 i32)
    return_call $some-wasm-func
  )
  (func (export "call_indirect-wasm") (result i32 i32 i32)
    i32.const 0
    call_indirect (result i32 i32 i32)
  )
  (func (export "return_call_indirect-wasm") (result i32 i32 i32)
    i32.const 0
    return_call_indirect (result i32 i32 i32)
  )
  (func (export "call_ref-wasm") (param (ref $ret-triple)) (result i32 i32 i32)
    local.get 0
    call_ref $ret-triple
  )
  (func (export "return_call_ref-wasm") (param (ref $ret-triple)) (result i32 i32 i32)
    local.get 0
    return_call_ref $ret-triple
  )

  (func (export "unreachable") unreachable)
  (func (export "divide-by-zero") (result i32)
    i32.const 100
    i32.const 0
    i32.div_s)

  (memory 1)
  (func (export "memory-intrinsics")
    (drop (i32.load (i32.const 0)))
    (i32.store (i32.const 0) (i32.const 0))
    (drop (memory.grow (i32.const 1)))
    (drop (i32.load (i32.const 0)))
    (i32.store (i32.const 0) (i32.const 0))
    (drop (memory.size))

    (memory.copy (i32.const 0) (i32.const 1) (i32.const 10))
    (memory.init $d (i32.const 0) (i32.const 1) (i32.const 3))
    (memory.fill (i32.const 0) (i32.const 10) (i32.const 10))

    (data.drop $d)
  )
  (data $d "abcd")

  (table 1 funcref)
  (func (export "table-intrinsics")
    (drop (table.get (i32.const 0)))
    (table.set (i32.const 0) (table.get (i32.const 0)))

    (drop (table.grow (ref.null func) (i32.const 100)))

    (drop (table.get (i32.const 1)))
    (table.set (i32.const 1) (table.get (i32.const 1)))

    (table.copy (i32.const 0) (i32.const 1) (i32.const 10))
    (table.init $e (i32.const 0) (i32.const 1) (i32.const 3))
    (table.fill (i32.const 0) (ref.func $empty) (i32.const 10))
  )
  (elem $e func $empty $empty $empty $empty)
  (func $empty)
)
