(module
  (func (export "x") (result i32)
    i32.const 0
    i32.ctz
  )
)

(assert_return (invoke "x") (i32.const 32))
