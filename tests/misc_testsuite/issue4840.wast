(module
  (func (export "f") (param f32 i32) (result f64)
    local.get 1
    f64.convert_i32_u
    i32.trunc_f64_u
    f64.convert_i32_s
    local.get 1
    f64.convert_i32_u
    global.set 0
    drop
    global.get 0
  )
  (global (;0;) (mut f64) f64.const 0)
)

(assert_return (invoke "f" (f32.const 1.23) (i32.const -2147483648)) (f64.const 2147483648))
