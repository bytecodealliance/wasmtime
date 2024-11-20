;;! simd = true

(module
  (func (param i32) (result v128)
    local.get 0
    i32x4.splat
    f64x2.convert_low_i32x4_u
  )
)

(module
  (func (result v128)
    i32.const 0
    i32x4.splat
    f64x2.convert_low_i32x4_u
  )
)
