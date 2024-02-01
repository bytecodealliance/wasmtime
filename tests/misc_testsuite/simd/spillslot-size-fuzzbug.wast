(module
  (func (export "test") (result f32 f32)
    i32.const 0
    f32.convert_i32_s
    v128.const i32x4 0 0 0 0
    data.drop 0
    f32x4.extract_lane 0
    data.drop 0)
  (data ""))

(assert_return (invoke "test") (f32.const 0.0) (f32.const 0.0))
