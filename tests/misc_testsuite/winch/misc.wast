;; Additional run tests for Winch not covered in the official spec test suite.

(module
  (func (export "br-table-ensure-sp") (result i32)
    (block (result i32)
       (i32.const 0)
    )
    (i32.const 0)
    (i32.const 0)
    (br_table 0)
  )
)

(assert_return (invoke "br-table-ensure-sp") (i32.const 0))
