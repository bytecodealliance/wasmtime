;;! simd = true
;;! flags = "-C cranelift-separate-constant-pool"

(module
  (func $v128 (export "v128") (result v128)
    v128.const i64x2 0x1122334455667788 0x99aabbccddeeff00)
  (func (export "two") (result v128 v128)
    v128.const i32x4 1 2 3 4
    v128.const i32x4 5 6 7 8)
  (func (export "reused") (result v128 v128 v128)
    v128.const i32x4 1 2 3 4
    v128.const i32x4 9 10 11 12
    v128.const i32x4 1 2 3 4)
  (func (export "call") (result v128 v128)
    v128.const i32x4 13 14 15 16
    call $v128)
  (func (export "i64") (result i64)
    i64.const 0xf34bf0a31212003a)
  (func (export "f64") (result f64)
    f64.const 0x1.23456789abcdep0)
)

(assert_return (invoke "v128")
  (v128.const i64x2 0x1122334455667788 0x99aabbccddeeff00))
(assert_return (invoke "two")
  (v128.const i32x4 1 2 3 4)
  (v128.const i32x4 5 6 7 8))
(assert_return (invoke "reused")
  (v128.const i32x4 1 2 3 4)
  (v128.const i32x4 9 10 11 12)
  (v128.const i32x4 1 2 3 4))
(assert_return (invoke "call")
  (v128.const i32x4 13 14 15 16)
  (v128.const i64x2 0x1122334455667788 0x99aabbccddeeff00))
(assert_return (invoke "i64") (i64.const 0xf34bf0a31212003a))
(assert_return (invoke "f64") (f64.const 0x1.23456789abcdep0))
