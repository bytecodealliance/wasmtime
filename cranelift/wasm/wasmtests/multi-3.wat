(module
  (func (export "multiIf") (param i32 i64 i64) (result i64 i64)
    (local.get 2)
    (local.get 1)
    (local.get 0)
    (if (param i64 i64) (result i64 i64)
      (then return)
      ;; Hits the code path for an `else` after a block that ends unreachable.
      (else
        (drop)
        (drop)
        (i64.const 0)
        (i64.const 0)))))
