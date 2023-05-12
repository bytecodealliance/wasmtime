(module
  (func (export "f") (param i64 i32) (result i64)
    (local.get 0)
    (local.get 1)
    ;; If with else. Same number of params and results.
    (if (param i64) (result i64)
      (then
        (drop)
        (i64.const -1))
      (else
        (drop)
        (i64.const -2)))))
