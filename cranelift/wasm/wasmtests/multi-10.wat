(module
  (func (export "f") (param i64 i32) (result i64 i64)
    (local.get 0)
    (local.get 1)
    ;; If with else. Fewer params than results.
    (if (param i64) (result i64 i64)
      (then
        (i64.const -1))
      (else
        (i64.const -2)))))
