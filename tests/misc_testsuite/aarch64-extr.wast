(module
  (memory 1)
  (func (export "a64-extr") (param i32 i32) (result i32)
    (i32.or
      (i32.shl (local.get 0) (i32.const 32))
      (i32.shr_u (local.get 1) (i32.const 0))
    )
  )
)

(assert_return (invoke "a64-extr" (i32.const 65536) (i32.const 0)) (i32.const 65536))
