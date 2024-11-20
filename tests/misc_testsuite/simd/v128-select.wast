;;! simd = true

(module
  (func (export "select") (param v128 v128 i32) (result v128)
    local.get 0
    local.get 1
    local.get 2
    select)
)

(assert_return (invoke "select"
                       (v128.const i64x2 1 1)
                       (v128.const i64x2 2 2)
                       (i32.const 0))
               (v128.const i64x2 2 2))

(assert_return (invoke "select"
                       (v128.const i64x2 1 1)
                       (v128.const i64x2 2 2)
                       (i32.const 1))
               (v128.const i64x2 1 1))
