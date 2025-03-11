;;! gc = true

(module
  (table $t 10 (ref null none))
  (func (export "f") (result (ref null none))
    (i32.const 99)
    (table.get $t)
  )
)

(assert_trap (invoke "f") "out of bounds table access")
