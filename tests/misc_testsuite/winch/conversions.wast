(module
  (func (export "i64.extend_i32_s") (param $x i32) (result i64) (i64.extend_i32_s (local.get $x)))
  (func (export "i64.extend_i32_u") (param $x i32) (result i64) (i64.extend_i32_u (local.get $x)))
  (func (export "i32.wrap_i64") (param $x i64) (result i32) (i32.wrap_i64 (local.get $x)))
)

(assert_return (invoke "i64.extend_i32_s" (i32.const 0)) (i64.const 0))
(assert_return (invoke "i64.extend_i32_s" (i32.const 10000)) (i64.const 10000))
(assert_return (invoke "i64.extend_i32_s" (i32.const -10000)) (i64.const -10000))
(assert_return (invoke "i64.extend_i32_s" (i32.const -1)) (i64.const -1))
(assert_return (invoke "i64.extend_i32_s" (i32.const 0x7fffffff)) (i64.const 0x000000007fffffff))
(assert_return (invoke "i64.extend_i32_s" (i32.const 0x80000000)) (i64.const 0xffffffff80000000))

(assert_return (invoke "i64.extend_i32_u" (i32.const 0)) (i64.const 0))
(assert_return (invoke "i64.extend_i32_u" (i32.const 10000)) (i64.const 10000))
(assert_return (invoke "i64.extend_i32_u" (i32.const -10000)) (i64.const 0x00000000ffffd8f0))
(assert_return (invoke "i64.extend_i32_u" (i32.const -1)) (i64.const 0xffffffff))
(assert_return (invoke "i64.extend_i32_u" (i32.const 0x7fffffff)) (i64.const 0x000000007fffffff))
(assert_return (invoke "i64.extend_i32_u" (i32.const 0x80000000)) (i64.const 0x0000000080000000))

(assert_return (invoke "i32.wrap_i64" (i64.const -1)) (i32.const -1))
(assert_return (invoke "i32.wrap_i64" (i64.const -100000)) (i32.const -100000))
(assert_return (invoke "i32.wrap_i64" (i64.const 0x80000000)) (i32.const 0x80000000))
(assert_return (invoke "i32.wrap_i64" (i64.const 0xffffffff7fffffff)) (i32.const 0x7fffffff))
(assert_return (invoke "i32.wrap_i64" (i64.const 0xffffffff00000000)) (i32.const 0x00000000))
(assert_return (invoke "i32.wrap_i64" (i64.const 0xfffffffeffffffff)) (i32.const 0xffffffff))
(assert_return (invoke "i32.wrap_i64" (i64.const 0xffffffff00000001)) (i32.const 0x00000001))
(assert_return (invoke "i32.wrap_i64" (i64.const 0)) (i32.const 0))
(assert_return (invoke "i32.wrap_i64" (i64.const 1311768467463790320)) (i32.const 0x9abcdef0))
(assert_return (invoke "i32.wrap_i64" (i64.const 0x00000000ffffffff)) (i32.const 0xffffffff))
(assert_return (invoke "i32.wrap_i64" (i64.const 0x0000000100000000)) (i32.const 0x00000000))
(assert_return (invoke "i32.wrap_i64" (i64.const 0x0000000100000001)) (i32.const 0x00000001))

;; Type check

(assert_invalid (module (func (result i32) (i32.wrap_i64 (f32.const 0)))) "type mismatch")
(assert_invalid (module (func (result i64) (i64.extend_i32_s (f32.const 0)))) "type mismatch")
(assert_invalid (module (func (result i64) (i64.extend_i32_u (f32.const 0)))) "type mismatch")
