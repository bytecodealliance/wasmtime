(module
  (func (export "mul16") (param i32) (result i32)
    local.get 0
    i32.const -7937
    i32.mul
    i32.extend16_s
  )
)

(assert_return (invoke "mul16" (i32.const 100)) (i32.const -7268))
