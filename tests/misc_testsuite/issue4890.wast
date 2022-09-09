(module
  (func (param i32) (result f32)
    f32.const 0
    local.get 0
    f32.load offset=1
    f32.copysign
  )
  (memory 1)
  (export "f" (func 0))
)

(assert_return (invoke "f" (i32.const 0)) (f32.const 0))
