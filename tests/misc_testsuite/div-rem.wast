(module
  (func (export "i32.div_s") (param i32) (param i32) (result i32)
    (i32.div_s (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "i32.div_s" (i32.const -1) (i32.const -1)) (i32.const 1))

(module
  (func (export "i32.rem_s") (param i32) (param i32) (result i32)
    (i32.rem_s (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "i32.rem_s" (i32.const 123121) (i32.const -1)) (i32.const 0))

(module
  (func (export "i64.div_s") (param i64) (param i64) (result i64)
    (i64.div_s (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "i64.div_s" (i64.const -1) (i64.const -1)) (i64.const 1))

(module
  (func (export "i64.rem_s") (param i64) (param i64) (result i64)
    (i64.rem_s (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "i64.rem_s" (i64.const 123121) (i64.const -1)) (i64.const 0))
