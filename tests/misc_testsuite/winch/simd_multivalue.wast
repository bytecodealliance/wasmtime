;;! simd = true

;; test that swapping the parameters results in swapped return values
(module (func (export "f") (param v128) (param v128) (result v128) (result v128) (local.get 1) (local.get 0)))
(assert_return (invoke "f" (v128.const i64x2 2 1) (v128.const i64x2 1 2)) (v128.const i64x2 1 2) (v128.const i64x2 2 1))

;; test 0 consts
(module (func (export "consts") (result v128) (result v128) (v128.const i64x2 0 0) (v128.const i64x2 0 0)))
(assert_return (invoke "consts") (v128.const i64x2 0 0) (v128.const i64x2 0 0))

;; test case where vector is neither the first return value nor the last return value
(module
  (func (export "not-first-nor-last") (param v128) (result i64 v128 i64)
    i64.const 0
    local.get 0
    i64.const 0
  )
)
(assert_return (invoke "not-first-nor-last" (v128.const i32x4 1 2 3 4)) (i64.const 0) (v128.const i32x4 1 2 3 4) (i64.const 0))
