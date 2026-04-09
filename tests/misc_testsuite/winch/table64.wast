;;! memory64 = true
;;! reference_types = true

(module
  (table $t i64 10 funcref)
  (func $leak (result i64 i64)
    table.size $t
    table.size $t
  )
  (func (export "test") (result i64)
    call $leak
    drop
  )
)
(assert_return (invoke "test") (i64.const 10))
