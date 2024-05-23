(module
  (func (result i32 i32 i32)
    i32.const 1
    i32.eqz
    f64.const 0
    f64.const 1
    f64.ne
    i32.const 1111
  )
  (export "d" (func 0))
)

(assert_return (invoke "d") (i32.const 0) (i32.const 1) (i32.const 1111))
