;;! simd = true

(module
  (func (result v128)
    i32.const 0
    v128.load32_splat align=1
    f64x2.convert_low_i32x4_u
  )
  (memory 0 1)
)
