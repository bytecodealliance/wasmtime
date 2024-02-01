(module
  (func (export "f") (param i64 i32) (result i64)
    (local.get 0)
    (local.get 1)
    (local.get 1)
    ;; If with else. More params than results.
    (if (param i64 i32) (result i64)
      (then
        (drop)
        (drop)
        (i64.const -1))
      (else
        (drop)
        (drop)
        (i64.const -2)))))
