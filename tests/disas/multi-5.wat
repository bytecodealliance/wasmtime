(module
  (func (export "foo")
    i32.const 1
    i64.const 2
    ;; More params than results.
    (block (param i32 i64) (result i32)
      drop
    )
    drop
  )
)
