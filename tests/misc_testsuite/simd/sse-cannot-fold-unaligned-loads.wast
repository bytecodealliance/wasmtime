;;! simd = true

(module
  (memory 1)
  (func (export "punpckhbw") (param v128 i32) (result v128)
    local.get 0
    (v128.load offset=1 (local.get 1))
    i8x16.shuffle
      0x08 0x18 0x09 0x19
      0x0a 0x1a 0x0b 0x1b
      0x0c 0x1c 0x0d 0x1d
      0x0e 0x1e 0x0f 0x1f
    return
  )
)

(assert_return (invoke "punpckhbw" (v128.const i64x2 0 0) (i32.const 0))
  (v128.const i64x2 0 0))
