;;! simd = true

(module
  (func (export "e1") (param v128 v128) (result i32)
    local.get 0
    local.get 1
    i8x16.eq
    i8x16.all_true)

  (func (export "e2") (param v128 v128) (result i32)
    local.get 0
    local.get 1
    i8x16.ne
    v128.any_true
    i32.eqz)

  (func (export "band") (param v128 v128) (result i32)
    local.get 0
    local.get 1
    v128.and
    v128.any_true)

  (func (export "band_not") (param v128 v128) (result i32)
    local.get 0
    local.get 1
    v128.not
    v128.and
    v128.any_true)
)

(assert_return (invoke "e1" (v128.const i32x4 0 0 0 0) (v128.const i32x4 0 0 0 0)) (i32.const 1))
(assert_return (invoke "e1" (v128.const i32x4 0 0 0 0) (v128.const i32x4 0 0 0 1)) (i32.const 0))

(assert_return (invoke "e2" (v128.const i32x4 0 0 0 0) (v128.const i32x4 0 0 0 0)) (i32.const 1))
(assert_return (invoke "e2" (v128.const i32x4 0 0 0 0) (v128.const i32x4 0 0 0 1)) (i32.const 0))

(assert_return (invoke "band" (v128.const i32x4 1 0 0 0) (v128.const i32x4 2 0 0 0)) (i32.const 0))
(assert_return (invoke "band" (v128.const i32x4 1 0 0 0) (v128.const i32x4 1 0 0 0)) (i32.const 1))
(assert_return (invoke "band_not" (v128.const i32x4 1 0 0 0) (v128.const i32x4 1 0 0 0)) (i32.const 0))
(assert_return (invoke "band_not" (v128.const i32x4 1 0 0 0) (v128.const i32x4 0 0 0 0)) (i32.const 1))
