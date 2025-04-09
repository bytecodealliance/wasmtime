;;! simd = true

(module
  (memory 0 0)
  (func (export "test") (result f32)
    i32.const 0
    if
      unreachable
    end
    i32.const 0
    v128.const i64x2 0 0
    v128.load64_lane align=1 0
    drop
    f32.const 0
  )
)

(assert_trap (invoke "test") "out of bounds memory access")
