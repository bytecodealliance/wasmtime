(module
  (func (export "multiIf2") (param i32 i64 i64) (result i64 i64)
    (local.get 2)
    (local.get 1)
    (local.get 0)
    (if (param i64 i64) (result i64 i64)
      (then
        i64.add
        i64.const 1)
      ;; Hits the code path for an `else` after a block that does not end unreachable.
      (else
        i64.sub
        i64.const 2))))
