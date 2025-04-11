(module
  (func (export "add") (param i32 i32) (result i32)
    (i32.add (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "add" (i32.const 1) (i32.const 2))
               (i32.const 3))
