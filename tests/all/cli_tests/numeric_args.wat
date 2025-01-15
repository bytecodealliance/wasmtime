(module
  (func $sum_i32 (export "sum_i32") (param i32 i32) (result i32)
    (i32.add
      (local.get 0)
      (local.get 1)))

  (func $sum_i64 (export "sum_i64") (param i64 i64) (result i64)
    (i64.add
      (local.get 0)
      (local.get 1)))

  (func $sum_f32 (export "sum_f32") (param f32 f32) (result f32)
    (f32.add
      (local.get 0)
      (local.get 1)))

  (func $sum_f64 (export "sum_f64") (param f64 f64) (result f64)
    (f64.add
      (local.get 0)
      (local.get 1))) 
