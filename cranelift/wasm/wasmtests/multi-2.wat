(module
  (func (export "multiLoop") (param i64 i64) (result i64 i64)
    (local.get 1)
    (local.get 0)
    (loop (param i64 i64) (result i64 i64)
       return)))
