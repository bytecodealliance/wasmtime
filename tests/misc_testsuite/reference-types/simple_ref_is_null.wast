(module
  (func (export "func_is_null") (param funcref) (result i32)
    (ref.is_null func (local.get 0))
  )
  (func (export "func_is_null_with_non_null_funcref") (result i32)
    (call 0 (ref.func 0))
  )
  (func (export "extern_is_null") (param externref) (result i32)
    (ref.is_null extern (local.get 0))
  )
)

(assert_return (invoke "func_is_null" (ref.null func)) (i32.const 1))
(assert_return (invoke "func_is_null_with_non_null_funcref") (i32.const 0))

(assert_return (invoke "extern_is_null" (ref.null extern)) (i32.const 1))
(assert_return (invoke "extern_is_null" (ref.extern 1)) (i32.const 0))
