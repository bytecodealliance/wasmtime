(module
  (func (export "foo")
    i32.const 1
    ;; Fewer params than results.
    (block (param i32) (result i32 i64)
      i64.const 2
    )
    drop
    drop
  )
)
