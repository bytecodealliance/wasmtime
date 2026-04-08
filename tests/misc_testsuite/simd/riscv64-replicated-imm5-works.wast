;;! simd = true

(module
  (memory (export "mem") 1)
  (func (export "add_and_extract") (param v128) (result i32)
    (i32x4.extract_lane 0
      (i32x4.add
        (local.get 0)
        (i32x4.splat (i32.const 0x01010101))))))

(assert_return (invoke "add_and_extract" (v128.const i32x4 0 0 0 0))
  (i32.const 0x01010101))
