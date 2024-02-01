(module
  (func (export "multiBlock") (param i64 i32) (result i32 i64 f64)
    (local.get 1)
    (local.get 0)
    (block (param i32 i64) (result i32 i64 f64)
      (f64.const 1234.5))))
