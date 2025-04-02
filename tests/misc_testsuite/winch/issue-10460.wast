;;! simd = true
(module
  (func (export "x")
    (param f32 f32 f32 f32 f32 f32 f32 f32 f32) (param $last v128)
    (result v128)
    local.get $last
  )
)

(assert_return
  (invoke "x"
    (f32.const 1)
    (f32.const 2)
    (f32.const 3)
    (f32.const 4)
    (f32.const 5)
    (f32.const 6)
    (f32.const 7)
    (f32.const 8)
    (f32.const 9)
    (v128.const i64x2 10 11)
  )
  (v128.const i64x2 10 11))
