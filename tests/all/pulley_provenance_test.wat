;; This file is run as part of `pulley_provenance_test` in
;; `tests/all/pulley.rs`. This is currently split out to be precompiled outside
;; of miri and to have the compiled bytecode loaded directly into miri.
(module
  (import "" "host-wrap" (func $host-wrap (result i32 i32 i32)))
  (import "" "host-new" (func $host-new (result i32 i32 i32)))
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
)
