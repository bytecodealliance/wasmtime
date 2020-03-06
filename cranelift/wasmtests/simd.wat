(module
  (func $test_splat (result i32)
    i32.const 42
    i32x4.splat
    i32x4.extract_lane 0
  )

  (func $test_insert_lane (result i32)
      v128.const i64x2 0 0
      i32.const 99
      i32x4.replace_lane 1
      i32x4.extract_lane 1
  )

  (func $test_const (result i32)
    v128.const i32x4 1 2 3 4
    i32x4.extract_lane 3
  )

  (export "test_splat" (func $test_splat))
  (export "test_insert_lane" (func $test_insert_lane))
  (export "test_const" (func $test_const))
)
