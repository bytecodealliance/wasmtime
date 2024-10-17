(module
  (func (export "") (result v128)
    v128.const i32x4 0x3f803f80 0x3f803f80 0x3f803f80 0x3f803f80
  )
)

(assert_return (invoke "") (v128.const i32x4 0x3f803f80 0x3f803f80 0x3f803f80 0x3f803f80))
