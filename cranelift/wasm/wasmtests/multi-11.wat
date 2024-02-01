(module
  (func (export "multiLoop") (param i64) (result i64 i64)
    (local.get 0)
    ;; Fewer params than results.
    (loop (param i64) (result i64 i64)
      i64.const 42
      return)))
